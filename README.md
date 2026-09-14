# VxClick

VxClick is a clean-room Windows automation application with a Rust core and a dark Tauri + React desktop UI.

It combines a high-precision auto clicker, global hotkeys, process-aware profiles, keyboard/mouse remapping, macro recording/playback, diagnostics and a sandboxed Lua automation layer.

## VxClick 1.0

Core capabilities in the 1.0 release branch include:

- native Windows `SendInput` for keyboard and mouse output
- QPC-based absolute-deadline click scheduling with drift correction
- native `WH_KEYBOARD_LL` / `WH_MOUSE_LL` input hooks
- global profile-aware toggle, hold and once hotkeys
- left/right modifier variants, chords and consume/pass-through behavior
- separation of physical input, external remaps and VxClick-generated input
- process/game profiles with foreground-process auto switching
- stateful key/mouse/chord remap engine with per-process scope
- QPC macro playback with 0.1x–10x speed scaling and cancellation
- held-input safety release on emergency stop and shutdown
- sandboxed Lua macro compilation into the same native event model
- measured CPS/interval/jitter/missed-deadline benchmark reporting
- session, crash and performance logs with rotation
- GitHub Releases updater plus SHA-256 verified small patch staging
- Windows `.exe` / installer build workflow

## Disclaimer

> **Use at your own risk.** The developer does not accept responsibility for account penalties, bans, suspensions, data loss, or other consequences resulting from the use of automation, macros, Lua scripts, or recoil scripts. You are responsible for complying with the rules and terms of service of any software or game you use VxClick with. Using recoil automation to gain an unfair advantage may violate those rules.

VxClick does not include anti-detection or anti-cheat bypass functionality.

## Build the desktop app

Requirements: Windows, current Rust stable and Node.js 24+.

```powershell
npm install
npm run tauri:build
```

The Tauri build produces the Windows desktop executable and installer bundles under `src-tauri/target/release` and its bundle directories.

## Core precision benchmark

The standalone core benchmark remains available for development checks:

```powershell
cargo run --release -- benchmark --cps 500 --seconds 10 --button left
cargo run --release -- benchmark --cps 2000 --seconds 5 --button left
```

VxClick reports measured results rather than presenting requested CPS as achieved CPS. The app benchmark exposes actual CPS, deviation, interval mean/p50/p95/p99, jitter mean/p50/p95/p99/worst and missed deadlines.

## Architecture

```text
src/
├─ engine/                    shared input types + timing statistics
├─ hotkeys.rs                 parsed hotkey/chord model
├─ macro_engine.rs            shared macro event model
├─ lua_runtime.rs             sandboxed Lua → macro actions
├─ remap.rs                   stateful remap rule engine
└─ platform/windows/
   ├─ clock.rs                QPC high-resolution clock
   ├─ hooks.rs                global keyboard/mouse hooks
   ├─ hotkeys.rs              source-aware native hotkey matcher
   ├─ input.rs                SendInput + held-input safety tracking
   ├─ macro_player.rs         QPC absolute-deadline macro playback
   └─ precision_clicker.rs    click scheduler + benchmark engine

src-tauri/src/
├─ benchmark.rs               desktop precision benchmark API
├─ diagnostics.rs             rotating local troubleshooting logs
├─ hotkeys_runtime.rs         profile-aware global runtime
├─ profiles.rs                persistent process/game profiles
├─ updater.rs                 GitHub release + patch verification
└─ main.rs                    Tauri command/runtime integration

ui/src/
├─ App.tsx                    desktop shell and clicker UI
├─ MacroStudio.tsx            macro editor/assignment workflow
├─ ProfilesPanel.tsx          process profile management
├─ BenchmarkPanel.tsx         measured timing diagnostics
└─ UpdatePanel.tsx            updates and in-app changelog
```

## Safety and stability rules

VxClick tags its own injected input so its hooks do not recursively trigger themselves. Emergency Stop performs a best-effort release of synthetic keys/buttons still held by VxClick. Raw typed text and macro recorder streams are not written into diagnostic logs by default.

The current safety cap is 20,000 requested CPS. Whether a target application actually observes input at that rate depends on Windows scheduling and how the target consumes input.

## Repository

Project: `Vxiey/VxClick`

See `CHANGELOG.md` for the full version history from 0.1.0 through 1.0.0.
