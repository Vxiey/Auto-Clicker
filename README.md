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

> **Development status:** v0.2.0 is an active development release. The precision clicker, native input backend, profiles, diagnostics and updater plumbing are functional. Several automation surfaces in the UI are still being connected to the native runtime; see [Feature status](docs/features.md) for the exact state.

## Highlights

- QPC (`QueryPerformanceCounter`) monotonic high-resolution clock.
- Absolute scheduling deadlines to avoid cumulative interval drift.
- Hybrid sleep / yield / spin waiting instead of a simple `sleep()` click loop.
- Native Windows `SendInput` for mouse and keyboard output.
- Left, right, middle, X1 and X2 mouse button support.
- Measured CPS, interval, jitter and missed-deadline benchmark telemetry.
- Low-level Windows keyboard/mouse hook and hotkey infrastructure.
- Versioned local profiles with foreground-process auto switching.
- Macro event model and sandboxed Lua compiler/runtime core.
- Tauri 2 + React 19 desktop UI.
- Local diagnostics, crash/performance logs and update checking.
- Injected-event tagging to help prevent self-triggering automation loops.

## Current feature status

| Area | Status | Notes |
| --- | --- | --- |
| Precision auto clicker | ✅ Working | Native Rust scheduler + `SendInput`, 1–20,000 CPS safety range |
| Mouse buttons | ✅ Working | Left, right, middle, X1, X2 |
| Live CPS counter | ✅ Working | Desktop polls native engine state |
| CLI precision benchmark | ✅ Working | CPS, interval, jitter, percentiles, missed deadlines |
| Profiles | ✅ Working | Persistent JSON profiles + foreground-process auto switch |
| Diagnostics / logs | ✅ Working | Session, crash and performance logging |
| Update checker | ✅ Working | GitHub update plumbing + optional SHA-256 patch verification |
| Global hooks / hotkey engine | 🟡 Core ready | Native core exists; full desktop control wiring is still being completed |
| Macro Studio | 🟡 In progress | Editor/recording UI exists; native playback/persistence integration is not complete |
| Key remapping | 🟡 In progress | Core data model/UI shell exist; runtime remap execution is not complete |
| Lua automation | 🟡 Core ready | Lua-to-native-event compiler/runtime exists; desktop workflow is not fully wired |
| Randomization / burst / position modes | 🟡 UI stage | Controls exist in the UI but are not all passed to the native clicker yet |

Full details: **[docs/features.md](docs/features.md)**.

## Quick start

### Requirements

- Windows 10/11 x64
- Rust toolchain with the MSVC target
- Visual Studio Build Tools / C++ build tools required by Rust/Tauri on Windows
- Node.js + npm
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

There is currently no published GitHub Release, so building from source is the reliable installation path for v0.2.0.

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
        ├── profiles / diagnostics / updater
        │
        ▼
Rust automation core (src/)
        │
        ├── scheduler + telemetry
        ├── macro / remap / hotkey models
        ├── Lua runtime
        │
        ▼
Windows platform layer
        ├── QueryPerformanceCounter
        ├── SendInput
        ├── low-level keyboard/mouse hooks
        └── global hotkey handling
```

The timing worker is intentionally kept separate from UI rendering and frontend polling.

## Timing design

For a target rate `CPS`, VxClick derives an interval from the QPC clock frequency and advances an **absolute deadline** after each click. It does not schedule the next click relative to when the previous click happened.

For long waits the engine sleeps briefly, then yields as the deadline approaches, and only spins inside a small final timing window. If Windows stalls the worker for several intervals, the live clicker re-anchors the schedule instead of emitting a large catch-up burst.

Read the implementation notes in **[docs/timing-engine.md](docs/timing-engine.md)**.

## Safety and responsible use

VxClick does **not** include anti-detection, anti-cheat bypass, process injection or stealth functionality.

Use automation only where it is permitted. You are responsible for complying with the rules and terms of service of any application or game you automate. Automation, macros or recoil-style scripts can lead to account penalties where prohibited.

## Contributing

Before changing timing or input hot paths, measure the current behavior and keep the smallest robust change. New features should not compromise stop latency, timing stability or input cleanup.

See **[docs/development.md](docs/development.md)** before opening a pull request.

## License

MIT — see [LICENSE](LICENSE).
