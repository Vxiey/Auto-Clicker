import { useState } from "react";
import {
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  Code2,
  Download,
  FileText,
  PackageCheck,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { updaterApi, type UpdateInfo } from "./api";
import { Button, Card } from "./components";
import { APP_VERSION } from "./version";

const CHANGELOG = [
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

export function UpdatePanel() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [message, setMessage] = useState("");
  const [showChanges, setShowChanges] = useState(true);

  const check = async () => {
    setChecking(true);
    setMessage("");
    try {
      setInfo(await updaterApi.check());
    } catch (error) {
      setMessage(String(error));
    } finally {
      setChecking(false);
    }
  };

  const installUpdate = async () => {
    if (!info?.available || !info.full_release) return;
    setInstalling(true);
    setMessage(`Downloading and verifying v${info.latest_version}...`);
    try {
      await updaterApi.installFull(info);
      setMessage(`Installer for v${info.latest_version} launched.`);
    } catch (error) {
      setMessage(String(error));
      setInstalling(false);
    }
  };

  const automaticInstallReady = Boolean(
    info?.available && info.full_release && info.full_release.sha256,
  );

  return (
    <Card>
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <div>
          <h2 className="card-title">About & Updates</h2>
          <div className="card-copy">Version history, project links, legal notices and verified GitHub updates.</div>
        </div>
        <div className="quick-actions" style={{ marginTop: 0 }}>
          <a className="button" href="https://github.com/Vxiey/VxClick/blob/main/TERMS.md" target="_blank" rel="noreferrer">
            <FileText size={14} /> Terms of Service
          </a>
          <a className="button" href="https://github.com/Vxiey/VxClick" target="_blank" rel="noreferrer">
            <Code2 size={14} /> GitHub
          </a>
          <Button disabled={checking || installing} onClick={() => void check()}>
            <RefreshCw size={14} /> Check updates
          </Button>
        </div>
      </div>

      <div className="divider" />
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <div className="inline">
          <strong>Version v{APP_VERSION}</strong>
          {info && (
            info.available
              ? <span className="status-pill status-recording">v{info.latest_version} available</span>
              : <span className="status-pill status-running">Up to date</span>
          )}
        </div>
        <Button onClick={() => setShowChanges((value) => !value)}>
          {showChanges ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          {showChanges ? "Hide Changes" : "Show Changes"}
        </Button>
      </div>

      {info?.available && (
        <div className="section-gap">
          {automaticInstallReady ? (
            <div className="quick-actions">
              <Button variant="primary" disabled={installing || checking} onClick={() => void installUpdate()}>
                <Download size={14} /> {installing ? `Installing v${info.latest_version}...` : `Install v${info.latest_version}`}
              </Button>
              <span className="card-copy">
                <ShieldCheck size={12} style={{ display: "inline", marginRight: 5 }} />
                Official GitHub installer · SHA-256 verified before launch
              </span>
            </div>
          ) : (
            <div className="card-copy" style={{ color: "var(--warning)" }}>
              <AlertTriangle size={12} style={{ display: "inline", marginRight: 5 }} />
              Automatic install is unavailable because this release does not expose a verified Windows installer digest.
            </div>
          )}
          {info.full_release && (
            <div className="card-copy section-gap">
              <PackageCheck size={12} style={{ display: "inline", marginRight: 5 }} />
              Installer: {info.full_release.name}
            </div>
          )}
          {info.patch && (
            <div className="card-copy section-gap">
              A verified delta patch is available ({info.patch.from_version} → {info.patch.to_version}), but VxClick uses the full verified installer until automatic patch application is implemented.
            </div>
          )}
        </div>
      )}
      {message && <div className="card-copy section-gap">{message}</div>}

      {showChanges && (
        <div className="section-gap" style={{ maxHeight: 430, overflowY: "auto", paddingRight: 8 }}>
          {CHANGELOG.map((release) => (
            <div key={release.version} style={{ marginBottom: 24 }}>
              <div className="inline" style={{ gap: 9 }}>
                <strong>v{release.version}</strong>
                <span className="card-copy">{release.date}</span>
              </div>
              {release.sections.map((section) => (
                <div key={section.title} style={{ marginTop: 12 }}>
                  <div style={{ fontSize: 12, fontWeight: 800, letterSpacing: ".06em" }}>{section.title}</div>
                  <ul className="card-copy" style={{ margin: "7px 0 0 18px", padding: 0, lineHeight: 1.55 }}>
                    {section.items.map((item) => <li key={item}>{item}</li>)}
                  </ul>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      <div className="divider" />
      <div className="card-copy" style={{ color: "var(--warning)" }}>
        <AlertTriangle size={13} style={{ display: "inline", marginRight: 6 }} />
        <strong>Use at your own risk.</strong> The developer does not accept responsibility for bans,
        suspensions, account penalties, or other consequences from using macros, Lua scripts, or recoil
        automation to gain an unfair advantage. You are responsible for following the rules and terms of
        service of the software or game you use VxClick with.
      </div>
    </Card>
  );
}
