# Auto Clicker

A clean-room, high-precision Windows auto clicker, key remapper, and macro engine written in Rust.

## v0.1 core milestone

The first development branch focuses on the native engine before UI work:

- QPC (`QueryPerformanceCounter`) clock
- absolute click deadlines to prevent cumulative interval drift
- hybrid coarse-wait / yield / active-spin scheduling
- native Windows `SendInput` for mouse and keyboard injection
- injected-event tagging for future macro/remap loop prevention
- elevated worker scheduling priority while the precision engine runs
- generated CPS, interval, jitter and missed-deadline telemetry
- shared macro/remap data model for keyboard and mouse actions
- Windows GitHub Actions compile/test/lint gate

## Build

```powershell
cargo build --release
```

## Precision benchmark

```powershell
cargo run --release -- benchmark --cps 500 --seconds 10 --button left
cargo run --release -- benchmark --cps 2000 --seconds 5 --button left
```

The benchmark deliberately reports what was actually generated instead of claiming an arbitrary CPS number.

## Current architecture

```text
src/
├─ engine/                  timing statistics + common input types
├─ macro_engine.rs          macro event/trigger model
├─ remap.rs                 remap binding model
└─ platform/windows/
   ├─ clock.rs              QPC high-resolution clock
   ├─ input.rs              SendInput mouse/keyboard backend
   └─ precision_clicker.rs  absolute-deadline scheduler
```

## Next milestones

1. High-resolution waitable timer for the coarse phase of longer intervals.
2. Global hotkey manager and emergency stop.
3. Low-level keyboard/mouse capture with self-injected event filtering.
4. Macro recorder/player using the same precision scheduler.
5. Quick Remap: key ↔ key, key ↔ mouse, mouse ↔ mouse.
6. Per-app profiles and process rules.
7. Lightweight DPI-aware Windows UI.
8. Reproducible performance benchmark suite for 1–10,000+ CPS.

The v0.1 engine has a temporary 20,000 CPS safety cap until real Windows benchmark results are collected.
