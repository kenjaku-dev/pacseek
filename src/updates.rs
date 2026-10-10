use serde::{Deserialize, Serialize};

/// One available update: installed `old_version` -> sync/AUR `new_version`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PkgUpdate {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    pub repo: String,
    pub out_of_date: bool,
}

/// Repo updates via libalpm: localdb vs syncdbs with `alpm::vercmp`.
/// Graceful: missing syncdbs (never `Sy`) yields empty, never an error.
pub fn check_repo_updates() -> anyhow::Result<Vec<PkgUpdate>> {
    let cfg = crate::search::repo::get_cached_config().or_else(|_| {
        pacmanconf::Config::new().map_err(|e| anyhow::anyhow!("pacmanconf failed: {e:?}"))
    })?;
    check_repo_updates_with_config(&cfg)
}

pub fn check_repo_updates_with_config(cfg: &pacmanconf::Config) -> anyhow::Result<Vec<PkgUpdate>> {
    let db_path = if cfg.db_path.is_empty() {
        "/var/lib/pacman".to_string()
    } else {
        cfg.db_path.clone()
    };
    let root_dir = if cfg.root_dir.is_empty() {
        "/".to_string()
    } else {
        cfg.root_dir.clone()
    };
    let handle = alpm::Alpm::new(root_dir.as_str(), db_path.as_str())
        .map_err(|e| anyhow::anyhow!("alpm init failed: {e:?}"))?;
    for repo in &cfg.repos {
        let _ = handle.register_syncdb(repo.name.as_str(), alpm::SigLevel::USE_DEFAULT);
    }
    let syncdbs = handle.syncdbs();
    if syncdbs.is_empty() {
        return Ok(vec![]);
    }
    let localdb = handle.localdb();
    let mut out = Vec::new();
    for pkg in localdb.pkgs().iter() {
        let name = pkg.name();
        let local_ver = pkg.version().to_string();
        // First syncdb hit wins (repo order = pacman priority).
        for db in syncdbs.iter() {
            let Ok(sync_pkg) = db.pkg(name) else {
                continue;
            };
            let sync_ver = sync_pkg.version().to_string();
            if alpm::vercmp(sync_ver.clone(), local_ver.clone()) == std::cmp::Ordering::Greater {
                out.push(PkgUpdate {
                    name: name.to_string(),
                    old_version: local_ver.clone(),
                    new_version: sync_ver,
                    repo: db.name().to_string(),
                    out_of_date: false,
                });
            }
            break;
        }
    }
    out.sort_by(|a, b| a.repo.cmp(&b.repo).then(a.name.cmp(&b.name)));
    Ok(out)
}

