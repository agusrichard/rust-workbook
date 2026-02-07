#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Stopped,
    Running,
    Paused,
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
}
