import { useCallback, useEffect, useState } from "react";
import { Gamepad2, MonitorDot, Plus, RefreshCw, Trash2, X } from "lucide-react";
import { profilesApi, type Profile, type ProfilesSnapshot } from "./api";
import { Button, Card, Field, Toggle } from "./components";

const preview: ProfilesSnapshot = {
  foreground_process: null,
  document: {
    schema_version: 1,
    active_profile_id: "default",
    auto_switch_enabled: true,
    profiles: [
      {
        id: "default",
        name: "Default",
        description: "General Windows automation",
        process_names: [],
        auto_switch: false,
        clicker: {
          cps: 250,
          button: "left",
          mode: "toggle",
          randomize: false,
          burst: false,
          start_hotkey: "F6",
          emergency_stop_hotkey: "F8",
        },
      },
    ],
  },
};

function normalizeProcess(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return "";
  const slash = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return trimmed.slice(slash + 1).toLowerCase();
}

export function ProfilesPanel({ onActiveProfile }: { onActiveProfile?: (profile: Profile) => void }) {
  const [snapshot, setSnapshot] = useState<ProfilesSnapshot>(preview);
  const [name, setName] = useState("");
  const [processName, setProcessName] = useState("");
  const [processDrafts, setProcessDrafts] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    try {
      const next = await profilesApi.snapshot();
      setSnapshot(next);
      const active = next.document.profiles.find((profile) => profile.id === next.document.active_profile_id);
      if (active) onActiveProfile?.(active);
      setError("");
    } catch {
      // Browser preview stays functional without Tauri.
    }
  }, [onActiveProfile]);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(refresh, 800);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const create = async () => {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const profile = await profilesApi.create(name, processName);
      await profilesApi.activate(profile.id);
      setName("");
      setProcessName("");
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const activate = async (profile: Profile) => {
    setBusy(true);
    try {
      await profilesApi.activate(profile.id);
      onActiveProfile?.(profile);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (profile: Profile) => {
    setBusy(true);
    try {
      await profilesApi.remove(profile.id);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const setAutoSwitch = async (enabled: boolean) => {
    setSnapshot((current) => ({ ...current, document: { ...current.document, auto_switch_enabled: enabled } }));
    try {
      await profilesApi.setAutoSwitch(enabled);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  };

  const saveProcesses = async (profile: Profile, processNames: string[]) => {
    setBusy(true);
    try {
      const unique = Array.from(new Set(processNames.map(normalizeProcess).filter(Boolean)));
      await profilesApi.save({
        ...profile,
        process_names: unique,
        auto_switch: unique.length > 0 ? profile.auto_switch : false,
      });
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const addProcess = async (profile: Profile, value?: string) => {
    const next = normalizeProcess(value ?? processDrafts[profile.id] ?? "");
    if (!next) return;
    if (profile.process_names.some((item) => normalizeProcess(item) === next)) {
      setProcessDrafts((current) => ({ ...current, [profile.id]: "" }));
      return;
    }
    await saveProcesses(profile, [...profile.process_names, next]);
    setProcessDrafts((current) => ({ ...current, [profile.id]: "" }));
  };

  const removeProcess = async (profile: Profile, process: string) => {
    await saveProcesses(profile, profile.process_names.filter((item) => item !== process));
  };

  const addForegroundToActive = async () => {
    const active = snapshot.document.profiles.find((profile) => profile.id === snapshot.document.active_profile_id);
    if (!active || !snapshot.foreground_process) return;
    await addProcess(active, snapshot.foreground_process);
  };

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <h1 className="page-title">Profiles</h1>
          <div className="page-subtitle">Per-game settings with removable process bindings and automatic foreground-process switching.</div>
        </div>
        <Button onClick={() => void refresh()}><RefreshCw size={14} /> Refresh</Button>
      </div>

      <div className="grid grid-2">
        <Card>
          <h2 className="card-title">Create game profile</h2>
          <div className="card-copy">Bind a profile to an executable such as game.exe and VxClick can activate it automatically.</div>
          <div className="form-row section-gap">
            <Field label="Profile name"><input className="input" placeholder="Game profile" value={name} onChange={(event) => setName(event.target.value)} /></Field>
            <Field label="Process name"><input className="input" placeholder="game.exe" value={processName} onChange={(event) => setProcessName(event.target.value)} /></Field>
          </div>
          <div className="quick-actions"><Button variant="primary" disabled={busy || !name.trim()} onClick={() => void create()}><Plus size={14} /> Create profile</Button></div>
        </Card>

        <Card>
          <h2 className="card-title">Automatic switching</h2>
          <div className="card-copy">Foreground process: {snapshot.foreground_process ?? "Not available in browser preview"}</div>
          <div className="quick-actions">
            <Button disabled={busy || !snapshot.foreground_process} onClick={() => void addForegroundToActive()}><Plus size={13} /> Add current app to active profile</Button>
          </div>
          <div className="divider" />
          <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
            <div><strong style={{ fontSize: 12 }}>Auto-switch by process</strong><div className="card-copy">Switch profile when one of its saved executables becomes active.</div></div>
            <Toggle value={snapshot.document.auto_switch_enabled} onChange={(value) => void setAutoSwitch(value)} />
          </div>
        </Card>
      </div>

      {error && <Card className="section-gap"><div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div></Card>}

      <div className="grid grid-2 section-gap">
        {snapshot.document.profiles.map((profile) => {
          const active = profile.id === snapshot.document.active_profile_id;
          const draft = processDrafts[profile.id] ?? "";
          return (
            <Card key={profile.id}>
              <div className="inline" style={{ justifyContent: "space-between", width: "100%", alignItems: "flex-start", flexWrap: "wrap" }}>
                <div>
                  <div className="inline"><Gamepad2 size={15} color="var(--cyan)" /><h2 className="card-title">{profile.name}</h2></div>
                  <div className="card-copy">{profile.description}</div>
                </div>
                {active ? <span className="status-pill status-running">Active</span> : <Button disabled={busy} onClick={() => void activate(profile)}>Activate</Button>}
              </div>

              <div className="divider" />
              <div className="card-copy"><MonitorDot size={12} style={{ display: "inline", marginRight: 6 }} />Process list</div>
              {profile.process_names.length > 0 ? (
                <div className="process-list">
                  {profile.process_names.map((process) => (
                    <div className="process-chip" key={process}>
                      <span title={process}>{process}</span>
                      <button disabled={busy} onClick={() => void removeProcess(profile, process)} title={`Remove ${process}`}><X size={12} /></button>
                    </div>
                  ))}
                </div>
              ) : <div className="card-copy">No process bindings.</div>}

              <div className="inline section-gap" style={{ alignItems: "stretch", flexWrap: "wrap" }}>
                <input
                  className="input"
                  style={{ flex: "1 1 180px" }}
                  placeholder="Add app.exe"
                  value={draft}
                  onChange={(event) => setProcessDrafts((current) => ({ ...current, [profile.id]: event.target.value }))}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void addProcess(profile);
                  }}
                />
                <Button disabled={busy || !draft.trim()} onClick={() => void addProcess(profile)}><Plus size={13} /> Add app</Button>
              </div>

              <div className="quick-actions">
                <span className="profile-chip">{profile.clicker.cps} CPS</span>
                <span className="profile-chip">{profile.clicker.button}</span>
                <span className="profile-chip">{profile.clicker.mode}</span>
                {profile.id !== "default" && <Button variant="danger" disabled={busy} onClick={() => void remove(profile)}><Trash2 size={13} /> Delete</Button>}
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
