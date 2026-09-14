import { useEffect, useMemo, useState } from "react";
import { Crosshair, Gamepad2, Pause, Play, Plus, Save, ShieldOff, SlidersHorizontal } from "lucide-react";
import {
  recoilApi,
  type RecoilGameProfile,
  type RecoilPreset,
  type RecoilSnapshot,
  type RecoilStep,
} from "./api";
import { Button, Card, Field, MetricCard, StatusPill, Toggle } from "./components";

const emptySnapshot: RecoilSnapshot = {
  document: {
    schema_version: 1,
    active_game_id: "generic",
    active_slot: 1,
    games: [],
  },
  running: false,
  active_preset_id: null,
};

export function RecoilScripts() {
  const [snapshot, setSnapshot] = useState<RecoilSnapshot>(emptySnapshot);
  const [selectedPresetId, setSelectedPresetId] = useState("");
  const [draft, setDraft] = useState<RecoilPreset | null>(null);
  const [patternText, setPatternText] = useState("0,4");
  const [newGameName, setNewGameName] = useState("");
  const [newGameProcess, setNewGameProcess] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = async () => {
    try {
      const next = await recoilApi.snapshot();
      setSnapshot(next);
      setError("");
    } catch (nextError) {
      setError(String(nextError));
    }
  };

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 500);
    return () => window.clearInterval(timer);
  }, []);

  const activeGame = useMemo(
    () => snapshot.document.games.find((game) => game.id === snapshot.document.active_game_id) ?? snapshot.document.games[0],
    [snapshot],
  );

  const slotPresets = useMemo(
    () => activeGame?.presets.filter((preset) => preset.slot === snapshot.document.active_slot) ?? [],
    [activeGame, snapshot.document.active_slot],
  );

  useEffect(() => {
    if (!activeGame) return;
    const activeId = snapshot.document.active_slot === 2 ? activeGame.active_secondary_id : activeGame.active_primary_id;
    const next = activeGame.presets.find((preset) => preset.id === activeId) ?? slotPresets[0];
    if (next && next.id !== selectedPresetId) {
      setSelectedPresetId(next.id);
      setDraft(structuredClone(next));
      setPatternText(formatPattern(next.pattern));
    }
  }, [activeGame, slotPresets, snapshot.document.active_slot, selectedPresetId]);

  const selectPreset = (id: string) => {
    const next = activeGame?.presets.find((preset) => preset.id === id);
    setSelectedPresetId(id);
    if (next) {
      setDraft(structuredClone(next));
      setPatternText(formatPattern(next.pattern));
    }
  };

  const activateGame = async (id: string) => {
    await recoilApi.activateGame(id);
    setSelectedPresetId("");
    await refresh();
  };

  const setSlot = async (slot: 1 | 2) => {
    await recoilApi.setSlot(slot);
    setSelectedPresetId("");
    await refresh();
  };

  const savePreset = async () => {
    if (!activeGame || !draft) return;
    setBusy(true);
    try {
      const pattern = parsePattern(patternText);
      const saved = await recoilApi.savePreset(activeGame.id, { ...draft, pattern });
      setDraft(saved);
      setPatternText(formatPattern(saved.pattern));
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setBusy(false);
    }
  };

  const addPreset = async () => {
    if (!activeGame) return;
    const slot = snapshot.document.active_slot;
    const id = `preset-${Date.now()}`;
    const preset: RecoilPreset = {
      id,
      name: slot === 1 ? "New Primary" : "New Secondary",
      weapon_name: "Custom",
      slot,
      enabled: true,
      vertical: 1,
      horizontal: 1,
      rpm: 600,
      activation_mode: "ads-fire",
      activation_hotkey: slot === 1 ? "F1" : "F2",
      pattern: [{ x: 0, y: 4 }],
    };
    try {
      await recoilApi.savePreset(activeGame.id, preset);
      setSelectedPresetId(id);
      setDraft(preset);
      setPatternText(formatPattern(preset.pattern));
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    }
  };

  const createGame = async () => {
    const name = newGameName.trim();
    if (!name) return;
    setBusy(true);
    try {
      await recoilApi.createGame(name, newGameProcess.trim() || undefined);
      setNewGameName("");
      setNewGameProcess("");
      setSelectedPresetId("");
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setBusy(false);
    }
  };

  const updateGame = async (patch: Partial<RecoilGameProfile>) => {
    if (!activeGame) return;
    try {
      await recoilApi.saveGame({ ...activeGame, ...patch });
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    }
  };

  const toggleRuntime = async () => {
    setBusy(true);
    try {
      if (snapshot.running) await recoilApi.stop();
      else await recoilApi.start();
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <h1 className="page-title">Recoil Scripts</h1>
          <div className="page-subtitle">Multi-game recoil profiles with separate primary/secondary weapon scripts.</div>
        </div>
        <StatusPill status={snapshot.running ? "Running" : "Ready"} />
      </div>

      <div className="grid grid-4">
        <MetricCard label="Game profile" value={activeGame?.name ?? "None"} accent icon={<Gamepad2 size={16} color="var(--cyan)" />} />
        <MetricCard label="Active slot" value={snapshot.document.active_slot === 1 ? "Primary" : "Secondary"} icon={<Crosshair size={16} color="var(--accent)" />} />
        <MetricCard label="Presets" value={(activeGame?.presets.length ?? 0).toString()} icon={<SlidersHorizontal size={16} color="var(--muted)" />} />
        <MetricCard label="Runtime" value={snapshot.running ? "Armed" : "Stopped"} icon={<ShieldOff size={16} color="var(--muted)" />} />
      </div>

      <div className="grid grid-2 section-gap">
        <Card>
          <h2 className="card-title">Game profile</h2>
          <div className="card-copy">Profiles are intentionally game-agnostic. Add process names now or later for per-game switching.</div>
          <div className="form-row section-gap">
            <Field label="Active game">
              <select className="select" value={activeGame?.id ?? ""} onChange={(event) => void activateGame(event.target.value)}>
                {snapshot.document.games.map((game) => <option key={game.id} value={game.id}>{game.name}</option>)}
              </select>
            </Field>
            <Field label="Processes">
              <input
                className="input"
                value={activeGame?.process_names.join(", ") ?? ""}
                onChange={(event) => void updateGame({ process_names: event.target.value.split(",").map((value) => value.trim()).filter(Boolean) })}
                placeholder="game.exe, game_dx12.exe"
              />
            </Field>
          </div>
          <div className="form-row section-gap">
            <Field label="New game profile"><input className="input" value={newGameName} onChange={(event) => setNewGameName(event.target.value)} placeholder="Game name" /></Field>
            <Field label="Optional process"><input className="input" value={newGameProcess} onChange={(event) => setNewGameProcess(event.target.value)} placeholder="game.exe" /></Field>
          </div>
          <div className="quick-actions"><Button onClick={() => void createGame()} disabled={busy || !newGameName.trim()}><Plus size={14} /> Add game</Button></div>
        </Card>

        <Card>
          <h2 className="card-title">Runtime</h2>
          <div className="card-copy">The recoil worker is separate from the clicker and only moves the cursor while the selected activation condition is true.</div>
          <div className="quick-actions section-gap">
            <Button variant={snapshot.running ? "danger" : "primary"} disabled={busy || !draft} onClick={() => void toggleRuntime()}>
              {snapshot.running ? <><Pause size={14} /> Stop recoil</> : <><Play size={14} /> Arm recoil</>}
            </Button>
            <Button onClick={() => void setSlot(1)} variant={snapshot.document.active_slot === 1 ? "primary" : undefined}>Primary</Button>
            <Button onClick={() => void setSlot(2)} variant={snapshot.document.active_slot === 2 ? "primary" : undefined}>Secondary</Button>
          </div>
          <div className="divider" />
          <div className="card-copy">No anti-cheat bypass, process injection, stealth or anti-detection behavior is part of this system.</div>
        </Card>
      </div>

      <div className="grid grid-2 section-gap">
        <Card>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
            <div><h2 className="card-title">Presets</h2><div className="card-copy">{snapshot.document.active_slot === 1 ? "Primary" : "Secondary"} scripts for {activeGame?.name ?? "the active game"}.</div></div>
            <Button onClick={() => void addPreset()}><Plus size={14} /> New preset</Button>
          </div>
          <div className="section-gap" style={{ display: "grid", gap: 8 }}>
            {slotPresets.map((preset) => (
              <button key={preset.id} className={`nav-button ${preset.id === selectedPresetId ? "active" : ""}`} onClick={() => selectPreset(preset.id)}>
                <Crosshair size={15} /><span>{preset.name} · {preset.weapon_name} · {preset.rpm.toFixed(0)} RPM</span>
              </button>
            ))}
          </div>
        </Card>

        <Card>
          <h2 className="card-title">Script editor</h2>
          {!draft ? <div className="card-copy section-gap">Select a preset to edit.</div> : <>
            <div className="form-row section-gap">
              <Field label="Preset name"><input className="input" value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></Field>
              <Field label="Weapon / label"><input className="input" value={draft.weapon_name} onChange={(event) => setDraft({ ...draft, weapon_name: event.target.value })} /></Field>
            </div>
            <div className="form-row section-gap">
              <Field label="RPM"><input className="input" type="number" min="30" max="3000" value={draft.rpm} onChange={(event) => setDraft({ ...draft, rpm: Number(event.target.value) })} /></Field>
              <Field label="Activation">
                <select className="select" value={draft.activation_mode} onChange={(event) => setDraft({ ...draft, activation_mode: event.target.value })}>
                  <option value="ads-fire">ADS + Fire</option>
                  <option value="fire">Fire only</option>
                  <option value="always">Always while armed</option>
                </select>
              </Field>
            </div>
            <div className="form-row section-gap">
              <Field label={`Vertical · ${draft.vertical.toFixed(2)}×`}><input className="range" type="range" min="0" max="10" step="0.05" value={draft.vertical} onChange={(event) => setDraft({ ...draft, vertical: Number(event.target.value) })} /></Field>
              <Field label={`Horizontal · ${draft.horizontal.toFixed(2)}×`}><input className="range" type="range" min="0" max="10" step="0.05" value={draft.horizontal} onChange={(event) => setDraft({ ...draft, horizontal: Number(event.target.value) })} /></Field>
            </div>
            <div className="section-gap"><Field label="Pattern · X,Y per shot"><textarea className="input" style={{ minHeight: 120, resize: "vertical" }} value={patternText} onChange={(event) => setPatternText(event.target.value)} placeholder={"0,4\n0,5\n1,5\n-1,6"} /></Field></div>
            <div className="inline section-gap" style={{ justifyContent: "space-between", width: "100%" }}>
              <div className="inline"><Toggle value={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })} /><span className="card-copy">Preset enabled</span></div>
              <Button variant="primary" disabled={busy} onClick={() => void savePreset()}><Save size={14} /> Save preset</Button>
            </div>
          </>}
        </Card>
      </div>

      {error && <Card className="section-gap"><div className="card-copy" style={{ color: "var(--danger)" }}>{error}</div></Card>}
    </div>
  );
}

function parsePattern(value: string): RecoilStep[] {
  const steps = value
    .split(/\r?\n|;/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [xRaw, yRaw, extra] = line.split(",").map((part) => part.trim());
      if (extra !== undefined) throw new Error(`Invalid pattern row: ${line}`);
      const x = Number(xRaw);
      const y = Number(yRaw);
      if (!Number.isFinite(x) || !Number.isFinite(y)) throw new Error(`Invalid pattern row: ${line}`);
      return { x: Math.round(x), y: Math.round(y) };
    });
  if (!steps.length) throw new Error("Pattern must contain at least one X,Y row");
  if (steps.length > 512) throw new Error("Pattern supports at most 512 steps");
  return steps;
}

function formatPattern(pattern: RecoilStep[]) {
  return pattern.map((step) => `${step.x},${step.y}`).join("\n");
}
