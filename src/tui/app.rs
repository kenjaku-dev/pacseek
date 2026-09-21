use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    widgets::{BorderType, ListState},
};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self},
};
use std::thread;
use std::time::{Duration, Instant};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::cli::{AurBy, Source};
use crate::config::{Config, ThemeStyles};
use crate::model::Package;
use crate::search::aur::search_aur_blocking_with_config;

use super::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Search,
    List,
}

/// TUI mode — Tab switches between installing new packages and removing installed ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Search,
    Installed,
}

impl Mode {
    pub fn toggle(self) -> Self {
        match self {
            Mode::Search => Mode::Installed,
            Mode::Installed => Mode::Search,
        }
    }
    pub fn tab_index(self) -> usize {
        match self {
            Mode::Search => 0,
            Mode::Installed => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    None,
    Info(Package),
    Confirm(Package),
    ConfirmRemove(Package),
    ConfirmRefresh,
    Help,
    Message(String),
}

pub struct App {
    pub input: Input,
    pub packages: Vec<Package>,
    pub list_state: ListState,
    pub focus: Focus,
    pub mode: Mode,
    pub popup: Popup,
    pub is_loading: bool,
    pub status: String,
    pub initial_query: String,
    pub last_query: String,
    /// Last query per mode — switching tabs restores each mode's results without refetch.
    pub last_query_search: String,
    pub last_query_installed: String,
    pub limit: usize,
    pub source: Source,
    pub aur_by: AurBy,
    pub use_regex: bool,
    pub installed_only: bool,
    pub no_color: bool,
    pub should_quit: bool,
    pub needs_search: bool,
    pub last_input_change: Instant,
    pub config: Config,
    pub styles: ThemeStyles,
    /// Cached border type — parsed once from config instead of per-widget per-frame.
    pub border_type: BorderType,
    // Phase B: non-blocking search channel + dedup id per ratatui async skill
    search_rx: Option<std::sync::mpsc::Receiver<(u64, Vec<Package>, String)>>,
    search_id: u64,
    next_search_id: u64,
    /// Small FIFO result cache (std-only) — retyping / tab-switching a recent
    /// query serves instantly with zero disk/network. Cleared on install/remove.
    search_cache: HashMap<String, Vec<Package>>,
    search_cache_order: VecDeque<String>,
    /// Redraw only when state changed (or while loading) instead of every poll tick.
    dirty: bool,
}

/// Max cached search results (each entry ≤ limit packages).
const SEARCH_CACHE_CAP: usize = 32;

impl App {
    pub fn new(initial_query: String) -> Self {
        let mut input = Input::default();
        if !initial_query.is_empty() {
            // pre-fill input
            for ch in initial_query.chars() {
                input.handle_event(&Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Char(ch),
                    KeyModifiers::empty(),
                )));
            }
        }
        let mut list_state = ListState::default();
        list_state.select(None);
        let config = Config::default();
        let styles = ThemeStyles::from(&config.theme);
        let border_type = crate::config::border_type_from_str(&config.tui.border);
        Self {
            input,
            packages: Vec::new(),
            list_state,
            focus: if initial_query.is_empty() {
                Focus::Search
            } else {
                Focus::List
            },
            mode: Mode::Search,
            popup: Popup::None,
            is_loading: false,
            status: "Type to search, Enter to search, Tab remover, ? help".into(),
            initial_query: initial_query.clone(),
            last_query: String::new(),
            last_query_search: String::new(),
            last_query_installed: String::new(),
            limit: 50,
            source: Source::All,
            aur_by: AurBy::NameDesc,
            use_regex: false,
            installed_only: false,
            no_color: std::env::var("NO_COLOR").is_ok(),
            should_quit: false,
            needs_search: !initial_query.is_empty(),
            last_input_change: Instant::now(),
            config,
            styles,
            border_type,
            search_rx: None,
            search_id: 0,
            next_search_id: 0,
            search_cache: HashMap::new(),
            search_cache_order: VecDeque::new(),
            dirty: true,
        }
    }

