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
const MAX_RECOIL_STEPS = 512;
const MAX_IMPORTED_MOVE = 10_000;
const SUPPORTED_RECOIL_EXTENSIONS = [
  ".json",
  ".vxrecoil",
  ".lua",
  ".ahk",
  ".csv",
  ".txt",
  ".recoil",
] as const;
const NUMBER_SOURCE = "[+-]?(?:\\d+(?:\\.\\d+)?|\\.\\d+)";

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
      if (!SUPPORTED_RECOIL_EXTENSIONS.some((extension) => lowerName.endsWith(extension))) {
        throw new Error("Supported recoil imports: .vxrecoil, .json, .lua, .ahk, .csv, .txt and .recoil");
      }
      if (file.size > MAX_RECOIL_IMPORT_BYTES) {
        throw new Error("Recoil script files cannot exceed 1 MiB");
      }
      const imported = parseImportedPreset(await file.text(), file.name, snapshot.document.active_slot);
      const saved = await recoilApi.savePreset(activeGame.id, imported.preset);
      if (saved.slot !== snapshot.document.active_slot) await recoilApi.setSlot(saved.slot);
      setSelectedPresetId(saved.id);
      setDraft(saved);
      setPatternText(formatPattern(saved.pattern));
      setImportMessage(
        `Imported ${saved.name} as ${imported.format} into ${activeGame.name}.${imported.warning ? ` ${imported.warning}` : ""}`,
      );
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
            accept=".json,.vxrecoil,.lua,.ahk,.csv,.txt,.recoil,application/json,text/plain,text/csv"
            style={{ display: "none" }}
            onChange={(event) => {
              const file = event.target.files?.[0] ?? null;
              event.target.value = "";
              void importPresetFile(file);
            }}
          />
          <div className="card-copy section-gap">
            Data-only import supports VXRECOIL/JSON, static Logitech-style or VxClick Lua mouse moves, AutoHotkey relative mouse moves, CSV and plain X,Y text. Script files are parsed as text and never executed. Imported IDs are always regenerated.
          </div>
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
      return normalizeImportedStep(Number(xRaw), Number(yRaw), `pattern row '${line}'`);
    });
  if (!steps.length) throw new Error("Pattern must contain at least one X,Y row");
  if (steps.length > MAX_RECOIL_STEPS) throw new Error(`Pattern supports at most ${MAX_RECOIL_STEPS} steps`);
  return steps;
}

type ImportedPresetResult = {
  preset: RecoilPreset;
  format: string;
  warning?: string;
};

type ImportTiming = {
  rpm: number;
  warning?: string;
};

