pub mod browser;
pub mod form;
pub mod keys;
pub mod project_browser;
pub mod scroll_list;

use std::io;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Enter raw mode and the alternate screen, returning a configured [`Tui`].
/// Call [`leave`] (or drop the [`TerminalGuard`]) when done.
pub fn enter() -> anyhow::Result<TerminalGuard> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(TerminalGuard { terminal })
}

/// RAII guard that restores the terminal on drop (including panics).
pub struct TerminalGuard {
    pub terminal: Tui,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort: ignore errors during cleanup.
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}
