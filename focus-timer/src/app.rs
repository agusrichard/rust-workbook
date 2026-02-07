use crate::timer::{Timer, TimerState};
use crate::tui::Tui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingWork,
    EditingBreak,
}

pub struct App {
    pub timer: Timer,
    pub should_quit: bool,
    pub work_duration: Duration,
    pub break_duration: Duration,
    pub input_mode: InputMode,
    pub work_input: String,
    pub break_input: String,
    pub sound_enabled: bool,
    pub notifications_enabled: bool,
}

impl App {
    pub fn new() -> Self {
        let work_duration = Duration::from_secs(25 * 60);
        let break_duration = Duration::from_secs(5 * 60);
        Self {
            timer: Timer::new(work_duration),
            should_quit: false,
            work_duration,
            break_duration,
            input_mode: InputMode::Normal,
            work_input: (work_duration.as_secs() / 60).to_string(),
            break_input: (break_duration.as_secs() / 60).to_string(),
            sound_enabled: true,
            notifications_enabled: true,
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
                    self.handle_event(key);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.timer.tick();
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    pub fn handle_event(&mut self, key: event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                KeyCode::Char(' ') => self.toggle_timer(),
                KeyCode::Char('w') => self.input_mode = InputMode::EditingWork,
                KeyCode::Char('b') => self.input_mode = InputMode::EditingBreak,
                _ => {}
            },
            InputMode::EditingWork => match key.code {
                KeyCode::Enter => {
                    if let Ok(mins) = self.work_input.parse::<u64>() {
                        if mins > 0 {
                            self.work_duration = Duration::from_secs(mins * 60);
                            self.timer.set_duration(self.work_duration);
                        }
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Char(c) if c.is_ascii_digit() => self.work_input.push(c),
                KeyCode::Backspace => {
                    self.work_input.pop();
                }
                _ => {}
            },
            InputMode::EditingBreak => match key.code {
                KeyCode::Enter => {
                    if let Ok(mins) = self.break_input.parse::<u64>() {
                        if mins > 0 {
                            self.break_duration = Duration::from_secs(mins * 60);
                            // We don't automatically switch to break mode timer here, 
                            // but if we were in break mode, it would be useful.
                            // For now let's just update work_duration since the timer starts as work.
                        }
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Char(c) if c.is_ascii_digit() => self.break_input.push(c),
                KeyCode::Backspace => {
                    self.break_input.pop();
                }
                _ => {}
            },
        }
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
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn press_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);
        app.handle_event(press_key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_event_switch_mode() {
        let mut app = App::new();
        app.handle_event(press_key(KeyCode::Char('w')));
        assert_eq!(app.input_mode, InputMode::EditingWork);
        
        app.handle_event(press_key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        
        app.handle_event(press_key(KeyCode::Char('b')));
        assert_eq!(app.input_mode, InputMode::EditingBreak);
    }

    #[test]
    fn test_handle_event_typing() {
        let mut app = App::new();
        app.work_input = String::new();
        
        app.handle_event(press_key(KeyCode::Char('w')));
        app.handle_event(press_key(KeyCode::Char('1')));
        app.handle_event(press_key(KeyCode::Char('2')));
        assert_eq!(app.work_input, "12");
        
        app.handle_event(press_key(KeyCode::Backspace));
        assert_eq!(app.work_input, "1");
    }

    #[test]
    fn test_toggle_timer() {
        let mut app = App::new();
        // Initially Stopped
        assert_eq!(app.timer.state, TimerState::Stopped);
        
        app.handle_event(press_key(KeyCode::Char(' ')));
        assert_eq!(app.timer.state, TimerState::Running);
        
        app.handle_event(press_key(KeyCode::Char(' ')));
        assert_eq!(app.timer.state, TimerState::Paused);
    }

    #[test]
    fn test_save_work_duration_valid() {
        let mut app = App::new();
        app.work_input = "10".to_string();
        app.input_mode = InputMode::EditingWork;
        app.handle_event(press_key(KeyCode::Enter));
        
        assert_eq!(app.work_duration, Duration::from_secs(10 * 60));
        assert_eq!(app.timer.duration, Duration::from_secs(10 * 60)); // Should update timer too
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_save_work_duration_invalid() {
        let mut app = App::new();
        app.work_input = "0".to_string(); // Invalid: must be > 0
        app.input_mode = InputMode::EditingWork;
        app.handle_event(press_key(KeyCode::Enter));
        
        // Should not update if invalid
        assert_eq!(app.work_duration, Duration::from_secs(25 * 60));
        assert_eq!(app.input_mode, InputMode::Normal);
        
        app.work_input = "".to_string();
        app.input_mode = InputMode::EditingWork;
        app.handle_event(press_key(KeyCode::Enter));
        assert_eq!(app.work_duration, Duration::from_secs(25 * 60));
    }

    #[test]
    fn test_app_default_durations() {
        let app = App::new();
        assert_eq!(app.work_duration, Duration::from_secs(25 * 60));
        assert_eq!(app.break_duration, Duration::from_secs(5 * 60));
    }

    #[test]
    fn test_app_input_mode() {
        let mut app = App::new();
        assert_eq!(app.input_mode, InputMode::Normal);
        
        app.input_mode = InputMode::EditingWork;
        assert_eq!(app.input_mode, InputMode::EditingWork);
    }

    #[test]
    fn test_notification_defaults() {
        let app = App::new();
        assert!(app.sound_enabled);
        assert!(app.notifications_enabled);
    }
}