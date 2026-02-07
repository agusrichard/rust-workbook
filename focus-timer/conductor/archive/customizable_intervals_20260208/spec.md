# Track Specification: Customizable Intervals

## Goal
Enable users to configure the duration of their work and break sessions directly within the application. This involves adding input fields to the UI, validating user input, and updating the timer logic to respect these custom settings.

## Features to Implement
1.  **UI Enhancements:**
    -   Add input fields for "Work Interval" and "Break Interval".
    -   Implement navigation/focus switching between inputs and timer controls.
2.  **Input Handling & Validation:**
    -   Accept numeric input (minutes).
    -   Validate that values are positive integers.
    -   Display error messages or prevent invalid input.
3.  **Timer Logic Updates:**
    -   Update `Timer` struct to store configurable durations.
    -   Implement state switching logic (Work -> Break -> Work).

## Tech Stack
-   **UI:** Ratatui
-   **State Management:** Rust structs

## Success Criteria
-   User can enter custom minutes for Work and Break.
-   Application prevents non-numeric or negative inputs.
-   Timer correctly counts down from the user-defined duration.
