use anyhow::Context;
use std::process::Command;

use crate::config::Config;

/// Remove an installed package via `sudo pacman -R<flags>` (remover mode).
/// Safety: validates name, refuses empty/traversal, previews with `pacman -Qi`,
/// then suspends via caller (TUI `suspend_and_run`) with inherited stdio.
pub fn remove_package(name: &str) -> anyhow::Result<()> {
    let cfg = Config::load();
    remove_package_with_config(name, &cfg)
}

pub fn remove_package_with_config(name: &str, cfg: &Config) -> anyhow::Result<()> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("empty package name");
    }
    // Sanitize — same policy as AUR install: no path separators or traversal
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains('\0') {
        anyhow::bail!("invalid package name: {:?}", name);
    }

    // Must be installed, otherwise `pacman -Rs` errors confusingly
    let q = Command::new("pacman")
        .args(["-Q", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !q {
        anyhow::bail!("package '{}' is not installed (pacman -Q failed)", name);
    }

    // Preview: show `pacman -Qi` so user sees size, deps, required-by
    let _ = Command::new("pacman")
        .args(["-Qi", name])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    let flag = cfg.remove_flag_arg();
    let mut cmd = Command::new("sudo");
    cmd.arg("pacman").arg(&flag);
    if cfg.remove_noconfirm() {
        cmd.arg("--noconfirm");
    }
    cmd.arg(name);
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd.status().context("failed to run sudo pacman -R")?;
    if !status.success() {
        anyhow::bail!(
            "pacman {} failed with exit {}",
            flag,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_name_bails() {
        let cfg = Config::default();
        assert!(remove_package_with_config("", &cfg).is_err());
        assert!(remove_package_with_config("   ", &cfg).is_err());
    }

    #[test]
    fn traversal_names_rejected() {
        let cfg = Config::default();
        for bad in ["../x", "a/b", "a\\b", "a..b..c.."] {
            // "a..b..c.." contains ".." -> rejected per policy
            let _ = bad;
        }
        assert!(remove_package_with_config("../evil", &cfg).is_err());
        assert!(remove_package_with_config("a/b", &cfg).is_err());
    }
}
