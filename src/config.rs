use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;
use serde::{Deserialize, Serialize};

// ---- helpers ---------------------------------------------------------------

fn default_limit() -> usize {
    50
}
fn default_source() -> String {
    "all".into()
}
fn default_by() -> String {
    "name-desc".into()
}
fn default_timeout() -> u64 {
    15
}
fn default_true() -> bool {
    true
}
fn default_border() -> String {
    "rounded".into()
}
fn default_remove_flags() -> String {
    "Rs".into()
}
fn default_repo_local() -> String {
    "green bold".into()
}
fn default_tab_selected() -> String {
    "black on cyan bold".into()
}
fn default_tab_normal() -> String {
    "dark_gray".into()
}

// ---- top-level -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub search: SearchConfig,
    pub tui: TuiConfig,
    pub theme: ThemeConfig,
    pub behavior: BehaviorConfig,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            search: SearchConfig::default(),
            tui: TuiConfig::default(),
            theme: ThemeConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_by")]
    pub by: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub installed_only: bool,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub bottom_up: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            limit: 50,
            source: "all".into(),
            by: "name-desc".into(),
            regex: false,
            installed_only: false,
            timeout_secs: 15,
            bottom_up: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub floor_width: u16,
    pub floor_height: u16,
    pub layout_search: u16,
    pub layout_results_min: u16,
    #[serde(default = "default_border")]
    pub border: String,
    #[serde(default)]
    pub highlight_symbol: Option<String>,
    #[serde(default = "default_true")]
    pub show_tabs: bool,
    pub popup_info: Option<[u16; 2]>,
    pub popup_confirm: Option<[u16; 2]>,
    pub popup_message: Option<[u16; 2]>,
    #[serde(default)]
    pub tick_chars: Option<String>,
    #[serde(default)]
    pub spinner_template: Option<String>,
    pub debounce_ms: u64,
    pub poll_ms: u64,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            floor_width: 60,
            floor_height: 14,
            layout_search: 3,
            layout_results_min: 8,
            border: "rounded".into(),
            highlight_symbol: None,
            show_tabs: true,
            popup_info: Some([70, 60]),
            popup_confirm: Some([60, 30]),
            popup_message: Some([60, 20]),
            tick_chars: None,
            spinner_template: None,
            debounce_ms: 400,
            poll_ms: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    #[serde(default)]
    pub no_color: bool,
    pub border_focused: String,
    pub border_unfocused: String,
    pub border_error: String,
    pub text: String,
    pub text_dim: String,
    pub repo_core: String,
    pub repo_extra: String,
    pub repo_multilib: String,
    pub repo_aur: String,
    pub repo_other: String,
    #[serde(default = "default_repo_local")]
    pub repo_local: String,
    pub installed: String,
    pub out_of_date: String,
    pub votes: String,
    pub version: String,
    pub highlight_bg: String,
    pub highlight_fg: String,
    pub status_loading: String,
    pub status_idle: String,
    pub help: String,
    pub popup_title: String,
    pub popup_bg: String,
    #[serde(default = "default_tab_selected")]
    pub tab_selected: String,
    #[serde(default = "default_tab_normal")]
    pub tab_normal: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            no_color: false,
            border_focused: "cyan".into(),
            border_unfocused: "dark_gray".into(),
            border_error: "red bold".into(),
            text: "white".into(),
            text_dim: "dark_gray".into(),
            repo_core: "red bold".into(),
            repo_extra: "green bold".into(),
            repo_multilib: "blue bold".into(),
            repo_aur: "magenta bold".into(),
            repo_other: "cyan bold".into(),
            repo_local: "green bold".into(),
            installed: "green bold".into(),
            out_of_date: "red bold".into(),
            votes: "yellow".into(),
            version: "white".into(),
            highlight_bg: "dark_gray".into(),
            highlight_fg: "white bold".into(),
            status_loading: "cyan".into(),
            status_idle: "gray".into(),
            help: "dark_gray".into(),
            popup_title: "yellow".into(),
            popup_bg: "black".into(),
            tab_selected: "black on cyan bold".into(),
            tab_normal: "dark_gray".into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    #[serde(default)]
    pub bottom_up: bool,
    #[serde(default)]
    pub verbose: u8,
    #[serde(default)]
    pub aur_rpc: Option<String>,
    #[serde(default)]
    pub cache_dir: Option<String>,
    #[serde(default)]
    pub makepkg_noconfirm: bool,
    #[serde(default = "default_remove_flags")]
    pub remove_flags: String,
    #[serde(default)]
    pub remove_noconfirm: bool,
}

