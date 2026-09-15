# Changelog

All notable VxClick changes are documented here and mirrored inside the app under **Settings → About & Updates**.

## [1.0.4] - 2026-09-15

### New
- Rebuilt **Dashboard** with a larger VxClick hero/status area, four feature cards, Quick Actions, live engine telemetry and a dedicated information rail.
- Dashboard shortcuts for Auto Clicker, Macros, Key Remap, Profiles, Recoil Scripts, Keybinds, Settings and update access.

### Changed
- Dashboard now uses live engine data for actual CPS, target CPS, session clicks, active profile and engine state instead of generic metric tiles.
- Dashboard styling is isolated in `DashboardPanel.tsx` and `dashboard.css` so future UI work does not keep growing `App.tsx`.
- Responsive dashboard layout now collapses the information rail and feature grid cleanly on smaller windows and higher Windows display scaling.

### Fixed
- Added the missing 1.0.3 and 1.0.4 entries to both repository and in-app changelogs.
- Release workflow now verifies that the current version exists in `CHANGELOG.md`, the in-app changelog and `docs/releases/vX.Y.Z.md` before publishing.

## [1.0.3] - 2026-09-15

### New
- Native **press-to-bind hotkey capture** across Auto Clicker, Keybinds and Macro Studio for keyboard keys, modifier chords and Mouse1–Mouse5.
- Expanded Recoil Script imports for `.lua`, `.ahk`, `.csv`, `.txt` and `.recoil` in addition to `.json` / `.vxrecoil`.
- Windows system-tray behavior with **Open VxClick** and **Exit VxClick** actions.

### Changed
- Macro Studio includes **New** and **Duplicate** macro actions.
- Timeline rows can be moved up/down and duplicated while keeping the existing native playback architecture.
- Added common keyboard assignments and useful delay presets for faster macro creation.
- Closing the VxClick window hides it to the tray instead of terminating the automation runtime.
- Terms of Service and GitHub links open through the Windows default browser instead of relying on WebView `_blank` behavior.

### Fixed
- Macro recording no longer creates streams of tiny delay-only rows from unsupported mouse-move hook traffic.
- Recorded event IDs are allocated correctly when delay rows are inserted, avoiding duplicate timeline IDs.
- Hotkey capture suppresses the captured input briefly so a newly assigned binding does not immediately fire.
- Fixed About & Updates links that could appear clickable without opening anything in the Tauri WebView.

## [1.0.2] - 2026-09-15

### New
- Dedicated **Keybinds** tab for clicker, emergency stop, macro recorder, macro triggers and remap triggers.
- Keybind conflict detection, including reserved recorder bindings F1/F2.
- Per-profile **Process List** with individual process removal, executable addition and one-click foreground-process binding.
- Recoil Script upload/import for `.json` and `.vxrecoil` preset files.

### Changed
- Profiles show process bindings as removable application entries instead of only comma-separated text.
- Imported recoil presets receive a new local ID before backend validation/saving.
- Layouts wrap more cleanly on smaller windows and higher Windows display scaling.

### Fixed
- Macro Studio now follows native recorder state so F1 start/F2 stop is visible immediately.
- Completed F1/F2 recordings refresh the saved macro list and surface the saved draft in the UI.
- Fixed page/content clipping that could hide lower controls or prevent vertical scrolling at some DPI/window sizes.

## [1.0.1] - 2026-09-15

### New
- Recoil Scripts with multi-game profiles, primary/secondary slots and user-created Game → Character/Operator → Weapon metadata.
- Persistent Lua Script Library with create, edit, save, upload, rename, duplicate and delete.
- Global macro recording with F1 to start and F2 to stop, saving recordings as unassigned macro drafts.
- Keyboard and mouse remapping support including Mouse1–Mouse5.
- Auto Clicker interval units for milliseconds, seconds, minutes and hours.
- In-app Terms of Service access with `TERMS.md` and `DISCLAIMER.md`.

### Changed
- Long click intervals use adaptive waiting to reduce CPU wakeups while keeping stop response fast.
- Low-CPS validation supports intervals below 1 CPS while retaining the high-CPS safety cap.
- Updater can select an official Windows installer, verify its GitHub SHA-256 digest, launch it and then exit VxClick.
- Recorder hotkeys F1/F2 are reserved to prevent conflicts with clicker, macro and remap bindings.

### Fixed
- Removed process-wide `HIGH_PRIORITY_CLASS` behavior that could starve the UI/system at high click rates.
- Live click scheduling avoids catch-up bursts after missed deadlines, reducing high-CPS overshoot and lag.
- Fixed recoil Windows input polling and cleaned Rust formatting/dead-code warnings caught by CI.

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
