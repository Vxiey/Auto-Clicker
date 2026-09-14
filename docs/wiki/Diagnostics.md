# Diagnostics

VxClick keeps diagnostics useful without logging raw typed text by default.

## Log files

- `session-<timestamp>-<pid>.log` — app/session lifecycle, subsystem errors and configuration-level events.
- `crash.log` — Rust panic information and source/thread context.
- `performance.log` — clicker and benchmark summaries.

The click hot path does not perform disk I/O per click.

## Precision benchmark

The benchmark reports requested CPS, actual CPS, mean interval, jitter percentiles, worst deviation and missed deadlines. Use it to compare changes before and after scheduler/timing modifications.

## Privacy

Normal keyboard text and Macro Recorder event streams are not dumped into diagnostic logs by default.
