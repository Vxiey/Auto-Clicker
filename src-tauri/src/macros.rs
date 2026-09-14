use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::lua_runtime::LuaMacroCompiler;
use auto_clicker_core::macro_engine::InputAction;
use auto_clicker_core::platform::windows::{
    CapturedInput, CapturedInputKind, HotkeyPhase, MacroPlayer, QpcClock,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const MACRO_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroEventRecord {
    pub id: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub label: String,
    #[serde(default)]
    pub delay_ms: Option<u64>,
    #[serde(default = "default_lane")]
    pub lane: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMacro {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub trigger: String,
    #[serde(rename = "macroType")]
    pub macro_type: String,
    #[serde(rename = "repeatDelayMs", default)]
    pub repeat_delay_ms: u64,
    #[serde(default = "default_speed")]
    pub speed: f64,
    pub events: Vec<MacroEventRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroDocument {
    pub schema_version: u32,
    pub macros: Vec<StoredMacro>,
}

impl Default for MacroDocument {
    fn default() -> Self {
        Self {
            schema_version: MACRO_SCHEMA_VERSION,
            macros: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MacroSnapshot {
    pub document: MacroDocument,
    pub recording: bool,
    pub playing_macro_id: Option<String>,
}

#[derive(Debug)]
struct PlaybackTask {
    macro_id: String,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

#[derive(Debug)]
struct RecordingSession {
    active: bool,
    record_delays: bool,
    standard_delay_ms: Option<u64>,
    lane: String,
    last_ticks: Option<i64>,
    next_id: u64,
    events: Vec<MacroEventRecord>,
}

impl Default for RecordingSession {
    fn default() -> Self {
        Self {
            active: false,
            record_delays: true,
            standard_delay_ms: None,
            lane: default_lane(),
            last_ticks: None,
            next_id: 1,
            events: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct MacroState {
    document: Arc<Mutex<MacroDocument>>,
    path: PathBuf,
    playback: Arc<Mutex<Option<PlaybackTask>>>,
    recording: Arc<Mutex<RecordingSession>>,
    clock: QpcClock,
}

impl MacroState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("failed to resolve config directory: {error}"))?;
        let path = dir.join("macros.json");
        let mut document = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            serde_json::from_slice::<MacroDocument>(&bytes)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
        } else {
            MacroDocument::default()
        };
        document.schema_version = MACRO_SCHEMA_VERSION;
        for macro_def in &document.macros {
            validate_macro(macro_def)?;
        }
        let state = Self {
            document: Arc::new(Mutex::new(document)),
            path,
            playback: Arc::new(Mutex::new(None)),
            recording: Arc::new(Mutex::new(RecordingSession::default())),
            clock: QpcClock::new()?,
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
            .map_err(|_| "macro document mutex poisoned".to_string())?
            .clone();
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("failed to serialize macros: {error}"))?;
        fs::write(&self.path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }

    pub fn runtime_macros(&self) -> Result<Vec<StoredMacro>, String> {
        Ok(self
            .document
            .lock()
            .map_err(|_| "macro document mutex poisoned".to_string())?
            .macros
            .clone())
    }

    pub fn capture(&self, input: &CapturedInput) {
        let Ok(mut session) = self.recording.lock() else {
            return;
        };
        if !session.active {
            return;
        }

        if session.record_delays {
            if let Some(last) = session.last_ticks {
                let elapsed = self.clock.ticks_to_micros(input.qpc_ticks.saturating_sub(last));
                let delay_ms = session
                    .standard_delay_ms
                    .unwrap_or_else(|| (elapsed.max(0.0) / 1_000.0).round() as u64);
                if delay_ms > 0 {
                    let id = next_record_id(&mut session);
                    let lane = session.lane.clone();
                    session.events.push(MacroEventRecord {
                        id,
                        event_type: "delay".into(),
                        label: format!("{delay_ms} ms"),
                        delay_ms: Some(delay_ms.min(60_000)),
                        lane,
                    });
                }
            }
        }

        let converted = captured_to_record(input, session.next_id, &session.lane);
        if let Some(mut record) = converted {
            session.next_id = session.next_id.saturating_add(1);
            record.id = record.id.max(1);
            session.events.push(record);
        }
        session.last_ticks = Some(input.qpc_ticks);
    }

    pub fn handle_hotkey(&self, macro_id: &str, phase: HotkeyPhase) -> Result<(), String> {
        let macro_def = self
            .runtime_macros()?
            .into_iter()
            .find(|item| item.id == macro_id)
            .ok_or_else(|| format!("unknown macro '{macro_id}'"))?;

        match (macro_def.macro_type.as_str(), phase) {
            ("repeat-hold", HotkeyPhase::Pressed) => self.start_macro_worker(macro_def, true, "main"),
            ("repeat-hold", HotkeyPhase::Released) => self.stop_playback(),
            ("toggle", HotkeyPhase::Pressed) => {
                if self.is_playing(macro_id) {
                    self.stop_playback()
                } else {
                    self.start_macro_worker(macro_def, true, "main")
                }
            }
            ("sequence", HotkeyPhase::Pressed) => self.start_sequence_press(macro_def),
            ("sequence", HotkeyPhase::Released) => {
                self.stop_playback()?;
                self.start_macro_worker(macro_def, false, "on-release")
            }
            (_, HotkeyPhase::Pressed) => self.start_macro_worker(macro_def, false, "main"),
            _ => Ok(()),
        }
    }

    fn start_sequence_press(&self, macro_def: StoredMacro) -> Result<(), String> {
        self.stop_playback()?;
        let on_press = actions_for_lane(&macro_def, "on-press")?;
        let holding = actions_for_lane(&macro_def, "while-holding")?;
        let repeat_delay = macro_def.repeat_delay_ms;
        let speed = macro_def.speed;
        let macro_id = macro_def.id.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("vxclick-sequence-macro".into())
            .spawn(move || {
                let Ok(player) = MacroPlayer::new() else {
                    return;
                };
                if player.play(&on_press, &worker_stop, speed).is_err() {
                    return;
                }
                while !worker_stop.load(Ordering::Acquire) {
                    if !holding.is_empty() && player.play(&holding, &worker_stop, speed).is_err() {
                        break;
                    }
                    if repeat_delay > 0 && !sleep_interruptible(repeat_delay, &worker_stop) {
                        break;
                    }
                    if holding.is_empty() {
                        break;
                    }
                }
            })
            .map_err(|error| format!("failed to start sequence macro: {error}"))?;
        *self
            .playback
            .lock()
            .map_err(|_| "macro playback mutex poisoned".to_string())? = Some(PlaybackTask {
            macro_id,
            stop,
            thread,
        });
        Ok(())
    }

    fn start_macro_worker(
        &self,
        macro_def: StoredMacro,
        repeat: bool,
        lane: &str,
    ) -> Result<(), String> {
        self.stop_playback()?;
        let actions = actions_for_lane(&macro_def, lane)?;
        let repeat_delay = macro_def.repeat_delay_ms;
        let speed = macro_def.speed;
        let macro_id = macro_def.id.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("vxclick-macro-player".into())
            .spawn(move || {
                let Ok(player) = MacroPlayer::new() else {
                    return;
                };
                loop {
                    if player.play(&actions, &worker_stop, speed).is_err() || !repeat {
                        break;
                    }
                    if repeat_delay > 0 && !sleep_interruptible(repeat_delay, &worker_stop) {
                        break;
                    }
                    if worker_stop.load(Ordering::Acquire) {
                        break;
                    }
                }
            })
            .map_err(|error| format!("failed to start macro playback: {error}"))?;
        *self
            .playback
            .lock()
            .map_err(|_| "macro playback mutex poisoned".to_string())? = Some(PlaybackTask {
            macro_id,
            stop,
            thread,
        });
        Ok(())
    }

    pub fn stop_playback(&self) -> Result<(), String> {
        let task = self
            .playback
            .lock()
            .map_err(|_| "macro playback mutex poisoned".to_string())?
            .take();
        if let Some(task) = task {
            task.stop.store(true, Ordering::Release);
            let _ = task.thread.join();
        }
        Ok(())
    }

    fn is_playing(&self, macro_id: &str) -> bool {
        self.playback
            .lock()
            .ok()
            .and_then(|task| task.as_ref().map(|task| task.macro_id == macro_id))
            .unwrap_or(false)
    }
}

fn next_record_id(session: &mut RecordingSession) -> u64 {
    let id = session.next_id;
    session.next_id = session.next_id.saturating_add(1);
    id
}

fn sleep_interruptible(milliseconds: u64, stop: &AtomicBool) -> bool {
    let mut remaining = milliseconds;
    while remaining > 0 {
        if stop.load(Ordering::Acquire) {
            return false;
        }
        let slice = remaining.min(10);
        thread::sleep(std::time::Duration::from_millis(slice));
        remaining -= slice;
    }
    true
}

fn captured_to_record(input: &CapturedInput, id: u64, lane: &str) -> Option<MacroEventRecord> {
    let (event_type, label) = match input.kind {
        CapturedInputKind::KeyDown { virtual_key, .. } => ("key-down", vk_to_label(virtual_key)),
        CapturedInputKind::KeyUp { virtual_key, .. } => ("key-up", vk_to_label(virtual_key)),
        CapturedInputKind::MouseDown(button) => ("mouse", format!("{} Down", button_label(button))),
        CapturedInputKind::MouseUp(button) => ("mouse", format!("{} Up", button_label(button))),
        CapturedInputKind::MouseWheel { delta } => ("mouse", format!("Wheel {delta}")),
        CapturedInputKind::MouseMove { .. } => return None,
    };
    Some(MacroEventRecord {
        id,
        event_type: event_type.into(),
        label,
        delay_ms: None,
        lane: lane.to_string(),
    })
}

fn actions_for_lane(macro_def: &StoredMacro, lane: &str) -> Result<Vec<InputAction>, String> {
    let mut actions = Vec::new();
    for event in macro_def.events.iter().filter(|event| event.lane == lane) {
        actions.extend(event_to_actions(event)?);
    }
    Ok(actions)
}

fn event_to_actions(event: &MacroEventRecord) -> Result<Vec<InputAction>, String> {
    match event.event_type.as_str() {
        "delay" => Ok(vec![InputAction::WaitMicros(
            event.delay_ms.unwrap_or(0).min(60_000).saturating_mul(1_000),
        )]),
        "key-down" => key_actions(&event.label, true),
        "key-up" => key_actions(&event.label, false),
        "mouse" => mouse_actions(&event.label),
        other => Err(format!("unsupported macro event type '{other}'")),
    }
}

fn key_actions(label: &str, down: bool) -> Result<Vec<InputAction>, String> {
    let command = label.trim().to_ascii_lowercase();
    let command_keys: Option<Vec<u16>> = match command.as_str() {
        "copy" => Some(vec![0xA2, b'C' as u16]),
        "paste" => Some(vec![0xA2, b'V' as u16]),
        "undo" => Some(vec![0xA2, b'Z' as u16]),
        "redo" => Some(vec![0xA2, b'Y' as u16]),
        _ => None,
    };
    if let Some(keys) = command_keys {
        return Ok(if down {
            keys.into_iter().map(InputAction::KeyDown).collect()
        } else {
            keys.into_iter().rev().map(InputAction::KeyUp).collect()
        });
    }
    let key = key_name_to_vk(label).ok_or_else(|| format!("unsupported macro key '{label}'"))?;
    Ok(vec![if down {
        InputAction::KeyDown(key)
    } else {
        InputAction::KeyUp(key)
    }])
}

fn mouse_actions(label: &str) -> Result<Vec<InputAction>, String> {
    let lower = label.trim().to_ascii_lowercase();
    if let Some(delta) = lower.strip_prefix("wheel ") {
        let delta = delta
            .parse::<i32>()
            .map_err(|_| format!("invalid wheel event '{label}'"))?;
        return Ok(vec![InputAction::MouseWheel { delta }]);
    }

    let down = lower.ends_with(" down");
    let up = lower.ends_with(" up");
    let base = lower
        .strip_suffix(" down")
        .or_else(|| lower.strip_suffix(" up"))
        .unwrap_or(&lower);
    let button = parse_button_label(base)
        .ok_or_else(|| format!("unsupported mouse macro event '{label}'"))?;
    if down {
        Ok(vec![InputAction::MouseDown(button)])
    } else if up {
        Ok(vec![InputAction::MouseUp(button)])
    } else {
        Ok(vec![
            InputAction::MouseDown(button),
            InputAction::MouseUp(button),
        ])
    }
}

fn parse_button_label(label: &str) -> Option<MouseButton> {
    match label.trim().to_ascii_lowercase().as_str() {
        "left" | "left click" => Some(MouseButton::Left),
        "right" | "right click" => Some(MouseButton::Right),
        "middle" | "middle click" => Some(MouseButton::Middle),
        "mouse 4" | "x1" => Some(MouseButton::X1),
        "mouse 5" | "x2" => Some(MouseButton::X2),
        _ => None,
    }
}

fn button_label(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "Left",
        MouseButton::Right => "Right",
        MouseButton::Middle => "Middle",
        MouseButton::X1 => "Mouse 4",
        MouseButton::X2 => "Mouse 5",
    }
}

fn key_name_to_vk(label: &str) -> Option<u16> {
    let upper = label.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Some(byte as u16);
        }
    }
    if let Some(number) = upper.strip_prefix('F').and_then(|value| value.parse::<u16>().ok()) {
        if (1..=24).contains(&number) {
            return Some(0x70 + number - 1);
        }
    }
    match upper.as_str() {
        "SPACE" => Some(0x20),
        "ENTER" | "RETURN" => Some(0x0D),
        "TAB" => Some(0x09),
        "ESC" | "ESCAPE" => Some(0x1B),
        "SHIFT" => Some(0xA0),
        "CTRL" | "CONTROL" => Some(0xA2),
        "ALT" => Some(0xA4),
        "CAPS LOCK" | "CAPSLOCK" => Some(0x14),
        "ARROW LEFT" | "LEFT" => Some(0x25),
        "ARROW UP" | "UP" => Some(0x26),
        "ARROW RIGHT" | "RIGHT" => Some(0x27),
        "ARROW DOWN" | "DOWN" => Some(0x28),
        "PLAY / PAUSE" => Some(0xB3),
        "VOLUME UP" => Some(0xAF),
        "VOLUME DOWN" => Some(0xAE),
        _ => None,
    }
}

fn vk_to_label(vk: u16) -> String {
    if (b'A' as u16..=b'Z' as u16).contains(&vk) || (b'0' as u16..=b'9' as u16).contains(&vk) {
        return char::from_u32(vk as u32).unwrap_or('?').to_string();
    }
    if (0x70..=0x87).contains(&vk) {
        return format!("F{}", vk - 0x70 + 1);
    }
    match vk {
        0x20 => "Space".into(),
        0x0D => "Enter".into(),
        0x09 => "Tab".into(),
        0x1B => "Esc".into(),
        0xA0 | 0xA1 => "Shift".into(),
        0xA2 | 0xA3 => "Ctrl".into(),
        0xA4 | 0xA5 => "Alt".into(),
        0x14 => "Caps Lock".into(),
        0x25 => "Arrow Left".into(),
        0x26 => "Arrow Up".into(),
        0x27 => "Arrow Right".into(),
        0x28 => "Arrow Down".into(),
        _ => format!("VK_{vk:02X}"),
    }
}

fn validate_macro(macro_def: &StoredMacro) -> Result<(), String> {
    let name = macro_def.name.trim();
    if name.is_empty() || name.len() > 96 {
        return Err("macro name must contain 1-96 characters".into());
    }
    if macro_def.trigger.trim().is_empty() {
        return Err("macro trigger cannot be empty".into());
    }
    if !matches!(
        macro_def.macro_type.as_str(),
        "no-repeat" | "repeat-hold" | "toggle" | "sequence"
    ) {
        return Err("macro type is invalid".into());
    }
    if !macro_def.speed.is_finite() || !(0.1..=10.0).contains(&macro_def.speed) {
        return Err("macro speed must be between 0.1x and 10x".into());
    }
    if macro_def.events.len() > 100_000 {
        return Err("macro contains too many events".into());
    }
    for event in &macro_def.events {
        if event.delay_ms.unwrap_or(0) > 60_000 {
            return Err("macro delay cannot exceed 60 seconds".into());
        }
        let _ = event_to_actions(event)?;
    }
    Ok(())
}

fn make_macro_id(name: &str) -> String {
    let slug = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{millis}", if slug.is_empty() { "macro" } else { &slug })
}

fn default_lane() -> String {
    "main".into()
}

fn default_speed() -> f64 {
    1.0
}

#[tauri::command]
pub fn macros_snapshot(state: State<'_, MacroState>) -> Result<MacroSnapshot, String> {
    let document = state
        .document
        .lock()
        .map_err(|_| "macro document mutex poisoned".to_string())?
        .clone();
    let recording = state
        .recording
        .lock()
        .map(|recording| recording.active)
        .unwrap_or(false);
    let playing_macro_id = state
        .playback
        .lock()
        .ok()
        .and_then(|task| task.as_ref().map(|task| task.macro_id.clone()));
    Ok(MacroSnapshot {
        document,
        recording,
        playing_macro_id,
    })
}

#[tauri::command]
pub fn save_macro(mut macro_def: StoredMacro, state: State<'_, MacroState>) -> Result<StoredMacro, String> {
    if macro_def.id.trim().is_empty() {
        macro_def.id = make_macro_id(&macro_def.name);
    }
    validate_macro(&macro_def)?;
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "macro document mutex poisoned".to_string())?;
        if let Some(existing) = document.macros.iter_mut().find(|item| item.id == macro_def.id) {
            *existing = macro_def.clone();
        } else {
            document.macros.push(macro_def.clone());
        }
    }
    state.persist()?;
    Ok(macro_def)
}

#[tauri::command]
pub fn delete_macro(id: String, state: State<'_, MacroState>) -> Result<(), String> {
    state.stop_playback()?;
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "macro document mutex poisoned".to_string())?;
        let before = document.macros.len();
        document.macros.retain(|item| item.id != id);
        if before == document.macros.len() {
            return Err(format!("unknown macro '{id}'"));
        }
    }
    state.persist()
}

