use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Stopped,
    Running,
    Paused,
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
        if self.state == TimerState::Stopped {
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

    pub fn tick(&mut self) {
        if self.state == TimerState::Running {
            self.remaining = self.remaining.saturating_sub(Duration::from_secs(1));
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
    fn test_timer_tick() {
        let mut timer = Timer::new(Duration::from_secs(60));
        timer.start();
        
        timer.tick();
        assert_eq!(timer.remaining, Duration::from_secs(59));
        
        // Ensure it doesn't go below 0
        let mut timer_short = Timer::new(Duration::from_secs(1));
        timer_short.start();
        timer_short.tick();
        assert_eq!(timer_short.remaining, Duration::from_secs(0));
        
        timer_short.tick(); // Should stay at 0
        assert_eq!(timer_short.remaining, Duration::from_secs(0));
        
        // Also check that it changes state to Stopped when it hits 0? 
        // Or maybe just stops counting. 
        // Usually a timer finishes. Let's assume for now it just stops counting.
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
    }
}
