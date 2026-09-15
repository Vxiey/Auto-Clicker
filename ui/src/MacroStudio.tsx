import { useEffect, useMemo, useState, type ChangeEvent } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  ArrowDown,
  ArrowUp,
  CircleStop,
  Clock3,
  Code2,
  Copy,
  Gamepad2,
  GripVertical,
  Keyboard,
  ListRestart,
  MousePointerClick,
  Play,
  Plus,
  Radio,
  Repeat2,
  RotateCcw,
  Save,
  Search,
  Settings2,
  Trash2,
  Zap,
} from "lucide-react";
import { macroApi, type MacroEventRecord, type StoredLuaScript, type StoredMacro } from "./api";
import { Button, Card, Field, Toggle } from "./components";
import "./macro-studio.css";

type MacroType = "no-repeat" | "repeat-hold" | "toggle" | "sequence";
type Lane = "main" | "on-press" | "while-holding" | "on-release";
type MacroEventType = "key-down" | "key-up" | "mouse" | "delay";
type AssignmentCategory = "commands" | "keys" | "actions" | "macros" | "system";

type MacroEvent = {
  id: number;
  type: MacroEventType;
  label: string;
  delayMs?: number;
  lane: Lane;
};

type Assignment = {
  category: AssignmentCategory;
  label: string;
  type: MacroEventType;
  icon: typeof Zap;
  delayMs?: number;
};

const macroTypes: Array<{ id: MacroType; title: string; copy: string; icon: typeof Zap }> = [
  { id: "no-repeat", title: "No Repeat", copy: "Runs once when the trigger is pressed.", icon: Zap },
  { id: "repeat-hold", title: "Repeat While Holding", copy: "Loops while the trigger stays pressed.", icon: Repeat2 },
  { id: "toggle", title: "Toggle", copy: "Press once to start looping, press again to stop.", icon: RotateCcw },
  { id: "sequence", title: "Sequence", copy: "Separate actions for press, hold and release.", icon: ListRestart },
];

const assignments: Assignment[] = [
  { category: "commands", label: "Copy", type: "key-down", icon: Keyboard },
  { category: "commands", label: "Paste", type: "key-down", icon: Keyboard },
  { category: "commands", label: "Undo", type: "key-down", icon: Keyboard },
  { category: "commands", label: "Redo", type: "key-down", icon: Keyboard },
  { category: "keys", label: "A", type: "key-down", icon: Keyboard },
  { category: "keys", label: "B", type: "key-down", icon: Keyboard },
  { category: "keys", label: "F", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Space", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Enter", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Tab", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Esc", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Shift", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Ctrl", type: "key-down", icon: Keyboard },
  { category: "keys", label: "Alt", type: "key-down", icon: Keyboard },
  { category: "actions", label: "Left Click", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Right Click", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Middle Click", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Mouse 4", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Mouse 5", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "10 ms Delay", type: "delay", icon: Clock3, delayMs: 10 },
  { category: "actions", label: "25 ms Delay", type: "delay", icon: Clock3, delayMs: 25 },
  { category: "actions", label: "50 ms Delay", type: "delay", icon: Clock3, delayMs: 50 },
  { category: "actions", label: "100 ms Delay", type: "delay", icon: Clock3, delayMs: 100 },
  { category: "actions", label: "250 ms Delay", type: "delay", icon: Clock3, delayMs: 250 },
  { category: "actions", label: "500 ms Delay", type: "delay", icon: Clock3, delayMs: 500 },
  { category: "macros", label: "Quick Action", type: "key-down", icon: ListRestart },
  { category: "macros", label: "Rapid Click", type: "mouse", icon: ListRestart },
  { category: "system", label: "Play / Pause", type: "key-down", icon: Settings2 },
  { category: "system", label: "Volume Up", type: "key-down", icon: Settings2 },
  { category: "system", label: "Volume Down", type: "key-down", icon: Settings2 },
];