#[tauri::command]
pub fn play_macro(id: String, state: State<'_, MacroState>) -> Result<(), String> {
    let macro_def = state
        .runtime_macros()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| format!("unknown macro '{id}'"))?;
    state.start_macro_worker(macro_def, false, "main")
}

#[tauri::command]
pub fn stop_macro(state: State<'_, MacroState>) -> Result<(), String> {
    state.stop_playback()
}

#[tauri::command]
pub fn start_macro_recording(
    record_delays: bool,
    standard_delay_ms: Option<u64>,
    lane: String,
    state: State<'_, MacroState>,
) -> Result<(), String> {
    if standard_delay_ms.is_some_and(|delay| delay > 60_000) {
        return Err("standard recording delay cannot exceed 60 seconds".into());
    }
    let mut recording = state
        .recording
        .lock()
        .map_err(|_| "macro recording mutex poisoned".to_string())?;
    if recording.active {
        return Err("macro recording is already active".into());
    }
    *recording = RecordingSession {
        active: true,
        record_delays,
        standard_delay_ms,
        lane: if lane.trim().is_empty() { default_lane() } else { lane },
        last_ticks: None,
        next_id: 1,
        events: Vec::new(),
    };
    Ok(())
}

#[tauri::command]
pub fn stop_macro_recording(state: State<'_, MacroState>) -> Result<Vec<MacroEventRecord>, String> {
    let mut recording = state
        .recording
        .lock()
        .map_err(|_| "macro recording mutex poisoned".to_string())?;
    if !recording.active {
        return Ok(Vec::new());
    }
    recording.active = false;
    recording.last_ticks = None;
    Ok(std::mem::take(&mut recording.events))
}

