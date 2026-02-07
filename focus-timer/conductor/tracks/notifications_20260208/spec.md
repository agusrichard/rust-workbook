# Track Specification: Notifications

## Goal
Notify the user when the timer reaches zero using both an audible alert (sound) and an OS-level system notification (pop-up). Provide the user with the ability to enable or disable these notifications.

## Features to Implement
1.  **Audible Alert:**
    -   Integrate `rodio` (or similar) for sound playback.
    -   Include a default alert sound file (e.g., a simple beep or chime).
    -   Play the sound when the timer finishes.
2.  **System Notification:**
    -   Use a library like `notify-rust` to send desktop notifications.
    -   Configure the notification with a title (e.g., "Focus Timer") and message (e.g., "Time is up!").
3.  **User Configuration:**
    -   Add a toggle in the `App` state to enable/disable sound.
    -   Add a toggle in the `App` state to enable/disable system notifications.
    -   Update the UI to display the current status of these settings.
    -   Handle keyboard shortcuts to toggle these settings (e.g., 's' for sound, 'n' for notifications).

## Tech Stack
-   **Audio:** Rodio
-   **Notifications:** notify-rust
-   **UI:** Ratatui

## Success Criteria
-   The application plays a sound when the timer hits zero (if enabled).
-   A system notification pop-up appears when the timer hits zero (if enabled).
-   User can toggle sound and notifications on/off via the keyboard.
-   The UI reflects the current notification settings.
