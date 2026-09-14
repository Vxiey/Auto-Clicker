import { useEffect, useState } from "react";
import { Plus, RefreshCw, Save, Trash2 } from "lucide-react";
import { remapApi, type StoredRemap } from "./api";
import { Button, Card, Field, Toggle } from "./components";

const emptyMapping = (): StoredRemap => ({
  id: "",
  name: "New Mapping",
  trigger: "Mouse 5",
  action: "F",
  process: "",
  enabled: true,
  consume: true,
});

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

  const save = async () => {
    setBusy(true);
    try {
      const saved = await remapApi.save(draft);
      setDraft(saved);
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
          <h1 className="page-title">Key Remap</h1>
          <div className="page-subtitle">Native global key/mouse remapping with chords, per-process scope and macro targets.</div>
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
          <div className="card-copy">Examples: Mouse 5 → F, Caps Lock → Ctrl+C, F7 → Left Click, or F8 → Macro:macro-id.</div>
          <div className="form-row section-gap">
            <Field label="Name"><input className="input" value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></Field>
            <Field label="Trigger"><input className="input" value={draft.trigger} onChange={(event) => setDraft({ ...draft, trigger: event.target.value })} /></Field>
          </div>
          <div className="form-row section-gap">
            <Field label="Action"><input className="input" value={draft.action} onChange={(event) => setDraft({ ...draft, action: event.target.value })} /></Field>
            <Field label="Process (blank = global)"><input className="input" placeholder="RainbowSix.exe" value={draft.process} onChange={(event) => setDraft({ ...draft, process: event.target.value })} /></Field>
          </div>
          <div className="divider" />
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", padding: "8px 0" }}>
            <div><strong style={{ fontSize: 12 }}>Enabled</strong><div className="card-copy">Register this mapping in the native low-level hook runtime.</div></div>
            <Toggle value={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })} />
          </div>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", padding: "8px 0" }}>
            <div><strong style={{ fontSize: 12 }}>Consume original input</strong><div className="card-copy">Prevent the trigger from also reaching the focused application.</div></div>
            <Toggle value={draft.consume} onChange={(consume) => setDraft({ ...draft, consume })} />
          </div>
          <div className="quick-actions"><Button variant="primary" disabled={busy} onClick={() => void save()}><Save size={14} /> Save Mapping</Button></div>
        </Card>

        <Card>
          <h2 className="card-title">Active mappings</h2>
          <div className="card-copy">Process-scoped mappings are registered only while that process is foreground, so consume/pass-through stays correct.</div>
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