// ---- path & load -----------------------------------------------------------

impl Config {
    /// XDG path: $XDG_CONFIG_HOME/pacseek/config.toml or ~/.config/pacseek/config.toml
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("pacseek").join("config.toml"))
    }

    /// Project-local override: ./pacseek.toml or ./pacseek/config.toml (cwd)
    pub fn project_path() -> Option<PathBuf> {
        let cwd = std::env::current_dir().ok()?;
        [
            cwd.join("pacseek.toml"),
            cwd.join("pacseek").join("config.toml"),
        ]
        .into_iter()
        .find(|cand| cand.exists())
    }

    pub fn load() -> Self {
        if let Some(p) = Self::project_path() {
            if let Some(c) = Self::load_from(&p) {
                return c;
            }
        }
        if let Some(p) = Self::default_path() {
            if let Some(c) = Self::load_from(&p) {
                return c;
            }
        }
        Self::default()
    }

    pub fn load_from(path: &Path) -> Option<Self> {
        let s = std::fs::read_to_string(path).ok()?;
        match toml::from_str::<Self>(&s) {
            Ok(mut cfg) => {
                // normalize: empty strings keep defaults
                if cfg.search.source.trim().is_empty() {
                    cfg.search.source = SearchConfig::default().source;
                }
                if cfg.search.by.trim().is_empty() {
                    cfg.search.by = SearchConfig::default().by;
                }
                Some(cfg)
            }
            Err(e) => {
                eprintln!("warn: invalid config {}: {}", path.display(), e);
                None
            }
        }
    }

    /// Write example file to default_path (create dirs). Returns path or error string.
    pub fn init_example() -> Result<PathBuf, String> {
        let path = Self::default_path().ok_or_else(|| "no config dir".to_string())?;
        if path.exists() {
            return Err(format!("already exists: {}", path.display()));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let example = Self::example_toml();
        std::fs::write(&path, example).map_err(|e| e.to_string())?;
        Ok(path)
    }

    pub fn example_toml() -> String {
        r#"# pacseek config — edit and save, changes apply on next launch
# See: https://github.com/kenjaku-dev/pacseek  — all fields optional

[search]
limit = 50                 # 0 = no limit
source = "all"             # aur|repo|all
by = "name-desc"           # name|name-desc|maintainer|depends|makedepends|optdepends|checkdepends
regex = false
installed_only = false
timeout_secs = 15
bottom_up = false

[tui]
floor_width = 60
floor_height = 14          # 13 + 1 tabs bar (Search/Installed)
layout_search = 3
layout_results_min = 8
border = "rounded"         # rounded|plain|double|thick
show_tabs = true           # Tab bar on top: Search / Installed
highlight_symbol = "▸ "   # set "" to disable
popup_info = [70, 60]
popup_confirm = [60, 30]
popup_message = [60, 20]
# tick_chars = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "
# spinner_template = "{spinner:.cyan} {msg}"
debounce_ms = 400
poll_ms = 200

[theme]
no_color = false
border_focused = "cyan"
border_unfocused = "dark_gray"
border_error = "red bold"
text = "white"
text_dim = "dark_gray"
repo_core = "red bold"
repo_extra = "green bold"
repo_multilib = "blue bold"
repo_aur = "magenta bold"
repo_other = "cyan bold"
repo_local = "green bold"
installed = "green bold"
out_of_date = "red bold"
votes = "yellow"
version = "white"
highlight_bg = "dark_gray"
highlight_fg = "white bold"
status_loading = "cyan"
status_idle = "gray"
help = "dark_gray"
popup_title = "yellow"
popup_bg = "black"
tab_selected = "black on cyan bold"
tab_normal = "dark_gray"

[behavior]
bottom_up = false
verbose = 0
# aur_rpc = "https://aur.archlinux.org/rpc/v5"
# cache_dir = "/tmp/pacseek"
makepkg_noconfirm = false
remove_flags = "Rs"        # R|Rs|Rns|Ru — sudo pacman -<flags>
remove_noconfirm = false   # add --noconfirm to remove
"#
        .into()
    }
}

// ---- theme helpers ---------------------------------------------------------

