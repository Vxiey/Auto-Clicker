import { useCallback, useEffect, useMemo, useState } from "react";
import { AppWindow, MonitorDot, Plus, RefreshCw, Trash2 } from "lucide-react";
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

export function ProcessListPanel() {
  const [snapshot, setSnapshot] = useState<ProfilesSnapshot>(preview);
  const [selectedProfileId, setSelectedProfileId] = useState("default");
  const [processName, setProcessName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    try {
      const next = await profilesApi.snapshot();
      setSnapshot(next);
      setSelectedProfileId((current) =>
        next.document.profiles.some((profile) => profile.id === current)
          ? current
          : next.document.active_profile_id,
      );
      setError("");
    } catch {
      // Browser preview remains usable without Tauri.
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(refresh, 1000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const bindings = useMemo(
    () => snapshot.document.profiles.flatMap((profile) =>
      profile.process_names.map((process) => ({ profile, process })),
    ),
    [snapshot],
  );

  const selectedProfile =
    snapshot.document.profiles.find((profile) => profile.id === selectedProfileId) ??
    snapshot.document.profiles.find((profile) => profile.id === snapshot.document.active_profile_id) ??
    snapshot.document.profiles[0];

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

  const addProcess = async (value: string) => {
    if (!selectedProfile) return;
    const next = normalizeProcess(value);
    if (!next) return;
    if (selectedProfile.process_names.some((item) => normalizeProcess(item) === next)) {
      setProcessName("");
      return;
    }
    await saveProcesses(selectedProfile, [...selectedProfile.process_names, next]);
    setProcessName("");
  };

  const removeProcess = async (profile: Profile, process: string) => {
    await saveProcesses(
      profile,
      profile.process_names.filter((item) => item !== process),
    );
  };

  const setAutoSwitch = async (enabled: boolean) => {
    setSnapshot((current) => ({
      ...current,
      document: { ...current.document, auto_switch_enabled: enabled },
    }));
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
          <h1 className="page-title">Process List</h1>
          <div className="page-subtitle">
            Manage the applications that VxClick uses for profile auto-switching.
          </div>
        </div>
        <Button onClick={() => void refresh()}>
          <RefreshCw size={14} /> Refresh
        </Button>
      </div>

      <div className="grid grid-2">
        <Card>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", alignItems: "flex-start" }}>
            <div>
              <h2 className="card-title">Foreground process</h2>
              <div className="card-copy">The application currently detected in the foreground.</div>
            </div>
            <MonitorDot size={18} color="var(--cyan)" />
          </div>
          <div className="process-list section-gap">
            <div className="process-chip">
              <AppWindow size={12} />
              <span>{snapshot.foreground_process ?? "No foreground process detected"}</span>
            </div>
          </div>
          <div className="quick-actions">
            <Button
              variant="primary"
              disabled={busy || !snapshot.foreground_process || !selectedProfile}
              onClick={() => snapshot.foreground_process && void addProcess(snapshot.foreground_process)}
            >
              <Plus size={13} /> Add current app
            </Button>
          </div>
        </Card>

        <Card>
          <h2 className="card-title">Process switching</h2>
          <div className="card-copy">Automatically activate the matching profile when one of its saved executables is focused.</div>
          <div className="divider" />
          <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
            <div>
              <strong style={{ fontSize: 12 }}>Auto-switch by process</strong>
              <div className="card-copy">Uses the same process bindings as Profiles.</div>
            </div>
            <Toggle value={snapshot.document.auto_switch_enabled} onChange={(value) => void setAutoSwitch(value)} />
          </div>
        </Card>
      </div>

      <Card className="section-gap">
        <h2 className="card-title">Add process binding</h2>
        <div className="card-copy">Choose the destination profile, then enter an executable name such as game.exe.</div>
        <div className="form-row section-gap">
          <Field label="Profile">
            <select
              className="select"
              value={selectedProfile?.id ?? ""}
              onChange={(event) => setSelectedProfileId(event.target.value)}
            >
              {snapshot.document.profiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}{profile.id === snapshot.document.active_profile_id ? " · Active" : ""}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Executable">
            <input
              className="input"
              placeholder="game.exe"
              value={processName}
              onChange={(event) => setProcessName(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void addProcess(processName);
              }}
            />
          </Field>
        </div>
        <div className="quick-actions">
          <Button
            variant="primary"
            disabled={busy || !processName.trim() || !selectedProfile}
            onClick={() => void addProcess(processName)}
          >
            <Plus size={13} /> Add process
          </Button>
        </div>
      </Card>

      {error && (
        <Card className="section-gap">
          <div style={{ color: "var(--danger)", fontSize: 12 }}>{error}</div>
        </Card>
      )}

      <Card className="section-gap">
        <div className="inline" style={{ justifyContent: "space-between", width: "100%", flexWrap: "wrap" }}>
          <div>
            <h2 className="card-title">Saved applications</h2>
            <div className="card-copy">{bindings.length} process binding{bindings.length === 1 ? "" : "s"} across {snapshot.document.profiles.length} profiles.</div>
          </div>
        </div>

        <div className="divider" />

        {bindings.length === 0 ? (
          <div className="card-copy">No applications are currently bound to profiles.</div>
        ) : (
          <div style={{ display: "grid", gap: 8 }}>
            {bindings.map(({ profile, process }) => (
              <div
                key={`${profile.id}:${process}`}
                className="process-list-row"
                style={{
                  display: "grid",
                  gridTemplateColumns: "minmax(0, 1fr) minmax(130px, auto) auto",
                  gap: 10,
                  alignItems: "center",
                  padding: "10px 12px",
                  border: "1px solid var(--border)",
                  borderRadius: 10,
                }}
              >
                <div className="inline" style={{ minWidth: 0 }}>
                  <AppWindow size={14} color="var(--cyan)" />
                  <strong style={{ fontSize: 12, overflow: "hidden", textOverflow: "ellipsis" }}>{process}</strong>
                </div>
                <span className="profile-chip" title={profile.name}>{profile.name}</span>
                <Button variant="danger" disabled={busy} onClick={() => void removeProcess(profile, process)}>
                  <Trash2 size={13} /> Remove
                </Button>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
