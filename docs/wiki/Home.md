# VxClick Wiki

VxClick is a Windows automation application built around a native Rust timing and input core. Version **1.0.0** includes the precision auto clicker, native global hotkeys, Macro Studio, key/mouse remapping, Lua automation, process-aware profiles, diagnostics and Windows release bundles.

## Download VxClick 1.0.0

For normal installation, use the x64 setup executable from the official GitHub Release. A portable executable and MSI package are also available.

- [VxClick 1.0.0 release](https://github.com/Vxiey/VxClick/releases/tag/v1.0.0)
- [Windows x64 setup](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64-setup.exe)
- [Portable VxClick.exe](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick.exe)
- [Windows x64 MSI](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64_en-US.msi)

## Start here

| Guide | Purpose |
| --- | --- |
| [Installation](Installation) | Install, run and update VxClick |
| [Auto Clicker](Auto-Clicker) | CPS, interval, modes, burst and position targeting |
| [Macro Studio](Macro-Studio) | Record, edit, save and play native macros |
| [Key Remapping](Key-Remapping) | Key, mouse and chord remapping |
| [Lua Automation](Lua-Automation) | Validate and run sandboxed Lua automation |
| [Profiles](Profiles) | Persistent profiles and process auto switching |
| [Diagnostics](Diagnostics) | Logs, benchmark telemetry and troubleshooting |
| [License](License) | MIT License terms and copyright notice |

## Technical documentation

For implementation details, use [Architecture](Architecture), [Timing Engine](Timing-Engine), [Automation Systems](Automation-Systems), [Feature Status](Feature-Status) and the [Development](Development) guide.

## License

VxClick is distributed under the **MIT License**.

Copyright (c) 2026 Vxiey

See the full [License](License) page or the canonical repository [`LICENSE`](https://github.com/Vxiey/VxClick/blob/main/LICENSE) file.

## Safety

Always configure an Emergency Stop hotkey before running long automation sessions. VxClick tags its own injected input to reduce recursive triggering and performs best-effort release of synthetic held inputs when automation is cancelled.

VxClick does **not** include anti-cheat bypass, anti-detection, process injection or stealth functionality. Use automation only where permitted by the software or service you are interacting with.