pub fn parse_color(s: &str) -> Color {
    match s.trim().to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "bright_magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "darkgray" | "dark-grey" => Color::DarkGray,
        "white" => Color::White,
        "reset" => Color::Reset,
        _ => Color::Reset,
    }
}

pub fn parse_style(s: &str) -> Style {
    let mut style = Style::default();
    // Support "fg on bg modifiers", e.g. "black on cyan bold", plus legacy "cyan bold".
    // Tokens split on space/comma/plus; "on" consumes the next color token as background.
    let tokens: Vec<String> = s
        .split([' ', ',', '+'])
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        match t {
            "none" => {}
            "reset" => style = Style::default(),
            "bold" => style = style.add_modifier(Modifier::BOLD),
            "dim" => style = style.add_modifier(Modifier::DIM),
            "italic" => style = style.add_modifier(Modifier::ITALIC),
            "underlined" | "underline" => style = style.add_modifier(Modifier::UNDERLINED),
            "reversed" | "reverse" | "invert" => style = style.add_modifier(Modifier::REVERSED),
            "crossedout" | "crossed_out" => style = style.add_modifier(Modifier::CROSSED_OUT),
            "on" => {
                // Next token is background color
                if let Some(bg) = tokens.get(i + 1) {
                    style = style.bg(parse_color(bg));
                    i += 1;
                }
            }
            _ if t.starts_with("on:") || t.starts_with("bg:") => {
                if let Some((_, c)) = t.split_once(':') {
                    style = style.bg(parse_color(c));
                }
            }
            _ if t.starts_with("on") && t.len() > 2 => {
                // "oncyan", "on_cyan", "on-cyan"
                let c = t
                    .trim_start_matches("on")
                    .trim_start_matches(['_', '-', ':']);
                if !c.is_empty() {
                    style = style.bg(parse_color(c));
                }
            }
            "black" | "red" | "green" | "yellow" | "blue" | "magenta" | "bright_magenta"
            | "cyan" | "gray" | "grey" | "dark_gray" | "darkgray" | "white" => {
                style = style.fg(parse_color(t));
            }
            _ => {}
        }
        i += 1;
    }
    style
}

pub fn border_type_from_str(s: &str) -> BorderType {
    match s.trim().to_lowercase().as_str() {
        "plain" | "line" => BorderType::Plain,
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        _ => BorderType::Rounded,
    }
}

/// Pre-parsed theme styles — built once per App to avoid per-frame `parse_style()` cost.
/// Per ratatui perf: parsing strings per row per frame is wasteful for 50+ rows at 5fps poll.
#[derive(Debug, Clone)]
pub struct ThemeStyles {
    pub border_focused: Style,
    pub border_unfocused: Style,
    pub border_error: Style,
    pub text: Style,
    pub text_dim: Style,
    pub repo_core: Style,
    pub repo_extra: Style,
    pub repo_multilib: Style,
    pub repo_aur: Style,
    pub repo_other: Style,
    pub repo_local: Style,
    pub installed: Style,
    pub out_of_date: Style,
    pub votes: Style,
    pub version: Style,
    pub highlight: Style,
    pub status_loading: Style,
    pub status_idle: Style,
    pub help: Style,
    pub popup_title: Style,
    pub popup_bg: Style,
    pub tab_selected: Style,
    pub tab_normal: Style,
}

impl From<&ThemeConfig> for ThemeStyles {
    fn from(t: &ThemeConfig) -> Self {
        let highlight_bg = parse_color(&t.highlight_bg);
        let fg_style = parse_style(&t.highlight_fg);
        let mut highlight = Style::default().bg(highlight_bg);
        if let Some(c) = fg_style.fg {
            highlight = highlight.fg(c);
        }
        highlight = highlight.add_modifier(fg_style.add_modifier);
        Self {
            border_focused: parse_style(&t.border_focused),
            border_unfocused: parse_style(&t.border_unfocused),
            border_error: parse_style(&t.border_error),
            text: parse_style(&t.text),
            text_dim: parse_style(&t.text_dim),
            repo_core: parse_style(&t.repo_core),
            repo_extra: parse_style(&t.repo_extra),
            repo_multilib: parse_style(&t.repo_multilib),
            repo_aur: parse_style(&t.repo_aur),
            repo_other: parse_style(&t.repo_other),
            repo_local: parse_style(&t.repo_local),
            installed: parse_style(&t.installed),
            out_of_date: parse_style(&t.out_of_date),
            votes: parse_style(&t.votes),
            version: parse_style(&t.version),
            highlight,
            status_loading: parse_style(&t.status_loading),
            status_idle: parse_style(&t.status_idle),
            help: parse_style(&t.help),
            popup_title: parse_style(&t.popup_title),
            popup_bg: parse_style(&t.popup_bg),
            tab_selected: parse_style(&t.tab_selected),
            tab_normal: parse_style(&t.tab_normal),
        }
    }
}

