use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Stopped,
    Running,
    Paused,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timer {
    pub state: TimerState,
    pub duration: Duration,
    pub remaining: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            state: TimerState::Stopped,
            duration,
            remaining: duration,
        }
    }

    pub fn start(&mut self) {
        if self.state == TimerState::Stopped || self.state == TimerState::Finished {
            self.remaining = self.duration;
            self.state = TimerState::Running;
        }
    }

    pub fn pause(&mut self) {
        if self.state == TimerState::Running {
            self.state = TimerState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == TimerState::Paused {
            self.state = TimerState::Running;
        }
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
        self.remaining = duration;
        self.state = TimerState::Stopped;
    }

    pub fn tick(&mut self) {
        if self.state == TimerState::Running {
            self.remaining = self.remaining.saturating_sub(Duration::from_secs(1));
            if self.remaining.is_zero() {
                self.state = TimerState::Finished;
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = TimerState::Stopped;
        self.remaining = self.duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_state_derives() {
        let state = TimerState::Stopped;
        assert_eq!(state, TimerState::Stopped);
        assert_ne!(state, TimerState::Running);
        
        let copy = state;
        assert_eq!(copy, state);
    }

    #[test]
    fn test_timer_new() {
        let duration = Duration::from_secs(60);
        let timer = Timer::new(duration);
        assert_eq!(timer.state, TimerState::Stopped);
        assert_eq!(timer.duration, duration);
        assert_eq!(timer.remaining, duration);
    }

    #[test]
    fn test_timer_start() {
        let mut timer = Timer::new(Duration::from_secs(60));
        timer.start();
        assert_eq!(timer.state, TimerState::Running);
        
        // Start from Finished should reset remaining
        timer.remaining = Duration::from_secs(0);
        timer.state = TimerState::Finished;
        timer.start();
        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.remaining, timer.duration);
    }

    #[test]
    fn test_timer_pause_resume() {
        let mut timer = Timer::new(Duration::from_secs(60));
        timer.start();
        timer.pause();
        assert_eq!(timer.state, TimerState::Paused);
        
        timer.resume();
        assert_eq!(timer.state, TimerState::Running);
    }

    #[test]
    fn test_timer_reset() {
        let mut timer = Timer::new(Duration::from_secs(60));
        timer.start();
        // Simulate some time passing (manually modifying remaining for test if field was pub, 
        // but currently we just check state reset)
        timer.state = TimerState::Running; 
        
        timer.reset();
        assert_eq!(timer.state, TimerState::Stopped);
        assert_eq!(timer.remaining, timer.duration);
    }

    #[test]
    fn test_timer_set_duration() {
        let mut timer = Timer::new(Duration::from_secs(60));
        timer.set_duration(Duration::from_secs(120));
        assert_eq!(timer.duration, Duration::from_secs(120));
        assert_eq!(timer.remaining, Duration::from_secs(120));
        assert_eq!(timer.state, TimerState::Stopped);
    }

    #[test]
    fn test_timer_tick() {
        let mut timer = Timer::new(Duration::from_secs(60));
        timer.start();
        
        timer.tick();
        assert_eq!(timer.remaining, Duration::from_secs(59));
        assert_eq!(timer.state, TimerState::Running);
        
        // Ensure it hits Finished when reaching 0
        let mut timer_short = Timer::new(Duration::from_secs(1));
        timer_short.start();
        timer_short.tick();
        assert_eq!(timer_short.remaining, Duration::from_secs(0));
        assert_eq!(timer_short.state, TimerState::Finished);
        
        timer_short.tick(); // Should stay at 0 and Finished
        assert_eq!(timer_short.remaining, Duration::from_secs(0));
        assert_eq!(timer_short.state, TimerState::Finished);
    }

    #[test]
    fn test_timer_tick_only_when_running() {
        let mut timer = Timer::new(Duration::from_secs(60));
        
        // Stopped
        timer.tick();
        assert_eq!(timer.remaining, Duration::from_secs(60));
        
        // Paused
        timer.start();
        timer.pause();
        timer.tick();
        assert_eq!(timer.remaining, Duration::from_secs(60));
        
        // Finished
        timer.resume();
        timer.remaining = Duration::from_secs(0);
        timer.state = TimerState::Finished;
        timer.tick();
        assert_eq!(timer.remaining, Duration::from_secs(0));
    }
}