/// AUR updates: foreign pkgs (installed, not in any syncdb) vs RPC `/info`.
/// Batched (50/request), timeout from config, graceful offline -> empty.
/// `resilience-failure`: bounded calls, fail-open to repo-only results.
pub async fn check_aur_updates(
    client: &reqwest::Client,
    cfg: &crate::config::Config,
) -> Vec<PkgUpdate> {
    let foreign = foreign_packages().unwrap_or_default();
    if foreign.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    for chunk in foreign.chunks(50) {
        match fetch_aur_info(client, cfg, chunk).await {
            Ok(infos) => {
                for (name, local_ver) in &infos.requested {
                    if let Some(remote) = infos.versions.get(name)
                        && alpm::vercmp(remote.clone(), local_ver.clone())
                            == std::cmp::Ordering::Greater
                    {
                        out.push(PkgUpdate {
                            name: name.clone(),
                            old_version: local_ver.clone(),
                            new_version: remote.clone(),
                            repo: "aur".to_string(),
                            out_of_date: infos.out_of_date.get(name).copied().flatten().is_some(),
                        });
                    }
                }
            }
            Err(e) => {
                tracing::warn!(err=?e, "AUR update check failed, continuing repo-only");
                break;
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

struct AurInfoBatch {
    requested: Vec<(String, String)>,
    versions: std::collections::HashMap<String, String>,
    out_of_date: std::collections::HashMap<String, Option<i64>>,
}

async fn fetch_aur_info(
    client: &reqwest::Client,
    cfg: &crate::config::Config,
    chunk: &[(String, String)],
) -> anyhow::Result<AurInfoBatch> {
    let rpc = cfg.aur_rpc_url();
    let mut url = format!("{}/info?", rpc);
    for (i, (name, _)) in chunk.iter().enumerate() {
        if i > 0 {
            url.push('&');
        }
        url.push_str("arg[]=");
        url.push_str(&url::form_urlencoded::byte_serialize(name.as_bytes()).collect::<String>());
    }
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("AUR info request failed: {e}"))?;
    if !resp.status().is_success() {
        anyhow::bail!("AUR RPC HTTP {}", resp.status());
    }
    let text = resp
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("AUR info read failed: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("AUR info parse: {e}"))?;
    let mut versions = std::collections::HashMap::new();
    let mut out_of_date = std::collections::HashMap::new();
    if let Some(arr) = v.get("results").and_then(|r| r.as_array()) {
        for item in arr {
            if let Some(name) = item.get("Name").and_then(|n| n.as_str()) {
                if let Some(ver) = item.get("Version").and_then(|x| x.as_str()) {
                    versions.insert(name.to_string(), ver.to_string());
                }
                let ood = item.get("OutOfDate").and_then(|x| x.as_i64());
                out_of_date.insert(name.to_string(), ood);
            }
        }
    }
    Ok(AurInfoBatch {
        requested: chunk.to_vec(),
        versions,
        out_of_date,
    })
}

/// Installed pkgs absent from every syncdb = AUR/foreign (name, version).
fn foreign_packages() -> anyhow::Result<Vec<(String, String)>> {
    let cfg = crate::search::repo::get_cached_config().or_else(|_| {
        pacmanconf::Config::new().map_err(|e| anyhow::anyhow!("pacmanconf failed: {e:?}"))
    })?;
    let db_path = if cfg.db_path.is_empty() {
        "/var/lib/pacman".to_string()
    } else {
        cfg.db_path.clone()
    };
    let root_dir = if cfg.root_dir.is_empty() {
        "/".to_string()
    } else {
        cfg.root_dir.clone()
    };
    let handle = alpm::Alpm::new(root_dir.as_str(), db_path.as_str())
        .map_err(|e| anyhow::anyhow!("alpm init failed: {e:?}"))?;
    for repo in &cfg.repos {
        let _ = handle.register_syncdb(repo.name.as_str(), alpm::SigLevel::USE_DEFAULT);
    }
    let syncdbs = handle.syncdbs();
    let localdb = handle.localdb();
    let mut out = Vec::new();
    for pkg in localdb.pkgs().iter() {
        let name = pkg.name();
        let mut in_sync = false;
        for db in syncdbs.iter() {
            if db.pkg(name).is_ok() {
                in_sync = true;
                break;
            }
        }
        if !in_sync {
            out.push((name.to_string(), pkg.version().to_string()));
        }
    }
    Ok(out)
}

/// Blocking AUR updates for TUI worker threads (no tokio runtime there).
/// Same batching/graceful logic as the async version.
pub fn check_aur_updates_blocking(cfg: &crate::config::Config) -> Vec<PkgUpdate> {
    check_aur_updates_blocking_cached(cfg)
}

/// Memoized wrapper (`caching` skill: TTL + explicit invalidate).
/// Tab-switching back to Updates within the TTL reuses the last fetch
/// instead of hammering the AUR RPC; `F5`/upgrade/downgrade paths call
/// `invalidate_aur_memo()` for a fresh check.
pub fn check_aur_updates_blocking_cached(cfg: &crate::config::Config) -> Vec<PkgUpdate> {
    const TTL_SECS: u64 = 300;
    let now = std::time::Instant::now();
    let memo = AUR_MEMO.get_or_init(|| std::sync::Mutex::new((None, Vec::new())));
    if let Ok(guard) = memo.lock()
        && let (Some(at), ref cached) = *guard
        // Cache hits include empty lists: a failed/offline fetch backs off
        // for the TTL instead of timing out on every Tab press. F5 forces
        // a fresh check via invalidate_aur_memo().
        && now.duration_since(at).as_secs() < TTL_SECS
    {
        return cached.clone();
    }
    let fresh = check_aur_updates_blocking_live(cfg);
    if let Ok(mut guard) = memo.lock() {
        *guard = (Some(now), fresh.clone());
    }
    fresh
}

/// Forget the memoized AUR update list (refresh/upgrade/clean paths).
pub fn invalidate_aur_memo() {
    let memo = AUR_MEMO.get_or_init(|| std::sync::Mutex::new((None, Vec::new())));
    if let Ok(mut guard) = memo.lock() {
        *guard = (None, Vec::new());
    }
}

static AUR_MEMO: std::sync::OnceLock<
    std::sync::Mutex<(Option<std::time::Instant>, Vec<PkgUpdate>)>,
> = std::sync::OnceLock::new();

fn check_aur_updates_blocking_live(cfg: &crate::config::Config) -> Vec<PkgUpdate> {
    let foreign = foreign_packages().unwrap_or_default();
    if foreign.is_empty() {
        return vec![];
    }
    let timeout = std::time::Duration::from_secs(cfg.search.timeout_secs.max(1));
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("pacseek/{}", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .build();
    let Ok(client) = client else {
        return vec![];
    };
    let rpc = cfg.aur_rpc_url();
    let mut out = Vec::new();
    for chunk in foreign.chunks(50) {
        let mut url = format!("{rpc}/info?");
        for (i, (name, _)) in chunk.iter().enumerate() {
            if i > 0 {
                url.push('&');
            }
            url.push_str("arg[]=");
            url.push_str(
                &url::form_urlencoded::byte_serialize(name.as_bytes()).collect::<String>(),
            );
        }
        let resp = client.get(&url).send();
        let Ok(resp) = resp else {
            tracing::warn!("AUR update check offline, repo-only");
            break;
        };
        if !resp.status().is_success() {
            break;
        }
        let Ok(text) = resp.text() else {
            break;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            break;
        };
        let mut versions = std::collections::HashMap::new();
        let mut ood = std::collections::HashMap::new();
        if let Some(arr) = v.get("results").and_then(|r| r.as_array()) {
            for item in arr {
                if let Some(name) = item.get("Name").and_then(|n| n.as_str()) {
                    if let Some(ver) = item.get("Version").and_then(|x| x.as_str()) {
                        versions.insert(name.to_string(), ver.to_string());
                    }
                    ood.insert(
                        name.to_string(),
                        item.get("OutOfDate").and_then(|x| x.as_i64()),
                    );
                }
            }
        }
        for (name, local_ver) in chunk {
            if let Some(remote) = versions.get(name)
                && alpm::vercmp(remote.clone(), local_ver.clone()) == std::cmp::Ordering::Greater
            {
                out.push(PkgUpdate {
                    name: name.clone(),
                    old_version: local_ver.clone(),
                    new_version: remote.clone(),
                    repo: "aur".to_string(),
                    out_of_date: ood.get(name).copied().flatten().is_some(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_updates_offline_never_errors() {
        // On Arch: returns list (maybe empty); off-Arch without pacman DB:
        // alpm init succeeds but no syncdbs -> empty. Must never panic/Err-fatal.
        let r = check_repo_updates();
        assert!(r.is_ok());
    }

    #[test]
    fn vercmp_orders_versions() {
        assert_eq!(
            alpm::vercmp("2-1".to_string(), "1-1".to_string()),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            alpm::vercmp("1-1".to_string(), "1-1".to_string()),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn aur_memo_invalidate_is_safe() {
        invalidate_aur_memo();
        // Cached wrapper works and second call hits the memo (same content).
        // Live network: hermetic on content equality, skip fully offline.
        if std::env::var("PACSEEK_SKIP_LIVE").is_ok() {
            return;
        }
        let cfg = crate::config::Config::default();
        let a = check_aur_updates_blocking_cached(&cfg);
        let b = check_aur_updates_blocking_cached(&cfg);
        assert_eq!(a, b);
        invalidate_aur_memo();
    }
}
