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
        list_state.select(Some(0));
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
            should_quit: false,
            needs_search: !initial_query.is_empty(),
            last_input_change: Instant::now(),
            search_rx: None,
            search_id: 0,
            next_search_id: 0,
        }
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
                    if self.focus == Focus::Search {
                        self.focus = Focus::List;
                    } else if self.popup != Popup::None {
                        self.popup = Popup::None;
                    } else {
                        // Esc in list goes to search
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
        // Increment search id for dedup (stale results ignored in poll_search_results)
        self.next_search_id = self.next_search_id.wrapping_add(1);
        self.search_id = self.next_search_id;
        let search_id = self.search_id;
        let limit = self.limit;
        let query_clone = query.clone();

        let (tx, rx) = mpsc::channel();
        self.search_rx = Some(rx);

        // Spawn background worker — parallel repo+aur, uses cached Config & OnceLock client
        thread::spawn(move || {
            // Parallel repo vs aur via two inner join handles
            let q1 = query_clone.clone();
            let repo_handle = thread::spawn(move || {
                crate::search::repo::search_repo(&q1, limit, false, false)
                    .or_else(|_| crate::search::repo::search_repo_fallback(&q1, limit))
                    .unwrap_or_default()
            });
            let q2 = query_clone.clone();
            let aur_handle = thread::spawn(move || {
                search_aur_blocking(&q2, "name-desc", limit, false).unwrap_or_else(|e| {
                    tracing::warn!(err=?e, "AUR blocking search failed");
                    vec![]
                })
            });

            let repo_res = repo_handle.join().unwrap_or_default();
            let aur_res = aur_handle.join().unwrap_or_default();

            let mut combined = Vec::with_capacity(repo_res.len() + aur_res.len());
            combined.extend(repo_res);
            combined.extend(aur_res);

            // Send result; if receiver dropped (new search started), ignore
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
}
