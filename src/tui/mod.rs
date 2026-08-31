pub mod app;
pub mod ui;

use color_eyre::Result;
use ratatui::DefaultTerminal;

use app::App;

pub fn run(initial_query: String) -> Result<()> {
    // tui-design + ratatui skill: install color_eyre before ratatui::try_init so panic hook wraps and restores
    let _ = color_eyre::install();
    let mut terminal = ratatui::try_init().map_err(|e| color_eyre::eyre::eyre!(e))?;
    // Guard ensures restore even if App::run panics (abort vs unwind handled in Cargo.toml:50)
    let result = App::new(initial_query).run(&mut terminal);
    // Best-effort restore — independent steps per tui-design lifecycle
    if let Err(e) = ratatui::try_restore() {
        // Fallback manual restore if ratatui helper fails (best-effort, no ?)
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show,
            crossterm::event::DisableMouseCapture
        );
        eprintln!("restore warning: {e}");
    }
    result
}

// For suspending TUI to run child process (pacman/makepkg) — per tui-design lifecycle
pub fn suspend_and_run<F, T>(terminal: &mut DefaultTerminal, f: F) -> color_eyre::Result<T>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    // 1. Restore shell modes — best-effort, log but don't abort child
    if let Err(e) = ratatui::try_restore() {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        eprintln!("suspend restore warning: {e}");
    }
    let child_result = f().map_err(|e| color_eyre::eyre::eyre!(e));
    // 2. Re-enter TUI — independent best-effort, per tui-design recipe (store child_result)
    let reinit = ratatui::try_init();
    match (child_result, reinit) {
        (Ok(v), Ok(t)) => {
            *terminal = t;
            let _ = terminal.clear();
            Ok(v)
        }
        (Ok(_v), Err(e)) => {
            // Child succeeded but reentry failed — report reentry
            Err(color_eyre::eyre::eyre!(
                "reentry failed after child success: {e}"
            ))
        }
        (Err(e), Ok(t)) => {
            // Child failed, but reentry ok — restore and return child error
            *terminal = t;
            let _ = terminal.clear();
            Err(e)
        }
        (Err(e1), Err(e2)) => Err(color_eyre::eyre::eyre!(
            "child failed: {e1}; reentry also failed: {e2}"
        )),
    }
}
