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

    // Tracing init
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

    // Spawn repo search on blocking thread (alpm is sync & not Send easily due to *mut)
    let query_repo = cli.query.clone();
    let limit_repo = cli.limit;
    let regex_repo = cli.regex;
    let installed_only = cli.installed_only;

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

    // Handle installed_only: AUR has no reliable installed state without tracking, so warn and skip AUR
    let aur_enabled = !matches!(cli.source, Source::Repo) && !cli.installed_only;
    if cli.installed_only
        && matches!(cli.source, Source::Aur | Source::All)
        && !matches!(cli.source, Source::Repo)
        && !cli.json
    {
        eprintln!("Note: --installed-only only applies to repo packages, AUR results hidden");
    }

    let query_aur = cli.query.clone();
    let by_str = cli.by.as_str().to_string();
    let limit_aur = cli.limit;
    let regex_aur = cli.regex;

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

    // Spinner for butter UX (only if not json and not verbose)
    let spinner = if !cli.json && cli.verbose == 0 {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Searching for '{}'...", cli.query));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let (repo_packages, aur_packages) = join(repo_future, aur_future).await;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // If both failed and we have zero results, exit 1
    if repo_packages.is_empty() && aur_packages.is_empty() {
        // Still print "No packages found" via output, but exit 1 if not json? Keep 0 for scriptability
        // We'll exit 0 but show message
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
