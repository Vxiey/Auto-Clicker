import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Keyboard, MousePointerClick, Plus, RefreshCw, Save, Trash2 } from "lucide-react";
import { remapApi, type StoredRemap } from "./api";
import { Button, Card, Field, Toggle } from "./components";

const triggerPresets = [
  "Mouse1", "Mouse2", "Mouse3", "Mouse4", "Mouse5",
  "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
  "A", "B", "C", "D", "E", "F", "G", "Q", "R", "T", "V", "X", "Z",
  "Space", "Enter", "Tab", "Esc", "CapsLock", "Ctrl", "Shift", "Alt",
  "Ctrl+Shift+F6", "Ctrl+Alt+F7",
] as const;

const actionPresets = [
  "Left Click", "Right Click", "Middle Click", "Mouse 4", "Mouse 5",
  "A", "B", "C", "D", "E", "F", "G", "Q", "R", "T", "V", "X", "Z",
  "Space", "Enter", "Tab", "Esc", "Caps Lock", "Ctrl", "Shift", "Alt",
  "Ctrl+C", "Ctrl+V", "Ctrl+Z",
] as const;

const emptyMapping = (): StoredRemap => ({
  id: "",
  name: "New Mapping",
  trigger: "Mouse5",
  action: "F",
  process: "",
  enabled: true,
  consume: true,
});