    pub fn new_with_cli(initial_query: String, cli: &crate::cli::Cli) -> Self {
        let mut app = Self::new(initial_query);
        app.limit = cli.limit;
        app.source = cli.source;
        app.aur_by = cli.by;
        app.use_regex = cli.regex;
        app.installed_only = cli.installed_only;
        app.no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();
        app
    }

    pub fn new_with_config(initial_query: String, cli: &crate::cli::Cli, cfg: &Config) -> Self {
        let mut app = Self::new(initial_query);
        // CLI already merged with cfg in main.rs, but keep cfg for theme/layout
        app.limit = cli.limit;
        app.source = cli.source;
        app.aur_by = cli.by;
        app.use_regex = cli.regex;
        app.installed_only = cli.installed_only;
        app.no_color = cli.no_color || cfg.theme.no_color || std::env::var("NO_COLOR").is_ok();
        app.config = cfg.clone();
        app.styles = ThemeStyles::from(&cfg.theme);
        app.border_type = crate::config::border_type_from_str(&cfg.tui.border);
        app
    }

    fn cache_key(&self, query: &str) -> String {
        format!(
            "{:?}|{}|{}|{:?}|{:?}|{}|{}",
            self.mode,
            query,
            self.limit,
            self.source,
            self.aur_by,
            self.use_regex,
            self.installed_only
        )
    }

    fn cache_get(&self, key: &str) -> Option<Vec<Package>> {
        self.search_cache.get(key).cloned()
    }

    fn cache_put(&mut self, key: String, pkgs: Vec<Package>) {
        if !self.search_cache.contains_key(&key) {
            while self.search_cache_order.len() >= SEARCH_CACHE_CAP {
                match self.search_cache_order.pop_front() {
                    Some(old) => {
                        self.search_cache.remove(&old);
                    }
                    None => break,
                }
            }
            self.search_cache_order.push_back(key.clone());
        }
        self.search_cache.insert(key, pkgs);
    }

    fn clear_cache(&mut self) {
        self.search_cache.clear();
        self.search_cache_order.clear();
    }

    /// Apply freshly fetched results: selection, status text, per-mode memory.
    fn apply_results(&mut self, pkgs: Vec<Package>, query: String) {
        let total = pkgs.len();
        self.packages = pkgs;
        self.list_state.select(if self.packages.is_empty() {
            None
        } else {
            Some(0)
        });
        self.is_loading = false;
        self.last_query = query.clone();
        if self.mode == Mode::Installed {
            // Remember per-mode query for Tab restore
            self.last_query_installed = query.clone();
            if total == 0 {
                self.status = if query.is_empty() {
                    "No installed packages".into()
                } else {
                    format!("No installed match for '{}'", query)
                };
            } else {
                self.status = format!("Installed {} (local {})", total, total);
            }
        } else {
            self.last_query_search = query.clone();
            let repo_count = self.packages.iter().filter(|p| p.repo != "aur").count();
            let aur_count = total.saturating_sub(repo_count);
            if total == 0 {
                self.status = format!("No results for '{}'", query);
            } else {
                self.status = format!("Found {} (repo {} aur {})", total, repo_count, aur_count);
            }
        }
        self.dirty = true;
    }

