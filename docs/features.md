# Feature status

VxClick 1.0.0 connects the desktop UI, native Windows runtime and Rust automation core end-to-end.

## Status key

- ✅ **Working** — implemented and connected in the current app/core.
- 🟡 **In progress** — meaningful implementation exists but end-to-end behavior is incomplete.
- ⚪ **Planned** — intended future work, not a current feature.

## Auto clicker

| Feature | Status | Current behavior |
| --- | --- | --- |
| Exact CPS target | ✅ | Native worker accepts 1–20,000 CPS |
| Left/right/middle click | ✅ | Injected through Windows `SendInput` |
| Mouse 4 / Mouse 5 | ✅ | X1/X2 injection supported |
| Start / stop from UI | ✅ | Starts/stops native precision worker |
| Live actual CPS | ✅ | Backend samples click counter and UI polls it |
| Absolute deadlines | ✅ | Scheduler advances a QPC deadline rather than chaining sleeps |
| Long-stall re-anchoring | ✅ | Live engine avoids large catch-up click bursts |
| Hold/toggle/once modes | ✅ | Global native hotkeys drive all three modes |
| Randomized intervals | ✅ | Controlled interval variation runs in the native clicker |
| Burst mode | ✅ | Native burst size is passed from the desktop/profile path |
| Fixed/multi-point clicking | ✅ | Native clicker executes fixed and rotating multi-point targets |
| Configurable start/emergency hotkeys | ✅ | Active-profile hotkeys are registered by the native runtime |

## Timing and telemetry

| Feature | Status | Notes |
| --- | --- | --- |
| QPC high-resolution clock | ✅ | Windows native monotonic counter |
| Hybrid sleep/yield/spin | ✅ | Final spin window is bounded |
| CLI CPS measurement | ✅ | Measures delivered clicks over elapsed time |
| Interval/jitter percentiles | ✅ | Benchmark reports mean/p50/p95/p99 and worst deviation |
| Missed deadlines | ✅ | Counted by benchmark engine |
| In-app precision benchmark | ✅ | Settings exposes the native benchmark API |
| Automated timing regression gate | ⚪ | Candidate for a future dedicated hardware/runner benchmark gate |

## Input, hooks and hotkeys

| Feature | Status | Notes |
| --- | --- | --- |
| `SendInput` mouse output | ✅ | Native backend |
| `SendInput` keyboard output | ✅ | Scan-code path with extended-key handling |
| Injected-event tagging | ✅ | VxClick-generated input is distinguished from physical/external input |
| Low-level keyboard/mouse hooks | ✅ | Shared native hook runtime |
| Chord hotkey matching | ✅ | Includes left/right modifier variants |
| Consume/pass-through policy | ✅ | Implemented in native hook matching |
| Desktop hotkey control surface | ✅ | Active profile drives clicker, emergency, macro and remap hotkeys |

## Profiles

| Feature | Status | Notes |
| --- | --- | --- |
| Persistent profile file | ✅ | Versioned `profiles.json` in app config directory |
| Profile validation | ✅ | Loaded document is migrated/validated before use |
| Create/edit/delete profile | ✅ | Tauri commands + React panel |
| Foreground process detection | ✅ | Windows foreground process name |
| Process auto switch | ✅ | Watcher checks active process and switches profiles |
| Apply clicker profile to UI/runtime | ✅ | CPS, button, mode, randomization, burst and hotkeys are applied |

## Macros and Lua

| Feature | Status | Notes |
| --- | --- | --- |
| Shared macro event model | ✅ | Keyboard/mouse/delay actions are represented natively |
| Macro Studio editor | ✅ | Rich React timeline/editor |
| Native global macro recording | ✅ | Low-level hook capture is connected to Macro Studio |
| Persistent macros | ✅ | Save/load/delete stored through the desktop backend |
| Native macro playback | ✅ | QPC absolute-deadline playback exposed through Tauri |
| Macro hotkeys | ✅ | Native hotkey runtime starts/stops saved macros |
| Lua compiler/runtime | ✅ | Sandboxed `mlua` runtime emits native macro events |
| Lua desktop workflow | ✅ | Validate/run/stop exposed through the desktop UI/backend |

## Remapping

| Feature | Status | Notes |
| --- | --- | --- |
| Remap data model | ✅ | Persistent Input → Rule → Action mappings |
| Key Remap UI | ✅ | Create/delete mappings in desktop UI |
| Runtime key/mouse remapping | ✅ | Native hook execution wired to stored rules |
| Process scopes | ✅ | Rules can be restricted to the foreground executable |
| Consume/pass-through | ✅ | Mapping can block or forward the original input |
| Recursive self-trigger prevention | ✅ | VxClick-injected events are tagged and filtered |

## Updates and diagnostics

| Feature | Status | Notes |
| --- | --- | --- |
| Local session logs | ✅ | Rotating diagnostics infrastructure |
| Crash/panic capture | ✅ | Panic hook writes diagnostic context |
| Performance log channel | ✅ | Precision worker records run summaries |
| Clear/snapshot diagnostics | ✅ | Tauri commands exposed |
| GitHub update check | ✅ | Updater module + Settings UI |
| SHA-256 patch verification | ✅ | Optional small patch staging path |
| Windows release bundle | ✅ | Release workflow builds portable EXE plus Tauri installer bundles |

## Completion rule

A VxClick feature is marked complete only when the runtime path, validation, stop behavior and failure handling are connected—not merely because a UI control exists.
