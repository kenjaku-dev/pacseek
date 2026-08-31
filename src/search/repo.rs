use std::sync::OnceLock;

use alpm::{Alpm, SigLevel};
use pacmanconf::Config;

use crate::model::Package;

static CACHED_CONFIG: OnceLock<Config> = OnceLock::new();

/// Get cached pacman config (parses pacman.conf once per process)
/// Falls back to fresh parse if cache not yet initialized or on error
pub fn get_cached_config() -> anyhow::Result<Config> {
    if let Some(cfg) = CACHED_CONFIG.get() {
        return Ok(cfg.clone());
    }
    let cfg = Config::new().map_err(|e| anyhow::anyhow!("pacmanconf failed: {e:?}"))?;
    let _ = CACHED_CONFIG.set(cfg.clone());
    Ok(cfg)
}

pub fn search_repo_with_config(
    config: &Config,
    query: &str,
    limit: usize,
    use_regex: bool,
    installed_only: bool,
) -> anyhow::Result<Vec<Package>> {
    let db_path = if config.db_path.is_empty() {
        "/var/lib/pacman".to_string()
    } else {
        config.db_path.clone()
    };
    let root_dir = if config.root_dir.is_empty() {
        "/".to_string()
    } else {
        config.root_dir.clone()
    };

    tracing::debug!(db_path=%db_path, root_dir=%root_dir, repos=?config.repos.iter().map(|r| &r.name).collect::<Vec<_>>(), "init alpm (cached)");

    let handle = Alpm::new(root_dir.as_str(), db_path.as_str())
        .map_err(|e| anyhow::anyhow!("alpm init failed: {e:?} (db_path={db_path})"))?;

    for repo in &config.repos {
        if let Err(e) = handle.register_syncdb(repo.name.as_str(), SigLevel::USE_DEFAULT) {
            tracing::warn!(repo=%repo.name, err=?e, "failed to register syncdb, skipping");
        }
    }

    let syncdbs = handle.syncdbs();
    if syncdbs.is_empty() {
        tracing::warn!("no syncdbs registered — is pacman DB synced? Run `sudo pacman -Sy`");
    }

    let localdb = handle.localdb();
    let mut results: Vec<Package> = Vec::new();
    let query_lower = query.to_lowercase();
    let re = if use_regex {
        Some(
            regex::RegexBuilder::new(query)
                .case_insensitive(true)
                .build()?,
        )
    } else {
        None
    };

    for db in syncdbs.iter() {
        let db_name = db.name().to_string();
        if use_regex {
            let list = match db.search([query].iter()) {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!(db=%db_name, err=?e, "search failed");
                    continue;
                }
            };
            for pkg in list.iter() {
                let is_installed = localdb.pkg(pkg.name()).is_ok();
                if installed_only && !is_installed {
                    continue;
                }
                if let Some(ref rx) = re {
                    let desc = pkg.desc().unwrap_or("");
                    if !rx.is_match(pkg.name()) && !rx.is_match(desc) {
                        continue;
                    }
                }
                let version = pkg.version().to_string();
                let desc_owned = pkg.desc().map(|s| s.to_string());
                let arch = pkg.arch().map(|a| a.to_string());
                let url = pkg.url().map(|u| u.to_string());
                results.push(Package {
                    name: pkg.name().to_string(),
                    version,
                    description: desc_owned,
                    repo: db_name.clone(),
                    arch,
                    url,
                    installed: is_installed,
                    votes: None,
                    popularity: None,
                    out_of_date: None,
                    maintainer: None,
                    num_votes: None,
                    last_modified: None,
                });
                if limit != 0 && results.len() >= limit {
                    break;
                }
            }
        } else {
            for pkg in db.pkgs().iter() {
                let name = pkg.name().to_lowercase();
                let desc_lower = pkg.desc().map(|d| d.to_lowercase()).unwrap_or_default();
                if !name.contains(&query_lower) && !desc_lower.contains(&query_lower) {
                    continue;
                }
                let is_installed = localdb.pkg(pkg.name()).is_ok();
                if installed_only && !is_installed {
                    continue;
                }
                let version = pkg.version().to_string();
                let desc_owned = pkg.desc().map(|s| s.to_string());
                let arch = pkg.arch().map(|a| a.to_string());
                let url = pkg.url().map(|u| u.to_string());
                results.push(Package {
                    name: pkg.name().to_string(),
                    version,
                    description: desc_owned,
                    repo: db_name.clone(),
                    arch,
                    url,
                    installed: is_installed,
                    votes: None,
                    popularity: None,
                    out_of_date: None,
                    maintainer: None,
                    num_votes: None,
                    last_modified: None,
                });
                if limit != 0 && results.len() >= limit {
                    break;
                }
            }
        }
        if limit != 0 && results.len() >= limit {
            break;
        }
    }

    results.sort_by(|a, b| a.repo.cmp(&b.repo).then(a.name.cmp(&b.name)));
    if limit != 0 && results.len() > limit {
        results.truncate(limit);
    }
    Ok(results)
}

pub fn search_repo(
    query: &str,
    limit: usize,
    use_regex: bool,
    installed_only: bool,
) -> anyhow::Result<Vec<Package>> {
    // Use cached config for performance — falls back to fresh parse if cache miss
    let config = get_cached_config()
        .or_else(|_| Config::new().map_err(|e| anyhow::anyhow!("pacmanconf failed: {e:?}")))?;
    search_repo_with_config(&config, query, limit, use_regex, installed_only)
}

// Fallback when alpm fails — parses `pacman -Ss` output ( Butter fallback )
// Handles wrapped descriptions (multiple lines) and distinguishes no-results vs error
pub fn search_repo_fallback(query: &str, limit: usize) -> anyhow::Result<Vec<Package>> {
    use std::process::Command;
    let output = Command::new("pacman")
        .args(["-Ss", query])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run pacman -Ss: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // pacman -Ss returns 1 when no results, but also on error; check stderr for real errors
        if stderr.contains("error:") || stderr.contains("failed") {
            tracing::warn!(stderr=%stderr, "pacman -Ss error");
            // Still return empty for TUI, but log
        }
        // No results or error -> empty
        return Ok(vec![]);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    let mut lines = stdout.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }
        // Package header always contains '/' and is not indented (desc lines are indented with 4 spaces)
        // But also check: if line starts with whitespace, it's continuation of previous desc, not header
        if line.starts_with(' ') || line.starts_with('\t') || !line.contains('/') {
            continue;
        }
        let mut parts = line.splitn(2, ' ');
        let repo_name = parts.next().unwrap_or("");
        let (repo, name) = repo_name.split_once('/').unwrap_or(("", repo_name));
        let version = parts
            .next()
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        let installed = line.contains("[installed");
        // Collect all following indented lines as description (may be wrapped across multiple lines)
        let mut desc_parts: Vec<String> = Vec::new();
        while let Some(peek) = lines.peek() {
            if peek.contains('/') && !peek.starts_with(' ') && !peek.starts_with('\t') {
                // Next package header
                break;
            }
            if peek.trim().is_empty() {
                lines.next();
                continue;
            }
            // It's a description line (indented)
            desc_parts.push(peek.trim().to_string());
            lines.next();
        }
        let desc = if desc_parts.is_empty() {
            None
        } else {
            Some(desc_parts.join(" "))
        };
        results.push(Package {
            name: name.to_string(),
            version,
            description: desc,
            repo: repo.to_string(),
            arch: None,
            url: None,
            installed,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
        });
        if limit != 0 && results.len() >= limit {
            break;
        }
    }
    Ok(results)
}
