use crate::timer::{Timer, TimerState};
use crate::tui::Tui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;
use std::time::{Duration, Instant};

pub struct App {
    pub timer: Timer,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::from_secs(25 * 60)),
            should_quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn run(&mut self, tui: &mut Tui) -> io::Result<()> {
        let tick_rate = Duration::from_secs(1);
        let mut last_tick = Instant::now();

        while !self.should_quit {
            tui.terminal.draw(|frame| {
                crate::ui::render(self, frame);
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                            KeyCode::Char(' ') => self.toggle_timer(),
                            _ => {}
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.timer.tick();
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn toggle_timer(&mut self) {
        match self.timer.state {
            TimerState::Running => self.timer.pause(),
            TimerState::Paused => self.timer.resume(),
            TimerState::Stopped => self.timer.start(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn test_toggle_timer() {
        let mut app = App::new();
        // Initially Stopped
        assert_eq!(app.timer.state, TimerState::Stopped);
        
        app.toggle_timer();
        assert_eq!(app.timer.state, TimerState::Running);
        
        app.toggle_timer();
        assert_eq!(app.timer.state, TimerState::Paused);
        
        app.toggle_timer();
        assert_eq!(app.timer.state, TimerState::Running);
    }
}