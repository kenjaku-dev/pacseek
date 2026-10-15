//! Flatpak backend (P1: detect + list installed).
//!
//! Uses `flatpak --json` (stable machine format) instead of TSV columns:
//! TSV breaks on empty versions (double-tab) and wrapped descriptions, JSON
//! does not. TSV parser is kept as fallback for old flatpak without `--json`.
//!
//! No sudo: flatpak is user/system scoped, never needs `sudo`. Missing binary
//! or missing remote is fail-open (empty / Err with hint), never a panic.
//! No `expect()`/`unwrap()` in production paths.

use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::SourceBackend;
use crate::model::Package;

/// JSON row from `flatpak list --app --json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct FlatpakListRow {
    #[serde(default, rename = "application_id")]
    application_id: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    arch: Option<String>,
}

/// Backend handle. `remote` defaults to `"flathub"`, `enabled` mirrors
/// `[behavior] flatpak_enabled`.
#[derive(Debug, Clone)]
pub struct FlatpakBackend {
    pub remote: String,
    pub enabled: bool,
}

impl FlatpakBackend {
    pub fn new(remote: String, enabled: bool) -> Self {
        let remote = if remote.trim().is_empty() {
            "flathub".to_string()
        } else {
            remote.trim().to_string()
        };
        Self { remote, enabled }
    }

    pub fn from_config(cfg: &crate::config::Config) -> Self {
        Self::new(cfg.flatpak_remote_name(), cfg.flatpak_enabled())
    }
}

impl SourceBackend for FlatpakBackend {
    fn id(&self) -> &'static str {
        "flatpak"
    }

    fn is_available(&self) -> bool {
        self.enabled && has_flatpak()
    }

    fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        list_installed()
    }
}

/// `which flatpak` probe. No output parsing, exit status only.
pub fn has_flatpak() -> bool {
    Command::new("which")
        .arg("flatpak")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// True when `remote` appears in `flatpak remote-list --columns=name`.
/// Missing binary -> false (fail-open, caller emits hint).
pub fn flatpak_remote_exists(remote: &str) -> bool {
    let remote = remote.trim();
    if remote.is_empty() || !has_flatpak() {
        return false;
    }
    match run_flatpak(&["remote-list", "--columns=name"], Duration::from_secs(10)) {
        Some(out) => parse_remote_list(&out).iter().any(|r| r == remote),
        None => false,
    }
}

/// Live list: `flatpak list --app --json`, 15s bound. Missing binary is an Err
/// with install hint (caller maps to exit 1 / TUI message, not a panic).
/// Empty install set is Ok(empty).
pub fn list_installed() -> anyhow::Result<Vec<Package>> {
    if !has_flatpak() {
        anyhow::bail!("flatpak not found — install flatpak to manage flatpak apps");
    }
    match run_flatpak(&["list", "--app", "--json"], Duration::from_secs(15)) {
        Some(out) => Ok(parse_flatpak_list_json(&out)),
        None => Ok(vec![]),
    }
}

/// Run flatpak with a wall-clock bound. std has no Command timeout, so park
/// the wait on a thread and bound the recv (`resilience-failure`: unbounded
/// waits hang TUI workers). None covers timeout/spawn failure (fail-open).
fn run_flatpak(args: &[&str], timeout: Duration) -> Option<String> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        let out = Command::new("flatpak").args(&refs).output();
        let text = out
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        let _ = tx.send(text);
    });
    rx.recv_timeout(timeout).ok().flatten()
}

/// Parse `flatpak list --app --json`. Defensive: garbage -> empty, rows without
/// `application_id` skipped, missing version -> `"?"` (flatpak leaves version
/// empty for runtimes/drivers, e.g. VAAPI).
pub fn parse_flatpak_list_json(text: &str) -> Vec<Package> {
    let rows: Vec<FlatpakListRow> = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let Some(app_id) = r
            .application_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let version = r
            .version
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "?".to_string());
        let mut pkg = Package::minimal(app_id, version, "flatpak".to_string());
        pkg.installed = true;
        pkg.description = r.name.or(r.description);
        pkg.arch = r.arch;
        pkg.url = r.origin.map(|o| format!("flatpak:{o}"));
        if let Some(b) = r.branch {
            // Branch is not part of Package; stash in version suffix when
            // non-stable so rows stay distinguishable in the list.
            if b != "stable" && !b.trim().is_empty() {
                pkg.version = format!("{} ({})", pkg.version, b.trim());
            }
        }
        out.push(pkg);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Fallback TSV parser for `flatpak list --columns=application,version,branch,origin`