    /// Switch Search <-> Installed (Tab). Clears list, restores per-mode hint,
    /// and triggers a fresh search (Installed with empty filter lists all).
    pub fn switch_mode(&mut self) {
        self.mode = self.mode.toggle();
        // Remember per-mode queries
        if self.mode == Mode::Installed {
            self.last_query_search = self.last_query.clone();
            self.last_query = self.last_query_installed.clone();
        } else {
            self.last_query_installed = self.last_query.clone();
            self.last_query = self.last_query_search.clone();
        }
        self.packages.clear();
        self.list_state.select(None);
        self.popup = Popup::None;
        self.search_rx = None;
        self.status = match self.mode {
            Mode::Search => "Type to search, Enter to search, Tab remover, ? help".into(),
            Mode::Installed => {
                "Installed mode — type to filter, Enter to remove, Tab back, ? help".into()
            }
        };
        // Installed with empty filter should list all; Search with empty clears.
        let should_fetch = self.mode == Mode::Installed || !self.input.value().trim().is_empty();
        if should_fetch {
            self.is_loading = true;
            self.trigger_search();
        } else {
            self.is_loading = false;
        }
        self.needs_search = false;
        self.dirty = true;
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // Best-effort signal handling — per tui-design lifecycle, ensure raw mode restored on SIGTERM/SIGINT
        let term_flag = Arc::new(AtomicBool::new(false));
        #[cfg(unix)]
        {
            let flag = term_flag.clone();
            let _ = signal_hook::flag::register(signal_hook::consts::SIGTERM, flag.clone());
            let _ = signal_hook::flag::register(signal_hook::consts::SIGINT, flag);
            let _ = signal_hook::flag::register(signal_hook::consts::SIGHUP, term_flag.clone());
        }

        // If initial query provided, do first search immediately (no debounce) — now non-blocking
        if self.needs_search {
            self.trigger_search();
            self.needs_search = false;
        }

        while !self.should_quit {
            if term_flag.load(Ordering::Relaxed) {
                self.should_quit = true;
                break;
            }

            // Poll for background search results before draw so spinner updates immediately
            self.poll_search_results();

            // Redraw only on state change (or while a search is in flight),
            // instead of every poll tick — idle frames cost zero ListItem rebuilds.
            if self.dirty || self.is_loading {
                terminal.draw(|f| ui::draw(f, self))?;
                self.dirty = false;
            }

            // poll with timeout to allow debounce & spinner & signal check
            let poll_ms = self.config.tui.poll_ms.max(50);
            if event::poll(Duration::from_millis(poll_ms))? {
                let ev = event::read()?;
                if self.handle_popup_event(&ev, terminal)? {
                    self.dirty = true;
                    continue;
                }
                self.handle_event(&ev, terminal)?;
                self.dirty = true;
            }

            // Also poll after handling event for immediate fetch after Enter
            self.poll_search_results();

            // Debounced search: if input changed and debounce passed without new key
            let debounce = Duration::from_millis(self.config.tui.debounce_ms.max(50));
            if self.focus == Focus::Search
                && self.needs_search
                && self.last_input_change.elapsed() > debounce
            {
                self.trigger_search();
                self.needs_search = false;
            }
        }
        Ok(())
    }

    fn poll_search_results(&mut self) {
        // Drain pending results first so method calls below don't fight the
        // channel borrow (only the latest search_id is accepted).
        let pending: Vec<(u64, Vec<Package>, String)> = match &self.search_rx {
            Some(rx) => {
                let mut v = Vec::new();
                while let Ok(msg) = rx.try_recv() {
                    v.push(msg);
                }
                v
            }
            None => return,
        };
        for (id, pkgs, query) in pending {
            // Only accept latest search_id; discard stale
            if id == self.search_id {
                let key = self.cache_key(&query);
                self.apply_results(pkgs.clone(), query);
                self.cache_put(key, pkgs);
            } else {
                // Stale result, ignore but keep is_loading if newer still pending
                // If this stale was the last expected, don't clear loading
                tracing::debug!(
                    id,
                    search_id = self.search_id,
                    "discard stale search result"
                );
            }
        }
    }

