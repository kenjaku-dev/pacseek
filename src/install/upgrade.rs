use anyhow::Context;
use std::process::Command;

use crate::config::Config;

/// Args for `sudo pacman -Syu`, extracted for unit testing without sudo.
pub fn upgrade_command_args(cfg: &Config) -> Vec<String> {
    let mut args = vec!["pacman".to_string(), "-Syu".to_string()];
    if cfg.upgrade_noconfirm() {
        args.push("--noconfirm".to_string());
    }
    args
}

/// Full system upgrade via `sudo pacman -Syu` with inherited stdio (sudo TTY).
/// Caller (CLI/TUI) owns confirmation + Arch-news guard.
pub fn upgrade_system(cfg: &Config) -> anyhow::Result<()> {
    // Readonly guard: defense in depth (CLI also refuses earlier).
    if cfg.is_readonly() {
        anyhow::bail!("refusing to upgrade: readonly mode is enabled");
    }
    let args = upgrade_command_args(cfg);
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut cmd = Command::new("sudo");
    cmd.args(&arg_refs);
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    let status = cmd.status().context("failed to run sudo pacman -Syu")?;
    if !status.success() {
        anyhow::bail!(
            "pacman -Syu failed with exit {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_args_respect_noconfirm() {
        let cfg = Config::default();
        assert_eq!(upgrade_command_args(&cfg), vec!["pacman", "-Syu"]);
        let mut nc = Config::default();
        nc.behavior.upgrade_noconfirm = true;
        assert_eq!(
            upgrade_command_args(&nc),
            vec!["pacman", "-Syu", "--noconfirm"]
        );
    }
}
