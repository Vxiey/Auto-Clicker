import {
  ArrowRight,
  Bolt,
  CircleStop,
  Crosshair,
  Gauge,
  Keyboard,
  KeyRound,
  Layers3,
  MousePointer2,
  Play,
  Settings,
  Sparkles,
  UserRoundCog,
  Zap,
} from "lucide-react";
import { Button } from "./components";
import { APP_VERSION } from "./version";
import "./dashboard.css";

type DashboardPage = "clicker" | "recoil" | "macros" | "keybinds" | "remap" | "profiles" | "settings";

type EngineStatus = {
  running: boolean;
  clicks: number;
  actual_cps: number;
  target_cps: number;
};

type DashboardPanelProps = {
  status: EngineStatus;
  cps: number;
  activeProfile: string;
  onStart: () => void;
  onStop: () => void;
  open: (page: DashboardPage) => void;
};

const featureCards = [
  {
    page: "clicker" as const,
    title: "Auto Clicker",
    copy: "Set click type, speed, interval and precision controls.",
    icon: MousePointer2,
    tone: "blue",
  },
  {
    page: "macros" as const,
    title: "Macros",
    copy: "Record and replay keyboard and mouse actions with native timing.",
    icon: Sparkles,
    tone: "purple",
  },
  {
    page: "remap" as const,
    title: "Key Remap",
    copy: "Remap keys and mouse buttons with process-aware rules.",
    icon: Keyboard,
    tone: "green",
  },
  {
    page: "profiles" as const,
    title: "Profiles",
    copy: "Keep automation settings and process bindings separated.",
    icon: Layers3,
    tone: "amber",
  },
];

