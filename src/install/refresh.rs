use anyhow::Context;
use std::process::Command;

use crate::config::Config;

/// Args for `sudo pacman -Sy` after `sudo`, extracted for unit testing
/// without spawning sudo in CI.
pub fn refresh_command_args(cfg: &Config) -> Vec<String> {
    let mut args = vec!["pacman".to_string(), "-Sy".to_string()];
    if cfg.refresh_noconfirm() {
        args.push("--noconfirm".to_string());
    }
    args
}

/// Refresh pacman sync databases via `sudo pacman -Sy`.
/// Caller must suspend TUI (`suspend_and_run`) with inherited stdio
/// so sudo can prompt for a password on a real TTY.
pub fn refresh_sync_db(cfg: &Config) -> anyhow::Result<()> {
    let args = refresh_command_args(cfg);
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut cmd = Command::new("sudo");
    cmd.args(&arg_refs);
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    let status = cmd.status().context("failed to run sudo pacman -Sy")?;
    if !status.success() {
        anyhow::bail!(
            "pacman -Sy failed with exit {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
