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

    fn search(
        &self,
        query: &str,
        limit: usize,
        use_regex: bool,
        cfg: &crate::config::Config,
    ) -> anyhow::Result<Vec<Package>> {
        search_flatpak_with_config(query, limit, use_regex, cfg)
    }

    fn install(&self, id: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
        install_flatpak_package(id, cfg)
    }

    fn remove(&self, id: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
        remove_flatpak_package(id, cfg)
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

/// Validate a flatpak application ID (reverse-DNS, e.g. `org.mozilla.firefox`).
/// Same traversal/NUL policy as the pacman paths; dots required (at least
/// one) since bare words are never valid flatpak IDs. Returns trimmed id.
pub fn validate_app_id(id: &str) -> anyhow::Result<String> {
    let id = id.trim();
    if id.is_empty() {
        anyhow::bail!("empty application id");
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('\0') {
        anyhow::bail!("invalid application id: {:?}", id);
    }
    if !id.contains('.') {
        anyhow::bail!(
            "invalid application id (need reverse-DNS like org.mozilla.firefox): {:?}",
            id
        );
    }
    if id.starts_with('.') || id.starts_with('-') || id.ends_with('.') || id.ends_with('-') {
        anyhow::bail!("invalid application id: {:?}", id);
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        anyhow::bail!("invalid application id: {:?}", id);
    }
    Ok(id.to_string())
}

/// Args for `flatpak install` after the binary name, extracted for unit
/// testing without spawning. No sudo — flatpak never needs it. `-y` only with
/// noconfirm (scripts / PACSEEK_NOCONFIRM=1); otherwise flatpak asks.
pub fn install_command_args(remote: &str, id: &str, noconfirm: bool) -> Vec<String> {
    let mut args = vec!["install".to_string(), remote.to_string(), id.to_string()];
    if noconfirm {
        args.push("-y".to_string());
    }
    args
}

/// Args for `flatpak uninstall`. Same `-y` policy as install.
pub fn remove_command_args(id: &str, noconfirm: bool) -> Vec<String> {
    let mut args = vec!["uninstall".to_string(), id.to_string()];
    if noconfirm {
        args.push("-y".to_string());
    }
    args
}

/// Args for unused-runtime cleanup (`flatpak uninstall --unused`).
pub fn unused_command_args(noconfirm: bool) -> Vec<String> {
    let mut args = vec!["uninstall".to_string(), "--unused".to_string()];
    if noconfirm {
        args.push("-y".to_string());
    }
    args
}

/// True when `id` is installed (`flatpak info` probe, 10s bound).
/// Missing binary -> false (fail-open).
pub fn is_flatpak_installed(id: &str) -> bool {
    if !has_flatpak() || validate_app_id(id).is_err() {
        return false;
    }
    let id_owned = id.trim().to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let ok = Command::new("flatpak")
            .args(["info", id_owned.as_str()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let _ = tx.send(ok);
    });
    rx.recv_timeout(Duration::from_secs(10)).unwrap_or(false)
}

/// Install a flatpak app with inherited stdio so the user sees progress.
/// No sudo. Caller owns confirmation; readonly refused here (defense in depth).
pub fn install_flatpak_package(id: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
    let id = validate_app_id(id)?;
    if cfg.is_readonly() {
        anyhow::bail!("refusing to install: readonly mode is enabled");
    }
    if !has_flatpak() {
        anyhow::bail!("flatpak not found — install flatpak to manage flatpak apps");
    }
    let remote = cfg.flatpak_remote_name();
    if !flatpak_remote_exists(&remote) {
        anyhow::bail!(
            "flatpak remote '{remote}' not found — add it or set [behavior] flatpak_remote"
        );
    }
    let args = install_command_args(&remote, &id, cfg.flatpak_noconfirm());
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let status = Command::new("flatpak")
        .args(&refs)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run flatpak install: {e}"))?;
    if !status.success() {
        anyhow::bail!(
            "flatpak install failed with exit {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Uninstall a flatpak app. Same safety/stdio contract as install.
pub fn remove_flatpak_package(id: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
    let id = validate_app_id(id)?;
    if cfg.is_readonly() {
        anyhow::bail!("refusing to remove: readonly mode is enabled");
    }
    if !has_flatpak() {
        anyhow::bail!("flatpak not found — install flatpak to manage flatpak apps");
    }
    if !is_flatpak_installed(&id) {
        anyhow::bail!("flatpak app '{id}' is not installed");
    }
    let args = remove_command_args(&id, cfg.flatpak_noconfirm());
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let status = Command::new("flatpak")
        .args(&refs)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run flatpak uninstall: {e}"))?;
    if !status.success() {
        anyhow::bail!(
            "flatpak uninstall failed with exit {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Remove unused runtimes (`flatpak uninstall --unused`). No preview exists
/// server-side, so callers confirm first (same TTY/exit-2 contract as clean).
pub fn clean_unused_flatpak(cfg: &crate::config::Config) -> anyhow::Result<()> {
    if cfg.is_readonly() {
        anyhow::bail!("refusing to clean: readonly mode is enabled");
    }
    if !has_flatpak() {
        anyhow::bail!("flatpak not found — install flatpak to manage flatpak apps");
    }
    let args = unused_command_args(cfg.flatpak_noconfirm());
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let status = Command::new("flatpak")
        .args(&refs)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run flatpak uninstall --unused: {e}"))?;
    if !status.success() {
        anyhow::bail!(
            "flatpak uninstall --unused failed with exit {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// JSON row from `flatpak search --json`.
#[derive(Debug, Clone, Deserialize)]
struct FlatpakSearchRow {
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
    remotes: Option<String>,
}

/// Parse `flatpak search --json`. Pure, never panics: garbage -> empty, rows
/// without `application_id` skipped, empty version -> `"?"`. Remote relevance
/// order preserved (no re-sort); `limit` truncates (0 = no limit).
pub fn parse_flatpak_search_json(text: &str, limit: usize) -> Vec<Package> {
    let rows: Vec<FlatpakSearchRow> = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
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
        pkg.installed = false;
        pkg.description = r.description.or(r.name);
        if let Some(rem) = r
            .remotes
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            pkg.url = Some(format!("flatpak:{rem}"));
        }
        if let Some(b) = r
            .branch
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            && b != "stable"
        {
            pkg.version = format!("{} ({})", pkg.version, b);
        }
        out.push(pkg);
        if limit != 0 && out.len() >= limit {
            break;
        }
    }
    out
}

/// Live search: `flatpak search <query> --json`, bounded by
/// `cfg.search.timeout_secs`. Fail-open `Ok(empty)` when disabled, binary
/// missing, query empty, or timeout — mirrors AUR graceful degradation.
/// `use_regex` filters client-side on app-id + description (flatpak has no
/// server-side regex); invalid regex is an Err like the AUR path.
pub fn search_flatpak_with_config(
    query: &str,
    limit: usize,
    use_regex: bool,
    cfg: &crate::config::Config,
) -> anyhow::Result<Vec<Package>> {
    if !cfg.flatpak_enabled() || !has_flatpak() || query.trim().is_empty() {
        return Ok(vec![]);
    }
    let timeout = Duration::from_secs(cfg.search.timeout_secs.max(1));
    let Some(out) = run_flatpak(&["search", query.trim(), "--json"], timeout) else {
        return Ok(vec![]);
    };
    let mut pkgs = parse_flatpak_search_json(&out, limit);
    if use_regex {
        let re = regex::RegexBuilder::new(query.trim())
            .case_insensitive(true)
            .build()?;
        pkgs.retain(|p| {
            re.is_match(&p.name)
                || p.description
                    .as_deref()
                    .map(|d| re.is_match(d))
                    .unwrap_or(false)
        });
        if limit != 0 && pkgs.len() > limit {
            pkgs.truncate(limit);
        }
    }
    Ok(pkgs)
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

    const SEARCH_JSON: &str = r#"[
      {"name":"Firefox","description":"Fast, Private & Safe Web Browser","application_id":"org.mozilla.firefox","version":"158.0","branch":"stable","remotes":"flathub"},
      {"name":"Nvidia VAAPI driver","description":"VA-API","application_id":"org.freedesktop.Platform.VAAPI.nvidia","version":"","branch":"26.08","remotes":"flathub"},
      {"name":"NoId","version":"1.0"}
    ]"#;

    #[test]
    fn parse_search_json_fields_limit_order() {
        let pkgs = parse_flatpak_search_json(SEARCH_JSON, 0);
        assert_eq!(pkgs.len(), 2);
        // Remote relevance order preserved (firefox first).
        assert_eq!(pkgs[0].name, "org.mozilla.firefox");
        assert_eq!(pkgs[0].version, "158.0");
        assert!(!pkgs[0].installed);
        let vaapi = &pkgs[1];
        assert_eq!(vaapi.version, "? (26.08)");
        let one = parse_flatpak_search_json(SEARCH_JSON, 1);
        assert_eq!(one.len(), 1);
    }

    #[test]
    fn parse_search_json_garbage_is_empty() {
        assert!(parse_flatpak_search_json("", 0).is_empty());
        assert!(parse_flatpak_search_json("nope", 0).is_empty());
        assert!(parse_flatpak_search_json("null", 0).is_empty());
    }

    #[test]
    fn search_empty_query_is_empty_without_spawning() {
        let cfg = crate::config::Config::default();
        let pkgs = search_flatpak_with_config("   ", 50, false, &cfg).unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn search_disabled_is_empty() {
        let mut cfg = crate::config::Config::default();
        cfg.behavior.flatpak_enabled = false;
        let pkgs = search_flatpak_with_config("firefox", 50, false, &cfg).unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn search_invalid_regex_is_err() {
        if !has_flatpak() {
            return;
        }
        let cfg = crate::config::Config::default();
        assert!(search_flatpak_with_config("([", 50, true, &cfg).is_err());
    }

    #[test]
    fn validate_app_id_shapes() {
        assert!(validate_app_id("org.mozilla.firefox").is_ok());
        assert!(validate_app_id("  org.cutwire.Drift  ").is_ok());
        assert_eq!(
            validate_app_id("  org.cutwire.Drift  ").unwrap(),
            "org.cutwire.Drift"
        );
        for bad in [
            "",
            "   ",
            "firefox",
            "../evil",
            "a/b",
            "a\\b",
            ".leading",
            "trailing.",
            "has space",
            "semi;colon",
            "foo\0bar",
            "a..b",
        ] {
            assert!(validate_app_id(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn install_args_no_sudo_confirm_gated() {
        let plain = install_command_args("flathub", "org.mozilla.firefox", false);
        assert_eq!(plain, vec!["install", "flathub", "org.mozilla.firefox"]);
        assert!(!plain.iter().any(|a| a == "sudo"));
        let nc = install_command_args("flathub", "org.mozilla.firefox", true);
        assert_eq!(nc.last().map(|s| s.as_str()), Some("-y"));
    }

    #[test]
    fn remove_args_no_sudo_confirm_gated() {
        let plain = remove_command_args("org.mozilla.firefox", false);
        assert_eq!(plain, vec!["uninstall", "org.mozilla.firefox"]);
        assert!(!plain.iter().any(|a| a == "sudo"));
        let nc = remove_command_args("org.mozilla.firefox", true);
        assert!(nc.contains(&"-y".to_string()));
        let unused = unused_command_args(true);
        assert_eq!(unused, vec!["uninstall", "--unused", "-y"]);
        assert!(!unused_command_args(false).contains(&"-y".to_string()));
    }

    #[test]
    fn readonly_refuses_mutation_without_spawning() {
        let mut cfg = crate::config::Config::default();
        cfg.behavior.readonly = true;
        assert!(install_flatpak_package("org.mozilla.firefox", &cfg).is_err());
        assert!(remove_flatpak_package("org.mozilla.firefox", &cfg).is_err());
        assert!(clean_unused_flatpak(&cfg).is_err());
    }

    #[test]
    fn invalid_id_never_spawns() {
        let cfg = crate::config::Config::default();
        assert!(install_flatpak_package("not-an-app-id", &cfg).is_err());
        assert!(remove_flatpak_package("", &cfg).is_err());
        assert!(!is_flatpak_installed("not-an-app-id"));
        assert!(!is_flatpak_installed("definitely.not.installed.xyz"));
    }

    #[test]
    fn backend_trait_delegates_to_runners() {
        // Invalid id errors surface through the trait (no silent success).
        let cfg = crate::config::Config::default();
        let b = FlatpakBackend::new("flathub".to_string(), true);
        assert!(b.install("bad", &cfg).is_err());
        assert!(b.remove("bad", &cfg).is_err());
        assert!(b.check_updates().unwrap().is_empty());
        assert!(b.search("", 10, false, &cfg).unwrap().is_empty());
    }
}
