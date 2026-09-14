use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use auto_clicker_core::platform::windows::{QpcClock, WindowsInput};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LBUTTON, VK_RBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::GetAsyncKeyState;

const RECOIL_SCHEMA_VERSION: u32 = 1;
const MAX_GAMES: usize = 64;
const MAX_PRESETS_PER_GAME: usize = 256;
const MAX_PATTERN_STEPS: usize = 512;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoilStep {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoilPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub character_name: String,
    pub weapon_name: String,
    pub slot: u8,
    pub enabled: bool,
    pub vertical: f64,
    pub horizontal: f64,
    pub rpm: f64,
    pub activation_mode: String,
    pub activation_hotkey: String,
    pub pattern: Vec<RecoilStep>,
}

impl RecoilPreset {
    fn default_primary() -> Self {
        Self {
            id: "default-primary".into(),
            name: "Primary".into(),
            character_name: String::new(),
            weapon_name: "Custom".into(),
            slot: 1,
            enabled: true,
            vertical: 1.0,
            horizontal: 1.0,
            rpm: 600.0,
            activation_mode: "ads-fire".into(),
            activation_hotkey: "F1".into(),
            pattern: vec![RecoilStep { x: 0, y: 4 }],
        }
    }

    fn default_secondary() -> Self {
        Self {
            id: "default-secondary".into(),
            name: "Secondary".into(),
            character_name: String::new(),
            weapon_name: "Custom".into(),
            slot: 2,
            enabled: true,
            vertical: 1.0,
            horizontal: 1.0,
            rpm: 450.0,
            activation_mode: "ads-fire".into(),
            activation_hotkey: "F2".into(),
            pattern: vec![RecoilStep { x: 0, y: 3 }],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoilGameProfile {
    pub id: String,
    pub name: String,
    pub process_names: Vec<String>,
    pub auto_switch: bool,
    pub active_primary_id: String,
    pub active_secondary_id: String,
    pub presets: Vec<RecoilPreset>,
}

impl RecoilGameProfile {
    fn default_game() -> Self {
        Self {
            id: "generic".into(),
            name: "Generic FPS".into(),
            process_names: Vec::new(),
            auto_switch: false,
            active_primary_id: "default-primary".into(),
            active_secondary_id: "default-secondary".into(),
            presets: vec![
                RecoilPreset::default_primary(),
                RecoilPreset::default_secondary(),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoilDocument {
    pub schema_version: u32,
    pub active_game_id: String,
    pub active_slot: u8,
    pub games: Vec<RecoilGameProfile>,
}

impl Default for RecoilDocument {
    fn default() -> Self {
        Self {
            schema_version: RECOIL_SCHEMA_VERSION,
            active_game_id: "generic".into(),
            active_slot: 1,
            games: vec![RecoilGameProfile::default_game()],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RecoilSnapshot {
    pub document: RecoilDocument,
    pub running: bool,
    pub active_preset_id: Option<String>,
}

#[derive(Clone)]
pub struct RecoilState {
    document: Arc<Mutex<RecoilDocument>>,
    path: PathBuf,
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    active_preset_id: Arc<Mutex<Option<String>>>,
}

impl RecoilState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("failed to resolve config directory: {error}"))?;
        let path = dir.join("recoil.json");
        let mut document = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            serde_json::from_slice::<RecoilDocument>(&bytes)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
        } else {
            RecoilDocument::default()
        };
        migrate_and_validate(&mut document)?;
        let state = Self {
            document: Arc::new(Mutex::new(document)),
            path,
            running: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            active_preset_id: Arc::new(Mutex::new(None)),
        };
        state.persist()?;
        Ok(state)
    }

    fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let document = self
            .document
            .lock()
            .map_err(|_| "recoil state mutex poisoned".to_string())?
            .clone();
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("failed to serialize recoil config: {error}"))?;
        fs::write(&self.path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }
}

#[tauri::command]
pub fn recoil_snapshot(state: State<'_, RecoilState>) -> Result<RecoilSnapshot, String> {
    Ok(RecoilSnapshot {
        document: state
            .document
            .lock()
            .map_err(|_| "recoil state mutex poisoned".to_string())?
            .clone(),
        running: state.running.load(Ordering::Acquire),
        active_preset_id: state
            .active_preset_id
            .lock()
            .map_err(|_| "recoil active preset mutex poisoned".to_string())?
            .clone(),
    })
}

#[tauri::command]
pub fn create_recoil_game(
    name: String,
    process_name: Option<String>,
    state: State<'_, RecoilState>,
) -> Result<RecoilGameProfile, String> {
    let name = clean_name(&name, "game name")?;
    let id = make_id(&name, "game");
    let process_names = process_name
        .filter(|value| !value.trim().is_empty())
        .map(|value| vec![normalize_process_name(&value)])
        .unwrap_or_default();
    let primary_id = format!("{id}-primary");
    let secondary_id = format!("{id}-secondary");
    let mut primary = RecoilPreset::default_primary();
    primary.id = primary_id.clone();
    let mut secondary = RecoilPreset::default_secondary();
    secondary.id = secondary_id.clone();
    let game = RecoilGameProfile {
        id: id.clone(),
        name,
        process_names,
        auto_switch: false,
        active_primary_id: primary_id,
        active_secondary_id: secondary_id,
        presets: vec![primary, secondary],
    };
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "recoil state mutex poisoned".to_string())?;
        if document.games.len() >= MAX_GAMES {
            return Err(format!("recoil supports at most {MAX_GAMES} game profiles"));
        }
        document.games.push(game.clone());
        document.active_game_id = id;
    }
    state.persist()?;
    Ok(game)
}

#[tauri::command]
pub fn save_recoil_game(
    game: RecoilGameProfile,
    state: State<'_, RecoilState>,
) -> Result<RecoilGameProfile, String> {
    validate_game(&game)?;
    let mut normalized = game;
    normalized.process_names = normalized
        .process_names
        .iter()
        .map(|value| normalize_process_name(value))
        .filter(|value| !value.is_empty())
        .collect();
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "recoil state mutex poisoned".to_string())?;
        let target = document
            .games
            .iter_mut()
            .find(|item| item.id == normalized.id)
            .ok_or_else(|| format!("unknown recoil game '{}'", normalized.id))?;
        *target = normalized.clone();
    }
    state.persist()?;
    Ok(normalized)
}

#[tauri::command]
pub fn activate_recoil_game(id: String, state: State<'_, RecoilState>) -> Result<(), String> {
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "recoil state mutex poisoned".to_string())?;
        if !document.games.iter().any(|game| game.id == id) {
            return Err(format!("unknown recoil game '{id}'"));
        }
        document.active_game_id = id;
    }
    state.persist()
}

