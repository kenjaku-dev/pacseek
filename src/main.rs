use std::io::IsTerminal;

use clap::Parser;
use futures::future::join;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::task;
use tracing_subscriber::{EnvFilter, fmt};

use pacseek::cli::{Cli, Source};
use pacseek::output::print_packages;
use pacseek::search::{search_aur, search_repo, search_repo_fallback};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

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
        // Respect NO_COLOR for TUI
        if cli.no_color || std::env::var("NO_COLOR").is_ok() {
            // ratatui will handle via style, but we set flag
        }
        if !is_tty {
            eprintln!("Error: --tui requires a terminal (TTY).");
            eprintln!("Try: pacseek <query> --no-tui   or   pacseek --help");
            eprintln!("Tip: In headless/CI, use --json or --no-tui");
            std::process::exit(1);
        }
        let initial = cli.query.clone().unwrap_or_default();
        // TUI owns terminal lifecycle — color_eyre installed inside tui::run per ratatui skill
        // Need to drop tracing subscriber that may write to stdout? Keep it quiet for TUI
        return pacseek::tui::run(initial).map_err(|e| anyhow::anyhow!("{e}"));
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

    if cli.no_color {
        colored::control::set_override(false);
    }

    let client = reqwest::Client::builder()
        .user_agent(format!("pacseek/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
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
            task::spawn_blocking(move || {
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
            .await
            .unwrap_or_default()
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
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Searching for '{}'...", query));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let (repo_packages, aur_packages) = join(repo_future, aur_future).await;

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