const initialEvents: MacroEvent[] = [
  { id: 1, type: "key-down", label: "F", lane: "main" },
  { id: 2, type: "delay", label: "30 ms", delayMs: 30, lane: "main" },
  { id: 3, type: "key-up", label: "F", lane: "main" },
  { id: 4, type: "delay", label: "40 ms", delayMs: 40, lane: "main" },
  { id: 5, type: "mouse", label: "Left Click", lane: "main" },
];

const sequenceEvents: MacroEvent[] = [
  { id: 1, type: "key-down", label: "Shift", lane: "on-press" },
  { id: 2, type: "key-down", label: "W", lane: "on-press" },
  { id: 3, type: "mouse", label: "Left Click", lane: "while-holding" },
  { id: 4, type: "delay", label: "75 ms", delayMs: 75, lane: "while-holding" },
  { id: 5, type: "key-up", label: "W", lane: "on-release" },
  { id: 6, type: "key-up", label: "Shift", lane: "on-release" },
];

const defaultLua = `-- VxClick Lua automation\nmouse_down("left")\nfor _ = 1, 5 do\n  move_mouse(0, 2)\n  sleep(10)\nend\nmouse_up("left")`;

let nextId = 100;

export function MacroStudio() {
  const [macroId, setMacroId] = useState("");
  const [storedMacros, setStoredMacros] = useState<StoredMacro[]>([]);
  const [name, setName] = useState("Quick Action");
  const [trigger, setTrigger] = useState("Mouse 4");
  const [macroType, setMacroType] = useState<MacroType>("no-repeat");
  const [events, setEvents] = useState<MacroEvent[]>(initialEvents);
  const [recording, setRecording] = useState(false);
  const [recordStatus, setRecordStatus] = useState("F1 starts global recording · F2 stops and saves");
  const [recordDelays, setRecordDelays] = useState(true);
  const [standardDelay, setStandardDelay] = useState(false);
  const [standardDelayMs, setStandardDelayMs] = useState(50);
  const [repeatDelayMs, setRepeatDelayMs] = useState(25);
  const [speed, setSpeed] = useState(1);
  const [assignmentCategory, setAssignmentCategory] = useState<AssignmentCategory>("keys");
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [luaScripts, setLuaScripts] = useState<StoredLuaScript[]>([]);
  const [luaId, setLuaId] = useState("");
  const [luaName, setLuaName] = useState("New Lua Script");
  const [luaScript, setLuaScript] = useState(defaultLua);
  const [luaStatus, setLuaStatus] = useState("");

  useEffect(() => {
    if (macroType === "sequence" && !events.some((event) => event.lane !== "main")) setEvents(sequenceEvents);
    if (macroType !== "sequence" && events.some((event) => event.lane !== "main")) setEvents(initialEvents);
  }, [macroType]);

  const refresh = async () => {
    try {
      const [snapshot, luaLibrary] = await Promise.all([macroApi.snapshot(), macroApi.luaScripts()]);
      setStoredMacros(snapshot.document.macros);
      setRecording(snapshot.recording);
      setLuaScripts(luaLibrary.scripts);
    } catch {
      // Browser preview remains usable without Tauri.
    }
  };

  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    void refresh();
    const timer = window.setInterval(() => void refresh(), 500);

    void (async () => {
      try {
        const stateUnlisten = await listen<boolean>("macro-recording-state", (event) => {
          if (disposed) return;
          const active = Boolean(event.payload);
          setRecording(active);
          setRecordStatus(active ? "Recording globally · press F2 to stop and save" : "Recording stopped");
        });
        if (disposed) stateUnlisten();
        else unlisteners.push(stateUnlisten);

        const recordedUnlisten = await listen<StoredMacro>("macro-recorded", (event) => {
          if (disposed) return;
          const saved = event.payload;
          setRecording(false);
          setMacroId(saved.id);
          setName(saved.name);
          setTrigger(saved.trigger);
          setMacroType(saved.macroType);
          setRepeatDelayMs(saved.repeatDelayMs);
          setSpeed(saved.speed);
          setEvents(saved.events.map(fromStoredEvent));
          setRecordStatus(`Recorded and saved · ${saved.events.filter((item) => item.type !== "delay").length} actions`);
          setError("");
          void refresh();
        });
        if (disposed) recordedUnlisten();
        else unlisteners.push(recordedUnlisten);
      } catch {
        // Browser preview has no Tauri event bridge.
      }
    })();

    return () => {
      disposed = true;
      window.clearInterval(timer);
      for (const unlisten of unlisteners) unlisten();
    };
  }, []);

  const loadMacro = (id: string) => {
    if (!id) {
      setMacroId("");
      return;
    }
    const selected = storedMacros.find((item) => item.id === id);
    if (!selected) return;
    setMacroId(selected.id);
    setName(selected.name);
    setTrigger(selected.trigger);
    setMacroType(selected.macroType);
    setRepeatDelayMs(selected.repeatDelayMs);
    setSpeed(selected.speed);
    setEvents(selected.events.map(fromStoredEvent));
  };

  const newMacro = () => {
    setMacroId("");
    setName("New Macro");
    setMacroType("no-repeat");
    setRepeatDelayMs(25);
    setSpeed(1);
    setEvents([]);
    setError("");
  };

  const duplicateCurrentMacro = () => {
    setMacroId("");
    setName(`${name.trim() || "Macro"} Copy`);
    setEvents((current) => current.map((event) => ({ ...event, id: nextId++ })));
    setError("");
  };

  const currentMacro = (): StoredMacro => ({
    id: macroId,
    name,
    trigger,
    macroType,
    repeatDelayMs,
    speed,
    events: events.map(toStoredEvent),
  });

  const save = async () => {
    setBusy(true);
    try {
      const saved = await macroApi.save(currentMacro());
      setMacroId(saved.id);
      setError("");
      await refresh();
      return saved;
    } catch (reason) {
      setError(String(reason));
      return null;
    } finally {
      setBusy(false);
    }
  };

  const play = async () => {
    const saved = await save();
    if (!saved) return;
    try {
      await macroApi.play(saved.id);
      setError("");
    } catch (reason) {
      setError(String(reason));
    }
  };

  const removeCurrent = async () => {
    if (!macroId) return;
    setBusy(true);
    try {
      await macroApi.remove(macroId);
      newMacro();
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const toggleRecording = async () => {
    setBusy(true);
    try {
      if (!recording) {
        const lane = macroType === "sequence" ? "on-press" : "main";
        await macroApi.startRecording(recordDelays, standardDelay ? standardDelayMs : null, lane);
        setRecording(true);
        setRecordStatus("Recording globally · use the button or F2 to stop");
      } else {
        const captured = await macroApi.stopRecording();
        setRecording(false);
        if (captured.length) {
          setEvents(captured.map(fromStoredEvent));
          setRecordStatus(`Recording stopped · ${captured.filter((item) => item.type !== "delay").length} actions captured`);
        } else {
          setRecordStatus("Recording stopped · no actions captured");
        }
      }
      setError("");
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const validateLua = async () => {
    try {
      const count = await macroApi.validateLua(luaScript);
      setLuaStatus(`Valid · ${count} native actions`);
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    }
  };

  const runLua = async () => {
    try {
      await macroApi.runLua(luaScript, speed);
      setLuaStatus("Running through the native precision macro player");
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    }
  };

  const loadLuaScript = (id: string) => {
    setLuaId(id);
    if (!id) {
      setLuaName("New Lua Script");
      setLuaScript(defaultLua);
      setLuaStatus("");
      return;
    }
    const selected = luaScripts.find((item) => item.id === id);
    if (!selected) return;
    setLuaName(selected.name);
    setLuaScript(selected.script);
    setLuaStatus(`Loaded · ${selected.name}`);
  };

  const saveLua = async () => {
    setBusy(true);
    try {
      const saved = await macroApi.saveLuaScript({ id: luaId, name: luaName, script: luaScript });
      setLuaId(saved.id);
      setLuaName(saved.name);
      setLuaStatus(`Saved · ${saved.name}`);
      await refresh();
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const renameLua = async () => {
    if (!luaId) {
      await saveLua();
      return;
    }
    setBusy(true);
    try {
      const renamed = await macroApi.renameLuaScript(luaId, luaName);
      setLuaName(renamed.name);
      setLuaStatus(`Renamed · ${renamed.name}`);
      await refresh();
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const duplicateLua = async () => {
    if (!luaId) return;
    setBusy(true);
    try {
      const copy = await macroApi.duplicateLuaScript(luaId);
      setLuaId(copy.id);
      setLuaName(copy.name);
      setLuaScript(copy.script);
      setLuaStatus(`Duplicated · ${copy.name}`);
      await refresh();
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const deleteLua = async () => {
    if (!luaId) return;
    setBusy(true);
    try {
      await macroApi.deleteLuaScript(luaId);
      setLuaId("");
      setLuaName("New Lua Script");
      setLuaScript(defaultLua);
      setLuaStatus("Deleted Lua script");
      await refresh();
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const uploadLua = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    if (!file.name.toLowerCase().endsWith(".lua")) {
      setLuaStatus("Error · select a .lua file");
      return;
    }
    if (file.size > 1024 * 1024) {
      setLuaStatus("Error · Lua file cannot exceed 1 MiB");
      return;
    }
    try {
      const script = await file.text();
      const proposedName = file.name.replace(/\.lua$/i, "").trim() || "Imported Lua Script";
      const saved = await macroApi.saveLuaScript({ id: "", name: proposedName, script });
      setLuaId(saved.id);
      setLuaName(saved.name);
      setLuaScript(saved.script);
      setLuaStatus(`Uploaded and saved · ${file.name}`);
      await refresh();
    } catch (reason) {
      setLuaStatus(`Error · ${String(reason)}`);
    }
  };

  const newLua = () => {
    setLuaId("");
    setLuaName("New Lua Script");
    setLuaScript(defaultLua);
    setLuaStatus("New unsaved script");
  };

  const totalDuration = useMemo(() => events.reduce((sum, event) => sum + (event.delayMs ?? 0), 0), [events]);

  const addEvent = (type: MacroEventType, lane: Lane = macroType === "sequence" ? "on-press" : "main", label?: string) => {
    const defaults: Record<MacroEventType, Omit<MacroEvent, "id" | "lane">> = {
      "key-down": { type: "key-down", label: label ?? "Key Down" },
      "key-up": { type: "key-up", label: label ?? "Key Up" },
      mouse: { type: "mouse", label: label ?? "Left Click" },
      delay: { type: "delay", label: label ?? "50 ms", delayMs: 50 },
    };
    setEvents((current) => [...current, { id: nextId++, lane, ...defaults[type] }]);
  };

  const addAssignment = (assignment: Assignment) => {
    const lane = macroType === "sequence" ? "on-press" : "main";
    if (assignment.type === "delay") {
      const delayMs = assignment.delayMs ?? 50;
      setEvents((current) => [...current, { id: nextId++, type: "delay", label: `${delayMs} ms`, delayMs, lane }]);
      return;
    }
    addEvent(assignment.type, lane, assignment.label);
    if (assignment.type === "key-down") {
      setEvents((current) => [...current, { id: nextId++, type: "key-up", label: assignment.label, lane }]);
    }
  };

  const updateDelay = (id: number, delayMs: number) => {
    const safe = Math.max(0, Math.min(60_000, delayMs || 0));
    setEvents((current) => current.map((event) => event.id === id ? { ...event, delayMs: safe, label: `${safe} ms` } : event));
  };

  const removeEvent = (id: number) => setEvents((current) => current.filter((event) => event.id !== id));

  const duplicateEvent = (id: number) => {
    setEvents((current) => {
      const index = current.findIndex((event) => event.id === id);
      if (index < 0) return current;
      const copy = { ...current[index], id: nextId++ };
      const next = [...current];
      next.splice(index + 1, 0, copy);
      return next;
    });
  };

  const moveEvent = (id: number, direction: -1 | 1) => {
    setEvents((current) => {
      const index = current.findIndex((event) => event.id === id);
      if (index < 0) return current;
      const lane = current[index].lane;
      let target = index + direction;
      while (target >= 0 && target < current.length && current[target].lane !== lane) target += direction;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  };

  return (
    <div className="page macro-page">
      <div className="page-header macro-page-header">
        <div>
          <h1 className="page-title">Macro Studio</h1>
          <div className="page-subtitle">Native global recording, persistent macros and QPC precision playback. F1 starts recording and F2 stops/saves it.</div>
        </div>
        <div className="macro-summary">
          <span>{events.filter((event) => event.type !== "delay").length} actions</span>
          <span>{totalDuration} ms delays</span>
          <span className={recording ? "macro-live" : ""}>{recording ? "Recording globally" : "Ready"}</span>
        </div>
      </div>

      {error && <Card><div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div></Card>}
      <Card className="section-gap">
        <div className="inline" style={{ justifyContent: "space-between", width: "100%", flexWrap: "wrap" }}>
          <div className="card-copy">{recordStatus}</div>
          <span className={`status-pill ${recording ? "status-recording" : "status-ready"}`}>{recording ? "F2 stops" : "F1 start · F2 stop"}</span>
        </div>
      </Card>

      <div className="macro-layout section-gap">
        <Card className="macro-type-panel">
          <div className="macro-panel-title">Macro type</div>
          <div className="macro-panel-copy">Choose how the assigned global trigger controls playback.</div>
          <div className="macro-type-list">
            {macroTypes.map(({ id, title, copy, icon: Icon }) => (
              <button key={id} className={`macro-type-card ${macroType === id ? "active" : ""}`} onClick={() => setMacroType(id)}>
                <div className="macro-type-icon"><Icon size={19} /></div>
                <div><strong>{title}</strong><span>{copy}</span></div>
              </button>
            ))}
          </div>
          {(macroType === "repeat-hold" || macroType === "toggle" || macroType === "sequence") && (
            <div className="macro-side-setting">
              <Field label="Delay between repeats">
                <div className="macro-number-wrap">
                  <input className="input" type="number" min="0" max="60000" value={repeatDelayMs} onChange={(event) => setRepeatDelayMs(Number(event.target.value))} />
                  <span>ms</span>
                </div>
              </Field>
            </div>
          )}
        </Card>

        <div className="macro-editor-stack">
          <Card className="macro-toolbar-card">
            <div className="macro-toolbar-grid">
              <Field label="Saved macro">
                <select className="select" value={macroId} onChange={(event) => loadMacro(event.target.value)}>
                  <option value="">New / unsaved</option>
                  {storedMacros.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
                </select>
              </Field>
              <Field label="Macro name"><input className="input" value={name} onChange={(event) => setName(event.target.value)} /></Field>
              <Field label="Trigger"><input className="input" value={trigger} onChange={(event) => setTrigger(event.target.value)} /></Field>
              <Field label="Playback speed"><input className="input" type="number" min="0.1" max="10" step="0.1" value={speed} onChange={(event) => setSpeed(Number(event.target.value))} /></Field>
            </div>
            <div className="macro-record-controls" style={{ marginTop: 12 }}>
              <Button onClick={newMacro}><Plus size={14} /> New</Button>
              <Button disabled={events.length === 0} onClick={duplicateCurrentMacro}><Copy size={14} /> Duplicate</Button>
              <Button variant={recording ? "danger" : "primary"} disabled={busy} onClick={() => void toggleRecording()}>
                {recording ? <CircleStop size={14} /> : <Radio size={14} />}{recording ? "Stop native recording" : "Start native recording"}
              </Button>
              <Button disabled={busy} onClick={() => void save()}><Save size={14} /> Save</Button>
              <Button variant="primary" disabled={busy || events.length === 0} onClick={() => void play()}><Play size={14} /> Play</Button>
              <Button onClick={() => void macroApi.stop()}><CircleStop size={14} /> Stop</Button>
              <Button onClick={() => setEvents([])}><Trash2 size={14} /> Clear</Button>
              <Button variant="danger" disabled={!macroId || busy} onClick={() => void removeCurrent()}><Trash2 size={14} /> Delete macro</Button>
            </div>
            <div className="macro-record-options">
              <div className="macro-option-row"><div><strong>Record delays</strong><span>Use QPC timestamps captured by the global Windows hooks.</span></div><Toggle value={recordDelays} onChange={setRecordDelays} /></div>
              <div className="macro-option-row"><div><strong>Use standard delay</strong><span>Replace captured timing with one fixed delay.</span></div><Toggle value={standardDelay} onChange={setStandardDelay} /></div>
              <div className="macro-standard-delay"><input className="input" type="number" min="0" max="60000" disabled={!standardDelay} value={standardDelayMs} onChange={(event) => setStandardDelayMs(Number(event.target.value))} /><span>ms</span></div>
            </div>
          </Card>

          {macroType === "sequence" ? (
            <div className="macro-sequence-grid">
              <MacroLane title="On press" lane="on-press" events={events} addEvent={addEvent} removeEvent={removeEvent} updateDelay={updateDelay} duplicateEvent={duplicateEvent} moveEvent={moveEvent} />
              <MacroLane title="While holding" lane="while-holding" events={events} addEvent={addEvent} removeEvent={removeEvent} updateDelay={updateDelay} duplicateEvent={duplicateEvent} moveEvent={moveEvent} />
              <MacroLane title="On release" lane="on-release" events={events} addEvent={addEvent} removeEvent={removeEvent} updateDelay={updateDelay} duplicateEvent={duplicateEvent} moveEvent={moveEvent} />
            </div>
          ) : (
            <div className="macro-workspace-grid">
              <AssignmentLibrary category={assignmentCategory} setCategory={setAssignmentCategory} search={search} setSearch={setSearch} onAdd={addAssignment} />
              <Card className="macro-timeline-card">
                <div className="macro-timeline-header">
                  <div><div className="macro-panel-title">Action timeline</div><div className="macro-panel-copy">Add from Assignments, record global input or edit every event manually. Use the row controls to reorder or duplicate actions.</div></div>
                  <div className="macro-add-actions">
                    <button onClick={() => addEvent("key-down")}><Keyboard size={14} /> Key</button>
                    <button onClick={() => addEvent("mouse")}><MousePointerClick size={14} /> Mouse</button>
                    <button onClick={() => addEvent("delay")}><Clock3 size={14} /> Delay</button>
                  </div>
                </div>
                <Timeline events={events.filter((event) => event.lane === "main")} removeEvent={removeEvent} updateDelay={updateDelay} duplicateEvent={duplicateEvent} moveEvent={moveEvent} />
              </Card>
            </div>
          )}
        </div>
      </div>

      <Card className="section-gap">
        <div className="inline" style={{ justifyContent: "space-between", width: "100%", alignItems: "flex-start", flexWrap: "wrap" }}>
          <div><div className="macro-panel-title"><Code2 size={15} style={{ display: "inline", marginRight: 7 }} />Lua script library</div><div className="macro-panel-copy">Create, upload, rename and save sandboxed Lua scripts. Scripts compile into the same native InputAction precision player.</div></div>
          <div className="quick-actions" style={{ marginTop: 0 }}>
            <Button onClick={newLua}><Plus size={14} /> New</Button>
            <Button onClick={() => document.getElementById("lua-file-upload")?.click()}>Upload .lua</Button>
            <Button disabled={busy} onClick={() => void saveLua()}><Save size={14} /> Save</Button>
            <Button disabled={busy || !luaId} onClick={() => void renameLua()}>Rename</Button>
            <Button disabled={busy || !luaId} onClick={() => void duplicateLua()}>Duplicate</Button>
            <Button variant="danger" disabled={busy || !luaId} onClick={() => void deleteLua()}><Trash2 size={14} /> Delete</Button>
          </div>
        </div>
        <input id="lua-file-upload" type="file" accept=".lua,text/plain" style={{ display: "none" }} onChange={(event) => void uploadLua(event)} />
        <div className="form-row section-gap">
          <Field label="Saved Lua script">
            <select className="select" value={luaId} onChange={(event) => loadLuaScript(event.target.value)}>
              <option value="">New / unsaved</option>
              {luaScripts.map((script) => <option key={script.id} value={script.id}>{script.name}</option>)}
            </select>
          </Field>
          <Field label="Script name"><input className="input" value={luaName} maxLength={96} onChange={(event) => setLuaName(event.target.value)} /></Field>
        </div>
        <textarea className="input section-gap" style={{ width: "100%", minHeight: 220, resize: "vertical", fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace" }} value={luaScript} onChange={(event) => setLuaScript(event.target.value)} spellCheck={false} />
        <div className="inline" style={{ justifyContent: "space-between", width: "100%", flexWrap: "wrap" }}>
          <div className="card-copy">Stored locally in VxClick · max 256 scripts · max 1 MiB per script.</div>
          <div className="quick-actions" style={{ marginTop: 0 }}><Button onClick={() => void validateLua()}>Validate</Button><Button variant="primary" onClick={() => void runLua()}><Play size={14} /> Run Lua</Button><Button onClick={() => void macroApi.stop()}><CircleStop size={14} /> Stop</Button></div>
        </div>
        {luaStatus && <div className="card-copy section-gap">{luaStatus}</div>}
      </Card>
    </div>
  );
}

function fromStoredEvent(event: MacroEventRecord): MacroEvent {
  return {
    id: event.id,
    type: event.type,
    label: event.label,
    delayMs: event.delay_ms ?? undefined,
    lane: event.lane,
  };
}

function toStoredEvent(event: MacroEvent): MacroEventRecord {
  return {
    id: event.id,
    type: event.type,
    label: event.label,
    delay_ms: event.delayMs ?? null,
    lane: event.lane,
  };
}

function AssignmentLibrary({ category, setCategory, search, setSearch, onAdd }: { category: AssignmentCategory; setCategory: (category: AssignmentCategory) => void; search: string; setSearch: (value: string) => void; onAdd: (assignment: Assignment) => void }) {
  const filtered = assignments.filter((item) => item.category === category && item.label.toLowerCase().includes(search.toLowerCase()));
  return (
    <Card className="assignment-library">
      <div className="assignment-library-head">
        <div className="macro-panel-title">Assignments</div>
        <div className="macro-panel-copy">Click an item to add it to the current macro.</div>
        <div className="assignment-tabs">
          {(["commands", "keys", "actions", "macros", "system"] as AssignmentCategory[]).map((item) => (
            <button key={item} className={`assignment-tab ${category === item ? "active" : ""}`} onClick={() => setCategory(item)}>{item}</button>
          ))}
        </div>
      </div>
      <div className="assignment-search">
        <div className="assignment-search-box"><Search size={13} /><input className="input" placeholder="Search assignments" value={search} onChange={(event) => setSearch(event.target.value)} /></div>
      </div>
      <div className="assignment-list">
        <div className="assignment-group-label">{category}</div>
        {filtered.map((assignment) => {
          const Icon = assignment.icon;
          return <button key={`${assignment.category}-${assignment.label}`} className="assignment-item" onClick={() => onAdd(assignment)}><span className="assignment-item-icon"><Icon size={13} /></span><span>{assignment.label}</span></button>;
        })}
        {filtered.length === 0 && <div className="macro-empty" style={{ minHeight: 160 }}><Search size={18} /><strong>No matches</strong><span>Try another search.</span></div>}
      </div>
    </Card>
  );
}

function MacroLane({ title, lane, events, addEvent, removeEvent, updateDelay, duplicateEvent, moveEvent }: { title: string; lane: Lane; events: MacroEvent[]; addEvent: (type: MacroEventType, lane?: Lane, label?: string) => void; removeEvent: (id: number) => void; updateDelay: (id: number, delayMs: number) => void; duplicateEvent: (id: number) => void; moveEvent: (id: number, direction: -1 | 1) => void }) {
  const laneEvents = events.filter((event) => event.lane === lane);
  return <Card className="macro-lane-card">
    <div className="macro-lane-header"><div><div className="macro-panel-title">{title}</div><div className="macro-panel-copy">{laneEvents.length} timeline items</div></div><button className="macro-round-add" onClick={() => addEvent("key-down", lane)}><Plus size={14} /></button></div>
    <Timeline events={laneEvents} removeEvent={removeEvent} updateDelay={updateDelay} duplicateEvent={duplicateEvent} moveEvent={moveEvent} compact />
    <div className="macro-lane-actions"><button onClick={() => addEvent("key-down", lane)}><Keyboard size={13} /> Key</button><button onClick={() => addEvent("mouse", lane)}><MousePointerClick size={13} /> Mouse</button><button onClick={() => addEvent("delay", lane)}><Clock3 size={13} /> Delay</button></div>
  </Card>;
}

function Timeline({ events, removeEvent, updateDelay, duplicateEvent, moveEvent, compact = false }: { events: MacroEvent[]; removeEvent: (id: number) => void; updateDelay: (id: number, delayMs: number) => void; duplicateEvent: (id: number) => void; moveEvent: (id: number, direction: -1 | 1) => void; compact?: boolean }) {
  if (events.length === 0) return <div className="macro-empty"><Gamepad2 size={22} /><strong>No actions yet</strong><span>Record input or add an assignment.</span></div>;
  return <div className={`macro-timeline ${compact ? "compact" : ""}`}>
    {events.map((event, index) => <div key={event.id} className={`macro-event macro-event-${event.type}`} style={{ gridTemplateColumns: compact ? "28px 25px minmax(0,1fr) 112px" : "32px 18px 30px minmax(0,1fr) 112px" }}>
      <div className="macro-event-index">{index + 1}</div><GripVertical size={14} className="macro-drag" /><div className="macro-event-icon">{eventIcon(event.type)}</div>
      <div className="macro-event-main"><strong>{eventLabel(event.type)}</strong>{event.type === "delay" ? <div className="macro-inline-delay"><input type="number" min="0" max="60000" value={event.delayMs ?? 0} onChange={(input) => updateDelay(event.id, Number(input.target.value))} /><span>ms</span></div> : <span>{event.label}</span>}</div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 1 }}>
        <button className="macro-event-delete" style={{ color: "var(--muted)" }} title="Move up" disabled={index === 0} onClick={() => moveEvent(event.id, -1)}><ArrowUp size={13} /></button>
        <button className="macro-event-delete" style={{ color: "var(--muted)" }} title="Move down" disabled={index === events.length - 1} onClick={() => moveEvent(event.id, 1)}><ArrowDown size={13} /></button>
        <button className="macro-event-delete" style={{ color: "var(--muted)" }} title="Duplicate action" onClick={() => duplicateEvent(event.id)}><Copy size={13} /></button>
        <button className="macro-event-delete" title="Delete action" onClick={() => removeEvent(event.id)}><Trash2 size={13} /></button>
      </div>
    </div>)}
  </div>;
}

function eventIcon(type: MacroEventType) {
  if (type === "mouse") return <MousePointerClick size={14} />;
  if (type === "delay") return <Clock3 size={14} />;
  return <Keyboard size={14} />;
}

function eventLabel(type: MacroEventType) {
  if (type === "key-down") return "Key down";
  if (type === "key-up") return "Key up";
  if (type === "mouse") return "Mouse";
  return "Delay";
}
