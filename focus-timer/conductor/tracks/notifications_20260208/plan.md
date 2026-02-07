# Implementation Plan - Notifications

## Phase 1: Notification State & UI
- [ ] Task: Add `sound_enabled` and `notifications_enabled` fields to `App` struct.
- [ ] Task: Update `ui::render` to show the status of sound and notifications.
- [ ] Task: Implement keyboard handlers in `app.rs` ('s' for sound, 'n' for notifications) to toggle state.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Notification State & UI' (Protocol in workflow.md)

## Phase 2: System Notifications (notify-rust)
- [ ] Task: Add `notify-rust` dependency to `Cargo.toml`.
- [ ] Task: Implement a helper function to trigger a system notification.
- [ ] Task: Trigger the notification in the app loop when `timer.remaining == 0` and `notifications_enabled` is true.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: System Notifications' (Protocol in workflow.md)

## Phase 3: Audible Alerts (rodio)
- [ ] Task: Verify `rodio` is ready in `Cargo.toml`.
- [ ] Task: Source or define a simple placeholder sound (or use a built-in system sound if possible).
- [ ] Task: Implement sound playback logic.
- [ ] Task: Trigger the sound in the app loop when `timer.remaining == 0` and `sound_enabled` is true.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Audible Alerts' (Protocol in workflow.md)
