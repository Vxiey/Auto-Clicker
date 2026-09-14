# VxClick regression checklist

This checklist is intentionally based on failure classes seen in public Windows automation tools, while VxClick's implementation remains clean-room.

## Hotkeys and input capture

- [ ] F1-F24, Print Screen, Pause/Break, Escape, Enter and navigation keys can be captured where Windows exposes them.
- [ ] Left/right Ctrl, Alt, Shift and Win are distinguishable.
- [ ] Standalone modifier hotkeys such as Left Alt are valid.
- [ ] Chords can contain up to five non-duplicate main keys.
- [ ] Inclusive modifier matching allows unrelated extra modifiers.
- [ ] Exact modifier matching rejects unrelated extra modifiers.
- [ ] A hotkey can be configured as pass-through or consumed.
- [ ] Hotkey capture happens in the native input layer, so keys such as F7 are not first handled by WebView/browser UI.
- [ ] VxClick's own SendInput events never re-trigger hotkeys, macros or remaps.
- [ ] Third-party remaps (for example a mouse utility mapping a side button to Num9) are accepted by default even when Windows marks them as injected.
- [ ] Physical-only mode can reject externally injected/remapped input when explicitly requested.
- [ ] Hotkey capture mode cannot accidentally start the clicker while a new binding is being selected.
- [ ] Conflicting start/stop/macro/remap bindings are detected before activation.
- [ ] Emergency stop always has higher priority than ordinary bindings.
- [ ] Fn is reported as unavailable when keyboard firmware does not expose it to Windows; the UI must not pretend it was captured.

## Keyboard injection

- [ ] Keyboard SendInput uses scan codes when Windows can map the virtual key.
- [ ] Extended keys set KEYEVENTF_EXTENDEDKEY correctly.
- [ ] Key-up uses the same scan/extended identity as key-down.
- [ ] Real hold means one key-down followed by one key-up, not repeated taps.
- [ ] Stop/cancel/error releases every VxClick-held key.

## Mouse safety

- [ ] Stop during a click hold still emits the matching mouse-up.
- [ ] Stop/cancel/error releases every VxClick-held mouse button.
- [ ] High CPS never leaves a button latched.
- [ ] Current-cursor mode does not queue stale cursor positions behind the pointer.
- [ ] Disabled stop zones are never evaluated.
- [ ] Edge/corner/zone coordinates are tested at 100/125/150/175/200% DPI and on mixed-DPI multi-monitor layouts.

## Timing

- [ ] Delay/interval units are validated at the config boundary.
- [ ] Absolute deadlines prevent cumulative drift.
- [ ] A long scheduling stall re-anchors instead of producing a catch-up click burst.
- [ ] 1, 10, 20, 50, 100, 500, 1000+ CPS benchmarks report actual CPS, jitter and missed deadlines.
- [ ] Idle/tray mode does not use a short polling loop; idle CPU is measured in CI/manual release testing.

## Profiles and processes

- [ ] Profile round-trip persists clicker settings, hotkeys, macro timelines, remaps, process rules and sequence points.
- [ ] Applying a profile updates both UI state and backend state atomically.
- [ ] Auto-switch has a deterministic fallback to Default when no process rule matches.
- [ ] Process/window titles are never truncated by raw UTF-8 byte offsets.
- [ ] Cyrillic, CJK, emoji and combining-character process titles do not panic or corrupt UI.

## Tray and lifecycle

- [ ] Only one VxClick instance owns the tray icon and global hooks.
- [ ] Launching VxClick while it is already in the tray activates the existing instance.
- [ ] Hooks are always removed during clean shutdown.
- [ ] Automation stop precedes shutdown so synthetic keys/buttons are released.
- [ ] Suspend/resume rebuilds any Windows handles that are no longer valid.
- [ ] Hotkey spam and start/stop spam cannot create multiple precision workers.

## UI input safety

- [ ] Mouse wheel over a number input does not silently modify the value unless the control is intentionally active.
- [ ] Long translations cannot hide required controls; pages remain scrollable.
- [ ] Window bounds are clamped to the current monitor work area.
- [ ] Theme changes do not alter font metrics/layout unexpectedly.

## Packaging and updater

- [ ] Clean Windows VM can install and launch without a separately installed VC++ runtime assumption.
- [ ] WebView2 absence is detected with a useful recovery message.
- [ ] Update artifacts come only from the official VxClick GitHub release source.
- [ ] Small patches are SHA-256 verified before staging.
- [ ] Failed update/install leaves the previous version runnable.
- [ ] Release artifacts/checksums are transparent to reduce false-positive/update trust problems.

## Crash containment

- [ ] Invalid/corrupt config falls back safely instead of aborting startup.
- [ ] Input hook queue saturation drops telemetry/recording events rather than blocking the Windows hook callback.
- [ ] Worker panic/error transitions engine state back to stopped.
- [ ] Local diagnostics record the last engine/hotkey/updater error without logging every click.