#[tauri::command]
pub fn set_recoil_slot(slot: u8, state: State<'_, RecoilState>) -> Result<(), String> {
    if !matches!(slot, 1 | 2) {
        return Err("recoil slot must be 1 or 2".into());
    }
    state
        .document
        .lock()
        .map_err(|_| "recoil state mutex poisoned".to_string())?
        .active_slot = slot;
    state.persist()
}

#[tauri::command]
pub fn save_recoil_preset(
    game_id: String,
    preset: RecoilPreset,
    state: State<'_, RecoilState>,
) -> Result<RecoilPreset, String> {
    validate_preset(&preset)?;
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "recoil state mutex poisoned".to_string())?;
        let game = document
            .games
            .iter_mut()
            .find(|game| game.id == game_id)
            .ok_or_else(|| format!("unknown recoil game '{game_id}'"))?;
        if let Some(existing) = game.presets.iter_mut().find(|item| item.id == preset.id) {
            *existing = preset.clone();
        } else {
            if game.presets.len() >= MAX_PRESETS_PER_GAME {
                return Err(format!(
                    "a recoil game supports at most {MAX_PRESETS_PER_GAME} presets"
                ));
            }
            game.presets.push(preset.clone());
        }
        if preset.slot == 1 {
            game.active_primary_id = preset.id.clone();
        } else {
            game.active_secondary_id = preset.id.clone();
        }
    }
    state.persist()?;
    Ok(preset)
}

