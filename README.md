<p align="center">
  <img src="assets/icon.png" width="96" alt="VxClick icon" />
</p>

<h1 align="center">VxClick</h1>

<p align="center">
  High-precision Windows automation built around a Rust timing and input core.
</p>

<p align="center">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D4" />
  <img alt="Core" src="https://img.shields.io/badge/core-Rust-000000" />
  <img alt="Desktop" src="https://img.shields.io/badge/desktop-Tauri%202-24C8DB" />
  <img alt="UI" src="https://img.shields.io/badge/UI-React%2019-61DAFB" />
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green" />
</p>

VxClick is a clean-room Windows automation project focused on **stable timing, low input latency, low overhead and safe start/stop behavior**. The native core uses Windows APIs for high-resolution timing and input injection, while the desktop application uses Tauri + React for the UI.

> **Development status:** v1.0.0 is in release validation. Native profile-aware hotkeys, QPC macro playback, held-input safety, the stateful remap core and the precision benchmark API/UI are implemented on the release branch. Macro Studio persistence/playback wiring, runtime remap integration and some advanced clicker controls remain explicitly marked as incomplete below rather than being presented as finished.

## Highlights

- QPC (`QueryPerformanceCounter`) monotonic high-resolution clock.
- Absolute scheduling deadlines to avoid cumulative interval drift.
- Hybrid sleep / yield / spin waiting instead of a simple `sleep()` click loop.
- Native Windows `SendInput` for mouse and keyboard output.
- Left, right, middle, X1 and X2 mouse button support.
- Measured CPS, interval, jitter and missed-deadline benchmark telemetry.
- Low-level Windows keyboard/mouse hooks and profile-aware global hotkeys.
- Versioned local profiles with foreground-process auto switching.
- Stateful key/mouse/chord remap rule engine with process scopes.
- QPC macro playback with cancellation and 0.1x–10x speed scaling.
- Central held-input tracking and best-effort emergency release.
- Sandboxed Lua compiler/runtime core.
- Tauri 2 + React 19 desktop UI.
- Local diagnostics, crash/performance logs and hardened GitHub update checking.
- Injected-event tagging to prevent self-triggering automation loops.

## Current feature status

| Area | Status | Notes |
| --- | --- | --- |
| Precision auto clicker | ✅ Working | Native Rust scheduler + `SendInput`, 1–20,000 CPS safety range |
| Mouse buttons | ✅ Working | Left, right, middle, X1, X2 |
| Live CPS counter | ✅ Working | Desktop polls native engine state |
| Precision benchmark | ✅ Working | Desktop + CLI reporting actual CPS, interval/jitter percentiles and missed deadlines |
| Profiles | ✅ Working | Persistent JSON profiles + foreground-process auto switch |
| Global hooks / clicker hotkeys | ✅ Working | Native profile-aware toggle/hold/once + emergency stop |
| Diagnostics / logs | ✅ Working | Session, crash and performance logging |
| Update checker | ✅ Working | Official `Vxiey/VxClick` releases + SHA-256 patch verification/staging |
| Macro playback core | ✅ Working | QPC absolute-deadline player with cancellation and held-input cleanup |
| Macro Studio | 🟡 In progress | Editor/recording UI exists; full persistence/playback workflow is not yet wired |
| Key remapping | 🟡 Core ready | Stateful rule engine exists; desktop/runtime hook execution is not complete |
| Lua automation | 🟡 Core ready | Lua-to-native-event compiler/runtime exists; desktop workflow is not fully wired |
| Randomization / burst / position modes | 🟡 UI stage | Controls exist in the UI but are not all passed to the native clicker yet |

Full details: **[docs/features.md](docs/features.md)**.

## Quick start

### Requirements

- Windows 10/11 x64
- Rust toolchain with the MSVC target
- Visual Studio Build Tools / C++ build tools required by Rust/Tauri on Windows
- Node.js 24+ and npm
- Microsoft Edge WebView2 Runtime

### Desktop app

```powershell
git clone https://github.com/Vxiey/VxClick.git
cd VxClick
npm install
npm run tauri:dev
```

Build a release bundle:

```powershell
npm run tauri:build
```

Until a signed/published v1.0 GitHub Release is available, building from source or using the verified GitHub Actions artifact is the reliable test path.

### Native core only

```powershell
cargo build --release
cargo test
```

Run a precision benchmark:

```powershell
cargo run --release -- benchmark --cps 500 --seconds 10 --button left
cargo run --release -- benchmark --cps 2000 --seconds 5 --button left
```

The benchmark reports measured output rather than assuming requested CPS equals delivered CPS.

## Documentation

Project documentation is versioned with the code under [`docs/`](docs/README.md):

- [Documentation home](docs/README.md)
- [Getting started](docs/getting-started.md)
- [Feature status](docs/features.md)
- [Architecture](docs/architecture.md)
- [Timing engine](docs/timing-engine.md)
- [Macros, hotkeys, remapping & Lua](docs/automation-systems.md)
- [Development guide](docs/development.md)
- [Diagnostics](docs/diagnostics.md)
- [Regression checklist](docs/regression-checklist.md)
- [Changelog](CHANGELOG.md)

The repository also includes a workflow that publishes these pages to the real GitHub Wiki after the Wiki has been initialized with its first page.

## Architecture at a glance

```text
React UI (ui/)
        │
        ▼
Tauri desktop bridge (src-tauri/)
        │
        ├── profiles / diagnostics / updater / benchmark
        │
        ├── native profile-aware hotkey runtime
        │
        ▼
Rust automation core (src/)
        │
        ├── precision scheduler + telemetry
        ├── macro player / remap / hotkey models
        ├── Lua runtime
        │
        ▼
Windows platform layer
        ├── QueryPerformanceCounter
        ├── SendInput
        ├── low-level keyboard/mouse hooks
        └── held-input safety release
```

The timing worker is intentionally kept separate from UI rendering and frontend polling.

## Timing design

For a target rate `CPS`, VxClick derives an interval from the QPC clock frequency and advances an **absolute deadline** after each click. It does not schedule the next click relative to when the previous click happened.

For long waits the engine sleeps briefly, then yields as the deadline approaches, and only spins inside a small final timing window. If Windows stalls the worker for several intervals, the live clicker re-anchors the schedule instead of emitting a large catch-up burst.

Read the implementation notes in **[docs/timing-engine.md](docs/timing-engine.md)**.

## Safety and responsible use

> **Use at your own risk.** The developer does not accept responsibility for account penalties, bans, suspensions, data loss, or other consequences resulting from automation, macros, Lua scripts or recoil-style automation. You are responsible for complying with the rules and terms of service of every application or game you automate.

VxClick does **not** include anti-detection, anti-cheat bypass, process injection or stealth functionality.

The emergency-stop path performs a best-effort release of synthetic keys and mouse buttons still tracked as held by VxClick. Raw typed text and Macro Recorder streams are not written to diagnostic logs by default.

## Contributing

Before changing timing or input hot paths, measure the current behavior and keep the smallest robust change. New features should not compromise stop latency, timing stability or input cleanup.

See **[docs/development.md](docs/development.md)** before opening a pull request.

## License

MIT — see [LICENSE](LICENSE).
