import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  Crosshair,
  Gamepad2,
  Pause,
  Play,
  Plus,
  Save,
  ShieldCheck,
  ShieldOff,
  SlidersHorizontal,
  Upload,
} from "lucide-react";
import {
  recoilApi,
  type RecoilGameProfile,
  type RecoilPreset,
  type RecoilSnapshot,
  type RecoilStep,
} from "./api";
import { Button, Card, Field, MetricCard, StatusPill, Toggle } from "./components";

const MAX_RECOIL_IMPORT_BYTES = 1024 * 1024;

const emptySnapshot: RecoilSnapshot = {
  document: {
    schema_version: 2,
    active_game_id: "generic",
    active_slot: 1,
    games: [],
    risk_acknowledgement: null,
  },
  running: false,
  active_preset_id: null,
  risk_acknowledgement_required: true,
  risk_acknowledgement_version: 1,
};

export function RecoilScripts() {
  const uploadInputRef = useRef<HTMLInputElement | null>(null);
  const [snapshot, setSnapshot] = useState<RecoilSnapshot>(emptySnapshot);
  const [selectedPresetId, setSelectedPresetId] = useState("");
  const [draft, setDraft] = useState<RecoilPreset | null>(null);
  const [patternText, setPatternText] = useState("0,4");
  const [newGameName, setNewGameName] = useState("");
  const [newGameProcess, setNewGameProcess] = useState("");
  const [confirmedAccountRisk, setConfirmedAccountRisk] = useState(false);
  const [confirmedThirdPartyRules, setConfirmedThirdPartyRules] = useState(false);
  const [importMessage, setImportMessage] = useState("");
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

  const acceptRisk = async () => {
    if (!confirmedAccountRisk || !confirmedThirdPartyRules) return;
    setBusy(true);
    try {
      await recoilApi.acceptRisk(confirmedAccountRisk, confirmedThirdPartyRules);
      setConfirmedAccountRisk(false);
      setConfirmedThirdPartyRules(false);
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setBusy(false);
    }
  };

  const savePreset = async () => {
    if (!activeGame || !draft) return;
    setBusy(true);
    try {
      const pattern = parsePattern(patternText);
      const saved = await recoilApi.savePreset(activeGame.id, { ...draft, pattern });
      setDraft(saved);
      setPatternText(formatPattern(saved.pattern));
      setImportMessage("");
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
      name: slot === 1 ? "New Primary Script" : "New Secondary Script",
      character_name: "",
      weapon_name: "Custom",
      slot,
      enabled: true,
      vertical: 1,
      horizontal: 1,
      rpm: 600,
      activation_mode: "ads-fire",
      activation_hotkey: slot === 1 ? "F9" : "F10",
      pattern: [{ x: 0, y: 4 }],
    };
    try {
      await recoilApi.savePreset(activeGame.id, preset);
      setSelectedPresetId(id);
      setDraft(preset);
      setPatternText(formatPattern(preset.pattern));
      setImportMessage("");
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    }
  };

  const importPresetFile = async (file: File | null) => {
    if (!file || !activeGame) return;
    setBusy(true);
    setImportMessage("");
    setError("");
    try {
      const lowerName = file.name.toLowerCase();
      if (!lowerName.endsWith(".json") && !lowerName.endsWith(".vxrecoil")) {
        throw new Error("Select a .json or .vxrecoil recoil script file");
      }
      if (file.size > MAX_RECOIL_IMPORT_BYTES) {
        throw new Error("Recoil script files cannot exceed 1 MiB");
      }
      const preset = parseImportedPreset(await file.text(), file.name, snapshot.document.active_slot);
      const saved = await recoilApi.savePreset(activeGame.id, preset);
      if (saved.slot !== snapshot.document.active_slot) await recoilApi.setSlot(saved.slot);
      setSelectedPresetId(saved.id);
      setDraft(saved);
      setPatternText(formatPattern(saved.pattern));
      setImportMessage(`Imported ${saved.name} into ${activeGame.name}.`);
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setBusy(false);
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
          <div className="page-subtitle">Create, import and save data-only recoil scripts with separate game, character/operator and weapon metadata.</div>
        </div>
        <StatusPill status={snapshot.running ? "Running" : "Ready"} />
      </div>

      <Card>
        <div className="inline" style={{ alignItems: "flex-start", gap: 12 }}>
          <AlertTriangle size={19} style={{ flex: "0 0 auto", marginTop: 1 }} />
          <div>
            <h2 className="card-title">Automation risk notice</h2>
            <div className="card-copy">
              Recoil compensation, macros and automation may violate the rules of games or other third-party software and may result in restrictions, suspensions or permanent account bans. VxClick is a personal hobby/open-source project and is not affiliated with or endorsed by game publishers. You are responsible for checking the rules that apply to your use. VxClick does not provide anti-cheat bypass or detection-evasion functionality.
            </div>
          </div>
        </div>
      </Card>

      {snapshot.risk_acknowledgement_required && (
        <Card className="section-gap">
          <div className="inline" style={{ alignItems: "flex-start", gap: 12 }}>
            <ShieldCheck size={20} style={{ flex: "0 0 auto", marginTop: 1 }} />
            <div style={{ width: "100%" }}>
              <h2 className="card-title">Risk acknowledgement required</h2>
              <div className="card-copy">
                Review and acknowledge version {snapshot.risk_acknowledgement_version} before recoil functionality can be armed. This acknowledgement records that the warning was shown; it does not make prohibited use permitted by any third party.
              </div>
              <label className="inline section-gap" style={{ alignItems: "flex-start", cursor: "pointer" }}>
                <input type="checkbox" checked={confirmedAccountRisk} onChange={(event) => setConfirmedAccountRisk(event.target.checked)} />
                <span className="card-copy">I understand that automation or recoil compensation may cause account restrictions, suspensions or permanent bans.</span>
              </label>
              <label className="inline" style={{ alignItems: "flex-start", cursor: "pointer", marginTop: 10 }}>
                <input type="checkbox" checked={confirmedThirdPartyRules} onChange={(event) => setConfirmedThirdPartyRules(event.target.checked)} />
                <span className="card-copy">I understand that I am responsible for checking and following the rules, terms and policies of software or services I choose to automate.</span>
              </label>
              <div className="quick-actions section-gap">
                <Button
                  variant="primary"
                  disabled={busy || !confirmedAccountRisk || !confirmedThirdPartyRules}
                  onClick={() => void acceptRisk()}
                >
                  <CheckCircle2 size={14} /> I Understand & Accept
                </Button>
                <span className="card-copy">Stored locally in VxClick configuration. Material warning changes can require acknowledgement again.</span>
              </div>
            </div>
          </div>
        </Card>
      )}

      <div className="grid grid-4 section-gap">
        <MetricCard label="Game profile" value={activeGame?.name ?? "None"} accent icon={<Gamepad2 size={16} color="var(--cyan)" />} />
        <MetricCard label="Active slot" value={snapshot.document.active_slot === 1 ? "Primary" : "Secondary"} icon={<Crosshair size={16} color="var(--accent)" />} />
        <MetricCard label="Scripts" value={(activeGame?.presets.length ?? 0).toString()} icon={<SlidersHorizontal size={16} color="var(--muted)" />} />
        <MetricCard label="Runtime" value={snapshot.running ? "Armed" : snapshot.risk_acknowledgement_required ? "Locked" : "Stopped"} icon={<ShieldOff size={16} color="var(--muted)" />} />
      </div>

      <div className="grid grid-2 section-gap">
        <Card>
          <h2 className="card-title">Game profile</h2>
          <div className="card-copy">Each game has its own process list and completely separate recoil script library. VxClick ships with a generic profile rather than publisher-specific competitive recoil presets.</div>
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
            <Button
              variant={snapshot.running ? "danger" : "primary"}
              disabled={busy || !draft || (!snapshot.running && snapshot.risk_acknowledgement_required)}
              onClick={() => void toggleRuntime()}
            >
              {snapshot.running ? <><Pause size={14} /> Stop recoil</> : <><Play size={14} /> Arm recoil</>}
            </Button>
            <Button onClick={() => void setSlot(1)} variant={snapshot.document.active_slot === 1 ? "primary" : undefined}>Primary</Button>
            <Button onClick={() => void setSlot(2)} variant={snapshot.document.active_slot === 2 ? "primary" : undefined}>Secondary</Button>
          </div>
          <div className="divider" />
          <div className="card-copy">Game rules differ. The user is responsible for checking whether automation is permitted. See TERMS.md, DISCLAIMER.md and LICENSE in the project repository.</div>
        </Card>
      </div>

      <div className="grid grid-2 section-gap">
        <Card>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", flexWrap: "wrap", alignItems: "flex-start" }}>
            <div><h2 className="card-title">Recoil scripts</h2><div className="card-copy">{snapshot.document.active_slot === 1 ? "Primary" : "Secondary"} scripts for {activeGame?.name ?? "the active game"}.</div></div>
            <div className="quick-actions" style={{ marginTop: 0 }}>
              <Button disabled={busy || !activeGame} onClick={() => uploadInputRef.current?.click()}><Upload size={14} /> Upload script</Button>
              <Button disabled={busy || !activeGame} onClick={() => void addPreset()}><Plus size={14} /> New script</Button>
            </div>
          </div>
          <input
            ref={uploadInputRef}
            type="file"
            accept=".json,.vxrecoil,application/json,text/plain"
            style={{ display: "none" }}
            onChange={(event) => {
              const file = event.target.files?.[0] ?? null;
              event.target.value = "";
              void importPresetFile(file);
            }}
          />
          <div className="card-copy section-gap">Imports are data-only JSON/VXRECOIL files. Imported IDs are regenerated so an upload cannot silently overwrite an existing script.</div>
          {importMessage && <div className="card-copy" style={{ color: "var(--success)" }}>{importMessage}</div>}
          <div className="section-gap" style={{ display: "grid", gap: 8 }}>
            {slotPresets.map((preset) => (
              <button key={preset.id} className={`nav-button ${preset.id === selectedPresetId ? "active" : ""}`} onClick={() => selectPreset(preset.id)}>
                <Crosshair size={15} />
                <span>{preset.name} · {preset.character_name ? `${preset.character_name} · ` : ""}{preset.weapon_name} · {preset.rpm.toFixed(0)} RPM</span>
              </button>
            ))}
          </div>
        </Card>

        <Card>
          <h2 className="card-title">Script editor</h2>
          {!draft ? <div className="card-copy section-gap">Select a script to edit.</div> : <>
            <div className="form-row section-gap">
              <Field label="Script name"><input className="input" value={draft.name} maxLength={64} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></Field>
              <Field label="Character / Operator (optional)"><input className="input" value={draft.character_name} maxLength={64} onChange={(event) => setDraft({ ...draft, character_name: event.target.value })} placeholder="e.g. Operator name" /></Field>
            </div>
            <div className="form-row section-gap">
              <Field label="Weapon"><input className="input" value={draft.weapon_name} maxLength={64} onChange={(event) => setDraft({ ...draft, weapon_name: event.target.value })} placeholder="e.g. Rifle A" /></Field>
              <Field label="RPM"><input className="input" type="number" min="30" max="3000" value={draft.rpm} onChange={(event) => setDraft({ ...draft, rpm: Number(event.target.value) })} /></Field>
            </div>
            <div className="form-row section-gap">
              <Field label="Activation">
                <select className="select" value={draft.activation_mode} onChange={(event) => setDraft({ ...draft, activation_mode: event.target.value as RecoilPreset["activation_mode"] })}>
                  <option value="ads-fire">ADS + Fire</option>
                  <option value="fire">Fire only</option>
                  <option value="always">Always while armed</option>
                </select>
              </Field>
              <Field label="Activation hotkey"><input className="input" value={draft.activation_hotkey} onChange={(event) => setDraft({ ...draft, activation_hotkey: event.target.value })} /></Field>
            </div>
            <div className="form-row section-gap">
              <Field label={`Vertical · ${draft.vertical.toFixed(2)}×`}><input className="range" type="range" min="0" max="10" step="0.05" value={draft.vertical} onChange={(event) => setDraft({ ...draft, vertical: Number(event.target.value) })} /></Field>
              <Field label={`Horizontal · ${draft.horizontal.toFixed(2)}×`}><input className="range" type="range" min="0" max="10" step="0.05" value={draft.horizontal} onChange={(event) => setDraft({ ...draft, horizontal: Number(event.target.value) })} /></Field>
            </div>
            <div className="section-gap"><Field label="Pattern · X,Y per shot"><textarea className="input" style={{ minHeight: 120, resize: "vertical" }} value={patternText} onChange={(event) => setPatternText(event.target.value)} placeholder={"0,4\n0,5\n1,5\n-1,6"} /></Field></div>
            <div className="inline section-gap" style={{ justifyContent: "space-between", width: "100%", flexWrap: "wrap" }}>
              <div className="inline"><Toggle value={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })} /><span className="card-copy">Script enabled</span></div>
              <Button variant="primary" disabled={busy} onClick={() => void savePreset()}><Save size={14} /> Save script</Button>
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

function parseImportedPreset(text: string, fileName: string, fallbackSlot: 1 | 2): RecoilPreset {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("Recoil script is not valid JSON");
  }
  const root = asRecord(parsed, "recoil script");
  const source = root.preset === undefined ? root : asRecord(root.preset, "preset");
  const patternSource = source.pattern;
  if (!Array.isArray(patternSource) || patternSource.length === 0 || patternSource.length > 512) {
    throw new Error("Imported recoil pattern must contain 1-512 steps");
  }
  const pattern = patternSource.map((value, index) => {
    let x: number;
    let y: number;
    if (Array.isArray(value)) {
      x = Number(value[0]);
      y = Number(value[1]);
    } else {
      const point = asRecord(value, `pattern step ${index + 1}`);
      x = Number(point.x);
      y = Number(point.y);
    }
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      throw new Error(`Invalid X/Y value in pattern step ${index + 1}`);
    }
    return { x: Math.round(x), y: Math.round(y) };
  });

  const slotValue = Number(source.slot);
  const slot: 1 | 2 = slotValue === 1 || slotValue === 2 ? slotValue : fallbackSlot;
  const activation = typeof source.activation_mode === "string" ? source.activation_mode : "ads-fire";
  const activationMode: RecoilPreset["activation_mode"] = activation === "fire" || activation === "always" ? activation : "ads-fire";
  const fallbackName = fileName.replace(/\.(?:json|vxrecoil)$/i, "").trim() || "Imported Recoil Script";

  return {
    id: `import-${Date.now()}`,
    name: typeof source.name === "string" && source.name.trim() ? source.name.trim() : fallbackName,
    character_name: typeof source.character_name === "string" ? source.character_name.trim() : "",
    weapon_name: typeof source.weapon_name === "string" && source.weapon_name.trim() ? source.weapon_name.trim() : "Custom",
    slot,
    enabled: typeof source.enabled === "boolean" ? source.enabled : true,
    vertical: source.vertical === undefined ? 1 : Number(source.vertical),
    horizontal: source.horizontal === undefined ? 1 : Number(source.horizontal),
    rpm: source.rpm === undefined ? 600 : Number(source.rpm),
    activation_mode: activationMode,
    activation_hotkey: typeof source.activation_hotkey === "string" && source.activation_hotkey.trim()
      ? source.activation_hotkey.trim()
      : slot === 1 ? "F9" : "F10",
    pattern,
  };
}

function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be a JSON object`);
  }
  return value as Record<string, unknown>;
}

function formatPattern(pattern: RecoilStep[]) {
  return pattern.map((step) => `${step.x},${step.y}`).join("\n");
}