#[tauri::command]
pub fn recoil_start(state: State<'_, RecoilState>) -> Result<(), String> {
    if state.running.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    state.stop.store(false, Ordering::Release);
    let preset = active_preset(&state)?;
    if !preset.enabled {
        state.running.store(false, Ordering::Release);
        return Err("the active recoil preset is disabled".into());
    }
    let stop = Arc::clone(&state.stop);
    let running = Arc::clone(&state.running);
    let active_preset_id = Arc::clone(&state.active_preset_id);
    *active_preset_id
        .lock()
        .map_err(|_| "recoil active preset mutex poisoned".to_string())? = Some(preset.id.clone());

    thread::Builder::new()
        .name("vxclick-recoil-worker".into())
        .spawn(move || {
            let _ = run_recoil_worker(&preset, &stop);
            if let Ok(mut active) = active_preset_id.lock() {
                *active = None;
            }
            running.store(false, Ordering::Release);
        })
        .map_err(|error| {
            state.running.store(false, Ordering::Release);
            format!("failed to start recoil worker: {error}")
        })?;
    Ok(())
}

#[tauri::command]
pub fn recoil_stop(state: State<'_, RecoilState>) {
    state.stop.store(true, Ordering::Release);
}

fn active_preset(state: &RecoilState) -> Result<RecoilPreset, String> {
    let document = state
        .document
        .lock()
        .map_err(|_| "recoil state mutex poisoned".to_string())?;
    let game = document
        .games
        .iter()
        .find(|game| game.id == document.active_game_id)
        .ok_or_else(|| "active recoil game is missing".to_string())?;
    let preset_id = if document.active_slot == 2 {
        &game.active_secondary_id
    } else {
        &game.active_primary_id
    };
    game.presets
        .iter()
        .find(|preset| &preset.id == preset_id)
        .cloned()
        .ok_or_else(|| "active recoil preset is missing".to_string())
}

fn run_recoil_worker(preset: &RecoilPreset, stop: &AtomicBool) -> Result<(), String> {
    let clock = QpcClock::new()?;
    let input = WindowsInput;
    let frequency = clock.frequency() as f64;
    let interval_ticks = frequency * (60.0 / preset.rpm);
    let mut deadline = clock.now_ticks() as f64;
    let mut step = 0_usize;

    while !stop.load(Ordering::Acquire) {
        if !activation_active(&preset.activation_mode) {
            step = 0;
            thread::sleep(Duration::from_millis(1));
            deadline = clock.now_ticks() as f64;
            continue;
        }

        while !stop.load(Ordering::Acquire) && clock.now_ticks() < deadline.round() as i64 {
            let remaining_ticks = deadline.round() as i64 - clock.now_ticks();
            let remaining_us = clock.ticks_to_micros(remaining_ticks);
            if remaining_us > 1_500.0 {
                thread::sleep(Duration::from_micros(500));
            } else if remaining_us > 250.0 {
                thread::yield_now();
            } else {
                std::hint::spin_loop();
            }
        }
        if stop.load(Ordering::Acquire) || !activation_active(&preset.activation_mode) {
            continue;
        }

        let pattern = &preset.pattern;
        let point = &pattern[step.min(pattern.len() - 1)];
        let dx = (point.x as f64 * preset.horizontal).round() as i32;
        let dy = (point.y as f64 * preset.vertical).round() as i32;
        if dx != 0 || dy != 0 {
            input.move_relative(dx, dy)?;
        }
        step = step.saturating_add(1);
        deadline += interval_ticks;

        let now = clock.now_ticks() as f64;
        if now > deadline + interval_ticks * 3.0 {
            deadline = now + interval_ticks;
        }
    }
    Ok(())
}

