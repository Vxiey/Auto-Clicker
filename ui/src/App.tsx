import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Activity,
  Bolt,
  CircleStop,
  Gauge,
  Keyboard,
  LayoutDashboard,
  Maximize2,
  Minimize2,
  MousePointer2,
  Play,
  Plus,
  Radio,
  Repeat2,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Square,
  UserRoundCog,
  X,
} from "lucide-react";
import { Button, Card, Field, MetricCard, StatusPill, Toggle } from "./components";

type Page = "dashboard" | "clicker" | "macros" | "remap" | "profiles" | "settings";
type EngineStatus = { running: boolean; clicks: number; actual_cps: number; target_cps: number };

const nav = [
  ["dashboard", "Dashboard", LayoutDashboard],
  ["clicker", "Auto Clicker", MousePointer2],
  ["macros", "Macros", Radio],
  ["remap", "Key Remap", Keyboard],
  ["profiles", "Profiles", UserRoundCog],
  ["settings", "Settings", Settings],
] as const;

const windowApi = getCurrentWindow();

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [status, setStatus] = useState<EngineStatus>({ running: false, clicks: 0, actual_cps: 0, target_cps: 250 });
  const [cps, setCps] = useState(250);
  const [button, setButton] = useState("left");
  const [mode, setMode] = useState("toggle");
  const [randomize, setRandomize] = useState(false);
  const [burst, setBurst] = useState(false);
  const [positionMode, setPositionMode] = useState("cursor");
  const [startHotkey, setStartHotkey] = useState("F6");
  const [stopHotkey, setStopHotkey] = useState("F8");
  const [tray, setTray] = useState(true);
  const [startup, setStartup] = useState(false);
  const [diagnostics, setDiagnostics] = useState(true);

  const intervalUs = useMemo(() => 1_000_000 / Math.max(cps, 0.001), [cps]);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const next = await invoke<EngineStatus>("engine_status");
        if (!cancelled) setStatus(next);
      } catch {
        // Browser preview works even when Tauri is not running.
      }
    };
    poll();
    const timer = window.setInterval(poll, 300);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const start = async () => {
    setStatus((current) => ({ ...current, running: true, target_cps: cps }));
    try {
      await invoke("start_clicker", { cps, button });
    } catch {
      // Keep preview interactive outside Tauri.
    }
  };

  const stop = async () => {
    setStatus((current) => ({ ...current, running: false, actual_cps: 0 }));
    try {
      await invoke("stop_clicker");
    } catch {
      // Keep preview interactive outside Tauri.
    }
  };

  const pageTitle = nav.find(([id]) => id === page)?.[1] ?? "Dashboard";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <img className="brand-mark" src="/app-icon.png" alt="VxClick" />
          <div>
            <div className="brand-title">VxClick</div>
            <div className="brand-subtitle">Windows automation</div>
          </div>
        </div>

        <nav className="nav">
          {nav.map(([id, label, Icon]) => (
            <button key={id} className={`nav-button ${page === id ? "active" : ""}`} onClick={() => setPage(id)}>
              <Icon size={17} strokeWidth={1.8} />
              <span>{label}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="inline"><span className={`status-dot ${status.running ? "status-running" : "status-ready"}`} style={{ backgroundColor: "currentColor" }} />{status.running ? "Engine running" : "Engine ready"}</div>
          <div style={{ marginTop: 7, opacity: 0.7 }}>v0.2.0 UI milestone</div>
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar" data-tauri-drag-region>
          <div className="topbar-left" data-tauri-drag-region>
            <img className="brand-mark" style={{ width: 28, height: 28, borderRadius: 9 }} src="/app-icon.png" alt="" />
            <strong>{pageTitle}</strong>
            <span className="profile-chip"><UserRoundCog size={13} /> Default</span>
          </div>
          <div className="topbar-right">
            <StatusPill status={status.running ? "Running" : "Ready"} />
            <div className="window-controls">
              <button className="icon-button" onClick={() => windowApi.minimize()} title="Minimize"><Minimize2 size={15} /></button>
              <button className="icon-button" onClick={() => windowApi.toggleMaximize()} title="Maximize"><Maximize2 size={14} /></button>
              <button className="icon-button danger" onClick={() => windowApi.close()} title="Close"><X size={16} /></button>
            </div>
          </div>
        </header>

        <div className="content">
          {page === "dashboard" && <Dashboard status={status} cps={cps} onStart={start} onStop={stop} open={(next) => setPage(next)} />}
          {page === "clicker" && (
            <AutoClicker
              status={status}
              cps={cps}
              setCps={setCps}
              intervalUs={intervalUs}
              button={button}
              setButton={setButton}
              mode={mode}
              setMode={setMode}
              randomize={randomize}
              setRandomize={setRandomize}
              burst={burst}
              setBurst={setBurst}
              positionMode={positionMode}
              setPositionMode={setPositionMode}
              startHotkey={startHotkey}
              setStartHotkey={setStartHotkey}
              stopHotkey={stopHotkey}
              setStopHotkey={setStopHotkey}
              onStart={start}
              onStop={stop}
            />
          )}
          {page === "macros" && <Macros />}
          {page === "remap" && <Remap />}
          {page === "profiles" && <Profiles />}
          {page === "settings" && <SettingsPage tray={tray} setTray={setTray} startup={startup} setStartup={setStartup} diagnostics={diagnostics} setDiagnostics={setDiagnostics} />}
        </div>
      </main>
    </div>
  );
}

function PageHeader({ title, subtitle, right }: { title: string; subtitle: string; right?: React.ReactNode }) {
  return <div className="page-header"><div><h1 className="page-title">{title}</h1><div className="page-subtitle">{subtitle}</div></div>{right}</div>;
}

function Dashboard({ status, cps, onStart, onStop, open }: { status: EngineStatus; cps: number; onStart: () => void; onStop: () => void; open: (p: Page) => void }) {
  return <div className="page">
    <PageHeader title="Dashboard" subtitle="Fast access to the click engine, macros and current profile." right={<StatusPill status={status.running ? "Running" : "Ready"} />} />
    <div className="grid grid-4">
      <MetricCard label="Status" value={status.running ? "Running" : "Ready"} accent icon={<Activity size={16} color="var(--cyan)" />} />
      <MetricCard label="Current CPS" value={(status.actual_cps || 0).toFixed(1)} accent icon={<Gauge size={16} color="var(--accent)" />} />
      <MetricCard label="Total clicks today" value={status.clicks.toLocaleString()} icon={<MousePointer2 size={16} color="var(--muted)" />} />
      <MetricCard label="Active profile" value="Default" icon={<UserRoundCog size={16} color="var(--muted)" />} />
    </div>

    <div className="grid grid-2 section-gap">
      <Card>
        <h2 className="card-title">Quick controls</h2>
        <div className="card-copy">Target {cps.toLocaleString()} CPS using the precision Rust engine.</div>
        <div className="quick-actions">
          <Button variant="primary" disabled={status.running} onClick={onStart}><Play size={14} /> Start</Button>
          <Button variant="danger" disabled={!status.running} onClick={onStop}><Square size={13} /> Stop</Button>
          <Button onClick={() => open("macros")}><Radio size={14} /> Record Macro</Button>
          <Button onClick={() => open("settings")}><Settings size={14} /> Open Settings</Button>
        </div>
      </Card>
      <Card>
        <h2 className="card-title">Precision engine</h2>
        <div className="card-copy">Absolute QPC deadlines, native SendInput backend and a dedicated high-priority worker.</div>
        <div className="quick-actions"><span className="profile-chip"><Bolt size={13} /> High performance</span><span className="profile-chip"><Sparkles size={13} /> Sub-ms ready</span></div>
      </Card>
    </div>

    <div className="grid grid-2 section-gap">
      <Card><h2 className="card-title">Recent activity</h2><div className="card-copy">Live diagnostics will appear here as sessions are recorded.</div><div className="divider" /><div className="inline" style={{ justifyContent: "space-between" }}><span className="card-copy">Click engine</span><span className="status-pill status-ready">Ready</span></div></Card>
      <TrayPreview running={status.running} />
    </div>
  </div>;
}

function AutoClicker(props: {
  status: EngineStatus; cps: number; setCps: (v: number) => void; intervalUs: number; button: string; setButton: (v: string) => void;
  mode: string; setMode: (v: string) => void; randomize: boolean; setRandomize: (v: boolean) => void; burst: boolean; setBurst: (v: boolean) => void;
  positionMode: string; setPositionMode: (v: string) => void; startHotkey: string; setStartHotkey: (v: string) => void; stopHotkey: string; setStopHotkey: (v: string) => void;
  onStart: () => void; onStop: () => void;
}) {
  return <div className="page">
    <PageHeader title="Auto Clicker" subtitle="Configure click behavior, precision, hotkeys and cursor targeting." right={<StatusPill status={props.status.running ? "Running" : "Stopped"} />} />
    <div className="grid grid-2">
      <Card>
        <h2 className="card-title">Click settings</h2><div className="card-copy">Core click behavior.</div>
        <div className="form-row section-gap">
          <Field label="Click type"><select className="select" value={props.button} onChange={(e) => props.setButton(e.target.value)}><option value="left">Left</option><option value="right">Right</option><option value="middle">Middle</option><option value="x1">X1 / Mouse 4</option><option value="x2">X2 / Mouse 5</option></select></Field>
          <Field label="Mode"><select className="select" value={props.mode} onChange={(e) => props.setMode(e.target.value)}><option value="hold">Hold</option><option value="toggle">Toggle</option><option value="once">Once</option></select></Field>
        </div>
        <div className="section-gap"><Field label={`CPS · ${props.cps.toFixed(1)}`}><input className="range" type="range" min="1" max="20000" step="1" value={props.cps} onChange={(e) => props.setCps(Number(e.target.value))} /></Field></div>
        <div className="form-row section-gap"><Field label="Exact CPS"><input className="input" type="number" min="1" max="20000" value={props.cps} onChange={(e) => props.setCps(Math.max(1, Math.min(20000, Number(e.target.value))))} /></Field><Field label="Interval"><input className="input" readOnly value={`${props.intervalUs.toFixed(3)} µs`} /></Field></div>
      </Card>

      <Card>
        <h2 className="card-title">Advanced timing</h2><div className="card-copy">Optional modes without cluttering the basic controls.</div>
        <SettingRow title="Randomization" copy="Controlled interval variation around the selected CPS." value={props.randomize} onChange={props.setRandomize} />
        <SettingRow title="Burst mode" copy="Maximum-throughput batches for workloads that do not need exact spacing." value={props.burst} onChange={props.setBurst} />
        <div className="divider" />
        <div className="card-copy">Precision mode stays enabled for normal click scheduling.</div>
      </Card>
    </div>

    <div className="grid grid-2 section-gap">
      <Card><h2 className="card-title">Position mode</h2><div className="form-row section-gap"><Field label="Target"><select className="select" value={props.positionMode} onChange={(e) => props.setPositionMode(e.target.value)}><option value="cursor">Current cursor</option><option value="fixed">Fixed position</option><option value="multi">Multi-point</option></select></Field><Field label="Coordinates"><input className="input" placeholder="X, Y" disabled={props.positionMode === "cursor"} /></Field></div></Card>
      <Card><h2 className="card-title">Hotkeys</h2><div className="form-row section-gap"><Field label="Start / Stop"><input className="input" value={props.startHotkey} onChange={(e) => props.setStartHotkey(e.target.value)} /></Field><Field label="Emergency stop"><input className="input" value={props.stopHotkey} onChange={(e) => props.setStopHotkey(e.target.value)} /></Field></div></Card>
    </div>

    <Card className="section-gap"><div className="inline" style={{ justifyContent: "space-between", width: "100%" }}><div><h2 className="card-title">Live status</h2><div className="card-copy">Target {props.cps.toFixed(1)} CPS · actual {props.status.actual_cps.toFixed(1)} CPS · {props.status.clicks.toLocaleString()} clicks</div></div><div className="quick-actions" style={{ marginTop: 0 }}><Button variant="primary" disabled={props.status.running} onClick={props.onStart}><Play size={14} /> Start</Button><Button variant="danger" disabled={!props.status.running} onClick={props.onStop}><CircleStop size={14} /> Stop</Button></div></div></Card>
  </div>;
}

function Macros() {
  return <div className="page"><PageHeader title="Macros" subtitle="Record, edit and bind keyboard and mouse sequences." right={<Button variant="primary"><Plus size={14} /> Create Macro</Button>} /><Card><table className="table"><thead><tr><th>Name</th><th>Trigger</th><th>Mode</th><th>Sequence</th><th>Status</th></tr></thead><tbody><tr><td>Quick Action</td><td><span className="kbd">Mouse 4</span></td><td>Once</td><td>F → 30 ms → Left Click</td><td><span className="status-pill status-ready">Enabled</span></td></tr></tbody></table></Card><div className="grid grid-2 section-gap"><Card><h2 className="card-title">Recorder</h2><div className="card-copy">Capture key down/up, mouse buttons, wheel, cursor movement and exact timing.</div><div className="quick-actions"><Button variant="primary"><Radio size={14}/> Record Macro</Button><Button><SlidersHorizontal size={14}/> Edit Sequence</Button></div></Card><Card><h2 className="card-title">Playback</h2><div className="card-copy">Repeat count, loop, hold activation and playback speed share the precision scheduler.</div></Card></div></div>;
}

function Remap() {
  return <div className="page"><PageHeader title="Key Remap" subtitle="Simple mappings for keyboard and mouse, with advanced rules when needed." right={<Button variant="primary"><Plus size={14}/> Add Mapping</Button>} /><Card><table className="table"><thead><tr><th>From</th><th></th><th>To</th><th>Context</th><th>Status</th></tr></thead><tbody><tr><td><span className="kbd">Mouse 5</span></td><td>→</td><td><span className="kbd">Ctrl + C</span></td><td>Global</td><td><span className="status-pill status-ready">Enabled</span></td></tr><tr><td><span className="kbd">Caps Lock</span></td><td>→</td><td><span className="kbd">F</span></td><td>Global</td><td><span className="status-pill status-ready">Enabled</span></td></tr></tbody></table></Card></div>;
}

function Profiles() {
  return <div className="page"><PageHeader title="Profiles" subtitle="Keep automation settings isolated and auto-switch them per process." right={<Button variant="primary"><Plus size={14}/> New Profile</Button>} /><div className="grid grid-2"><Profile name="Default" active copy="General Windows automation"/><Profile name="Game 1" copy="Custom click and macro settings"/><Profile name="Browser" copy="Browser-specific remaps"/><Profile name="Custom" copy="Create your own process rules"/></div></div>;
}
function Profile({ name, copy, active = false }: { name: string; copy: string; active?: boolean }) { return <Card><div className="inline" style={{ justifyContent:"space-between", width:"100%" }}><div><h2 className="card-title">{name}</h2><div className="card-copy">{copy}</div></div>{active ? <span className="status-pill status-running">Active</span> : <Button>Activate</Button>}</div></Card>; }

function SettingsPage({ tray, setTray, startup, setStartup, diagnostics, setDiagnostics }: { tray:boolean; setTray:(v:boolean)=>void; startup:boolean; setStartup:(v:boolean)=>void; diagnostics:boolean; setDiagnostics:(v:boolean)=>void }) {
  return <div className="page"><PageHeader title="Settings" subtitle="Windows behavior, diagnostics and precision tuning." />
    <div className="grid grid-2"><Card><h2 className="card-title">Windows behavior</h2><SettingRow title="Start with Windows" copy="Launch VxClick automatically after sign-in." value={startup} onChange={setStartup}/><SettingRow title="Minimize to tray" copy="Keep VxClick available without leaving the main window open." value={tray} onChange={setTray}/><SettingRow title="Diagnostics" copy="Store timing and engine diagnostics locally." value={diagnostics} onChange={setDiagnostics}/></Card><TrayPreview running={false}/></div>
    <div className="grid grid-2 section-gap"><Card><h2 className="card-title">Precision tuning</h2><div className="card-copy">Advanced scheduler values remain tucked away from normal users.</div><div className="form-row section-gap"><Field label="Spin window"><input className="input" defaultValue="350 µs" /></Field><Field label="Coarse wait"><input className="input" defaultValue="2000 µs" /></Field></div></Card><Card><h2 className="card-title">Theme</h2><div className="card-copy">VxClick Dark · #071225 background · #168BFF blue · #26E0FF cyan.</div><div className="quick-actions"><span className="profile-chip"><span className="status-dot" style={{background:"#168BFF"}}/> Accent Blue</span><span className="profile-chip"><span className="status-dot" style={{background:"#26E0FF"}}/> Accent Cyan</span></div></Card></div>
  </div>;
}

function SettingRow({ title, copy, value, onChange }: { title:string; copy:string; value:boolean; onChange:(next:boolean)=>void }) { return <div className="inline" style={{ justifyContent:"space-between", width:"100%", padding:"14px 0", borderBottom:"1px solid rgba(157,183,214,.08)" }}><div><div style={{fontSize:13,fontWeight:700}}>{title}</div><div className="card-copy">{copy}</div></div><Toggle value={value} onChange={onChange}/></div>; }

function TrayPreview({ running }: { running: boolean }) {
  return <Card><h2 className="card-title">Tray preview</h2><div className="card-copy">Compact controls using the same visual language as the main app.</div><div className="tray-preview section-gap"><div className="inline"><img className="brand-mark" style={{width:28,height:28,borderRadius:8}} src="/app-icon.png" alt=""/><div><strong>VxClick</strong><div className="card-copy">{running ? "Running" : "Ready"}</div></div></div><div className="divider"/><div className="tray-row"><span>Start / Stop</span><span className="kbd">F6</span></div><div className="tray-row"><span>Active profile</span><span>Default</span></div><div className="tray-row"><span>Open VxClick</span><MousePointer2 size={14}/></div><div className="divider"/><div className="tray-row"><span>Exit</span><X size={14}/></div></div></Card>;
}
