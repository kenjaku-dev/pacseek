use crate::model::Package;

/// Orphan = installed as dependency with nothing requiring it (`pacman -Qdt`).
/// Enriches with required_by/optional_for/reason for the info pane.
pub fn list_orphans() -> anyhow::Result<Vec<Package>> {
    let cfg = crate::search::repo::get_cached_config().or_else(|_| {
        pacmanconf::Config::new().map_err(|e| anyhow::anyhow!("pacmanconf failed: {e:?}"))
    })?;
    list_orphans_with_config(&cfg)
}

pub fn list_orphans_with_config(cfg: &pacmanconf::Config) -> anyhow::Result<Vec<Package>> {
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
    let localdb = handle.localdb();
    let mut out = Vec::new();
    for pkg in localdb.pkgs().iter() {
        if pkg.reason() != alpm::PackageReason::Depend {
            continue;
        }
        let required: Vec<String> = pkg.required_by().iter().map(|s| s.to_string()).collect();
        let optional: Vec<String> = pkg.optional_for().iter().map(|s| s.to_string()).collect();
        if !required.is_empty() {
            continue;
        }
        // Match `pacman -Qdt`: exclude optional-for-only? No — pacman counts
        // optional-for as still required, so keep that exclusion too.
        if !optional.is_empty() {
            continue;
        }
        let depends: Vec<String> = pkg.depends().iter().map(|d| d.to_string()).collect();
        let optdepends: Vec<String> = pkg.optdepends().iter().map(|d| d.to_string()).collect();
        out.push(Package {
            name: pkg.name().to_string(),
            version: pkg.version().to_string(),
            description: pkg.desc().map(|s| s.to_string()),
            repo: "local".to_string(),
            arch: pkg.arch().map(|a| a.to_string()),
            url: pkg.url().map(|u| u.to_string()),
            installed: true,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
            depends: if depends.is_empty() {
                None
            } else {
                Some(depends)
            },
            optdepends: if optdepends.is_empty() {
                None
            } else {
                Some(optdepends)
            },
            required_by: Some(required),
            optional_for: Some(optional),
            reason: Some("dependency".to_string()),
            orphan: true,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Fallback when ALPM unavailable — parses `pacman -Qdt` (names only).
pub fn list_orphans_fallback() -> anyhow::Result<Vec<Package>> {
    let output = std::process::Command::new("pacman")
        .args(["-Qdt"])
        .output()
        .map_err(|e| anyhow::anyhow!("pacman -Qdt failed: {e}"))?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let name = parts.next().unwrap_or("").trim();
        let version = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let mut p = Package::minimal(name.to_string(), version, "local".to_string());
        p.installed = true;
        p.reason = Some("dependency".to_string());
        p.orphan = true;
        out.push(p);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Dependency details for the info pane: depends/optdepends/required-by/optional-for/reason.
pub fn package_details(name: &str) -> Option<Package> {
    let cfg = crate::search::repo::get_cached_config().ok()?;
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
    let handle = alpm::Alpm::new(root_dir.as_str(), db_path.as_str()).ok()?;
    let localdb = handle.localdb();
    let pkg = localdb.pkg(name.trim()).ok()?;
    let required: Vec<String> = pkg.required_by().iter().map(|s| s.to_string()).collect();
    let optional: Vec<String> = pkg.optional_for().iter().map(|s| s.to_string()).collect();
    let depends: Vec<String> = pkg.depends().iter().map(|d| d.to_string()).collect();
    let optdepends: Vec<String> = pkg.optdepends().iter().map(|d| d.to_string()).collect();
    let reason = match pkg.reason() {
        alpm::PackageReason::Explicit => "explicit",
        alpm::PackageReason::Depend => "dependency",
    };
    let orphan =
        pkg.reason() == alpm::PackageReason::Depend && required.is_empty() && optional.is_empty();
    Some(Package {
        name: pkg.name().to_string(),
        version: pkg.version().to_string(),
        description: pkg.desc().map(|s| s.to_string()),
        repo: "local".to_string(),
        arch: pkg.arch().map(|a| a.to_string()),
        url: pkg.url().map(|u| u.to_string()),
        installed: true,
        votes: None,
        popularity: None,
        out_of_date: None,
        maintainer: None,
        num_votes: None,
        last_modified: None,
        depends: if depends.is_empty() {
            None
        } else {
            Some(depends)
        },
        optdepends: if optdepends.is_empty() {
            None
        } else {
            Some(optdepends)
        },
        required_by: Some(required),
        optional_for: Some(optional),
        reason: Some(reason.to_string()),
        orphan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphans_never_errors() {
        assert!(list_orphans().is_ok());
        assert!(list_orphans_fallback().is_ok());
    }

    #[test]
    fn details_missing_is_none() {
        assert!(package_details("definitely-not-a-real-pkg-xyz-123").is_none());
    }
}