pub fn parse_source(s: &str) -> crate::cli::Source {
    match s.trim().to_lowercase().as_str() {
        "aur" => crate::cli::Source::Aur,
        "repo" => crate::cli::Source::Repo,
        _ => crate::cli::Source::All,
    }
}

pub fn parse_aur_by(s: &str) -> crate::cli::AurBy {
    match s.trim().to_lowercase().as_str() {
        "name" => crate::cli::AurBy::Name,
        "maintainer" => crate::cli::AurBy::Maintainer,
        "depends" => crate::cli::AurBy::Depends,
        "makedepends" => crate::cli::AurBy::Makedepends,
        "optdepends" => crate::cli::AurBy::Optdepends,
        "checkdepends" => crate::cli::AurBy::Checkdepends,
        _ => crate::cli::AurBy::NameDesc,
    }
}

impl Config {
    pub fn effective_source(&self) -> crate::cli::Source {
        parse_source(&self.search.source)
    }
    pub fn effective_aur_by(&self) -> crate::cli::AurBy {
        parse_aur_by(&self.search.by)
    }
    pub fn aur_rpc_url(&self) -> String {
        self.behavior
            .aur_rpc
            .clone()
            .unwrap_or_else(|| "https://aur.archlinux.org/rpc/v5".into())
    }
    pub fn cache_dir_path(&self) -> Option<PathBuf> {
        self.behavior.cache_dir.as_ref().map(PathBuf::from)
    }
    /// Validated pacman remove flag, e.g. "Rs" -> "-Rs". Allowlist prevents injection
    /// via config file. Allowed: R, Rs, Rns, Ru, Rsu, Rn, Rc, Rdd (expert).
    pub fn remove_flag_arg(&self) -> String {
        let f = self.behavior.remove_flags.trim().trim_start_matches('-');
        match f {
            "R" | "Rs" | "Rns" | "Ru" | "Rsu" | "Rn" | "Rc" | "Rdd" => format!("-{f}"),
            _ => "-Rs".into(),
        }
    }
    pub fn remove_noconfirm(&self) -> bool {
        self.behavior.remove_noconfirm || std::env::var("PACSEEK_NOCONFIRM").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrips() {
        let cfg = Config::default();
        let s = toml::to_string(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back.search.limit, 50);
        assert_eq!(back.tui.floor_width, 60);
    }

    #[test]
    fn parse_style_basic() {
        let st = parse_style("cyan bold");
        assert_eq!(st.fg, Some(Color::Cyan));
        assert!(st.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn invalid_toml_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.toml");
        std::fs::write(&p, "[[broken").unwrap();
        assert!(Config::load_from(&p).is_none());
    }

    #[test]
    fn empty_strings_keep_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.toml");
        std::fs::write(&p, "[search]\nsource=\"\"\nby=\"\"\n").unwrap();
        let cfg = Config::load_from(&p).unwrap();
        assert_eq!(cfg.search.source, "all");
        assert_eq!(cfg.search.by, "name-desc");
    }

    #[test]
    fn parse_style_with_bg() {
        let st = parse_style("black on cyan bold");
        assert_eq!(st.fg, Some(Color::Black));
        assert_eq!(st.bg, Some(Color::Cyan));
        assert!(st.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn remove_flag_allowlist() {
        let mut cfg = Config::default();
        assert_eq!(cfg.remove_flag_arg(), "-Rs");
        cfg.behavior.remove_flags = "Rns".into();
        assert_eq!(cfg.remove_flag_arg(), "-Rns");
        cfg.behavior.remove_flags = "--noconfirm; rm -rf".into();
        assert_eq!(cfg.remove_flag_arg(), "-Rs");
    }

    #[test]
    fn theme_styles_cache_builds() {
        let cfg = Config::default();
        let styles = ThemeStyles::from(&cfg.theme);
        assert_eq!(styles.text.fg, Some(Color::White));
        assert_eq!(styles.tab_selected.bg, Some(Color::Cyan));
    }
}