function parseImportedPreset(text: string, fileName: string, fallbackSlot: 1 | 2): ImportedPresetResult {
  const source = text.replace(/^\uFEFF/, "");
  if (!source.trim()) throw new Error("Recoil script file is empty");
  const extension = fileName.toLowerCase().match(/\.[^.]+$/)?.[0] ?? "";
  const trimmed = source.trimStart();

  if (extension === ".json" || extension === ".vxrecoil" || trimmed.startsWith("{") || trimmed.startsWith("[")) {
    return parseImportedJson(source, fileName, fallbackSlot);
  }

  if (extension === ".lua" || /\b(?:MoveMouseRelative|move_mouse)\s*\(/i.test(source)) {
    return parseImportedLua(source, fileName, fallbackSlot);
  }

  if (extension === ".ahk" || /\b(?:MouseMove|mouse_event|DllCall)\b/i.test(source)) {
    return parseImportedAhk(source, fileName, fallbackSlot);
  }

  return parseImportedDelimited(source, fileName, fallbackSlot, extension === ".csv" ? "CSV" : "plain text");
}

function parseImportedJson(text: string, fileName: string, fallbackSlot: 1 | 2): ImportedPresetResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("Recoil script is not valid JSON");
  }

  if (Array.isArray(parsed)) {
    const pattern = parseJsonPattern(parsed);
    return {
      preset: makeImportedPreset(fileName, fallbackSlot, pattern, 600),
      format: "JSON pattern",
    };
  }

  const root = asRecord(parsed, "recoil script");
  const source = root.preset === undefined ? root : asRecord(root.preset, "preset");
  let patternSource = firstDefined(source, ["pattern", "steps", "points", "recoil_pattern"]);
  if (patternSource === undefined && source.recoil !== undefined) {
    if (Array.isArray(source.recoil)) patternSource = source.recoil;
    else if (source.recoil && typeof source.recoil === "object") {
      patternSource = firstDefined(asRecord(source.recoil, "recoil"), ["pattern", "steps", "points"]);
    }
  }
  if (!Array.isArray(patternSource)) {
    throw new Error("Imported JSON must contain a pattern/steps/points array");
  }
  const pattern = parseJsonPattern(patternSource);

  const slotValue = Number(firstDefined(source, ["slot"]));
  const slot: 1 | 2 = slotValue === 1 || slotValue === 2 ? slotValue : fallbackSlot;
  const activation = firstDefined(source, ["activation_mode", "activation"]);
  const activationMode: RecoilPreset["activation_mode"] = activation === "fire" || activation === "always" ? activation : "ads-fire";
  const rpmValue = firstDefined(source, ["rpm", "RPM", "fire_rate", "fireRate", "rate"]);
  const rpm = rpmValue === undefined ? 600 : validateImportedRpm(Number(rpmValue));
  const verticalValue = firstDefined(source, ["vertical"]);
  const horizontalValue = firstDefined(source, ["horizontal"]);
  const vertical = verticalValue === undefined ? 1 : Number(verticalValue);
  const horizontal = horizontalValue === undefined ? 1 : Number(horizontalValue);
  if (!Number.isFinite(vertical) || !Number.isFinite(horizontal)) {
    throw new Error("Imported vertical/horizontal multiplier is invalid");
  }

  return {
    preset: {
      ...makeImportedPreset(fileName, slot, pattern, rpm),
      name: readNonEmptyString(source, ["name"]) ?? fallbackImportName(fileName),
      character_name: readString(source, ["character_name", "character", "operator"]) ?? "",
      weapon_name: readNonEmptyString(source, ["weapon_name", "weapon", "gun"]) ?? "Custom",
      enabled: typeof source.enabled === "boolean" ? source.enabled : true,
      vertical,
      horizontal,
      activation_mode: activationMode,
      activation_hotkey: readNonEmptyString(source, ["activation_hotkey", "hotkey"]) ?? (slot === 1 ? "F9" : "F10"),
    },
    format: "JSON/VXRECOIL",
  };
}

function parseJsonPattern(patternSource: unknown[]): RecoilStep[] {
  if (patternSource.length === 0 || patternSource.length > MAX_RECOIL_STEPS) {
    throw new Error(`Imported recoil pattern must contain 1-${MAX_RECOIL_STEPS} steps`);
  }
  return patternSource.map((value, index) => {
    let x: number;
    let y: number;
    if (Array.isArray(value)) {
      x = Number(value[0]);
      y = Number(value[1]);
    } else {
      const point = asRecord(value, `pattern step ${index + 1}`);
      x = Number(firstDefined(point, ["x", "dx", "move_x", "moveX"]));
      y = Number(firstDefined(point, ["y", "dy", "move_y", "moveY"]));
    }
    return normalizeImportedStep(x, y, `pattern step ${index + 1}`);
  });
}

