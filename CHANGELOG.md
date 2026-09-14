# Changelog

All notable VxClick changes are documented here and mirrored inside the app under **Settings → About & Updates**.

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
