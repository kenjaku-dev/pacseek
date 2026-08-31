use anyhow::Context;
use std::process::Command;

/// Install official repo package via sudo pacman -S (aur-guides:aur-pacman, aur-helpers)
/// Delegates to pacman binary — safer than reimplementing alpm transaction per plan.
pub fn install_repo_package(name: &str) -> anyhow::Result<()> {
    // Guard: prevent empty name
    if name.trim().is_empty() {
        anyhow::bail!("empty package name");
    }

    // Check if already installed via pacman -Q (optional, just info)
    let already = Command::new("pacman")
        .args(["-Q", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if already {
        eprintln!("Package {} already installed, reinstalling...", name);
    }

    // Build sudo pacman -S command inheriting tty
    // Do NOT use --noconfirm by default — let pacman ask. Caller can add --noconfirm via env
    let mut cmd = Command::new("sudo");
    cmd.args(["pacman", "-S", "--needed", name]);

    // Inherit stdio so user sees pacman progress + can enter password
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd.status().context("failed to run sudo pacman -S")?;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        anyhow::bail!("pacman -S failed with exit {}", code);
    }
    Ok(())
}
