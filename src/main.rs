use std::io::IsTerminal;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::task;
use tracing_subscriber::{EnvFilter, fmt};

use pacseek::cli::{Cli, Source};
use pacseek::config::Config;
use pacseek::output::print_packages;
use pacseek::search::{search_aur, search_repo, search_repo_fallback};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    // --init-config / --show-config early exit (no TUI, no search)
    if cli.init_config {
        match Config::init_example() {
            Ok(p) => {
                println!("Created {}", p.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("init-config: {}", e);
                std::process::exit(1);
            }
        }
    }
    if cli.show_config {
        if let Some(p) = cli.config.clone().or_else(Config::default_path) {
            println!("{}", p.display());
        } else {
            println!("(no config dir)");
        }
        if let Some(p) = Config::project_path() {
            println!("project: {}", p.display());
        }
        return Ok(());
    }

    // Load config: --config PATH > project-local > XDG
    let cfg = if let Some(ref p) = cli.config {
        Config::load_from(p).unwrap_or_default()
    } else {
        Config::load()
    };

    // Merge config into cli where cli is still at default (so config becomes default)
    // This keeps CLI as override — user intent preserved per precedence: defaults < config < CLI
    if cli.limit == 50 && cfg.search.limit != 50 {
        cli.limit = cfg.search.limit;
    }
    if cli.source == Source::All && cfg.effective_source() != Source::All {
        cli.source = cfg.effective_source();
    }
    if cli.by == pacseek::cli::AurBy::NameDesc
        && cfg.effective_aur_by() != pacseek::cli::AurBy::NameDesc
    {
        cli.by = cfg.effective_aur_by();
    }
    if !cli.regex && cfg.search.regex {
        cli.regex = true;
    }
    if !cli.installed_only && cfg.search.installed_only {
        cli.installed_only = true;
    }
    if !cli.bottom_up && (cfg.search.bottom_up || cfg.behavior.bottom_up) {
        cli.bottom_up = true;
    }
    if !cli.no_color && cfg.theme.no_color {
        cli.no_color = true;
    }

    // Determine TUI mode per tui-design lifecycle + cli-basics
    // --tui forces TUI, --no-tui forces plain, --json forces plain, otherwise auto
    let is_tty = std::io::stdout().is_terminal() && std::io::stdin().is_terminal();
    let should_tui = if cli.no_tui || cli.json {
        false
    } else if cli.tui {
        true
    } else if cli.query.is_none() {
        // No query and TTY -> TUI (your drawing: term window with search bar)
        is_tty
    } else {
        false
    };

    if should_tui {
        if !is_tty {
            eprintln!("Error: --tui requires a terminal (TTY).");
            eprintln!("Try: pacseek <query> --no-tui   or   pacseek --help");
            eprintln!("Tip: In headless/CI, use --json or --no-tui");
            std::process::exit(1);
        }
        let initial = cli.query.clone().unwrap_or_default();
        // TUI owns terminal lifecycle — color_eyre installed inside tui::run per ratatui skill
        return pacseek::tui::run_with_config(initial, &cli, &cfg)
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    // --- CLI one-shot mode (existing) ---
    // Need query now
    let query = match cli.query {
        Some(q) if !q.trim().is_empty() => q,
        _ => {
            eprintln!("Usage: pacseek <query>  or  pacseek --tui");
            eprintln!("Try 'pacseek --help' for more information.");
            std::process::exit(2);
        }
    };

    // Tracing init (only for CLI, not TUI which logs to file per skill)
    let filter = match cli.verbose {
        0 => EnvFilter::new("warn"),
        1 => EnvFilter::new("info"),
        2 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();

    if cli.no_color || std::env::var("NO_COLOR").is_ok() {
        colored::control::set_override(false);
    }

    let timeout = std::time::Duration::from_secs(cfg.search.timeout_secs.max(1));
    let client = reqwest::Client::builder()
        .user_agent(format!("pacseek/{}", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .build()?;

    let query_repo = query.clone();
    let query_aur = query.clone();
    let limit_repo = cli.limit;
    let limit_aur = cli.limit;
    let regex_repo = cli.regex;
    let regex_aur = cli.regex;
    let installed_only = cli.installed_only;
    let by_str = cli.by.as_str().to_string();

    let repo_future = async {
        if matches!(cli.source, Source::Aur) {
            Vec::new()
        } else {
            let res = task::spawn_blocking(move || {
                match search_repo(&query_repo, limit_repo, regex_repo, installed_only) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(err=?e, "alpm search failed, falling back to pacman -Ss");
                        match search_repo_fallback(&query_repo, limit_repo) {
                            Ok(v) => v,
                            Err(e2) => {
                                tracing::error!(err=?e2, "fallback also failed");
                                vec![]
                            }
                        }
                    }
                }
            })
            .await;
            match res {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(err=?e, "spawn_blocking JoinError (repo search panicked)");
                    vec![]
                }
            }
        }
    };

    let aur_enabled = !matches!(cli.source, Source::Repo) && !cli.installed_only;
    if cli.installed_only
        && matches!(cli.source, Source::Aur | Source::All)
        && !matches!(cli.source, Source::Repo)
        && !cli.json
    {
        eprintln!("Note: --installed-only only applies to repo packages, AUR results hidden");
    }

    let aur_future = async {
        if !aur_enabled {
            Vec::new()
        } else {
            match search_aur(&client, &query_aur, &by_str, limit_aur, regex_aur).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(err=?e, "AUR search failed");
                    if cli.json {
                        eprintln!("AUR error: {}", e);
                    } else {
                        eprintln!("AUR search error: {}", e);
                    }
                    Vec::new()
                }
            }
        }
    };

    let spinner = if !cli.json && cli.verbose == 0 {
        let pb = ProgressBar::new_spinner();
        let tick = cfg
            .tui
            .tick_chars
            .clone()
            .unwrap_or_else(|| "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ".into());
        let tmpl = cfg
            .tui
            .spinner_template
            .clone()
            .unwrap_or_else(|| "{spinner:.cyan} {msg}".into());
        let style = ProgressStyle::default_spinner()
            .tick_chars(&tick)
            .template(&tmpl)
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
        pb.set_style(style);
        pb.set_message(format!("Searching for '{}'...", query));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let (repo_packages, aur_packages) = tokio::join!(repo_future, aur_future);

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    print_packages(
        &repo_packages,
        &aur_packages,
        cli.source,
        cli.bottom_up,
        cli.json,
        cli.no_color,
    )?;

    Ok(())
}
