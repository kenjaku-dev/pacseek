use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self},
};
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, widgets::ListState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::cli::{AurBy, Source};
use crate::model::Package;
use crate::search::aur::search_aur_blocking;

use super::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Search,
    List,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    None,
    Info(Package),
    Confirm(Package),
    Message(String),
}

pub struct App {
    pub input: Input,
    pub packages: Vec<Package>,
    pub list_state: ListState,
    pub focus: Focus,
    pub popup: Popup,
    pub is_loading: bool,
    pub status: String,
    pub initial_query: String,
    pub last_query: String,
    pub limit: usize,
    pub source: Source,
    pub aur_by: AurBy,
    pub use_regex: bool,
    pub installed_only: bool,
    pub no_color: bool,
    pub should_quit: bool,
    pub needs_search: bool,
    pub last_input_change: Instant,
    // Phase B: non-blocking search channel + dedup id per ratatui async skill
    search_rx: Option<std::sync::mpsc::Receiver<(u64, Vec<Package>, String)>>,
    search_id: u64,
    next_search_id: u64,
}

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
        Self {
            input,
            packages: Vec::new(),
            list_state,
            focus: if initial_query.is_empty() {
                Focus::Search
            } else {
                Focus::List
            },
            popup: Popup::None,
            is_loading: false,
            status: "Type to search, Enter to search, ↑↓ navigate, Enter install, i info, q quit"
                .into(),
            initial_query: initial_query.clone(),
            last_query: String::new(),
            limit: 50,
            source: Source::All,
            aur_by: AurBy::NameDesc,
            use_regex: false,
            installed_only: false,
            no_color: std::env::var("NO_COLOR").is_ok(),
            should_quit: false,
            needs_search: !initial_query.is_empty(),
            last_input_change: Instant::now(),
            search_rx: None,
            search_id: 0,
            next_search_id: 0,
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

            terminal.draw(|f| ui::draw(f, self))?;

            // poll with timeout to allow debounce & spinner & signal check
            if event::poll(Duration::from_millis(200))? {
                let ev = event::read()?;
                if self.handle_popup_event(&ev, terminal)? {
                    continue;
                }
                self.handle_event(&ev, terminal)?;
            }

            // Also poll after handling event for immediate fetch after Enter
            self.poll_search_results();

            // Debounced search: if input changed and 400ms passed without new key
            if self.focus == Focus::Search
                && self.needs_search
                && self.last_input_change.elapsed() > Duration::from_millis(400)
            {
                self.trigger_search();
                self.needs_search = false;
            }
        }
        Ok(())
    }

    fn poll_search_results(&mut self) {
        if let Some(rx) = &self.search_rx {
            // Try to receive without blocking; handle multiple pending results (only latest matters)
            while let Ok((id, pkgs, query)) = rx.try_recv() {
                // Only accept latest search_id; discard stale
                if id == self.search_id {
                    let total = pkgs.len();
                    let repo_count = pkgs.iter().filter(|p| p.repo != "aur").count();
                    let aur_count = total.saturating_sub(repo_count);
                    self.packages = pkgs;
                    self.list_state.select(if self.packages.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                    self.is_loading = false;
                    self.last_query = query.clone();
                    if total == 0 {
                        self.status = format!("No results for '{}'", query);
                    } else {
                        self.status =
                            format!("Found {} (repo {} aur {})", total, repo_count, aur_count);
                    }
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
                        KeyCode::Enter => {
                            // Confirm install
                            if let Popup::Confirm(pkg) = self.popup.clone() {
                                self.popup = Popup::None;
                                self.do_install(pkg, terminal)?;
                            } else {
                                self.popup = Popup::None;
                            }
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Popup::Confirm(pkg) = self.popup.clone() {
                                self.popup = Popup::None;
                                self.do_install(pkg, terminal)?;
                            }
                        }
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
                        // Install selected
                        if let Some(idx) = self.list_state.selected() {
                            if let Some(pkg) = self.packages.get(idx).cloned() {
                                // Show confirm popup
                                self.popup = Popup::Confirm(pkg);
                            }
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
    pub(crate) fn trigger_search(&mut self) {
        let query = self.input.value().trim().to_string();
        if query.is_empty() {
            self.packages.clear();
            self.status = "Type a query and press Enter".into();
            self.is_loading = false;
            self.search_rx = None;
            return;
        }
        if query == self.last_query && !self.packages.is_empty() {
            self.is_loading = false;
            return;
        }
        self.is_loading = true;
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
                    search_aur_blocking(&q2, &aur_by_str, limit, use_regex).unwrap_or_else(|e| {
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
        self.trigger_search();
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
}
