# Macros, hotkeys, remapping and Lua

VxClick is designed around reusable native input/timing infrastructure rather than separate automation implementations for every feature.

## Shared direction

The long-term flow is:

```text
Trigger / recorded input
        │
        ▼
Macro or remap rule
        │
        ▼
Shared native event model
        │
        ├── key down / key up
        ├── mouse down / mouse up / click
        ├── delay
        └── loops / repetitions / sequencing
        │
        ▼
Precision scheduler + Windows input backend
```

This prevents Macro Studio, Lua and remapping from each creating their own timing/injection stack.

## Macro engine

The Rust core contains the macro event/trigger model in `src/macro_engine.rs`.

The React Macro Studio already provides an editor with concepts such as:

- no-repeat
- repeat while holding
- toggle
- sequence lanes (press / hold / release)
- key down/up events
- mouse actions
- delays
- assignment library
- editable delay values

### Current integration boundary

The current Macro Studio is not yet a complete native macro runtime. Its recording path listens to frontend keyboard events and the Tauri backend does not currently expose a native macro playback command.

The next robust step is to connect the editor to the existing native event model, then run playback through the precision scheduler/input backend.

## Windows hooks

`src/platform/windows/hooks.rs` contains low-level keyboard and mouse hook infrastructure.

Native hooks are required for features that must work when VxClick is not focused, including:

- global macro recording
- advanced global hotkeys
- key/mouse remapping
- hold-to-run automation

Hook callbacks should do minimal work. Expensive processing belongs outside the hook path.

## Hotkeys

The hotkey stack distinguishes:

- physical input
- externally injected/remapped input
- VxClick-generated input

The core includes support for modifier/chord handling, including left/right variants of Ctrl, Alt, Shift and Win, plus consume/pass-through behavior.

This is important for safe recursion handling. A VxClick output event should not automatically retrigger the rule that created it.

### Desktop status

The native hotkey engine exists, but the desktop's editable Start/Stop and Emergency Stop fields are not fully connected to the Tauri clicker commands yet.

An emergency stop should eventually be treated as a high-priority global control path shared by clicker, macros and remapping.

## Remapping

The architecture target is:

```text
Input → Rule → Action
```

Examples:

```text
Mouse4 → Space
CapsLock → Ctrl
Key → Macro
Mouse5 → Mouse1
```

`src/remap.rs` currently provides the core model, while the Key Remap page is still a UI shell rather than a complete runtime mapping editor/executor.

### Recursion prevention

Remapping must distinguish generated input from physical input. The existing injected-event tagging/filtering foundation should be reused rather than adding feature-specific recursion hacks.

## Lua runtime

`src/lua_runtime.rs` uses `mlua` with vendored Lua 5.4.

The design goal is for Lua scripts to compile/emit the same native macro event model used elsewhere. Lua should not bypass validation, timing or input safety by directly creating an unrelated injection path.

### Current desktop status

The Lua core exists, but there is not yet a complete end-user Lua authoring/execution workflow exposed by the current Tauri command list.

## Input cleanup

Macro/remap work must track synthetic key/button state so stopping automation cannot leave a key or mouse button logically held down whenever cleanup is possible.

Important failure cases include:

- user presses emergency stop during a held key
- macro playback errors midway through a sequence
- profile changes while a macro is active
- application shutdown
- Windows suspend/resume
- repeated start/stop or hotkey spam

Cleanup should be centralized where possible instead of duplicated per feature.
