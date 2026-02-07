# Implementation Plan - Notifications

## Phase 1: Notification State & UI [checkpoint: 845703c]
- [x] Task: Add `sound_enabled` and `notifications_enabled` fields to `App` struct. ac9dc4d
- [x] Task: Update `ui::render` to show the status of sound and notifications. 3f162c0
- [x] Task: Implement keyboard handlers in `app.rs` ('s' for sound, 'n' for notifications) to toggle state. 8c03c12
- [x] Task: Conductor - User Manual Verification 'Phase 1: Notification State & UI' (Protocol in workflow.md)

## Phase 2: System Notifications (notify-rust) [checkpoint: a94843c]
- [x] Task: Add `notify-rust` dependency to `Cargo.toml`. 3a1d317
- [x] Task: Implement a helper function to trigger a system notification. 34f513b
- [x] Task: Trigger the notification in the app loop when `timer.remaining == 0` and `notifications_enabled` is true. ffe6e0b
- [x] Task: Conductor - User Manual Verification 'Phase 2: System Notifications' (Protocol in workflow.md)

## Phase 3: Audible Alerts (rodio) [checkpoint: ecc9c9a]
- [x] Task: Verify `rodio` is ready in `Cargo.toml`. 3cf4895
- [x] Task: Source or define a simple placeholder sound (or use a built-in system sound if possible). (Will use programmatic SineWave) c8cb85e
- [x] Task: Implement sound playback logic. c8cb85e
- [x] Task: Trigger the sound in the app loop when `timer.remaining == 0` and `sound_enabled` is true. c8cb85e
- [x] Task: Conductor - User Manual Verification 'Phase 3: Audible Alerts' (Protocol in workflow.md)
