import { useState } from "react";
import {
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  Download,
  Github,
  PackageCheck,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { updaterApi, type UpdateInfo } from "./api";
import { Button, Card } from "./components";

const CHANGELOG = [
  {
    version: "0.2.0",
    date: "14.09.2026",
    sections: [
      {
        title: "NEW",
        items: [
          "New dark blue/cyan Tauri + React desktop UI with Dashboard, Auto Clicker, Macro Studio, Key Remap, Profiles and Settings.",
          "Native Windows low-level keyboard and mouse hooks for global recording and advanced hotkeys.",
          "Game/application profiles with process-aware auto switching.",
          "Macro Studio with assignment library, editable timeline, repeat modes and native-event model.",
          "Sandboxed Lua macro compiler that outputs into the same native macro event pipeline.",
          "GitHub Releases updater with support for SHA-256 verified small patch packages.",
          "Local troubleshooting logs: session, crash and performance logs with rotation.",
        ],
      },
      {
        title: "CHANGED",
        items: [
          "Hotkey matching now distinguishes physical input, external remaps and VxClick-generated input.",
          "Hotkeys support left/right Ctrl, Alt, Shift and Win variants, chords and consume/pass-through behavior.",
          "Keyboard output prefers scan-code SendInput and keeps extended-key identity where required.",
          "Precision engine remains isolated from UI work and uses native scheduling/input paths.",
        ],
      },
      {
        title: "FIXED",
        items: [
          "Prevent VxClick-generated SendInput events from recursively triggering VxClick hotkeys.",
          "Preserve extended-key flags for arrows, navigation keys and right-side modifier keys.",
          "Release Lua runtime closures before extracting the compiled macro timeline.",
          "Removed the legacy eframe UI path that conflicted with the new Tauri architecture.",
          "Updated GitHub Actions to Node 24 compatible action versions.",
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

  const stagePatch = async () => {
    if (!info?.patch) return;
    setChecking(true);
    try {
      const staged = await updaterApi.stagePatch(info.patch);
      setMessage(`Patch verified and staged: ${staged.to_version}`);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setChecking(false);
    }
  };

  return (
    <Card>
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <div>
          <h2 className="card-title">About & Updates</h2>
          <div className="card-copy">Version history, project links and GitHub update channel.</div>
        </div>
        <div className="quick-actions" style={{ marginTop: 0 }}>
          <a
            className="button"
            href="https://github.com/Vxiey/Auto-Clicker"
            target="_blank"
            rel="noreferrer"
          >
            <Github size={14} /> GitHub
          </a>
          <Button disabled={checking} onClick={() => void check()}>
            <RefreshCw size={14} /> Check updates
          </Button>
        </div>
      </div>

      <div className="divider" />
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <div className="inline">
          <strong>Version v0.2.0</strong>
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

      {info && (
        <div className="section-gap">
          {info.patch && (
            <div className="quick-actions">
              <Button variant="primary" disabled={checking} onClick={() => void stagePatch()}>
                <Download size={14} /> Stage small patch
              </Button>
              <span className="card-copy">
                <ShieldCheck size={12} style={{ display: "inline", marginRight: 5 }} />
                {info.patch.from_version} → {info.patch.to_version} · SHA-256 verified
              </span>
            </div>
          )}
          {info.full_release && (
            <div className="card-copy section-gap">
              <PackageCheck size={12} style={{ display: "inline", marginRight: 5 }} />
              Full package: {info.full_release.name}
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