export function DashboardPanel({ status, cps, activeProfile, onStart, onStop, open }: DashboardPanelProps) {
  const actualCps = Number.isFinite(status.actual_cps) ? status.actual_cps : 0;
  const targetCps = Number.isFinite(cps) ? cps : 0;

  return (
    <div className="page dashboard-page">
      <section className="dashboard-hero">
        <div className="dashboard-hero-copy">
          <div className="dashboard-eyebrow"><Zap size={13} /> Windows automation, refined</div>
          <h1>Welcome to <span>VxClick</span></h1>
          <p>Fast, precise automation tools built around a native Rust timing engine.</p>
          <div className="dashboard-hero-tags">
            <span><Bolt size={13} /> High precision</span>
            <span><Gauge size={13} /> {targetCps < 1 ? targetCps.toFixed(6) : targetCps.toFixed(0)} target CPS</span>
            <span><UserRoundCog size={13} /> {activeProfile}</span>
          </div>
        </div>
        <div className={`dashboard-system-state ${status.running ? "running" : "ready"}`}>
          <span className="dashboard-state-dot" />
          <div>
            <strong>{status.running ? "Running" : "Ready"}</strong>
            <span>{status.running ? "Automation engine active" : "All systems operational"}</span>
          </div>
        </div>
      </section>

      <div className="dashboard-shell">
        <main className="dashboard-main-column">
          <section className="dashboard-feature-grid">
            {featureCards.map(({ page, title, copy, icon: Icon, tone }) => (
              <button key={page} className={`dashboard-feature-card tone-${tone}`} onClick={() => open(page)}>
                <span className="dashboard-feature-icon"><Icon size={23} /></span>
                <span className="dashboard-feature-copy">
                  <strong>{title}</strong>
                  <span>{copy}</span>
                </span>
                <ArrowRight className="dashboard-feature-arrow" size={19} />
              </button>
            ))}
          </section>

          <section className="dashboard-bottom-grid">
            <div className="dashboard-panel dashboard-quick-panel">
              <div className="dashboard-panel-head">
                <div>
                  <span className="dashboard-panel-kicker">Quick Actions</span>
                  <h2>Control VxClick</h2>
                </div>
                <span className={`dashboard-mini-state ${status.running ? "running" : "ready"}`}>
                  <span /> {status.running ? "Active" : "Standby"}
                </span>
              </div>
              <div className="dashboard-quick-actions">
                <Button variant="primary" disabled={status.running} onClick={onStart}><Play size={15} /> Start Clicking</Button>
                <Button variant="danger" disabled={!status.running} onClick={onStop}><CircleStop size={15} /> Stop</Button>
                <Button onClick={() => open("macros")}><Sparkles size={15} /> Record Macro</Button>
                <Button onClick={() => open("recoil")}><Crosshair size={15} /> Recoil Scripts</Button>
              </div>
            </div>

            <div className="dashboard-panel dashboard-status-panel">
              <div className="dashboard-panel-head">
                <div>
                  <span className="dashboard-panel-kicker">Live Status</span>
                  <h2>Engine telemetry</h2>
                </div>
                <Button onClick={() => open("settings")}><Settings size={14} /> Settings</Button>
              </div>
              <div className="dashboard-status-list">
                <StatusRow label="Actual CPS" value={actualCps.toFixed(1)} emphasis />
                <StatusRow label="Target CPS" value={targetCps < 1 ? targetCps.toFixed(6) : targetCps.toFixed(1)} />
                <StatusRow label="Session clicks" value={status.clicks.toLocaleString()} />
                <StatusRow label="Active profile" value={activeProfile} />
              </div>
            </div>
          </section>
        </main>

        <aside className="dashboard-rail">
          <div className="dashboard-panel dashboard-rail-card profile-card">
            <div className="dashboard-rail-icon"><UserRoundCog size={18} /></div>
            <div>
              <span className="dashboard-panel-kicker">Active Profile</span>
              <h3>{activeProfile}</h3>
              <p>Process-aware settings and hotkeys are ready.</p>
            </div>
            <button className="dashboard-text-link" onClick={() => open("profiles")}>Manage profile <ArrowRight size={14} /></button>
          </div>

          <div className="dashboard-panel dashboard-rail-card">
            <div className="dashboard-panel-head compact">
              <div><span className="dashboard-panel-kicker">Hotkeys</span><h3>Global controls</h3></div>
              <KeyRound size={17} />
            </div>
            <div className="dashboard-keybind-list">
              <div><span>Macro recorder</span><kbd>F1</kbd></div>
              <div><span>Stop recorder</span><kbd>F2</kbd></div>
              <div><span>Clicker / remaps</span><button onClick={() => open("keybinds")}>View</button></div>
            </div>
          </div>

          <div className="dashboard-panel dashboard-rail-card update-card">
            <div className="dashboard-panel-head compact">
              <div><span className="dashboard-panel-kicker">VxClick</span><h3>Version {APP_VERSION}</h3></div>
              <Bolt size={17} />
            </div>
            <p>Official GitHub release channel with verified Windows updates.</p>
            <button className="dashboard-text-link" onClick={() => open("settings")}>Check updates <ArrowRight size={14} /></button>
          </div>

          <div className="dashboard-panel dashboard-rail-card activity-card">
            <div className="dashboard-panel-head compact">
              <div><span className="dashboard-panel-kicker">Activity</span><h3>This session</h3></div>
              <Gauge size={17} />
            </div>
            <div className="dashboard-activity-number">{status.clicks.toLocaleString()}</div>
            <span>clicks recorded by the engine</span>
            <div className="dashboard-activity-bar"><span style={{ width: `${Math.min(100, Math.max(4, actualCps / Math.max(1, targetCps) * 100))}%` }} /></div>
          </div>
        </aside>
      </div>
    </div>
  );
}

function StatusRow({ label, value, emphasis = false }: { label: string; value: string; emphasis?: boolean }) {
  return (
    <div className="dashboard-status-row">
      <span>{label}</span>
      <strong className={emphasis ? "emphasis" : ""}>{value}</strong>
    </div>
  );
}
