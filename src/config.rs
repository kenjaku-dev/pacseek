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
            floor_height: 13,
            layout_search: 3,
            layout_results_min: 8,
            border: "rounded".into(),
            highlight_symbol: None,
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
# See: https://github.com/achraf/pacseek  — all fields optional

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
floor_height = 13
layout_search = 3
layout_results_min = 8
border = "rounded"         # rounded|plain|double|thick
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

[behavior]
bottom_up = false
verbose = 0
# aur_rpc = "https://aur.archlinux.org/rpc/v5"
# cache_dir = "/tmp/pacseek"
makepkg_noconfirm = false
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
    for token in s.split([' ', ',', '+']) {
        let t = token.trim().to_lowercase();
        match t.as_str() {
            "" | "none" => {}
            "reset" => style = Style::default(),
            "bold" => style = style.add_modifier(Modifier::BOLD),
            "dim" => style = style.add_modifier(Modifier::DIM),
            "italic" => style = style.add_modifier(Modifier::ITALIC),
            "underlined" | "underline" => style = style.add_modifier(Modifier::UNDERLINED),
            "reversed" | "reverse" | "invert" => style = style.add_modifier(Modifier::REVERSED),
            "crossedout" | "crossed_out" => style = style.add_modifier(Modifier::CROSSED_OUT),
            "black" | "red" | "green" | "yellow" | "blue" | "magenta" | "bright_magenta"
            | "cyan" | "gray" | "grey" | "dark_gray" | "darkgray" | "white" => {
                style = style.fg(parse_color(&t));
            }
            _ => {}
        }
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
}
