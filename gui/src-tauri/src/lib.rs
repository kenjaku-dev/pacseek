//! pakmap GUI backend: Tauri commands over the pakmap core library.
//!
//! AB phase: thin spike — every command delegates to `pakmap::*`; the IPC
//! contract (commands, streaming events, error envelope, generation guard)
//! is specified in `docs/gui-architecture.md` and expanded in phase AC.

use pakmap::model::Package;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConfigSummary {
    /// GUI package version
    pub app_version: &'static str,
    /// Configured default result limit (`search.limit`)
    pub limit: usize,
    /// Configured default source precedence (`search.source`)
    pub source: String,
    /// Configured search timeout seconds (`search.timeout_secs`)
    pub timeout_secs: u64,
    /// `behavior.flatpak_enabled`
    pub flatpak_enabled: bool,
    /// `behavior.aur_rpc` overridden (custom mirror)
    pub custom_aur_rpc: bool,
}

#[derive(Serialize)]
pub struct PackageDto {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub repo: String,
    pub installed: bool,
}

impl From<&Package> for PackageDto {
    fn from(p: &Package) -> Self {
        Self {
            name: p.name.clone(),
            version: p.version.clone(),
            description: p.description.clone(),
            repo: p.repo.clone(),
            installed: p.installed,
        }
    }
}

/// IPC contract: `{ code, message, hint }` — mirrors the CLI's stderr hints
/// so the frontend never invents its own copy of recovery text.
#[derive(Serialize, Clone)]
pub struct GuiError {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
}

fn gui_error(code: &str, message: String, hint: Option<String>) -> GuiError {
    GuiError {
        code: code.to_string(),
        message,
        hint,
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from pakmap-gui!")
}

/// Resolved-effective config summary (user file with fail-open defaults).
#[tauri::command]
fn get_config_summary() -> ConfigSummary {
    let cfg = pakmap::config::Config::load();
    ConfigSummary {
        app_version: env!("CARGO_PKG_VERSION"),
        limit: cfg.search.limit,
        source: cfg.search.source.clone(),
        timeout_secs: cfg.search.timeout_secs,
        flatpak_enabled: cfg.behavior.flatpak_enabled,
        custom_aur_rpc: cfg.behavior.aur_rpc.is_some(),
    }
}

/// AB spike: synchronous repo-only local-database search (no network) proving
/// the IPC round trip through the core lib. The real staged streaming search
/// (repo/aur/flatpak legs + generation guard) is phase AC.
#[tauri::command]
fn search_once(query: &str) -> Result<Vec<PackageDto>, GuiError> {
    let query = query.trim();
    if query.is_empty() {
        return Err(gui_error("empty_query", "empty query".to_string(), None));
    }
    let pkgs = pakmap::search::repo::search_repo(query, 8, false, false)
        .map_err(|e| gui_error("repo_search_failed", e.to_string(), None))?;
    Ok(pkgs.iter().map(PackageDto::from).collect())
}

/// AB spike smoke hook: `PAKMAP_GUI_SMOKE_MS=<ms>` auto-quits after the delay
/// so `cargo tauri build` artifacts can be launched in scripts/xvfb CI
/// without a human closing the window. Same pattern as `PAKMAP_SKIP_LIVE`.
fn arm_smoke_timer() {
    if let Some(ms) = std::env::var("PAKMAP_GUI_SMOKE_MS")
        .ok()
        .and_then(|ms| ms.parse::<u64>().ok())
    {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            std::process::exit(0);
        });
    }
}

/// App entry: no unwrap/expect — a failed startup prints and exits 1
/// (the run loop's error is terminal anyway).
pub fn run() {
    arm_smoke_timer();
    if let Err(e) = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            get_config_summary,
            search_once
        ])
        .run(tauri::generate_context!())
    {
        eprintln!("pakmap-gui exited: {e}");
        std::process::exit(1);
    }
}