function canonical(value: string) {
  return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

export function RemapPanel() {
  const [mappings, setMappings] = useState<StoredRemap[]>([]);
  const [draft, setDraft] = useState<StoredRemap>(emptyMapping());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = async () => {
    try {
      const snapshot = await remapApi.snapshot();
      setMappings(snapshot.mappings);
      setError("");
    } catch {
      // Browser preview remains usable without Tauri.
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const duplicateTrigger = useMemo(() => {
    const current = canonical(draft.trigger);
    if (!current) return null;
    return mappings.find((mapping) => mapping.id !== draft.id && mapping.enabled && canonical(mapping.trigger) === current) ?? null;
  }, [draft.id, draft.trigger, mappings]);

  const reservedRecorderKey = ["f1", "f2"].includes(canonical(draft.trigger));
  const selfMapping = canonical(draft.trigger) === canonical(draft.action);

  const save = async () => {
    if (duplicateTrigger) {
      setError(`Trigger conflict: ${draft.trigger} is already used by ${duplicateTrigger.name}.`);
      return;
    }
    if (reservedRecorderKey) {
      setError("F1 and F2 are reserved for Start/Stop Macro Recording in VxClick 1.0.1.");
      return;
    }
    setBusy(true);
    try {
      const saved = await remapApi.save(draft);
      setDraft(saved);
      setError("");
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    setBusy(true);
    try {
      await remapApi.remove(id);
      if (draft.id === id) setDraft(emptyMapping());
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <h1 className="page-title">Key & Mouse Remap</h1>
          <div className="page-subtitle">Native global keyboard and Mouse1–Mouse5 remapping with chords, per-process scope and macro targets.</div>
        </div>
        <div className="quick-actions" style={{ marginTop: 0 }}>
          <Button onClick={() => void refresh()}><RefreshCw size={14} /> Refresh</Button>
          <Button variant="primary" onClick={() => setDraft(emptyMapping())}><Plus size={14} /> Add Mapping</Button>
        </div>
      </div>

      {error && <Card><div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div></Card>}

      <div className="grid grid-2">
        <Card>
          <h2 className="card-title">Mapping editor</h2>
          <div className="card-copy">Map keyboard keys, chords and mouse buttons in either direction. VxClick-tagged injected input is ignored by the hook runtime to prevent recursive self-triggering.</div>

          <div className="form-row section-gap">
            <Field label="Name"><input className="input" value={draft.name} maxLength={96} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></Field>
            <Field label="Process (blank = global)"><input className="input" placeholder="game.exe" value={draft.process} onChange={(event) => setDraft({ ...draft, process: event.target.value })} /></Field>
          </div>

          <div className="form-row section-gap">
            <Field label="Trigger">
              <div>
                <input className="input" list="vx-remap-triggers" value={draft.trigger} onChange={(event) => setDraft({ ...draft, trigger: event.target.value })} />
                <datalist id="vx-remap-triggers">{triggerPresets.map((value) => <option key={value} value={value} />)}</datalist>
              </div>
            </Field>
            <Field label="Action">
              <div>
                <input className="input" list="vx-remap-actions" value={draft.action} onChange={(event) => setDraft({ ...draft, action: event.target.value })} />
                <datalist id="vx-remap-actions">{actionPresets.map((value) => <option key={value} value={value} />)}</datalist>
              </div>
            </Field>
          </div>

          <div className="section-gap">
            <div className="card-copy" style={{ marginBottom: 7 }}>Quick mouse triggers</div>
            <div className="quick-actions" style={{ marginTop: 0 }}>
              {["Mouse1", "Mouse2", "Mouse3", "Mouse4", "Mouse5"].map((value) => <Button key={value} onClick={() => setDraft({ ...draft, trigger: value })}><MousePointerClick size={13} /> {value}</Button>)}
            </div>
          </div>

          <div className="section-gap">
            <div className="card-copy" style={{ marginBottom: 7 }}>Quick mouse actions</div>
            <div className="quick-actions" style={{ marginTop: 0 }}>
              {["Left Click", "Right Click", "Middle Click", "Mouse 4", "Mouse 5"].map((value) => <Button key={value} onClick={() => setDraft({ ...draft, action: value })}><MousePointerClick size={13} /> {value}</Button>)}
            </div>
          </div>

          <div className="section-gap">
            <div className="card-copy" style={{ marginBottom: 7 }}>Quick keyboard actions</div>
            <div className="quick-actions" style={{ marginTop: 0 }}>
              {["Space", "Enter", "Ctrl", "Shift", "F", "R"].map((value) => <Button key={value} onClick={() => setDraft({ ...draft, action: value })}><Keyboard size={13} /> {value}</Button>)}
            </div>
          </div>

          {(duplicateTrigger || reservedRecorderKey || selfMapping) && (
            <div className="section-gap card-copy" style={{ color: reservedRecorderKey || duplicateTrigger ? "var(--warning)" : "var(--muted)" }}>
              <AlertTriangle size={13} style={{ display: "inline", marginRight: 6 }} />
              {duplicateTrigger
                ? `${draft.trigger} already belongs to “${duplicateTrigger.name}”.`
                : reservedRecorderKey
                  ? "F1/F2 are reserved for global macro recording."
                  : "Trigger and action are the same. This is safe from recursion, but normally has no useful effect."}
            </div>
          )}

          <div className="divider" />
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", padding: "8px 0" }}>
            <div><strong style={{ fontSize: 12 }}>Enabled</strong><div className="card-copy">Register this mapping in the native low-level hook runtime.</div></div>
            <Toggle value={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })} />
          </div>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", padding: "8px 0" }}>
            <div><strong style={{ fontSize: 12 }}>Consume original input</strong><div className="card-copy">Prevent the physical trigger from also reaching the focused application.</div></div>
            <Toggle value={draft.consume} onChange={(consume) => setDraft({ ...draft, consume })} />
          </div>
          <div className="quick-actions"><Button variant="primary" disabled={busy || Boolean(duplicateTrigger) || reservedRecorderKey} onClick={() => void save()}><Save size={14} /> Save Mapping</Button></div>
        </Card>

        <Card>
          <h2 className="card-title">Active mappings</h2>
          <div className="card-copy">Process-scoped mappings are active only while that process is foreground. Mouse1–Mouse5 can be used as triggers; Left/Right/Middle/Mouse4/Mouse5 can be emitted as actions.</div>
          <div className="divider" />
          {mappings.length === 0 ? <div className="card-copy">No mappings saved yet.</div> : (
            <table className="table">
              <thead><tr><th>From</th><th>To</th><th>Context</th><th></th></tr></thead>
              <tbody>
                {mappings.map((mapping) => (
                  <tr key={mapping.id}>
                    <td><button className="kbd" onClick={() => setDraft(mapping)}>{mapping.trigger}</button></td>
                    <td><span className="kbd">{mapping.action}</span></td>
                    <td>{mapping.process || "Global"}</td>
                    <td><div className="inline"><span className={`status-pill ${mapping.enabled ? "status-ready" : "status-stopped"}`}>{mapping.enabled ? "Enabled" : "Disabled"}</span><button className="icon-button danger" onClick={() => void remove(mapping.id)} title="Delete"><Trash2 size={13} /></button></div></td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Card>
      </div>
    </div>
  );
}