fn activation_active(mode: &str) -> bool {
    unsafe {
        match mode {
            "always" => true,
            "fire" => GetAsyncKeyState(VK_LBUTTON as i32) < 0,
            _ => GetAsyncKeyState(VK_LBUTTON as i32) < 0 && GetAsyncKeyState(VK_RBUTTON as i32) < 0,
        }
    }
}

fn migrate_and_validate(document: &mut RecoilDocument) -> Result<(), String> {
    if document.schema_version > RECOIL_SCHEMA_VERSION {
        return Err(format!(
            "recoil schema {} is newer than supported schema {}",
            document.schema_version, RECOIL_SCHEMA_VERSION
        ));
    }
    document.schema_version = RECOIL_SCHEMA_VERSION;
    if document.games.is_empty() {
        document.games.push(RecoilGameProfile::default_game());
    }
    for game in &document.games {
        validate_game(game)?;
    }
    if !document
        .games
        .iter()
        .any(|game| game.id == document.active_game_id)
    {
        document.active_game_id = document.games[0].id.clone();
    }
    if !matches!(document.active_slot, 1 | 2) {
        document.active_slot = 1;
    }
    Ok(())
}

fn validate_game(game: &RecoilGameProfile) -> Result<(), String> {
    if game.id.trim().is_empty() || game.id.len() > 96 {
        return Err("invalid recoil game id".into());
    }
    clean_name(&game.name, "game name")?;
    if game.presets.is_empty() || game.presets.len() > MAX_PRESETS_PER_GAME {
        return Err("recoil game must contain 1-256 presets".into());
    }
    for preset in &game.presets {
        validate_preset(preset)?;
    }
    if !game
        .presets
        .iter()
        .any(|preset| preset.id == game.active_primary_id)
    {
        return Err("active primary recoil preset is missing".into());
    }
    if !game
        .presets
        .iter()
        .any(|preset| preset.id == game.active_secondary_id)
    {
        return Err("active secondary recoil preset is missing".into());
    }
    Ok(())
}

fn validate_preset(preset: &RecoilPreset) -> Result<(), String> {
    if preset.id.trim().is_empty() || preset.id.len() > 128 {
        return Err("invalid recoil preset id".into());
    }
    clean_name(&preset.name, "preset name")?;
    if preset.character_name.len() > 64 {
        return Err("character/operator name cannot exceed 64 characters".into());
    }
    clean_name(&preset.weapon_name, "weapon name")?;
    if !matches!(preset.slot, 1 | 2) {
        return Err("recoil preset slot must be 1 or 2".into());
    }
    if !preset.rpm.is_finite() || !(30.0..=3_000.0).contains(&preset.rpm) {
        return Err("recoil RPM must be between 30 and 3,000".into());
    }
    if !preset.vertical.is_finite() || !(0.0..=10.0).contains(&preset.vertical) {
        return Err("vertical multiplier must be between 0 and 10".into());
    }
    if !preset.horizontal.is_finite() || !(0.0..=10.0).contains(&preset.horizontal) {
        return Err("horizontal multiplier must be between 0 and 10".into());
    }
    if !matches!(
        preset.activation_mode.as_str(),
        "ads-fire" | "fire" | "always"
    ) {
        return Err("activation mode must be ads-fire, fire, or always".into());
    }
    if preset.pattern.is_empty() || preset.pattern.len() > MAX_PATTERN_STEPS {
        return Err(format!(
            "recoil pattern must contain 1-{MAX_PATTERN_STEPS} steps"
        ));
    }
    Ok(())
}

fn clean_name(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 64 {
        return Err(format!("{field} must contain 1-64 characters"));
    }
    Ok(value.to_string())
}

fn make_id(name: &str, prefix: &str) -> String {
    let slug: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!(
        "{prefix}-{}-{millis}",
        if slug.is_empty() { "custom" } else { &slug }
    )
}

fn normalize_process_name(value: &str) -> String {
    std::path::Path::new(value.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value.trim())
        .to_ascii_lowercase()
}