#[tauri::command]
pub fn validate_lua_script(script: String) -> Result<usize, String> {
    Ok(LuaMacroCompiler::compile(&script)?.len())
}

#[tauri::command]
pub fn run_lua_script(
    script: String,
    speed: f64,
    state: State<'_, MacroState>,
) -> Result<(), String> {
    if !speed.is_finite() || !(0.1..=10.0).contains(&speed) {
        return Err("Lua playback speed must be between 0.1x and 10x".into());
    }
    let actions = LuaMacroCompiler::compile(&script)?;
    state.stop_playback()?;
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let thread = thread::Builder::new()
        .name("vxclick-lua-player".into())
        .spawn(move || {
            if let Ok(player) = MacroPlayer::new() {
                let _ = player.play(&actions, &worker_stop, speed);
            }
        })
        .map_err(|error| format!("failed to start Lua playback: {error}"))?;
    *state
        .playback
        .lock()
        .map_err(|_| "macro playback mutex poisoned".to_string())? = Some(PlaybackTask {
        macro_id: "lua-script".into(),
        stop,
        thread,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MacroEventRecord, StoredMacro, event_to_actions, key_name_to_vk, validate_macro};

    #[test]
    fn key_names_cover_common_macro_keys() {
        assert_eq!(key_name_to_vk("A"), Some(0x41));
        assert_eq!(key_name_to_vk("F12"), Some(0x7B));
        assert_eq!(key_name_to_vk("Space"), Some(0x20));
    }

    #[test]
    fn mouse_click_expands_to_down_and_up() {
        let event = MacroEventRecord {
            id: 1,
            event_type: "mouse".into(),
            label: "Left Click".into(),
            delay_ms: None,
            lane: "main".into(),
        };
        assert_eq!(event_to_actions(&event).expect("mouse event").len(), 2);
    }

    #[test]
    fn rejects_invalid_macro_speed() {
        let macro_def = StoredMacro {
            id: "test".into(),
            name: "Test".into(),
            trigger: "F6".into(),
            macro_type: "no-repeat".into(),
            repeat_delay_ms: 0,
            speed: 0.0,
            events: Vec::new(),
        };
        assert!(validate_macro(&macro_def).is_err());
    }
}
