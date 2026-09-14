import { useEffect, useMemo, useRef, useState } from "react";
import {
  CircleStop,
  Clock3,
  Gamepad2,
  GripVertical,
  Keyboard,
  ListRestart,
  MousePointerClick,
  Plus,
  Radio,
  Repeat2,
  RotateCcw,
  Search,
  Settings2,
  Trash2,
  Zap,
} from "lucide-react";
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
  { category: "actions", label: "Left Click", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Right Click", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Middle Click", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Mouse 4", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "Mouse 5", type: "mouse", icon: MousePointerClick },
  { category: "actions", label: "50 ms Delay", type: "delay", icon: Clock3 },
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

let nextId = 100;

export function MacroStudio() {
  const [name, setName] = useState("Quick Action");
  const [trigger, setTrigger] = useState("Mouse 4");
  const [macroType, setMacroType] = useState<MacroType>("no-repeat");
  const [events, setEvents] = useState<MacroEvent[]>(initialEvents);
  const [recording, setRecording] = useState(false);
  const [recordDelays, setRecordDelays] = useState(true);
  const [standardDelay, setStandardDelay] = useState(false);
  const [standardDelayMs, setStandardDelayMs] = useState(50);
  const [repeatDelayMs, setRepeatDelayMs] = useState(25);
  const [assignmentCategory, setAssignmentCategory] = useState<AssignmentCategory>("keys");
  const [search, setSearch] = useState("");
  const lastEventAt = useRef(performance.now());

  useEffect(() => {
    if (macroType === "sequence" && !events.some((event) => event.lane !== "main")) setEvents(sequenceEvents);
    if (macroType !== "sequence" && events.some((event) => event.lane !== "main")) setEvents(initialEvents);
  }, [macroType]);

  useEffect(() => {
    if (!recording) return;
    lastEventAt.current = performance.now();

    const recordKey = (event: globalThis.KeyboardEvent, type: "key-down" | "key-up") => {
      if (event.repeat) return;
      event.preventDefault();
      const now = performance.now();
      const elapsed = Math.max(0, Math.round(now - lastEventAt.current));
      const lane: Lane = macroType === "sequence" ? "on-press" : "main";
      setEvents((current) => {
        const next = [...current];
        if (recordDelays && current.length > 0) {
          const delay = standardDelay ? standardDelayMs : elapsed;
          if (delay > 0) next.push({ id: nextId++, type: "delay", label: `${delay} ms`, delayMs: delay, lane });
        }
        next.push({ id: nextId++, type, label: prettyKey(event.key), lane });
        return next;
      });
      lastEventAt.current = now;
    };

    const down = (event: globalThis.KeyboardEvent) => recordKey(event, "key-down");
    const up = (event: globalThis.KeyboardEvent) => recordKey(event, "key-up");
    window.addEventListener("keydown", down, true);
    window.addEventListener("keyup", up, true);
    return () => {
      window.removeEventListener("keydown", down, true);
      window.removeEventListener("keyup", up, true);
    };
  }, [macroType, recordDelays, recording, standardDelay, standardDelayMs]);

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
    if (assignment.type === "delay") {
      setEvents((current) => [...current, { id: nextId++, type: "delay", label: "50 ms", delayMs: 50, lane: macroType === "sequence" ? "on-press" : "main" }]);
      return;
    }
    addEvent(assignment.type, macroType === "sequence" ? "on-press" : "main", assignment.label);
    if (assignment.type === "key-down") {
      setEvents((current) => [...current, { id: nextId++, type: "key-up", label: assignment.label, lane: macroType === "sequence" ? "on-press" : "main" }]);
    }
  };

  const updateDelay = (id: number, delayMs: number) => {
    const safe = Math.max(0, Math.min(60_000, delayMs || 0));
    setEvents((current) => current.map((event) => event.id === id ? { ...event, delayMs: safe, label: `${safe} ms` } : event));
  };

  const removeEvent = (id: number) => setEvents((current) => current.filter((event) => event.id !== id));

  return (
    <div className="page macro-page">
      <div className="page-header macro-page-header">
        <div>
          <h1 className="page-title">Macro Studio</h1>
          <div className="page-subtitle">G HUB-inspired assignment flow with VxClick precision timing and editable event playback.</div>
        </div>
        <div className="macro-summary">
          <span>{events.filter((event) => event.type !== "delay").length} actions</span>
          <span>{totalDuration} ms delays</span>
          <span className={recording ? "macro-live" : ""}>{recording ? "Recording" : "Ready"}</span>
        </div>
      </div>

      <div className="macro-layout">
        <Card className="macro-type-panel">
          <div className="macro-panel-title">Macro type</div>
          <div className="macro-panel-copy">Choose how the assigned trigger controls playback.</div>
          <div className="macro-type-list">
            {macroTypes.map(({ id, title, copy, icon: Icon }) => (
              <button key={id} className={`macro-type-card ${macroType === id ? "active" : ""}`} onClick={() => setMacroType(id)}>
                <div className="macro-type-icon"><Icon size={19} /></div>
                <div><strong>{title}</strong><span>{copy}</span></div>
              </button>
            ))}
          </div>
          {(macroType === "repeat-hold" || macroType === "toggle") && (
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
              <Field label="Macro name"><input className="input" value={name} onChange={(event) => setName(event.target.value)} /></Field>
              <Field label="Trigger"><input className="input" value={trigger} onChange={(event) => setTrigger(event.target.value)} /></Field>
              <div className="macro-record-controls">
                <Button variant={recording ? "danger" : "primary"} onClick={() => setRecording((value) => !value)}>
                  {recording ? <CircleStop size={14} /> : <Radio size={14} />}{recording ? "Stop recording" : "Start recording"}
                </Button>
                <Button onClick={() => setEvents([])}><Trash2 size={14} /> Clear</Button>
              </div>
            </div>
            <div className="macro-record-options">
              <div className="macro-option-row"><div><strong>Record delays</strong><span>Keep timing between recorded inputs.</span></div><Toggle value={recordDelays} onChange={setRecordDelays} /></div>
              <div className="macro-option-row"><div><strong>Use standard delay</strong><span>Replace recorded timing with one fixed delay.</span></div><Toggle value={standardDelay} onChange={setStandardDelay} /></div>
              <div className="macro-standard-delay"><input className="input" type="number" min="0" max="60000" disabled={!standardDelay} value={standardDelayMs} onChange={(event) => setStandardDelayMs(Number(event.target.value))} /><span>ms</span></div>
            </div>
          </Card>

          {macroType === "sequence" ? (
            <div className="macro-sequence-grid">
              <MacroLane title="On press" lane="on-press" events={events} addEvent={addEvent} removeEvent={removeEvent} updateDelay={updateDelay} />
              <MacroLane title="While holding" lane="while-holding" events={events} addEvent={addEvent} removeEvent={removeEvent} updateDelay={updateDelay} />
              <MacroLane title="On release" lane="on-release" events={events} addEvent={addEvent} removeEvent={removeEvent} updateDelay={updateDelay} />
            </div>
          ) : (
            <div className="macro-workspace-grid">
              <AssignmentLibrary category={assignmentCategory} setCategory={setAssignmentCategory} search={search} setSearch={setSearch} onAdd={addAssignment} />
              <Card className="macro-timeline-card">
                <div className="macro-timeline-header">
                  <div><div className="macro-panel-title">Action timeline</div><div className="macro-panel-copy">Add from Assignments, record live input or edit every event manually.</div></div>
                  <div className="macro-add-actions">
                    <button onClick={() => addEvent("key-down")}><Keyboard size={14} /> Key</button>
                    <button onClick={() => addEvent("mouse")}><MousePointerClick size={14} /> Mouse</button>
                    <button onClick={() => addEvent("delay")}><Clock3 size={14} /> Delay</button>
                  </div>
                </div>
                <Timeline events={events.filter((event) => event.lane === "main")} removeEvent={removeEvent} updateDelay={updateDelay} />
              </Card>
            </div>
          )}
        </div>
      </div>
    </div>
  );
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

function MacroLane({ title, lane, events, addEvent, removeEvent, updateDelay }: { title: string; lane: Lane; events: MacroEvent[]; addEvent: (type: MacroEventType, lane?: Lane, label?: string) => void; removeEvent: (id: number) => void; updateDelay: (id: number, delayMs: number) => void }) {
  const laneEvents = events.filter((event) => event.lane === lane);
  return <Card className="macro-lane-card">
    <div className="macro-lane-header"><div><div className="macro-panel-title">{title}</div><div className="macro-panel-copy">{laneEvents.length} timeline items</div></div><button className="macro-round-add" onClick={() => addEvent("key-down", lane)}><Plus size={14} /></button></div>
    <Timeline events={laneEvents} removeEvent={removeEvent} updateDelay={updateDelay} compact />
    <div className="macro-lane-actions"><button onClick={() => addEvent("key-down", lane)}><Keyboard size={13} /> Key</button><button onClick={() => addEvent("mouse", lane)}><MousePointerClick size={13} /> Mouse</button><button onClick={() => addEvent("delay", lane)}><Clock3 size={13} /> Delay</button></div>
  </Card>;
}

function Timeline({ events, removeEvent, updateDelay, compact = false }: { events: MacroEvent[]; removeEvent: (id: number) => void; updateDelay: (id: number, delayMs: number) => void; compact?: boolean }) {
  if (events.length === 0) return <div className="macro-empty"><Gamepad2 size={22} /><strong>No actions yet</strong><span>Record input or add an assignment.</span></div>;
  return <div className={`macro-timeline ${compact ? "compact" : ""}`}>
    {events.map((event, index) => <div key={event.id} className={`macro-event macro-event-${event.type}`}>
      <div className="macro-event-index">{index + 1}</div><GripVertical size={14} className="macro-drag" /><div className="macro-event-icon">{eventIcon(event.type)}</div>
      <div className="macro-event-main"><strong>{eventLabel(event.type)}</strong>{event.type === "delay" ? <div className="macro-inline-delay"><input type="number" min="0" max="60000" value={event.delayMs ?? 0} onChange={(input) => updateDelay(event.id, Number(input.target.value))} /><span>ms</span></div> : <span>{event.label}</span>}</div>
      <button className="macro-event-delete" onClick={() => removeEvent(event.id)}><Trash2 size={13} /></button>
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

function prettyKey(key: string) {
  if (key === " ") return "Space";
  if (key.length === 1) return key.toUpperCase();
  return key.replace(/^Arrow/, "Arrow ");
}
