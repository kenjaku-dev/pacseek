use anyhow::Context;
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;

/// Install AUR package: git clone + makepkg -si (aur-guides:aur-makepkg, aur-pkgbuild)
/// Follows AUR best practices: never run makepkg as root, show PKGBUILD, use --syncdeps
pub fn install_aur_package(pkg: &crate::model::Package) -> anyhow::Result<()> {
    // Safety: refuse if running as root — makepkg must not be root per Arch Wiki + aur-package-guidelines
    if nix::unistd::geteuid().is_root() {
        anyhow::bail!(
            "AUR install must not run as root — run pacseek as normal user, sudo will be used for pacman -U"
        );
    }

    let name = &pkg.name;
    if name.trim().is_empty() {
        anyhow::bail!("empty package name");
    }

    // Sanitize package name — prevent path-traversal (../, /, \) per aur-package-guidelines
    let path_binding = PathBuf::from(name);
    let safe_name = path_binding
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if safe_name.is_empty()
        || safe_name != name.as_str()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
    {
        anyhow::bail!("invalid package name: {:?}", name);
    }

    // Determine cache dir: config [behavior]cache_dir or $XDG_CACHE_HOME/pacseek or ~/.cache/pacseek or /tmp/pacseek
    // Use 0o700 to avoid world-writable race in /tmp fallback (aur-makepkg)
    let cache_base = if let Some(p) = Config::load().cache_dir_path() {
        p
    } else {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("pacseek")
    };
    std::fs::create_dir_all(&cache_base).context("create cache dir")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&cache_base, std::fs::Permissions::from_mode(0o700));
    }
    let clone_dir = cache_base.join(safe_name);

    // Clone or pull
    if clone_dir.exists() {
        eprintln!("Updating existing clone {} ...", clone_dir.display());
        let status = Command::new("git")
            .args(["-C", &clone_dir.to_string_lossy(), "pull", "--ff-only"])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("git pull")?;
        if !status.success() {
            eprintln!("git pull failed, continuing with existing clone...");
        }
    } else {
        let url = format!("https://aur.archlinux.org/{}.git", name);
        eprintln!("Cloning {} ...", url);
        let status = Command::new("git")
            .args(["clone", &url, &clone_dir.to_string_lossy()])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("git clone")?;
        if !status.success() {
            anyhow::bail!("git clone failed for {}", url);
        }
    }

    // Show PKGBUILD for review (aur-audit) — use bat if available, else cat
    let pkgbuild = clone_dir.join("PKGBUILD");
    if pkgbuild.exists() {
        eprintln!("\n--- PKGBUILD: {} ---", pkgbuild.display());
        let has_bat = Command::new("which")
            .arg("bat")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let mut cmd = if has_bat {
            let mut c = Command::new("bat");
            c.args([
                "--style=plain",
                "--color=always",
                pkgbuild.to_string_lossy().as_ref(),
            ]);
            c
        } else {
            let mut c = Command::new("cat");
            c.arg(&pkgbuild);
            c
        };
        let _ = cmd.status();
        eprintln!("--- end PKGBUILD ---\n");

        // Optional namcap audit if installed (aur-audit)
        let has_namcap = Command::new("which")
            .arg("namcap")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if has_namcap {
            eprintln!("Running namcap audit...");
            let _ = Command::new("namcap")
                .arg(&pkgbuild)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();
        }

        // Optional shellcheck for PKGBUILD (aur-audit, aur-package-guidelines)
        let has_shellcheck = Command::new("which")
            .arg("shellcheck")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if has_shellcheck {
            eprintln!("Running shellcheck...");
            let _ = Command::new("shellcheck")
                .args([
                    "--shell=bash",
                    "--exclude=SC2034,SC2154,SC2164",
                    &pkgbuild.to_string_lossy(),
                ])
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();
        }
    } else {
        eprintln!("Warning: PKGBUILD not found in {}", clone_dir.display());
    }

    // Confirm already done in TUI, but double-check if run from CLI
    // Ask once more if not in TUI? We assume TUI already confirmed, so proceed.

    // Build & install via makepkg -si (aur-makepkg) — default asks, --noconfirm only if explicit
    // Consistency with repo.rs: repo uses `pacman -S --needed` without --noconfirm; AUR should also ask by default
    // Enable non-interactive with PACSEEK_NOCONFIRM=1 or config behavior.makepkg_noconfirm
    let cfg = Config::load();
    let noconfirm = std::env::var("PACSEEK_NOCONFIRM").is_ok() || cfg.behavior.makepkg_noconfirm;
    eprintln!(
        "\nBuilding and installing {} with makepkg -si{} ...",
        name,
        if noconfirm { " --noconfirm" } else { "" }
    );
    let mut cmd = Command::new("makepkg");
    if noconfirm {
        cmd.args(["-si", "--noconfirm"]);
    } else {
        cmd.args(["-si"]);
    }
    cmd.current_dir(&clone_dir)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd.status().context("makepkg -si")?;
    if !status.success() {
        anyhow::bail!("makepkg failed with exit {}", status.code().unwrap_or(-1));
    }

    eprintln!("\n✓ AUR package {} installed", name);
    Ok(())
}
