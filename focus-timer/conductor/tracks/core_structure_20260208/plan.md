# Implementation Plan - Core Application Structure

## Phase 1: Project Setup & Dependencies [checkpoint: 4e9d462]
- [x] Task: Update `Cargo.toml` with required dependencies (ratatui, crossterm, clap, chrono). 3cf4895
- [x] Task: Create module structure (`app.rs`, `ui.rs`, `tui.rs`, `event.rs` if needed). 5ccfda6
- [x] Task: Conductor - User Manual Verification 'Phase 1: Project Setup & Dependencies' (Protocol in workflow.md)

## Phase 2: Core Domain Logic (Timer) [checkpoint: d328ec5]
- [x] Task: Create `timer.rs` module and define `TimerState` enum (Stopped, Running, Paused). 59a3ab1
- [x] Task: Implement `Timer` struct with `new`, `start`, `pause`, `resume`, `reset`. (TDD: Write tests first). 4ff7859
- [x] Task: Implement `tick` method to update remaining time. (TDD: Write tests first). 29e3126
- [x] Task: Conductor - User Manual Verification 'Phase 2: Core Domain Logic (Timer)' (Protocol in workflow.md)

## Phase 3: TUI Infrastructure & Event Loop
- [x] Task: Implement `Tui` struct in `tui.rs` to handle terminal init/exit (enter/leave raw mode). 58fa457
- [x] Task: Implement main event loop in `main.rs` (or `app.rs`) to handle key events (Exit, Pause/Resume) and Tick events. a36280e
- [x] Task: Connect `Timer` logic to the event loop. a36280e
- [~] Task: Conductor - User Manual Verification 'Phase 3: TUI Infrastructure & Event Loop' (Protocol in workflow.md)

## Phase 4: UI Rendering
- [ ] Task: Implement `ui::render` function to draw the interface using `Ratatui`.
- [ ] Task: Design and render a Block showing the current Timer value (MM:SS).
- [ ] Task: Render status indicators (Work/Break mode, Running/Paused state).
- [ ] Task: Conductor - User Manual Verification 'Phase 4: UI Rendering' (Protocol in workflow.md)
