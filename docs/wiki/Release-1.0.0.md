# VxClick 1.0.0

VxClick 1.0.0 is the first complete Windows release of the precision automation stack.

## Downloads

| Package | Use case |
| --- | --- |
| [VxClick_1.0.0_x64-setup.exe](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64-setup.exe) | Recommended Windows installer |
| [VxClick.exe](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick.exe) | Portable executable |
| [VxClick_1.0.0_x64_en-US.msi](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64_en-US.msi) | MSI deployment/install package |

Official release page: [VxClick 1.0.0 on GitHub](https://github.com/Vxiey/VxClick/releases/tag/v1.0.0).

## Included in 1.0.0

- Precision auto clicker with CPS and interval control
- Left, right, middle, X1 and X2 mouse buttons
- Toggle, hold and once hotkey behavior
- Randomized intervals and burst clicking
- Current-cursor, fixed-position and multi-point targeting
- Native global Windows hotkeys and Emergency Stop
- Macro Studio with native recording, persistence, editing and QPC playback
- Key, mouse and chord remapping with process scopes
- Sandboxed Lua validation and execution workflow
- Foreground-process profile auto switching
- Precision benchmark telemetry, local diagnostics and crash/performance logs
- GitHub update checking and Windows release bundles

## Timing architecture

The clicker uses `QueryPerformanceCounter`, absolute deadlines and bounded hybrid sleep/yield/spin waiting. Scheduling is kept outside the UI thread, and long stalls are re-anchored rather than converted into large catch-up bursts.

See [Timing Engine](Timing-Engine) for the implementation design.

## Verification

The final 1.0.0 source state passed the Windows CI pipeline including frontend build, Rust formatting checks, core compilation, **27 unit tests**, Clippy with warnings denied and the Tauri compile check.

## License

VxClick 1.0.0 is released under the **MIT License**.

Copyright (c) 2026 Vxiey

See the full [License](License) page or the repository [`LICENSE`](https://github.com/Vxiey/VxClick/blob/main/LICENSE) file.

## Safety

Emergency Stop requests automation cancellation and performs best-effort release of VxClick-held synthetic inputs.

VxClick does **not** include anti-cheat bypass, anti-detection, process injection or stealth functionality. You are responsible for following the rules and terms of service of the applications you automate.
