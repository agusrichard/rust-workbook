# Implementation Plan - Core Application Structure

## Phase 1: Project Setup & Dependencies
- [x] Task: Update `Cargo.toml` with required dependencies (ratatui, crossterm, clap, chrono). 3cf4895
- [ ] Task: Create module structure (`app.rs`, `ui.rs`, `tui.rs`, `event.rs` if needed).
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Project Setup & Dependencies' (Protocol in workflow.md)

## Phase 2: Core Domain Logic (Timer)
- [ ] Task: Create `timer.rs` module and define `TimerState` enum (Stopped, Running, Paused).
- [ ] Task: Implement `Timer` struct with `new`, `start`, `pause`, `resume`, `reset`. (TDD: Write tests first).
- [ ] Task: Implement `tick` method to update remaining time. (TDD: Write tests first).
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Core Domain Logic (Timer)' (Protocol in workflow.md)

## Phase 3: TUI Infrastructure & Event Loop
- [ ] Task: Implement `Tui` struct in `tui.rs` to handle terminal init/exit (enter/leave raw mode).
- [ ] Task: Implement main event loop in `main.rs` (or `app.rs`) to handle key events (Exit, Pause/Resume) and Tick events.
- [ ] Task: Connect `Timer` logic to the event loop.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: TUI Infrastructure & Event Loop' (Protocol in workflow.md)

## Phase 4: UI Rendering
- [ ] Task: Implement `ui::render` function to draw the interface using `Ratatui`.
- [ ] Task: Design and render a Block showing the current Timer value (MM:SS).
- [ ] Task: Render status indicators (Work/Break mode, Running/Paused state).
- [ ] Task: Conductor - User Manual Verification 'Phase 4: UI Rendering' (Protocol in workflow.md)
