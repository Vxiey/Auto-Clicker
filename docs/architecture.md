# Architecture

VxClick is intentionally split into a native automation core, a Windows platform layer, a Tauri desktop backend and a React frontend.

## High-level flow

```text
┌──────────────────────────────────────────────┐
│ React UI — ui/                               │
│ Dashboard · Clicker · Macros · Remap · etc. │
└──────────────────────┬───────────────────────┘
                       │ Tauri invoke/events
┌──────────────────────▼───────────────────────┐
│ Desktop backend — src-tauri/src/             │
│ engine state · profiles · diagnostics        │
│ updater · desktop command boundary           │
└──────────────────────┬───────────────────────┘
                       │ auto-clicker-core
┌──────────────────────▼───────────────────────┐
│ Shared Rust core — src/                      │
│ engine · hotkeys · macros · remap · Lua      │
└──────────────────────┬───────────────────────┘
                       │ Windows implementation
┌──────────────────────▼───────────────────────┐
│ src/platform/windows/                        │
│ QPC · SendInput · hooks · hotkeys · clicker │
└──────────────────────────────────────────────┘
```

## Core crate: `src/`

The root package is `auto-clicker-core`. It should remain usable without the desktop UI.

### `src/engine/`

Shared input/timing types and benchmark statistics. This is where measured CPS, interval and jitter data is represented.

### `src/platform/windows/clock.rs`

Wraps `QueryPerformanceCounter` / QPC frequency conversion for a monotonic high-resolution timeline.

### `src/platform/windows/input.rs`

Native Windows input injection. Mouse and keyboard output use `SendInput`; generated input is tagged so the hook/hotkey layer can identify VxClick-generated events.

### `src/platform/windows/precision_clicker.rs`

The precision click scheduler. Responsibilities include:

- validating CPS
- deriving QPC interval ticks
- advancing absolute deadlines
- bounded hybrid waiting
- dispatching mouse input
- stop checks
- live stall re-anchoring
- benchmark sample collection

The scheduler must not depend on React, Tauri rendering or frontend polling.

### `src/platform/windows/hooks.rs`

Low-level keyboard and mouse capture infrastructure used by advanced hotkeys/recording/remapping work.

### `src/platform/windows/hotkeys.rs` and `src/hotkeys.rs`

Global hotkey matching and higher-level hotkey logic, including modifier variants and injected-event distinctions.

### `src/macro_engine.rs`

Generic macro event/trigger model. Macro playback should ultimately use the same scheduler principles as click automation rather than a separate low-quality timing loop.

### `src/remap.rs`

Remap model for the `Input → Rule/Binding → Action` architecture.

### `src/lua_runtime.rs`

Sandboxed Lua runtime/compiler built on `mlua`. Lua is treated as another producer of native macro events rather than a separate input backend.

## Desktop backend: `src-tauri/`

The desktop package is `vxclick` v0.2.0.

### `src-tauri/src/main.rs`

Owns the Tauri command boundary and live clicker state.

Current clicker path:

```text
React start button
  → invoke("start_clicker", { cps, button })
  → validate input
  → spawn vxclick-precision-worker
  → PrecisionClicker::run_until_stopped(...)
  → Windows SendInput
```

The worker state uses atomics for running/stop/click count. UI status sampling is separate from the scheduling loop.

### `profiles.rs`

Stores a schema-versioned profile document in the application config directory and runs a process watcher. The watcher checks the foreground process every 500 ms and emits `profile-changed` when an auto-switch profile matches.

### `diagnostics.rs`

Local diagnostics, panic capture and performance logging. See [Diagnostics](diagnostics.md).

### `updater.rs`

Update metadata/checking and optional SHA-256 verified patch staging.

## Frontend: `ui/`

React 19 + TypeScript UI built with Vite. It provides the product surface but should not own precision-critical timing.

The frontend currently contains:

- `App.tsx` — shell, dashboard, clicker, remap/settings views
- `MacroStudio.tsx` — macro editor/recording UI
- `ProfilesPanel.tsx` — profile management
- `UpdatePanel.tsx` — update/about surface
- `api.ts` — typed frontend API helpers
- CSS/theme modules

## Dependency direction

Preferred dependency direction:

```text
UI → Tauri boundary → core abstractions → Windows platform implementation
```

Avoid moving timing or input hot paths into the frontend. Also avoid making core logic depend on Tauri when it can stay reusable/testable in the root crate.

## Concurrency rules

- Precision input scheduling runs off the UI thread.
- Stop state should remain cheap to read from hot paths.
- Avoid per-click logging and unnecessary allocations.
- Avoid mutex contention in the scheduler loop.
- UI polling must never determine click timing.
- Any hook/remap system must filter VxClick-generated events to prevent recursive mappings.

## Shutdown and safety direction

Automation systems should have one predictable stop path and should release any synthetic key/button state on stop or failure whenever possible. Future macro/remap work should preserve this property instead of implementing isolated cleanup behavior per feature.
