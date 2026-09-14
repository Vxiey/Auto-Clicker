# Installation

## Recommended installation

1. Download the official [VxClick 1.0.0 x64 setup](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64-setup.exe).
2. Run the installer and complete the Windows installation flow.
3. Start VxClick.
4. Confirm the active profile and configure an **Emergency Stop** hotkey before enabling automation.
5. Test the clicker at a low CPS first, then increase the rate as needed.

The complete release page is available at [VxClick 1.0.0](https://github.com/Vxiey/VxClick/releases/tag/v1.0.0).

## Portable build

Download [VxClick.exe](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick.exe) if you want the portable build without running an installer.

The portable executable uses the same native Rust/Tauri application runtime as the installed build. Configuration, profiles, macros and diagnostics are stored in VxClick's application data locations rather than beside the executable.

## MSI package

For deployment or MSI-based installation, use [VxClick_1.0.0_x64_en-US.msi](https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64_en-US.msi).

## Requirements

- Windows 10 or Windows 11 x64
- Microsoft Edge WebView2 Runtime
- Standard user privileges for normal desktop automation

Windows input isolation still applies. If the target application is running elevated, Windows may require VxClick to run at the same integrity level before injected input can interact with it.

## First-run checklist

- Set an Emergency Stop hotkey.
- Verify the selected mouse button and CPS/interval.
- Check whether the correct profile is active.
- Test Macro Studio playback before assigning unattended loops.
- Use Diagnostics if actual CPS or hotkey behavior differs from the requested settings.

## Updating

Open **Settings → About & Updates → Check updates**. Full updates are sourced from the official `Vxiey/VxClick` GitHub Releases.

For troubleshooting after installation, see [Diagnostics](Diagnostics).
