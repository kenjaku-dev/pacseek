use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::config::Config;

/// Preview of what `paccache` would remove. Always computed live
/// (`caching` skill: destructive previews must never be served stale).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CleanPreview {
    pub files: usize,
    pub bytes: u64,
    pub keep: u32,
}

/// Args for `paccache` after the binary name, extracted for unit testing
/// without spawning anything. Preview uses `-d` (dryrun op); real run uses
/// `-r` (remove op) — they are distinct operations in paccache 1.x.
pub fn clean_command_args(keep: u32, dry_run: bool) -> Vec<String> {
    let op = if dry_run { "-d" } else { "-r" };
    vec![
        op.to_string(),
        "-k".to_string(),
        keep.max(1).to_string(),
        "--nocolor".to_string(),
    ]
}

fn has_tool(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn has_paccache() -> bool {
    has_tool("paccache")
}

/// Parse a `paccache` size token: `870.63 MiB`, `51.03 MiB`, `300 KiB`,
/// `2.0 GiB`, `512 B`. Returns bytes. Pure, unit-tested — paccache output
/// format drift must degrade to 0, never panic.
pub fn parse_size_token(num: f64, unit: &str) -> Option<u64> {
    if !num.is_finite() || num < 0.0 {
        return None;
    }
    let mult: f64 = match unit {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((num * mult) as u64)
}

/// Parse full `paccache -d` output: file count from the candidate list plus
/// bytes from the `disk space saved:` trailer. Defensive: unparseable output
/// yields counts of what was recognizable, never an error.
pub fn parse_dryrun(output: &str, keep: u32) -> CleanPreview {
    let mut files = 0usize;
    let mut bytes = 0u64;
    let mut in_list = false;
    for line in output.lines() {
        let t = line.trim();
        if t.starts_with("==> Candidate packages:") {
            in_list = true;
            continue;
        }
        if t.starts_with("==> finished dry run:") {
            in_list = false;
            // `==> finished dry run: 16 candidates (disk space saved: 870.63 MiB)`
            if let Some(idx) = t.find("disk space saved:")
                && let (Some(n), Some(u)) = {
                    let rest = t[idx + "disk space saved:".len()..].trim();
                    let rest = rest.trim_end_matches(')');
                    let mut parts = rest.split_whitespace();
                    (parts.next(), parts.next())
                }
                && let Ok(num) = n.replace(',', ".").parse::<f64>()
                && let Some(b) = parse_size_token(num, u)
            {
                bytes = b;
            }
            continue;
        }
        if in_list && !t.is_empty() && !t.starts_with("==>") {
            files += 1;
        }
    }
    CleanPreview { files, bytes, keep }
}

/// Live preview via `paccache -d -kN`. Missing paccache or spawn failure is
/// an error with an install hint (caller maps to exit 1, not a panic).
pub fn preview_clean(keep: u32) -> anyhow::Result<CleanPreview> {
    if !has_paccache() {
        anyhow::bail!("paccache not found — install pacman-contrib to clean the cache");
    }
    let args = clean_command_args(keep, true);
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    // Verbose lists candidates so the count is exact even if the trailer drifts.
    let output = Command::new("paccache")
        .args(&arg_refs)
        .arg("-v")
        .output()
        .context("failed to run paccache dry run")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_dryrun(&stdout, keep.max(1)))
}

/// Real clean via `sudo paccache -r -kN` with inherited stdio so the user
/// sees progress (`observability`: the tool's own output is the audit trail).
/// Caller owns confirmation; readonly is refused here too (defense in depth).
pub fn run_clean(cfg: &Config, keep: u32) -> anyhow::Result<()> {
    if cfg.is_readonly() {
        anyhow::bail!("refusing to clean: readonly mode is enabled");
    }
    if !has_paccache() {
        anyhow::bail!("paccache not found — install pacman-contrib to clean the cache");
    }
    let args = clean_command_args(keep, false);
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut cmd = Command::new("sudo");
    if cfg.clean_noconfirm() {
        // paccache has no --noconfirm; `sudo -n` fails fast instead of hanging
        // on a password prompt in scripts.
        cmd.arg("-n");
    }
    cmd.arg("paccache").args(&arg_refs);
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    let status = cmd.status().context("failed to run sudo paccache -r")?;
    if !status.success() {
        anyhow::bail!(
            "paccache -r failed with exit {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_args_preview_vs_real() {
        assert_eq!(
            clean_command_args(2, true),
            vec!["-d", "-k", "2", "--nocolor"]
        );
        assert_eq!(
            clean_command_args(1, false),
            vec!["-r", "-k", "1", "--nocolor"]
        );
        // keep=0 clamps to 1 (keep nothing would nuke the cache).
        assert_eq!(clean_command_args(0, true)[2], "1");
    }

    #[test]
    fn parse_sizes() {
        assert_eq!(parse_size_token(512.0, "B"), Some(512));
        assert_eq!(parse_size_token(2.0, "GiB"), Some(2 * 1024 * 1024 * 1024));
        let mib = parse_size_token(870.63, "MiB").unwrap();
        assert!(mib > 870 * 1024 * 1024 && mib < 871 * 1024 * 1024);
        assert_eq!(parse_size_token(f64::NAN, "MiB"), None);
        assert_eq!(parse_size_token(-1.0, "MiB"), None);
        assert_eq!(parse_size_token(1.0, "XB"), None);
    }

    #[test]
    fn parse_dryrun_counts_and_bytes() {
        let out = "==> Candidate packages:\na-1.pkg.tar.zst\na-1.pkg.tar.zst.sig\nb-2.pkg.tar.zst\n==> finished dry run: 3 candidates (disk space saved: 51.03 MiB)\n";
        let p = parse_dryrun(out, 2);
        assert_eq!(p.files, 3);
        assert_eq!(p.keep, 2);
        assert!(p.bytes > 50 * 1024 * 1024 && p.bytes < 52 * 1024 * 1024);
    }

    #[test]
    fn parse_dryrun_empty_is_zero() {
        let p = parse_dryrun(
            "==> finished dry run: 0 candidates (disk space saved: 0 B)\n",
            2,
        );
        assert_eq!((p.files, p.bytes), (0, 0));
    }

    #[test]
    fn parse_dryrun_garbage_never_panics() {
        let p = parse_dryrun("garbage\n==> weird (disk space saved: lots)\n", 2);
        assert_eq!((p.files, p.bytes), (0, 0));
    }
}