function parseImportedLua(text: string, fileName: string, fallbackSlot: 1 | 2): ImportedPresetResult {
  const source = stripLuaComments(text);
  const moves = collectRegexMoves(
    source,
    new RegExp(`(?:MoveMouseRelative|move_mouse)\\s*\\(\\s*(${NUMBER_SOURCE})\\s*,\\s*(${NUMBER_SOURCE})\\s*\\)`, "gi"),
    "Lua mouse move",
  );

  let pattern = moves;
  let usedPatternTable = false;
  if (!pattern.length && /\b(?:recoil|pattern)\b/i.test(source)) {
    pattern = collectRegexMoves(
      source,
      new RegExp(`\\{\\s*x\\s*=\\s*(${NUMBER_SOURCE})\\s*,\\s*y\\s*=\\s*(${NUMBER_SOURCE})\\s*\\}`, "gi"),
      "Lua recoil table",
    );
    if (!pattern.length) {
      pattern = collectRegexMoves(
        source,
        new RegExp(`\\{\\s*(${NUMBER_SOURCE})\\s*,\\s*(${NUMBER_SOURCE})\\s*\\}`, "g"),
        "Lua recoil table",
      );
    }
    usedPatternTable = pattern.length > 0;
  }

  ensureImportedPattern(pattern, "Lua file does not contain literal MoveMouseRelative/move_mouse calls or a numeric recoil/pattern table");
  const explicitRpm = readAssignedRpm(source);
  const delays = collectNumbers(source, new RegExp(`(?:Sleep|sleep)\\s*\\(\\s*(${NUMBER_SOURCE})\\s*\\)`, "gi"));
  const timing = explicitRpm === undefined ? inferRpmFromDelays(delays) : { rpm: explicitRpm };
  const dynamicCalls = source.match(/\b(?:MoveMouseRelative|move_mouse)\s*\(/gi)?.length ?? 0;
  const dynamicWarning = !usedPatternTable && dynamicCalls > moves.length
    ? "Some dynamic Lua mouse moves could not be converted; only literal moves were imported."
    : undefined;

  return {
    preset: makeImportedPreset(fileName, fallbackSlot, pattern, timing.rpm),
    format: "Lua",
    warning: joinWarnings(timing.warning, dynamicWarning),
  };
}

function parseImportedAhk(text: string, fileName: string, fallbackSlot: 1 | 2): ImportedPresetResult {
  const source = text
    .split(/\r?\n/)
    .map((line) => line.replace(/\s+;.*$/, ""))
    .join("\n");
  const pattern: RecoilStep[] = [];
  const mouseMoveRegex = new RegExp(`^\\s*MouseMove(?:\\s*,|\\s+)\\s*(${NUMBER_SOURCE})\\s*,\\s*(${NUMBER_SOURCE})(.*)$`, "gim");
  for (const match of source.matchAll(mouseMoveRegex)) {
    const tail = match[3] ?? "";
    if (!/(?:,\s*R\b|["']R["'])/i.test(tail)) continue;
    pattern.push(normalizeImportedStep(Number(match[1]), Number(match[2]), "AutoHotkey MouseMove"));
  }

  const mouseEventRegex = new RegExp(
    `DllCall\\(\\s*["']mouse_event["'][^\\r\\n)]*?["']U?Int["']\\s*,\\s*(?:0x0*1|1)\\s*,\\s*["']Int["']\\s*,\\s*(${NUMBER_SOURCE})\\s*,\\s*["']Int["']\\s*,\\s*(${NUMBER_SOURCE})`,
    "gi",
  );
  for (const match of source.matchAll(mouseEventRegex)) {
    pattern.push(normalizeImportedStep(Number(match[1]), Number(match[2]), "AutoHotkey mouse_event"));
  }

  ensureImportedPattern(pattern, "AutoHotkey file does not contain literal relative MouseMove or mouse_event recoil moves");
  const explicitRpm = readAssignedRpm(source);
  const delays = collectNumbers(
    source,
    new RegExp(`^\\s*Sleep(?:\\s*,|\\s+|\\()\\s*(${NUMBER_SOURCE})`, "gim"),
  );
  const timing = explicitRpm === undefined ? inferRpmFromDelays(delays) : { rpm: explicitRpm };

  return {
    preset: makeImportedPreset(fileName, fallbackSlot, pattern, timing.rpm),
    format: "AutoHotkey",
    warning: timing.warning,
  };
}

function parseImportedDelimited(
  text: string,
  fileName: string,
  fallbackSlot: 1 | 2,
  format: string,
): ImportedPresetResult {
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.replace(/\s+(?:#|\/\/).*$/, "").trim())
    .filter(Boolean);
  if (!lines.length) throw new Error("Recoil pattern file is empty");

  const firstColumns = splitDelimitedLine(lines[0]).map((value) => value.toLowerCase());
  const xHeader = findHeader(firstColumns, ["x", "dx", "move_x", "movex"]);
  const yHeader = findHeader(firstColumns, ["y", "dy", "move_y", "movey"]);
  const delayHeader = findHeader(firstColumns, ["delay", "delay_ms", "sleep", "sleep_ms", "interval", "interval_ms"]);
  const rpmHeader = findHeader(firstColumns, ["rpm", "fire_rate", "firerate"]);
  const hasHeader = xHeader >= 0 && yHeader >= 0;
  const pattern: RecoilStep[] = [];
  const delays: number[] = [];
  let explicitRpm: number | undefined;

  for (let index = hasHeader ? 1 : 0; index < lines.length; index += 1) {
    const line = lines[index];
    const named = line.match(new RegExp(`^x\\s*[:=]\\s*(${NUMBER_SOURCE})[\\s,]+y\\s*[:=]\\s*(${NUMBER_SOURCE})$`, "i"));
    if (named) {
      pattern.push(normalizeImportedStep(Number(named[1]), Number(named[2]), `line ${index + 1}`));
      continue;
    }

    const columns = splitDelimitedLine(line);
    const xIndex = hasHeader ? xHeader : 0;
    const yIndex = hasHeader ? yHeader : 1;
    if (columns.length <= Math.max(xIndex, yIndex)) {
      throw new Error(`Invalid recoil row ${index + 1}: ${line}`);
    }
    pattern.push(normalizeImportedStep(Number(columns[xIndex]), Number(columns[yIndex]), `line ${index + 1}`));

    const rowDelay = hasHeader && delayHeader >= 0 ? Number(columns[delayHeader]) : (!hasHeader && columns.length === 3 ? Number(columns[2]) : Number.NaN);
    if (Number.isFinite(rowDelay) && rowDelay > 0) delays.push(rowDelay);
    if (hasHeader && rpmHeader >= 0 && explicitRpm === undefined) {
      const candidate = Number(columns[rpmHeader]);
      if (Number.isFinite(candidate)) explicitRpm = validateImportedRpm(candidate);
    }
  }

  ensureImportedPattern(pattern, "Recoil text file does not contain any X,Y rows");
  const timing = explicitRpm === undefined ? inferRpmFromDelays(delays) : { rpm: explicitRpm };
  return {
    preset: makeImportedPreset(fileName, fallbackSlot, pattern, timing.rpm),
    format,
    warning: timing.warning,
  };
}

function makeImportedPreset(
  fileName: string,
  slot: 1 | 2,
  pattern: RecoilStep[],
  rpm: number,
): RecoilPreset {
  ensureImportedPattern(pattern, "Imported recoil pattern is empty");
  return {
    id: `import-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name: fallbackImportName(fileName),
    character_name: "",
    weapon_name: "Custom",
    slot,
    enabled: true,
    vertical: 1,
    horizontal: 1,
    rpm: validateImportedRpm(rpm),
    activation_mode: "ads-fire",
    activation_hotkey: slot === 1 ? "F9" : "F10",
    pattern,
  };
}

function normalizeImportedStep(x: number, y: number, label: string): RecoilStep {
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    throw new Error(`Invalid X/Y value in ${label}`);
  }
  const roundedX = Math.round(x);
  const roundedY = Math.round(y);
  if (Math.abs(roundedX) > MAX_IMPORTED_MOVE || Math.abs(roundedY) > MAX_IMPORTED_MOVE) {
    throw new Error(`${label} exceeds the safe import range of ±${MAX_IMPORTED_MOVE} pixels per step`);
  }
  return { x: roundedX, y: roundedY };
}

function ensureImportedPattern(pattern: RecoilStep[], emptyMessage: string) {
  if (!pattern.length) throw new Error(emptyMessage);
  if (pattern.length > MAX_RECOIL_STEPS) {
    throw new Error(`Imported recoil pattern supports at most ${MAX_RECOIL_STEPS} steps`);
  }
}

function collectRegexMoves(source: string, regex: RegExp, label: string): RecoilStep[] {
  const pattern: RecoilStep[] = [];
  for (const match of source.matchAll(regex)) {
    pattern.push(normalizeImportedStep(Number(match[1]), Number(match[2]), label));
    if (pattern.length > MAX_RECOIL_STEPS) {
      throw new Error(`Imported recoil pattern supports at most ${MAX_RECOIL_STEPS} steps`);
    }
  }
  return pattern;
}

function collectNumbers(source: string, regex: RegExp): number[] {
  const values: number[] = [];
  for (const match of source.matchAll(regex)) {
    const value = Number(match[1]);
    if (Number.isFinite(value) && value > 0) values.push(value);
  }
  return values;
}

function inferRpmFromDelays(delays: number[]): ImportTiming {
  if (!delays.length) return { rpm: 600 };
  const sorted = [...delays].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
  if (!Number.isFinite(median) || median <= 0) return { rpm: 600 };
  const inferred = 60_000 / median;
  const clamped = Math.min(3_000, Math.max(30, inferred));
  const warning = clamped !== inferred
    ? `Source delays imply about ${Math.round(inferred)} RPM; VxClick clamped the import to ${Math.round(clamped)} RPM. Review timing before use.`
    : undefined;
  return { rpm: clamped, warning };
}

function readAssignedRpm(source: string): number | undefined {
  const match = source.match(new RegExp(`\\b(?:rpm|fire_?rate|firerate)\\s*[:=]\\s*(${NUMBER_SOURCE})`, "i"));
  if (!match) return undefined;
  return validateImportedRpm(Number(match[1]));
}

function validateImportedRpm(value: number): number {
  if (!Number.isFinite(value) || value < 30 || value > 3_000) {
    throw new Error("Imported recoil RPM must be between 30 and 3,000");
  }
  return value;
}

function splitDelimitedLine(line: string): string[] {
  if (line.includes(",")) return line.split(",").map((value) => value.trim());
  if (line.includes(";")) return line.split(";").map((value) => value.trim());
  if (line.includes("\t")) return line.split("\t").map((value) => value.trim());
  return line.split(/\s+/).map((value) => value.trim());
}

function findHeader(columns: string[], names: string[]): number {
  return columns.findIndex((value) => names.includes(value.replace(/[\s-]/g, "_")));
}

function stripLuaComments(value: string): string {
  return value.replace(/--\[\[[\s\S]*?\]\]/g, "").replace(/--[^\r\n]*/g, "");
}

function fallbackImportName(fileName: string): string {
  return fileName.replace(/\.[^.]+$/, "").trim().slice(0, 64) || "Imported Recoil Script";
}

function firstDefined(record: Record<string, unknown>, keys: string[]): unknown {
  for (const key of keys) {
    if (record[key] !== undefined && record[key] !== null) return record[key];
  }
  return undefined;
}

function readString(record: Record<string, unknown>, keys: string[]): string | undefined {
  const value = firstDefined(record, keys);
  return typeof value === "string" ? value.trim().slice(0, 64) : undefined;
}

function readNonEmptyString(record: Record<string, unknown>, keys: string[]): string | undefined {
  const value = readString(record, keys);
  return value ? value : undefined;
}

function joinWarnings(...warnings: Array<string | undefined>): string | undefined {
  const values = warnings.filter((value): value is string => Boolean(value));
  return values.length ? values.join(" ") : undefined;
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
