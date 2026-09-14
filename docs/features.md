# Feature status

This page separates native behavior that is currently connected from UI or core work that is still being integrated.

## Status key

- ✅ **Working** — implemented and connected in the current app/core.
- 🟡 **In progress / core ready** — meaningful implementation exists, but end-to-end desktop behavior is incomplete.
- ⚪ **Planned** — intended direction, not a current feature.

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
| Hold/toggle/once modes | 🟡 | UI state exists; native start path currently receives only CPS + button |
| Randomized intervals | 🟡 | UI control/profile field exists; not wired to the live scheduler yet |
| Burst mode | 🟡 | UI control/profile field exists; not wired to the live scheduler yet |
| Fixed/multi-point clicking | 🟡 | UI controls exist; native position execution is not connected |
| Configurable start/emergency hotkeys | 🟡 | Hotkey infrastructure exists in core; current desktop start command is not driven by these UI fields |

## Timing and telemetry

| Feature | Status | Notes |
| --- | --- | --- |
| QPC high-resolution clock | ✅ | Windows native monotonic counter |
| Hybrid sleep/yield/spin | ✅ | Final spin window is bounded |
| CLI CPS measurement | ✅ | Measures delivered clicks over elapsed time |
| Interval/jitter percentiles | ✅ | Benchmark reports mean/p95/p99 and worst jitter |
| Missed deadlines | ✅ | Counted by benchmark engine |
| Full in-app jitter dashboard | 🟡 | Diagnostics foundation exists; UI currently shows basic live CPS/click count |
| Automated timing regression gate | ⚪ | Desired future benchmark gate |

## Input, hooks and hotkeys

| Feature | Status | Notes |
| --- | --- | --- |
| `SendInput` mouse output | ✅ | Native backend |
| `SendInput` keyboard output | ✅ | Scan-code path with extended-key handling |
| Injected-event tagging | ✅ | Used to distinguish VxClick-generated input |
| Low-level keyboard/mouse hooks | ✅ Core | Implemented in Windows platform core |
| Chord hotkey matching | ✅ Core | Includes left/right modifier variants |
| Consume/pass-through policy | ✅ Core | Implemented in hotkey layer |
| Desktop hotkey control surface | 🟡 | Full Tauri/UI wiring remains to be completed |

## Profiles

| Feature | Status | Notes |
| --- | --- | --- |
| Persistent profile file | ✅ | Versioned `profiles.json` in app config directory |
| Profile validation | ✅ | Loaded document is migrated/validated before use |
| Create/edit/delete profile | ✅ | Tauri commands + React panel |
| Foreground process detection | ✅ | Windows foreground process name |
| Process auto switch | ✅ | Watcher checks every 500 ms |
| Apply clicker profile to UI | ✅ | Active profile updates displayed settings |
| Apply every advanced profile field to runtime | 🟡 | Depends on advanced clicker/hotkey wiring |

## Macros and Lua

| Feature | Status | Notes |
| --- | --- | --- |
| Shared macro event model | ✅ Core | Keyboard/mouse/delay actions are represented natively |
| Macro Studio editor | 🟡 | Rich React editor exists |
| Macro Studio keyboard recording | 🟡 | Current UI recorder uses frontend keyboard events |
| Native global macro recording | 🟡 | Low-level hook infrastructure exists but is not connected to Macro Studio end-to-end |
| Native macro playback | 🟡 | Core model exists; desktop playback command is not currently exposed |
| Lua compiler/runtime | ✅ Core | Sandboxed `mlua` runtime emits native macro events |
| Lua desktop workflow | 🟡 | Not fully exposed through current Tauri UI |

## Remapping

| Feature | Status | Notes |
| --- | --- | --- |
| Remap data model | ✅ Core | Input → action mapping model exists |
| Key Remap UI | 🟡 | Current page is a UI shell/demo table |
| Runtime key/mouse remapping | 🟡 | Native execution/wiring still required |
| Recursive self-trigger prevention | ✅ Foundation | Injected event tagging/filtering exists in the input/hook stack |

## Updates and diagnostics

| Feature | Status | Notes |
| --- | --- | --- |
| Local session logs | ✅ | Rotating diagnostics infrastructure |
| Crash/panic capture | ✅ | Panic hook writes diagnostic context |
| Performance log channel | ✅ | Precision worker records run summaries |
| Clear/snapshot diagnostics | ✅ | Tauri commands exposed |
| GitHub update check | ✅ | Updater module + Settings UI |
| SHA-256 patch verification | ✅ | Optional small patch staging path |
| Published GitHub Release | ⚪ | No release is currently published |

## Completion rule

For VxClick, a feature should not be marked complete solely because its UI exists. The runtime path, validation, stop behavior, failure handling and regression checks must also be in place.
