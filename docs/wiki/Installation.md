# Installation

## Recommended

1. Download the Windows installer from the VxClick 1.0.0 GitHub Release.
2. Run the installer.
3. Start VxClick and verify that the global emergency-stop hotkey is configured before enabling automation.

## Portable build

The release also contains `VxClick.exe` for portable use. Keep it together with its local configuration/log directories when moving it between systems.

## Requirements

- Windows 10 or Windows 11 x64
- Microsoft WebView2 Runtime for the Tauri UI
- Standard user privileges are sufficient for normal desktop automation; applications running at a higher integrity level may require VxClick to run at the same level for Windows input interaction.

## Updating

Use **Settings → About & Updates → Check updates**. Full updates are sourced from this repository's GitHub Releases. Small patches are accepted only through the verified patch-manifest path used by VxClick.
