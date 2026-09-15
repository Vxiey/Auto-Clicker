# VxClick Wiki

This directory is the version-controlled Wiki and technical documentation for VxClick. Documentation changes live in the same history as the code so architecture, feature status, releases and license information can evolve together.

## Start here

| Page | Purpose |
| --- | --- |
| [Getting started](getting-started.md) | Build and run the desktop app or native core |
| [Feature status](features.md) | Current native/UI feature status |
| [Architecture](architecture.md) | Core/UI boundaries and module responsibilities |
| [Timing engine](timing-engine.md) | QPC deadlines, waiting strategy, drift handling and telemetry |
| [Automation systems](automation-systems.md) | Macros, hotkeys, remapping, hooks and Lua |
| [Development guide](development.md) | Local checks, coding rules, release/version workflow |
| [Diagnostics](diagnostics.md) | Local logs and diagnostic data |
| [Regression checklist](regression-checklist.md) | Timing/input/UI validation before merging |
| [VxClick Wiki](wiki/Home.md) | User-facing Wiki pages synchronized to GitHub Wiki |
| [MIT License](wiki/License.md) | License terms and copyright notice |
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

VxClick targets Windows. Windows-specific behavior is isolated under `src/platform/windows/` where practical, while automation models and shared logic remain in the core crate.

## Version

The current public release is **VxClick 1.0.0**. Desktop, core documentation, release notes and Wiki pages should describe the same release state.

## License

VxClick is released under the **MIT License**.

Copyright (c) 2026 Vxiey

The canonical license text is stored in the repository root [`LICENSE`](../LICENSE). The GitHub Wiki mirrors it on the [License](wiki/License.md) page.
