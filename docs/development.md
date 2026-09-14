# Development guide

## Principles

VxClick prioritizes:

1. stability
2. timing precision
3. input latency
4. CPU/RAM efficiency
5. safe start/stop behavior
6. usability
7. features
8. visual polish

Prefer the smallest robust fix that addresses root cause. Avoid broad rewrites when a contained change is safer.

## Repository layout

```text
.
├── src/                         Rust automation core
│   ├── engine/                  shared types + benchmark telemetry
│   ├── hotkeys.rs               higher-level hotkey logic
│   ├── lua_runtime.rs           Lua compiler/runtime
│   ├── macro_engine.rs          macro event model
│   ├── remap.rs                 remap model
│   └── platform/windows/        QPC, SendInput, hooks, hotkeys, scheduler
├── src-tauri/                   Tauri desktop backend
│   └── src/
│       ├── main.rs              commands + live engine state
│       ├── profiles.rs          persistent/process-aware profiles
│       ├── diagnostics.rs       logs/panic/performance diagnostics
│       └── updater.rs           update/patch handling
├── ui/                          React + TypeScript frontend
├── docs/                        version-controlled Wiki
├── CHANGELOG.md
└── Cargo.toml                   core crate
```

## Local checks

Run Rust formatting/tests/lints:

```powershell
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Build the frontend:

```powershell
npm install
npm run build
```

Run the full desktop app:

```powershell
npm run tauri:dev
```

Build a desktop release bundle:

```powershell
npm run tauri:build
```

For timing/input changes, also run representative native benchmarks:

```powershell
cargo run --release -- benchmark --cps 10 --seconds 10 --button left
cargo run --release -- benchmark --cps 100 --seconds 10 --button left
cargo run --release -- benchmark --cps 500 --seconds 10 --button left
cargo run --release -- benchmark --cps 2000 --seconds 10 --button left
```

Use [regression-checklist.md](regression-checklist.md) before treating a timing/input change as finished.

## Hot-path rules

Avoid in click/hook hot paths:

- per-event heap allocation when it can be avoided
- mutex contention
- per-click disk or console logging
- frontend round trips
- unbounded polling
- permanent busy spinning

Optimize after measuring. Do not trade away stability or stop behavior for a synthetic microbenchmark win.

## Input rules

- Use the existing Windows input abstraction instead of calling `SendInput` from arbitrary modules.
- Preserve scan-code/extended-key identity where required.
- Tag/filter VxClick-generated input so hotkeys/remaps cannot recursively trigger themselves.
- Track held synthetic input when adding playback/remapping features.
- Ensure stop/error/shutdown paths release held state wherever possible.

## Scheduler rules

- Preserve monotonic high-resolution timing.
- Prefer absolute deadlines over relative delay chains.
- Keep UI timing separate from scheduler timing.
- Check stop state during long waits and bounded spin periods.
- Do not emit a large catch-up burst after a long system stall.
- Benchmark short and long runs before changing thresholds/priority behavior.

## Configuration rules

Persistent configuration should be:

- schema-versioned
- validated on load
- migrated where practical
- safe against invalid values
- backward-compatible where reasonable

`profiles.json` already follows a schema-versioned document approach and should be used as the pattern for future persistent settings.

## Desktop/native boundary

A control appearing in React does not make a feature complete. When adding a setting:

1. add validated native representation
2. add/update Tauri command or persistent model
3. connect the UI
4. verify runtime behavior
5. add diagnostics only where they are useful
6. add regression coverage/checklist items

This avoids UI state that silently has no effect on the automation engine.

## Version updates

The desktop version currently appears in several places. When preparing a release, verify at minimum:

- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `package.json`
- the visible UI version string
- `CHANGELOG.md`

The root `auto-clicker-core` package has its own version and does not need to match the desktop version automatically.

## Pull request checklist

Before merging:

- identify the root cause/goal
- inspect affected modules, not only the first failing file
- run relevant Rust/frontend checks
- benchmark timing changes
- test start/stop spam and invalid values where relevant
- inspect the final diff
- update Wiki/README/CHANGELOG when behavior changes
- avoid claiming a feature is implemented if only the UI/model exists

## Clean-room rule

Other automation tools may be studied for feature ideas, UX patterns and benchmarking concepts, but VxClick implementation should remain independent unless third-party code has an explicitly compatible license and reuse is intentional/documented.
