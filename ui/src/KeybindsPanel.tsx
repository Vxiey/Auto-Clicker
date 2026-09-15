import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, KeyRound, Radio, RefreshCw, Save } from "lucide-react";
import {
  macroApi,
  profilesApi,
  remapApi,
  type Profile,
  type StoredMacro,
  type StoredRemap,
} from "./api";
import { Button, Card, Field } from "./components";
import { HotkeyCaptureInput } from "./HotkeyCaptureInput";

type BindingEntry = {
  id: string;
  label: string;
  value: string;
};

const RESERVED_RECORDER_KEYS = ["F1", "F2"] as const;

function canonical(value: string) {
  return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

function isUnassigned(value: string) {
  const normalized = value.trim().toLowerCase();
  return !normalized || normalized === "(unassigned)";
}

export function KeybindsPanel() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [profileId, setProfileId] = useState("");
  const [startHotkey, setStartHotkey] = useState("F6");
  const [stopHotkey, setStopHotkey] = useState("F8");
  const [macros, setMacros] = useState<StoredMacro[]>([]);
  const [macroDrafts, setMacroDrafts] = useState<Record<string, string>>({});
  const [remaps, setRemaps] = useState<StoredRemap[]>([]);
  const [remapDrafts, setRemapDrafts] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [status, setStatus] = useState("");

  const refresh = async () => {
    try {
      const [profileSnapshot, macroSnapshot, remapSnapshot] = await Promise.all([
        profilesApi.snapshot(),
        macroApi.snapshot(),
        remapApi.snapshot(),
      ]);
      setProfiles(profileSnapshot.document.profiles);
      const nextProfileId = profileSnapshot.document.profiles.some((profile) => profile.id === profileId)
        ? profileId
        : profileSnapshot.document.active_profile_id;
      setProfileId(nextProfileId);
      const selected = profileSnapshot.document.profiles.find((profile) => profile.id === nextProfileId);
      if (selected) {
        setStartHotkey(selected.clicker.start_hotkey);
        setStopHotkey(selected.clicker.emergency_stop_hotkey);
      }
      setMacros(macroSnapshot.document.macros);
      setMacroDrafts(Object.fromEntries(macroSnapshot.document.macros.map((macro) => [macro.id, macro.trigger])));
      setRemaps(remapSnapshot.mappings);
      setRemapDrafts(Object.fromEntries(remapSnapshot.mappings.map((mapping) => [mapping.id, mapping.trigger])));
      setError("");
    } catch (reason) {
      setError(String(reason));
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const selectedProfile = profiles.find((profile) => profile.id === profileId) ?? null;

  useEffect(() => {
    if (!selectedProfile) return;
    setStartHotkey(selectedProfile.clicker.start_hotkey);
    setStopHotkey(selectedProfile.clicker.emergency_stop_hotkey);
  }, [profileId]);

  const bindings = useMemo<BindingEntry[]>(() => {
    const entries: BindingEntry[] = [
      { id: "recorder-start", label: "Macro recorder start", value: "F1" },
      { id: "recorder-stop", label: "Macro recorder stop", value: "F2" },
    ];
    if (selectedProfile) {
      entries.push(
        { id: "clicker-start", label: `${selectedProfile.name} · Clicker start`, value: startHotkey },
        { id: "clicker-stop", label: `${selectedProfile.name} · Emergency stop`, value: stopHotkey },
      );
    }
    for (const macro of macros) {
      const value = macroDrafts[macro.id] ?? macro.trigger;
      if (!isUnassigned(value)) entries.push({ id: `macro:${macro.id}`, label: `Macro · ${macro.name}`, value });
    }
    for (const mapping of remaps.filter((item) => item.enabled)) {
      const value = remapDrafts[mapping.id] ?? mapping.trigger;
      if (!isUnassigned(value)) entries.push({ id: `remap:${mapping.id}`, label: `Remap · ${mapping.name}`, value });
    }
    return entries;
  }, [selectedProfile, startHotkey, stopHotkey, macros, macroDrafts, remaps, remapDrafts]);

  const conflicts = useMemo(() => {
    const groups = new Map<string, BindingEntry[]>();
    for (const entry of bindings) {
      const key = canonical(entry.value);
      if (!key) continue;
      const list = groups.get(key) ?? [];
      list.push(entry);
      groups.set(key, list);
    }
    return Array.from(groups.entries()).filter(([, entries]) => entries.length > 1);
  }, [bindings]);

  const conflictFor = (id: string, value: string) => {
    if (isUnassigned(value)) return null;
    const key = canonical(value);
    const match = bindings.find((entry) => entry.id !== id && canonical(entry.value) === key);
    return match ?? null;
  };

  const saveProfileHotkeys = async () => {
    if (!selectedProfile) return;
    const startConflict = conflictFor("clicker-start", startHotkey);
    const stopConflict = conflictFor("clicker-stop", stopHotkey);
    if (startConflict || stopConflict) {
      setError(`Resolve keybind conflict with ${(startConflict ?? stopConflict)?.label} before saving.`);
      return;
    }
    setBusy(true);
    try {
      await profilesApi.save({
        ...selectedProfile,
        clicker: {
          ...selectedProfile.clicker,
          start_hotkey: startHotkey.trim(),
          emergency_stop_hotkey: stopHotkey.trim(),
        },
      });
      setStatus("Profile keybinds saved.");
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const saveMacroTrigger = async (macro: StoredMacro) => {
    const trigger = (macroDrafts[macro.id] ?? macro.trigger).trim() || "(unassigned)";
    const conflict = conflictFor(`macro:${macro.id}`, trigger);
    if (conflict) {
      setError(`${trigger} conflicts with ${conflict.label}.`);
      return;
    }
    setBusy(true);
    try {
      await macroApi.save({ ...macro, trigger });
      setStatus(`Saved ${macro.name} keybind.`);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const saveRemapTrigger = async (mapping: StoredRemap) => {
    const trigger = (remapDrafts[mapping.id] ?? mapping.trigger).trim();
    const conflict = conflictFor(`remap:${mapping.id}`, trigger);
    if (conflict) {
      setError(`${trigger} conflicts with ${conflict.label}.`);
      return;
    }
    setBusy(true);
    try {
      await remapApi.save({ ...mapping, trigger });
      setStatus(`Saved ${mapping.name} trigger.`);
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
          <h1 className="page-title">Keybinds</h1>
          <div className="page-subtitle">Manage runtime hotkeys in one place and detect conflicts before they reach the native hook runtime.</div>
        </div>
        <Button onClick={() => void refresh()}><RefreshCw size={14} /> Refresh</Button>
      </div>

      {error && <Card><div className="card-copy" style={{ color: "var(--danger)" }}>{error}</div></Card>}
      {status && !error && <Card><div className="card-copy">{status}</div></Card>}

      {conflicts.length > 0 && (
        <Card className="section-gap">
          <div className="inline" style={{ alignItems: "flex-start" }}>
            <AlertTriangle size={17} color="var(--warning)" />
            <div>
              <h2 className="card-title">Keybind conflicts</h2>
              {conflicts.map(([key, entries]) => (
                <div className="card-copy" key={key}><span className="kbd">{entries[0].value}</span> is used by {entries.map((entry) => entry.label).join(" and ")}.</div>
              ))}
            </div>
          </div>
        </Card>
      )}

      <div className="grid grid-2 section-gap">
        <Card>
          <h2 className="card-title"><KeyRound size={15} style={{ display: "inline", marginRight: 7 }} />Clicker keybinds</h2>
          <div className="card-copy">Click Bind, then press a keyboard key, chord or Mouse1–Mouse5. F1/F2 are reserved for macro recording.</div>
          <div className="section-gap">
            <Field label="Profile">
              <select className="select" value={profileId} onChange={(event) => setProfileId(event.target.value)}>
                {profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
              </select>
            </Field>
          </div>
          <div className="form-row section-gap">
            <Field label="Start / toggle"><HotkeyCaptureInput value={startHotkey} onChange={setStartHotkey} reserved={RESERVED_RECORDER_KEYS} /></Field>
            <Field label="Emergency stop"><HotkeyCaptureInput value={stopHotkey} onChange={setStopHotkey} reserved={RESERVED_RECORDER_KEYS} /></Field>
          </div>
          <div className="quick-actions"><Button variant="primary" disabled={busy || !selectedProfile} onClick={() => void saveProfileHotkeys()}><Save size={14} /> Save clicker keybinds</Button></div>
        </Card>

        <Card>
          <h2 className="card-title"><Radio size={15} style={{ display: "inline", marginRight: 7 }} />Macro recorder</h2>
          <div className="card-copy">Recorder keys are global and intentionally reserved so other VxClick bindings cannot steal them.</div>
          <div className="form-row section-gap">
            <Field label="Start recording"><input className="input" value="F1" readOnly /></Field>
            <Field label="Stop recording"><input className="input" value="F2" readOnly /></Field>
          </div>
        </Card>
      </div>

      <div className="grid grid-2 section-gap">
        <Card>
          <h2 className="card-title">Macro triggers</h2>
          <div className="card-copy">Click Bind to capture a trigger, or type one manually. Use “(unassigned)” for no global trigger.</div>
          <div className="section-gap" style={{ display: "grid", gap: 10 }}>
            {macros.length === 0 && <div className="card-copy">No saved macros.</div>}
            {macros.map((macro) => {
              const value = macroDrafts[macro.id] ?? macro.trigger;
              const conflict = conflictFor(`macro:${macro.id}`, value);
              return (
                <div key={macro.id} className="form-row" style={{ alignItems: "end" }}>
                  <Field label={macro.name}>
                    <HotkeyCaptureInput
                      value={value}
                      onChange={(next) => setMacroDrafts((current) => ({ ...current, [macro.id]: next }))}
                      reserved={RESERVED_RECORDER_KEYS}
                    />
                  </Field>
                  <div className="inline" style={{ alignItems: "center", flexWrap: "wrap" }}>
                    <Button disabled={busy || Boolean(conflict)} onClick={() => void saveMacroTrigger(macro)}><Save size={13} /> Save</Button>
                    {conflict && <span className="card-copy" style={{ color: "var(--warning)" }}>Conflicts with {conflict.label}</span>}
                  </div>
                </div>
              );
            })}
          </div>
        </Card>

        <Card>
          <h2 className="card-title">Remap triggers</h2>
          <div className="card-copy">Keyboard keys, chords and Mouse1–Mouse5 can be captured here. Actions remain in the Remap tab.</div>
          <div className="section-gap" style={{ display: "grid", gap: 10 }}>
            {remaps.length === 0 && <div className="card-copy">No saved remaps.</div>}
            {remaps.map((mapping) => {
              const value = remapDrafts[mapping.id] ?? mapping.trigger;
              const conflict = mapping.enabled ? conflictFor(`remap:${mapping.id}`, value) : null;
              return (
                <div key={mapping.id} className="form-row" style={{ alignItems: "end" }}>
                  <Field label={`${mapping.name} → ${mapping.action}`}>
                    <HotkeyCaptureInput
                      value={value}
                      onChange={(next) => setRemapDrafts((current) => ({ ...current, [mapping.id]: next }))}
                      reserved={RESERVED_RECORDER_KEYS}
                    />
                  </Field>
                  <div className="inline" style={{ alignItems: "center", flexWrap: "wrap" }}>
                    <Button disabled={busy || Boolean(conflict)} onClick={() => void saveRemapTrigger(mapping)}><Save size={13} /> Save</Button>
                    {!mapping.enabled && <span className="status-pill status-stopped">Disabled</span>}
                    {conflict && <span className="card-copy" style={{ color: "var(--warning)" }}>Conflicts with {conflict.label}</span>}
                  </div>
                </div>
              );
            })}
          </div>
        </Card>
      </div>
    </div>
  );
}
