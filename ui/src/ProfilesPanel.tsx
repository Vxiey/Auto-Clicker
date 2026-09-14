import { useCallback, useEffect, useState } from "react";
import { Gamepad2, MonitorDot, Plus, RefreshCw, Trash2 } from "lucide-react";
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

export function ProfilesPanel({ onActiveProfile }: { onActiveProfile?: (profile: Profile) => void }) {
  const [snapshot, setSnapshot] = useState<ProfilesSnapshot>(preview);
  const [name, setName] = useState("");
  const [processName, setProcessName] = useState("");
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

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <h1 className="page-title">Profiles</h1>
          <div className="page-subtitle">Per-game settings with automatic foreground-process switching.</div>
        </div>
        <Button onClick={() => void refresh()}><RefreshCw size={14} /> Refresh</Button>
      </div>

      <div className="grid grid-2">
        <Card>
          <h2 className="card-title">Create game profile</h2>
          <div className="card-copy">Bind a profile to an executable such as game.exe and VxClick can activate it automatically.</div>
          <div className="form-row section-gap">
            <Field label="Profile name"><input className="input" placeholder="Rainbow Six Siege" value={name} onChange={(event) => setName(event.target.value)} /></Field>
            <Field label="Process name"><input className="input" placeholder="RainbowSix.exe" value={processName} onChange={(event) => setProcessName(event.target.value)} /></Field>
          </div>
          <div className="quick-actions"><Button variant="primary" disabled={busy || !name.trim()} onClick={() => void create()}><Plus size={14} /> Create profile</Button></div>
        </Card>

        <Card>
          <h2 className="card-title">Automatic switching</h2>
          <div className="card-copy">Foreground process: {snapshot.foreground_process ?? "Not available in browser preview"}</div>
          <div className="divider" />
          <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
            <div><strong style={{ fontSize: 12 }}>Auto-switch by process</strong><div className="card-copy">Switch profile when its bound executable becomes active.</div></div>
            <Toggle value={snapshot.document.auto_switch_enabled} onChange={(value) => void setAutoSwitch(value)} />
          </div>
        </Card>
      </div>

      {error && <Card className="section-gap"><div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div></Card>}

      <div className="grid grid-2 section-gap">
        {snapshot.document.profiles.map((profile) => {
          const active = profile.id === snapshot.document.active_profile_id;
          return (
            <Card key={profile.id}>
              <div className="inline" style={{ justifyContent: "space-between", width: "100%", alignItems: "flex-start" }}>
                <div>
                  <div className="inline"><Gamepad2 size={15} color="var(--cyan)" /><h2 className="card-title">{profile.name}</h2></div>
                  <div className="card-copy">{profile.description}</div>
                </div>
                {active ? <span className="status-pill status-running">Active</span> : <Button disabled={busy} onClick={() => void activate(profile)}>Activate</Button>}
              </div>
              <div className="divider" />
              <div className="card-copy"><MonitorDot size={12} style={{ display: "inline", marginRight: 6 }} />{profile.process_names.length ? profile.process_names.join(", ") : "No process binding"}</div>
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
