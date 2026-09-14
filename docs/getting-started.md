# Getting started

## Requirements

VxClick currently targets Windows 10/11 x64.

You need:

- Rust with the MSVC Windows target
- Visual Studio Build Tools with C++ build support
- Node.js and npm
- Microsoft Edge WebView2 Runtime
- Git

## Clone

```powershell
git clone https://github.com/Vxiey/VxClick.git
cd VxClick
```

## Run the desktop app

Install frontend dependencies:

```powershell
npm install
```

Start Tauri in development mode:

```powershell
npm run tauri:dev
```

The Tauri configuration starts the Vite development server automatically and opens the VxClick desktop window.

## Build the desktop app

```powershell
npm run tauri:build
```

Tauri writes platform bundles under its normal target output inside `src-tauri/target/`.

As of v0.2.0 there is no published GitHub Release, so do not assume an installer is available from Releases yet.

## Build and test the Rust core

```powershell
cargo build --release
cargo test
```

The root crate is the automation core used by the desktop app.

## Precision benchmark

Run the CLI benchmark without the desktop UI:

```powershell
cargo run --release -- benchmark --cps 100 --seconds 10 --button left
```

Supported button values:

- `left`
- `right`
- `middle`
- `x1`
- `x2`

Example high-rate runs:

```powershell
cargo run --release -- benchmark --cps 500 --seconds 10 --button left
cargo run --release -- benchmark --cps 2000 --seconds 5 --button left
```

The current safety cap is 20,000 CPS.

## Benchmark output

The CLI currently reports:

- target CPS
- actual CPS
- click count
- elapsed time
- mean interval
- p95 interval
- p99 interval
- mean jitter
- p95 jitter
- p99 jitter
- worst jitter
- missed deadlines

Use these values when evaluating scheduler changes. A change that looks faster but increases jitter, missed deadlines, stop latency or CPU use is not automatically an improvement.

## Desktop controls in v0.2.0

The desktop app contains Dashboard, Auto Clicker, Macros, Key Remap, Profiles and Settings pages.

The basic desktop clicker path is native and functional: the UI sends target CPS and mouse button to the Tauri backend, which starts a dedicated Rust precision worker.

Some advanced controls are not fully connected to the worker yet. Check [Feature status](features.md) before relying on a UI option in automation.
