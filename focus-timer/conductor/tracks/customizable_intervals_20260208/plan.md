# Implementation Plan - Customizable Intervals

## Phase 1: Input State & UI
- [x] Task: Add `work_duration` and `break_duration` fields to `App` state (defaulting to 25/5). 36970b9
- [x] Task: Create a new `InputMode` enum in `app.rs` (Normal, EditingWork, EditingBreak). f6259f7
- [ ] Task: Update `ui::render` to display input fields for Work and Break.
- [ ] Task: Implement input handling in `main.rs` loop (entering/exiting edit mode, typing digits).
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Input State & UI' (Protocol in workflow.md)

## Phase 2: Validation & Logic Integration
- [ ] Task: Implement validation logic (ensure parsed string is > 0).
- [ ] Task: Update `Timer::new` or add `set_duration` method to accept dynamic values.
- [ ] Task: Connect input values to the `Timer` instance (update timer when values change or on "Reset").
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Validation & Logic Integration' (Protocol in workflow.md)