    fn handle_popup_event(&mut self, ev: &Event, terminal: &mut DefaultTerminal) -> Result<bool> {
        match &self.popup {
            Popup::None => Ok(false),
            _ => {
                if let Event::Key(k) = ev {
                    if k.kind != KeyEventKind::Press {
                        return Ok(true);
                    }
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.popup = Popup::None;
                        }
                        KeyCode::Char('?') => {
                            // Toggle help
                            if self.popup == Popup::Help {
                                self.popup = Popup::None;
                            }
                        }
                        KeyCode::Enter => {
                            // Confirm install / remove / refresh
                            match self.popup.clone() {
                                Popup::Confirm(pkg) => {
                                    self.popup = Popup::None;
                                    self.do_install(pkg, terminal)?;
                                }
                                Popup::ConfirmRemove(pkg) => {
                                    self.popup = Popup::None;
                                    self.do_remove(pkg, terminal)?;
                                }
                                Popup::ConfirmRefresh => {
                                    self.popup = Popup::None;
                                    self.do_refresh(terminal)?;
                                }
                                _ => {
                                    self.popup = Popup::None;
                                }
                            }
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') => match self.popup.clone() {
                            Popup::Confirm(pkg) => {
                                self.popup = Popup::None;
                                self.do_install(pkg, terminal)?;
                            }
                            Popup::ConfirmRemove(pkg) => {
                                self.popup = Popup::None;
                                self.do_remove(pkg, terminal)?;
                            }
                            Popup::ConfirmRefresh => {
                                self.popup = Popup::None;
                                self.do_refresh(terminal)?;
                            }
                            _ => {}
                        },
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            self.popup = Popup::None;
                        }
                        _ => {}
                    }
                }
                Ok(true)
            }
        }
    }

    fn handle_event(&mut self, ev: &Event, terminal: &mut DefaultTerminal) -> Result<()> {
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                // Tab switches Search <-> Installed (remover) from anywhere — never types.
                KeyCode::Tab | KeyCode::BackTab => {
                    self.switch_mode();
                }
                KeyCode::Char('?') if self.focus == Focus::List && self.popup == Popup::None => {
                    self.popup = Popup::Help;
                }
                KeyCode::Char('r') | KeyCode::Char('R')
                    if self.focus == Focus::List && self.popup == Popup::None =>
                {
                    self.popup = Popup::ConfirmRefresh;
                }
                KeyCode::F(5) if self.focus == Focus::List && self.popup == Popup::None => {
                    self.popup = Popup::ConfirmRefresh;
                }
                KeyCode::Char('q') if self.focus == Focus::List && self.popup == Popup::None => {
                    // In search focus, q should type, not quit. Only quit from list mode when not typing
                    // Check ctrl+c too
                    self.should_quit = true;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    // Popup already handled in handle_popup_event, so here popup==None
                    // Toggle focus between Search and List
                    if self.focus == Focus::Search {
                        self.focus = Focus::List;
                    } else {
                        self.focus = Focus::Search;
                    }
                }
                KeyCode::Char('/') => {
                    self.focus = Focus::Search;
                }
                KeyCode::Enter => {
                    if self.focus == Focus::Search {
                        // Trigger search and move to list
                        self.do_search(terminal)?;
                        self.focus = Focus::List;
                    } else if self.popup == Popup::None {
                        // Install or remove selected depending on mode
                        if let Some(idx) = self.list_state.selected() {
                            if let Some(pkg) = self.packages.get(idx).cloned() {
                                self.popup = match self.mode {
                                    Mode::Search => Popup::Confirm(pkg),
                                    Mode::Installed => Popup::ConfirmRemove(pkg),
                                };
                            }
                        }
                    }
                }
                KeyCode::Delete | KeyCode::Backspace
                    if self.mode == Mode::Installed
                        && self.focus == Focus::List
                        && self.popup == Popup::None =>
                {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(pkg) = self.packages.get(idx).cloned() {
                            self.popup = Popup::ConfirmRemove(pkg);
                        }
                    }
                }
                KeyCode::Char('d')
                | KeyCode::Char('D')
                | KeyCode::Char('x')
                | KeyCode::Char('X')
                    if self.mode == Mode::Installed
                        && self.focus == Focus::List
                        && self.popup == Popup::None =>
                {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(pkg) = self.packages.get(idx).cloned() {
                            self.popup = Popup::ConfirmRemove(pkg);
                        }
                    }
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    if self.focus == Focus::List && self.popup == Popup::None {
                        if let Some(idx) = self.list_state.selected() {
                            if let Some(pkg) = self.packages.get(idx).cloned() {
                                self.popup = Popup::Info(pkg);
                            }
                        }
                    } else if self.focus == Focus::Search {
                        // type i in search
                        self.input.handle_event(ev);
                        self.needs_search = true;
                        self.last_input_change = Instant::now();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.focus == Focus::List {
                        self.select_prev();
                    } else {
                        self.input.handle_event(ev);
                        self.needs_search = true;
                        self.last_input_change = Instant::now();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.focus == Focus::List {
                        self.select_next();
                    } else {
                        self.input.handle_event(ev);
                        self.needs_search = true;
                        self.last_input_change = Instant::now();
                    }
                }
                _ => {
                    // For search focus, forward all to tui-input; for list, only char input switches to search?
                    if self.focus == Focus::Search {
                        let prev = self.input.value().to_string();
                        self.input.handle_event(ev);
                        if prev != self.input.value() {
                            self.needs_search = true;
                            self.last_input_change = Instant::now();
                        }
                    } else {
                        // If typing in list mode (letter), jump to search and insert
                        if let KeyCode::Char(c) = key.code {
                            if !key.modifiers.contains(KeyModifiers::CONTROL)
                                && !key.modifiers.contains(KeyModifiers::ALT)
                            {
                                self.focus = Focus::Search;
                                self.input.handle_event(ev);
                                self.needs_search = true;
                                self.last_input_change = Instant::now();
                                let _ = c; // suppress unused
                            }
                        }
                    }
                }
            },
            Event::Resize(_, _) => {
                // ratatui will re-layout next draw, nothing extra
            }
            _ => {}
        }
        Ok(())
    }

    fn select_next(&mut self) {
        if self.packages.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.packages.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.packages.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.packages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    // Phase B: non-blocking — spawn background thread, parallel repo+aur via OnceLock client & cached Config
    // Phase C: respect CLI filters (source, by, regex, installed_only, limit) per tui audit
    // v0.3.0: Installed mode uses localdb (allows empty filter = list all); Search uses cached cfg for AUR.
    pub(crate) fn trigger_search(&mut self) {
        // Installed mode: empty filter lists all installed (up to limit)
        if self.mode == Mode::Installed {
            let query = self.input.value().trim().to_string();
            if query == self.last_query && !self.packages.is_empty() {
                self.is_loading = false;
                return;
            }
            // Serve recent filters instantly (no disk scan).
            let key = self.cache_key(&query);
            if let Some(pkgs) = self.cache_get(&key) {
                self.search_rx = None;
                self.apply_results(pkgs, query);
                return;
            }
            self.is_loading = true;
            self.dirty = true;
            self.status = if query.is_empty() {
                "Listing installed packages...".into()
            } else {
                format!("Filtering installed for '{}'...", query)
            };
            self.next_search_id = self.next_search_id.wrapping_add(1);
            self.search_id = self.next_search_id;
            let search_id = self.search_id;
            let limit = self.limit;
            let use_regex = self.use_regex;
            let query_clone = query.clone();
            let (tx, rx) = mpsc::channel();
            self.search_rx = Some(rx);
            thread::spawn(move || {
                let res = crate::search::repo::search_local(&query_clone, limit, use_regex)
                    .or_else(|_| crate::search::repo::search_local_fallback(&query_clone, limit))
                    .unwrap_or_default();
                let _ = tx.send((search_id, res, query_clone));
            });
            return;
        }

        let query = self.input.value().trim().to_string();
        if query.is_empty() {
            self.packages.clear();
            self.status = "Type a query and press Enter".into();
            self.is_loading = false;
            self.search_rx = None;
            self.dirty = true;
            return;
        }
        if query == self.last_query && !self.packages.is_empty() {
            self.is_loading = false;
            return;
        }
        // Serve recent queries instantly (no disk/network).
        let key = self.cache_key(&query);
        if let Some(pkgs) = self.cache_get(&key) {
            self.search_rx = None;
            self.apply_results(pkgs, query);
            return;
        }
        self.is_loading = true;
        self.dirty = true;
        self.status = format!("Searching for '{}'...", query);
        self.next_search_id = self.next_search_id.wrapping_add(1);
        self.search_id = self.next_search_id;
        let search_id = self.search_id;
        let limit = self.limit;
        let source = self.source;
        let aur_by = self.aur_by;
        let use_regex = self.use_regex;
        let installed_only = self.installed_only;
        let query_clone = query.clone();

        let (tx, rx) = mpsc::channel();
        self.search_rx = Some(rx);
        // Clone cached config once — avoids Config::load() file IO per keystroke in AUR path.
        let cfg = self.config.clone();

        thread::spawn(move || {
            let aur_by_str = aur_by.as_str().to_string();
            // Spawn repo and aur in parallel, respecting source filter
            let q1 = query_clone.clone();
            let repo_handle = if matches!(source, Source::Aur) {
                None
            } else {
                Some(thread::spawn(move || {
                    crate::search::repo::search_repo(&q1, limit, use_regex, installed_only)
                        .or_else(|_| crate::search::repo::search_repo_fallback(&q1, limit))
                        .unwrap_or_default()
                }))
            };
            let q2 = query_clone.clone();
            let aur_handle = if matches!(source, Source::Repo) || installed_only {
                None
            } else {
                Some(thread::spawn(move || {
                    search_aur_blocking_with_config(&q2, &aur_by_str, limit, use_regex, &cfg)
                        .unwrap_or_else(|e| {
                            tracing::warn!(err=?e, "AUR blocking search failed");
                            vec![]
                        })
                }))
            };

            let repo_res = repo_handle
                .map(|h| h.join().unwrap_or_default())
                .unwrap_or_default();
            let aur_res = aur_handle
                .map(|h| h.join().unwrap_or_default())
                .unwrap_or_default();

            let mut combined = Vec::with_capacity(repo_res.len() + aur_res.len());
            // Respect source order: repo first unless bottom_up handled in UI; keep repo+aur
            combined.extend(repo_res);
            combined.extend(aur_res);

            let _ = tx.send((search_id, combined, query_clone));
        });
    }

    fn do_search(&mut self, _terminal: &mut DefaultTerminal) -> Result<()> {
        // Keep API compat for handle_event and initial search; now non-blocking
        self.trigger_search();
        Ok(())
    }

    fn do_install(&mut self, pkg: Package, terminal: &mut DefaultTerminal) -> Result<()> {
        // Suspend TUI and run pacman / makepkg per aur-guides + tui-design lifecycle
        let pkg_clone = pkg.clone();
        let is_aur = pkg.repo == "aur";
        let name = pkg.name.clone();
        let name_for_closure = name.clone();

        // Prepare install function that runs outside TUI
        let install_result = super::super::tui::suspend_and_run(terminal, move || {
            if is_aur {
                crate::install::aur::install_aur_package(&pkg_clone)
            } else {
                crate::install::repo::install_repo_package(&name_for_closure)
            }
        });

        match install_result {
            Ok(()) => {
                self.status = format!("Installed {}", name);
                self.popup = Popup::Message(format!("✓ Installed {}", name));
            }
            Err(e) => {
                self.status = format!("Install failed: {}", e);
                self.popup = Popup::Message(format!("✗ Failed {}: {}", name, e));
            }
        }
        // Refresh package list to update [installed] marker — force even if query==last_query
        self.last_query.clear();
        self.clear_cache();
        self.dirty = true;
        self.trigger_search();
        Ok(())
    }

    fn do_remove(&mut self, pkg: Package, terminal: &mut DefaultTerminal) -> Result<()> {
        let name = pkg.name.clone();
        let name_for_closure = name.clone();
        let cfg = self.config.clone();
        let flag_preview = cfg.remove_flag_arg();

        let remove_result = super::super::tui::suspend_and_run(terminal, move || {
            crate::install::remove::remove_package_with_config(&name_for_closure, &cfg)
        });

        match remove_result {
            Ok(()) => {
                self.status = format!("Removed {} ({})", name, flag_preview);
                self.popup = Popup::Message(format!("✓ Removed {} ({})", name, flag_preview));
            }
            Err(e) => {
                self.status = format!("Remove failed: {}", e);
                self.popup = Popup::Message(format!("✗ Failed {}: {}", name, e));
            }
        }
        // Refresh installed list — force even if filter unchanged
        self.last_query.clear();
        self.last_query_installed.clear();
        self.clear_cache();
        self.dirty = true;
        self.trigger_search();
        Ok(())
    }

    fn do_refresh(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // Suspend TUI and run `sudo pacman -Sy` with inherited stdio (real TTY for sudo)
        let cfg = self.config.clone();
        let refresh_result =
            super::super::tui::suspend_and_run(terminal, move || {
                crate::install::refresh::refresh_sync_db(&cfg)
            });

        match refresh_result {
            Ok(()) => {
                self.status = "Sync databases refreshed".into();
                self.popup = Popup::Message("✓ Sync databases refreshed".into());
            }
            Err(e) => {
                self.status = format!("Refresh failed: {e}");
                self.popup = Popup::Message(format!("✗ Refresh failed: {e}"));
            }
        }
        // Sync DBs changed — force re-query even if query==last_query
        self.last_query.clear();
        self.last_query_search.clear();
        self.last_query_installed.clear();
        self.clear_cache();
        self.dirty = true;
        let should_fetch = self.mode == Mode::Installed || !self.input.value().trim().is_empty();
        if should_fetch {
            self.is_loading = true;
            self.trigger_search();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn trigger_search_is_non_blocking() {
        let mut app = App::new("firefox".into());
        let start = Instant::now();
        app.trigger_search();
        let elapsed = start.elapsed();
        // Should return immediately without blocking on network/disk (background thread)
        assert!(
            elapsed < Duration::from_millis(100),
            "trigger_search should be non-blocking, took {:?}",
            elapsed
        );
        assert!(app.is_loading, "should be loading after trigger");
        // Poll should not panic even if no result yet
        app.poll_search_results();
    }

    #[test]
    fn trigger_search_empty_clears() {
        let mut app = App::new("".into());
        app.trigger_search();
        assert!(!app.is_loading);
        assert!(app.packages.is_empty());
    }

    #[test]
    fn q_in_search_does_not_quit() {
        let mut app = App::new("".into());
        app.focus = Focus::Search;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert!(!app.should_quit, "q in Search should type, not quit");
        assert_eq!(app.input.value(), "q");
    }

    #[test]
    fn q_in_list_quits() {
        let mut app = App::new("firefox".into());
        app.focus = Focus::List;
        app.popup = Popup::None;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert!(app.should_quit, "q in List should quit");
    }

    #[test]
    fn new_with_cli_respects_filters() {
        let cli = crate::cli::Cli {
            query: Some("test".into()),
            tui: false,
            no_tui: false,
            source: crate::cli::Source::Aur,
            by: crate::cli::AurBy::Name,
            limit: 10,
            json: false,
            regex: true,
            installed_only: true,
            bottom_up: false,
            verbose: 0,
            no_color: true,
            config: None,
            init_config: false,
            show_config: false,
            remove: None,
            refresh: false,
        };
        let app = App::new_with_cli("test".into(), &cli);
        assert_eq!(app.limit, 10);
        assert_eq!(app.source, crate::cli::Source::Aur);
        assert_eq!(app.aur_by, crate::cli::AurBy::Name);
        assert!(app.use_regex);
        assert!(app.installed_only);
        assert!(app.no_color);
    }

    #[test]
    fn list_state_none_on_empty() {
        let app = App::new("".into());
        assert_eq!(app.list_state.selected(), None);
        assert!(app.packages.is_empty());
    }

    #[test]
    fn tab_switches_to_installed_mode() {
        let mut app = App::new("".into());
        assert_eq!(app.mode, Mode::Search);
        app.switch_mode();
        assert_eq!(app.mode, Mode::Installed);
        // Empty filter in Installed lists all -> loading
        assert!(app.is_loading);
        app.switch_mode();
        assert_eq!(app.mode, Mode::Search);
    }

    #[test]
    fn tab_key_event_switches_mode() {
        let mut app = App::new("".into());
        app.focus = Focus::Search;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert_eq!(app.mode, Mode::Installed);
        // Tab must not be typed into input
        assert!(!app.input.value().contains('\t'));
    }

    #[test]
    fn enter_in_installed_opens_confirm_remove() {
        let mut app = App::new("".into());
        app.mode = Mode::Installed;
        app.focus = Focus::List;
        app.packages = vec![crate::model::Package {
            name: "vim".into(),
            version: "1-1".into(),
            description: None,
            repo: "local".into(),
            arch: None,
            url: None,
            installed: true,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
        }];
        app.list_state.select(Some(0));
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert!(matches!(app.popup, Popup::ConfirmRemove(_)));
    }

    #[test]
    fn d_in_installed_opens_confirm_remove() {
        let mut app = App::new("".into());
        app.mode = Mode::Installed;
        app.focus = Focus::List;
        app.packages = vec![crate::model::Package {
            name: "vim".into(),
            version: "1-1".into(),
            description: None,
            repo: "local".into(),
            arch: None,
            url: None,
            installed: true,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
        }];
        app.list_state.select(Some(0));
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('d'),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert!(matches!(app.popup, Popup::ConfirmRemove(_)));
    }

    #[test]
    fn cache_hit_serves_instantly() {
        let mut app = App::new("".into());
        app.input = Input::new("vim".into());
        let pkg = crate::model::Package {
            name: "vim".into(),
            version: "1-1".into(),
            description: None,
            repo: "local".into(),
            arch: None,
            url: None,
            installed: true,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
        };
        let key = app.cache_key("vim");
        app.cache_put(key, vec![pkg]);
        app.trigger_search();
        // Cache hit: synchronous, no background thread, no loading spinner
        assert!(!app.is_loading);
        assert_eq!(app.packages.len(), 1);
        assert_eq!(app.packages[0].name, "vim");
    }

    #[test]
    fn cache_evicts_oldest_beyond_cap() {
        let mut app = App::new("".into());
        let mk = |n: &str| crate::model::Package {
            name: n.into(),
            version: "1-1".into(),
            description: None,
            repo: "local".into(),
            arch: None,
            url: None,
            installed: true,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
        };
        for i in 0..(SEARCH_CACHE_CAP + 5) {
            app.cache_put(format!("k{i}"), vec![mk("x")]);
        }
        assert!(app.search_cache.len() <= SEARCH_CACHE_CAP);
        assert!(app.cache_get("k0").is_none());
    }

    #[test]
    fn question_in_list_opens_help() {
        let mut app = App::new("".into());
        app.focus = Focus::List;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('?'),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert_eq!(app.popup, Popup::Help);
    }

    #[test]
    fn r_in_list_opens_confirm_refresh() {
        let mut app = App::new("".into());
        app.focus = Focus::List;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('r'),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert_eq!(app.popup, Popup::ConfirmRefresh);
    }

    #[test]
    fn f5_in_list_opens_confirm_refresh() {
        let mut app = App::new("".into());
        app.focus = Focus::List;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::F(5),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert_eq!(app.popup, Popup::ConfirmRefresh);
    }

    #[test]
    fn r_in_search_types_instead_of_refresh() {
        let mut app = App::new("".into());
        app.focus = Focus::Search;
        let ev = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('r'),
            KeyModifiers::empty(),
        ));
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = app.handle_event(&ev, &mut terminal);
        assert_ne!(app.popup, Popup::ConfirmRefresh);
        assert_eq!(app.focus, Focus::Search);
    }
}
