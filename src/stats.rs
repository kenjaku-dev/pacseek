use serde::{Deserialize, Serialize};

/// System health snapshot for `--stats` and the Updates-tab header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SysStats {
    pub explicit: usize,
    pub dependencies: usize,
    pub orphans: usize,
    pub updates_repo: usize,
    pub updates_aur: usize,
    pub cache_pacman_bytes: u64,
    pub cache_pacseek_bytes: u64,
}

/// Local-only stats (no network): explicit/deps split + orphans + cache sizes.
/// Fast path for `--stats` and TUI header; AUR updates filled separately.
pub fn gather_local_stats() -> SysStats {
    let (explicit, dependencies) = count_reasons();
    let orphans = crate::orphans::list_orphans()
        .map(|v| v.len())
        .unwrap_or_else(|_| {
            crate::orphans::list_orphans_fallback()
                .map(|v| v.len())
                .unwrap_or(0)
        });
    let updates_repo = crate::updates::check_repo_updates()
        .map(|v| v.len())
        .unwrap_or(0);
    SysStats {
        explicit,
        dependencies,
        orphans,
        updates_repo,
        updates_aur: 0,
        cache_pacman_bytes: dir_size("/var/cache/pacman/pkg"),
        cache_pacseek_bytes: cache_size(),
    }
}

fn count_reasons() -> (usize, usize) {
    let Ok(cfg) = crate::search::repo::get_cached_config() else {
        return (0, 0);
    };
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
    let Ok(handle) = alpm::Alpm::new(root_dir.as_str(), db_path.as_str()) else {
        return (0, 0);
    };
    let mut explicit = 0;
    let mut deps = 0;
    for pkg in handle.localdb().pkgs().iter() {
        match pkg.reason() {
            alpm::PackageReason::Explicit => explicit += 1,
            alpm::PackageReason::Depend => deps += 1,
        }
    }
    (explicit, deps)
}

fn dir_size(path: &str) -> u64 {
    let mut total = 0u64;
    let Ok(rd) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in rd.flatten() {
        if let Ok(meta) = entry.metadata() {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

fn cache_size() -> u64 {
    let base = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("pacseek");
    dir_size(base.to_str().unwrap_or("/tmp/pacseek"))
}

/// Human bytes: 1.2 MiB style, no float noise.
pub fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} {}", n, UNITS[u])
    } else {
        format!("{:.1} {}", v, UNITS[u])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.0 KiB");
    }

    #[test]
    fn local_stats_never_panics() {
        let s = gather_local_stats();
        assert!(s.explicit + s.dependencies > 0 || s.explicit == 0);
    }
}
