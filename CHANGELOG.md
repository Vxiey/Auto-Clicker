# Changelog

All notable VxClick changes are documented here and mirrored inside the app under **Settings → About & Updates**.

## [1.0.0] - 2026-09-14

### New
- First VxClick 1.0 release candidate with the Rust automation core and Tauri desktop UI integrated.
- Precision benchmark UI/API for measured CPS, interval percentiles, jitter and missed deadlines.

### Changed
- Core, desktop bundle and frontend versions are synchronized at 1.0.0.
- Release/update links now use the renamed `Vxiey/VxClick` repository.

## [0.9.0] - 2026-09-14

### Changed
- Hardened the updater to accept only official VxClick release URLs and VxClick-named Windows packages.
- Small patches accept only `bsdiff` or `zstd`, require valid source/target versions, SHA-256 and the size safety limit.

## [0.8.0] - 2026-09-14

### New
- Native precision benchmark command reporting target/actual CPS, deviation, interval mean/p50/p95/p99/worst, jitter mean/p50/p95/p99/worst and missed deadlines.

## [0.7.0] - 2026-09-14

### Fixed
- Track synthetic held keys and mouse buttons centrally and release them on Emergency Stop/runtime shutdown.
- A failed click injection makes a best-effort mouse-up to reduce stuck-button states.

## [0.6.0] - 2026-09-14

### New
- Native clicker hotkeys automatically follow the active process/game profile.
- Toggle, hold-to-click and once modes execute from the native Windows hook path.

## [0.5.0] - 2026-09-14

### New
- Stateful remap rule engine for key, mouse and chord triggers.
- Global/per-process remap scopes and consume/pass-through decisions.

### Fixed
- Chords fire on their activation edge instead of repeatedly on keyboard repeat events.

## [0.4.0] - 2026-09-14

### New
- QPC-based macro playback with absolute deadlines, cancellation and 0.1x–10x speed scaling.
- Playback supports keyboard, mouse buttons, wheel, relative/absolute movement and waits.

### Fixed
- Macro playback releases held inputs after completion, cancellation or an error.

## [0.3.0] - 2026-09-14

### New
- Native global clicker hotkey runtime shared with the low-level input hook backend.
- Global start/stop and emergency stop work without WebView focus.

## [0.2.0] - 2026-09-14

### New
- Dark blue/cyan Tauri + React desktop UI with Dashboard, Auto Clicker, Macro Studio, Key Remap, Profiles and Settings.
- Native Windows low-level keyboard and mouse hooks for global recording and advanced hotkeys.
- Game/application profiles with process-aware auto switching.
- Macro Studio with assignment library, editable timeline and repeat modes.
- Sandboxed Lua macro compiler that emits the same native macro event model.
- GitHub Releases updater with optional SHA-256 verified small patch packages.
- Local session, crash and performance logs with automatic rotation.

### Changed
- Hotkey matching distinguishes physical input, external remaps and VxClick-generated input.
- Left/right Ctrl, Alt, Shift and Win variants are supported along with chords and consume/pass-through behavior.
- Keyboard output prefers scan-code SendInput and preserves extended-key identity where required.
- Precision engine remains isolated from UI work.

### Fixed
- Prevent VxClick-generated SendInput events from recursively triggering VxClick hotkeys.
- Preserve extended-key flags for arrows, navigation keys and right-side modifier keys.
- Release Lua runtime closures before extracting compiled timelines.
- Remove the legacy eframe UI path that conflicted with Tauri.
- Update GitHub Actions to Node 24 compatible action versions.

## [0.1.0] - 2026-09-14

### New
- Initial Rust automation core.
- Native SendInput mouse and keyboard injection.
- QPC-based precision scheduler with absolute deadlines.
- Initial macro/remap event models and Windows platform abstraction.
