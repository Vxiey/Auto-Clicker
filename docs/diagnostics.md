# Diagnostics and troubleshooting logs

VxClick keeps diagnostic files in the platform log directory resolved by Tauri. The exact folder is exposed through the `diagnostics_snapshot` command so the UI can show it without hard-coding a Windows path.

## Files

- `session-<timestamp>-<pid>.log` — one bounded session log per launch. Keeps startup, engine start/stop, validation failures and subsystem errors. The newest eight sessions are retained.
- `crash.log` — panic/crash information including session id, thread and source location when available. Rotates at 2 MiB to `crash.log.1`.
- `performance.log` — completed run summaries such as target CPS, measured CPS, elapsed time and generated click count. It intentionally does not log every click. Rotates at 2 MiB to `performance.log.1`.

## Privacy rule

Diagnostics must never record arbitrary keyboard input, typed text, macro recorder streams or passwords by default. Hotkey and remap troubleshooting should log binding/configuration metadata and state transitions only when necessary, not the user's general keystrokes.

## Performance rule

Do not write logs from the per-click or sub-millisecond scheduler hot path. Collect counters in memory and write a summary when a run stops or when a benchmark completes. This avoids disk I/O affecting timing precision.

## Frontend commands

- `diagnostics_snapshot` returns current paths, session id and file sizes.
- `diagnostics_client_log` lets the React layer report UI errors/warnings to the current session log.
- `clear_diagnostics` removes old session/crash/performance logs while preserving the currently active session file.

When asking users for troubleshooting information, prefer the current session log plus `crash.log`. Request `performance.log` only for timing/CPS/jitter investigations.
