# VxClick Wiki

This directory is the version-controlled Wiki for VxClick. Documentation changes live in the same history as the code so architecture and feature status can evolve together.

## Start here

| Page | Purpose |
| --- | --- |
| [Getting started](getting-started.md) | Build and run the desktop app or native core |
| [Feature status](features.md) | What works now, what is partially wired, and what is planned |
| [Architecture](architecture.md) | Core/UI boundaries and module responsibilities |
| [Timing engine](timing-engine.md) | QPC deadlines, waiting strategy, drift handling and telemetry |
| [Automation systems](automation-systems.md) | Macros, hotkeys, remapping, hooks and Lua |
| [Development guide](development.md) | Local checks, coding rules, release/version workflow |
| [Diagnostics](diagnostics.md) | Local logs and diagnostic data |
| [Regression checklist](regression-checklist.md) | Timing/input/UI validation before merging |
| [Changelog](../CHANGELOG.md) | Version history |

## Project priorities

VxClick development should preserve this order:

1. Stability
2. Timing precision
3. Low input latency
4. Low CPU/RAM use
5. Safe start/stop behavior
6. Ease of use
7. Features
8. Visual polish

A feature is not considered complete merely because a control exists in the UI. Native behavior, stop handling, validation and regression coverage are part of completion.

## Platform scope

VxClick currently targets Windows. Windows-specific behavior is isolated under `src/platform/windows/` where practical, while automation models and shared logic remain in the core crate.

## Versioning note

The desktop app is currently `0.2.0`. The root Rust core crate remains versioned independently as `0.1.0`.
