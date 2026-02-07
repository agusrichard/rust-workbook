# Track Specification: Core Application Structure

## Goal
Establish the foundational architecture of the Focus Timer application. This includes setting up the necessary dependencies, implementing the core timer logic (domain layer), and creating the basic Terminal User Interface (TUI) loop to visualize and control the timer.

## Features to Implement
1.  **Project Configuration:**
    -   Add `ratatui`, `crossterm` for UI.
    -   Add `clap` for argument parsing.
    -   Add `chrono` for time management.
    -   Add `rodio` for audio (placeholder setup).
2.  **Domain Logic (Timer):**
    -   `Timer` struct to manage duration, remaining time, and state.
    -   Support for 'Work' and 'Break' modes.
    -   Start, Pause, Resume, and Stop functionality.
    -   Input validation (ensure durations are positive integers).
3.  **Terminal User Interface (TUI):**
    -   Initialize and clean up the terminal raw mode.
    -   Main event loop handling 60fps (or appropriate) tick rate.
    -   Handle keyboard events:
        -   `q` or `Ctrl+c` to quit.
        -   `Space` to pause/resume.
    -   Basic UI Layout:
        -   Display the countdown timer (MM:SS).
        -   Display current status (Work/Break, Running/Paused).

## Tech Stack
-   **UI:** Ratatui, Crossterm
-   **Time:** Chrono (or std::time::Duration)
-   **CLI Args:** Clap

## Success Criteria
-   Application compiles and runs.
-   User can start the application with default settings.
-   TUI displays a timer that counts down.
-   User can pause and resume the timer using the keyboard.
-   User can quit the application cleanly.
-   Unit tests cover the `Timer` logic (start, pause, tick, state transitions).