/// (tab-separated, no header). Empty version field preserved via `\t` split —
/// never split on whitespace here.
pub fn parse_flatpak_list_columns(output: &str) -> Vec<Package> {
    let mut out = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Application") {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.is_empty() {
            continue;
        }
        let app_id = cols.first().map(|s| s.trim()).unwrap_or("");
        if app_id.is_empty() {
            continue;
        }
        let version = cols
            .get(1)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("?");
        let branch = cols.get(2).map(|s| s.trim()).unwrap_or("");
        let origin = cols.get(3).map(|s| s.trim()).unwrap_or("");
        let mut pkg = Package::minimal(
            app_id.to_string(),
            version.to_string(),
            "flatpak".to_string(),
        );
        pkg.installed = true;
        if !origin.is_empty() {
            pkg.url = Some(format!("flatpak:{origin}"));
        }
        if !branch.is_empty() && branch != "stable" {
            pkg.version = format!("{version} ({branch})");
        }
        out.push(pkg);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Parse `flatpak remote-list --columns=name` (one name per line, TSV or plain).
pub fn parse_remote_list(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|l| l.split('\t').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty() && s != "Name")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_JSON: &str = r#"[
      {"name":"Firefox","application_id":"org.mozilla.firefox","version":"158.0","branch":"stable","arch":"x86_64","origin":"flathub","installation":"system"},
      {"name":"Nvidia VAAPI driver","application_id":"org.freedesktop.Platform.VAAPI.nvidia","version":"","branch":"26.08","arch":"x86_64","origin":"flathub","installation":"system"},
      {"name":"","version":"1.0"}
    ]"#;

    #[test]
    fn parse_list_json_counts_and_fields() {
        let pkgs = parse_flatpak_list_json(LIST_JSON);
        // Third row has no application_id -> skipped.
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].repo, "flatpak");
        assert!(pkgs.iter().all(|p| p.installed));
        let ff = pkgs
            .iter()
            .find(|p| p.name == "org.mozilla.firefox")
            .unwrap();
        assert_eq!(ff.version, "158.0");
        assert_eq!(ff.arch.as_deref(), Some("x86_64"));
    }

    #[test]
    fn parse_list_json_empty_version_becomes_qmark_with_branch() {
        let pkgs = parse_flatpak_list_json(LIST_JSON);
        let vaapi = pkgs
            .iter()
            .find(|p| p.name == "org.freedesktop.Platform.VAAPI.nvidia")
            .unwrap();
        assert_eq!(vaapi.version, "? (26.08)");
    }

    #[test]
    fn parse_list_json_garbage_is_empty_never_panics() {
        assert!(parse_flatpak_list_json("").is_empty());
        assert!(parse_flatpak_list_json("not json").is_empty());
        assert!(parse_flatpak_list_json("{\"a\":1}").is_empty());
        assert!(parse_flatpak_list_json("null").is_empty());
    }

    #[test]
    fn parse_columns_preserves_empty_version() {
        let out = "org.mozilla.firefox\t158.0\tstable\tflathub\norg.freedesktop.Platform.VAAPI.nvidia\t\t26.08\tflathub\n";
        let pkgs = parse_flatpak_list_columns(out);
        assert_eq!(pkgs.len(), 2);
        let vaapi = pkgs.iter().find(|p| p.name.contains("VAAPI")).unwrap();
        assert_eq!(vaapi.version, "? (26.08)");
    }

    #[test]
    fn parse_remote_list_names() {
        let out = "Name\nflathub\nfilmcraft-origin\n";
        assert_eq!(parse_remote_list(out), vec!["flathub", "filmcraft-origin"]);
        assert!(parse_remote_list("").is_empty());
    }

    #[test]
    fn backend_id_and_availability_never_panics() {
        let b = FlatpakBackend::new("flathub".to_string(), true);
        assert_eq!(b.id(), "flatpak");
        // Live probe: boolean only, hermetic on both outcomes.
        let _ = b.is_available();
        let _ = has_flatpak();
        // Disabled backend is never available, even with binary present.
        let off = FlatpakBackend::new("flathub".to_string(), false);
        assert!(!off.is_available());
    }

    #[test]
    fn live_list_smoke() {
        if !has_flatpak() {
            return;
        }
        let pkgs = list_installed().unwrap_or_default();
        // Machine has flatpaks; off-Arch CI skips via early return above.
        assert!(pkgs.iter().all(|p| p.repo == "flatpak" && p.installed));
    }
}
