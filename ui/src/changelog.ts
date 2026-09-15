export const CHANGELOG = [
  {
    version: "1.0.4",
    date: "15.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Rebuilt Dashboard with a larger VxClick hero/status area, four feature cards, Quick Actions, live engine telemetry and a dedicated information rail.",
          "Dashboard shortcuts for Auto Clicker, Macros, Key Remap, Profiles, Recoil Scripts, Keybinds, Settings and update access.",
        ],
      },
      {
        title: "CHANGED",
        items: [
          "Dashboard now uses live engine data for actual CPS, target CPS, session clicks, active profile and engine state instead of generic metric tiles.",
          "Dashboard styling is isolated in DashboardPanel.tsx and dashboard.css so future UI work does not keep growing App.tsx.",
          "Responsive dashboard layout now collapses the information rail and feature grid cleanly on smaller windows and higher Windows display scaling.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "Added the missing 1.0.3 and 1.0.4 entries to both repository and in-app changelogs.",
          "Release validation now blocks publishing if the current version is missing from the repository changelog, in-app changelog or release notes.",
        ],
      },
    ],
  },
  {
    version: "1.0.3",
    date: "15.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Native press-to-bind hotkey capture across Auto Clicker, Keybinds and Macro Studio for keyboard keys, modifier chords and Mouse1-Mouse5.",
          "Expanded Recoil Script imports for .lua, .ahk, .csv, .txt and .recoil in addition to .json / .vxrecoil.",
          "Windows system-tray behavior with Open VxClick and Exit VxClick actions.",
        ],
      },
      {
        title: "CHANGED",
        items: [
          "Macro Studio includes New and Duplicate macro actions.",
          "Timeline rows can be moved up/down and duplicated while keeping the existing native playback architecture.",
          "Added common keyboard assignments and useful delay presets for faster macro creation.",
          "Closing the VxClick window hides it to the tray instead of terminating the automation runtime.",
          "Terms of Service and GitHub links open through the Windows default browser instead of relying on WebView _blank behavior.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "Macro recording no longer creates streams of tiny delay-only rows from unsupported mouse-move hook traffic.",
          "Recorded event IDs are allocated correctly when delay rows are inserted, avoiding duplicate timeline IDs.",
          "Hotkey capture suppresses the captured input briefly so a newly assigned binding does not immediately fire.",
          "Fixed About & Updates links that could appear clickable without opening anything in the Tauri WebView.",
        ],
      },
    ],
  },
  {
    version: "1.0.2",
    date: "15.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Dedicated Keybinds tab for clicker, emergency stop, recorder, macro and remap bindings.",
          "Keybind conflict detection including reserved F1/F2 macro-recorder bindings.",
          "Per-profile Process List with individual process removal, executable addition and one-click foreground-process binding.",
          "Recoil Script upload/import for .json and .vxrecoil preset files.",
        ],
      },
      {
        title: "CHANGED",
        items: [
          "Profiles show bound applications as individual removable entries instead of only comma-separated process text.",
          "Imported recoil presets receive a new local ID before backend validation and saving.",
          "Layouts wrap more cleanly on smaller windows and higher Windows display scaling.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "Macro Studio now follows native recorder state so F1 start and F2 stop are visible immediately.",
          "Completed F1/F2 recordings refresh the saved macro list and surface the saved draft in the UI.",
          "Fixed content clipping that could hide lower controls or prevent vertical scrolling at some DPI/window sizes.",
        ],
      },
    ],
  },
  {
    version: "1.0.1",
    date: "15.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Recoil Scripts with multi-game profiles, primary/secondary slots and user-created Game -> Character/Operator -> Weapon metadata.",
          "Persistent Lua Script Library with create, edit, save, upload, rename, duplicate and delete.",
          "Global macro recording with F1 to start and F2 to stop, saving recordings as unassigned macro drafts.",
          "Keyboard and mouse remapping support including Mouse1-Mouse5.",
          "Auto Clicker interval units for milliseconds, seconds, minutes and hours.",
          "In-app Terms of Service access with TERMS.md and DISCLAIMER.md.",
        ],
      },
      {
        title: "CHANGED",
        items: [
          "Long click intervals use adaptive waiting to reduce CPU wakeups while keeping stop response fast.",
          "Low-CPS validation supports intervals below 1 CPS while retaining the high-CPS safety cap.",
          "Updater can select an official Windows installer, verify its GitHub SHA-256 digest, launch it and then exit VxClick.",
          "Recorder hotkeys F1/F2 are reserved to prevent conflicts with clicker, macro and remap bindings.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "Removed process-wide HIGH_PRIORITY_CLASS behavior that could starve the UI/system at high click rates.",
          "Live click scheduling avoids catch-up bursts after missed deadlines, reducing high-CPS overshoot and lag.",
          "Fixed recoil Windows input polling and cleaned Rust formatting/dead-code warnings caught by CI.",
        ],
      },
    ],
  },
  {
    version: "1.0.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "First VxClick 1.0 release candidate with unified Windows automation core and desktop UI.",
          "Precision benchmark panel/API for measured CPS, interval percentiles, jitter and missed deadlines.",
        ],
      },
      {
        title: "CHANGED",
        items: [
          "Core, desktop bundle and frontend versions are synchronized at 1.0.0.",
          "Release links and updater channel now target the renamed Vxiey/VxClick repository.",
        ],
      },
    ],
  },
  {
    version: "0.9.0",
    date: "14.09.2026",
    sections: [
      {
        title: "CHANGED",
        items: [
          "Hardened GitHub updater to accept only official VxClick release assets and supported patch formats.",
          "Small patches require valid source/target versions, SHA-256, size limits and official release URLs.",
        ],
      },
    ],
  },
  {
    version: "0.8.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Native precision benchmark command returning actual CPS, deviation, interval p50/p95/p99, jitter and missed deadlines.",
        ],
      },
    ],
  },
  {
    version: "0.7.0",
    date: "14.09.2026",
    sections: [
      {
        title: "FIXED",
        items: [
          "Synthetic held keys and mouse buttons are tracked and released on emergency stop or runtime shutdown.",
          "Failed click injection makes a best-effort mouse-up to reduce stuck-button states.",
        ],
      },
    ],
  },
  {
    version: "0.6.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Native hotkeys now follow the active game/application profile automatically.",
          "Toggle, hold-to-click and single-click activation modes are handled outside the WebView.",
        ],
      },
    ],
  },
  {
    version: "0.5.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Stateful remap rule engine with key, mouse and chord triggers, process scopes and consume/pass-through decisions.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "Chord mappings fire on their activation edge instead of repeating on every keyboard repeat event.",
        ],
      },
    ],
  },
  {
    version: "0.4.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "QPC-based macro playback with absolute deadlines, 0.1x–10x speed scaling and cancellation support.",
          "Macro playback releases locally held keys/buttons after completion, cancellation or failure.",
        ],
      },
    ],
  },
  {
    version: "0.3.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Native global clicker hotkey runtime that works while VxClick is unfocused or minimized.",
          "Emergency stop is processed in the Windows hook path rather than relying on browser focus.",
        ],
      },
    ],
  },
  {
    version: "0.2.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Dark blue/cyan Tauri + React UI with Dashboard, Auto Clicker, Macro Studio, Key Remap, Profiles and Settings.",
          "Native Windows low-level keyboard and mouse hooks for recording and advanced hotkeys.",
          "Game/application profiles with process-aware auto switching.",
          "Sandboxed Lua macro compiler and GitHub updater with SHA-256 verified small patch support.",
          "Session, crash and performance logs with rotation.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "VxClick-generated SendInput no longer recursively triggers VxClick hotkeys.",
          "Extended-key flags are preserved for arrows, navigation keys and right-side modifiers.",
        ],
      },
    ],
  },
  {
    version: "0.1.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "Initial Rust automation core with native SendInput mouse and keyboard injection.",
          "QPC-based precision scheduler with absolute deadlines and click benchmarking.",
          "Initial macro/remap event models and Windows platform abstraction.",
        ],
      },
    ],
  },
] as const;
