pub mod app;
pub mod ui;

use color_eyre::Result;
use ratatui::DefaultTerminal;

use app::App;

pub fn run(initial_query: String) -> Result<()> {
    // tui-design + ratatui skill: install color_eyre before ratatui::init so panic hook wraps
    // Use try_init to avoid panic in non-TTY (e.g. cargo test, headless)
    let _ = color_eyre::install();
    let mut terminal = ratatui::try_init().map_err(|e| color_eyre::eyre::eyre!(e))?;
    let result = App::new(initial_query).run(&mut terminal);
    let _ = ratatui::try_restore();
    result
}

// For suspending TUI to run child process (pacman/makepkg) — per tui-design lifecycle
pub fn suspend_and_run<F, T>(terminal: &mut DefaultTerminal, f: F) -> color_eyre::Result<T>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    // Pause input reading, restore terminal to shell mode — independent best-effort
    let _ = ratatui::try_restore();
    let result = f().map_err(|e| color_eyre::eyre::eyre!(e));
    // Re-enter TUI — independent best-effort steps, per tui-design lifecycle
    *terminal = ratatui::try_init().map_err(|e| color_eyre::eyre::eyre!(e))?;
    let _ = terminal.clear();
    result
}
