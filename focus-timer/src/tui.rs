use std::io;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<io::Stderr>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
        Ok(Self { terminal })
    }

    pub fn enter(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }

    pub fn exit(&mut self) -> io::Result<()> {
        execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_new_does_not_panic() {
        // Tui::new() might fail if not in a terminal, so we handle the result.
        // In CI/headless, this might return Err.
        let _ = Tui::new(); 
    }
}