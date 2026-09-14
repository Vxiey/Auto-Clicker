use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::hotkeys::HotkeyBinding;
use auto_clicker_core::platform::windows::{HotkeyPhase, RegisteredHotkey, WindowsInput};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::macros::MacroState;

const REMAP_SCHEMA_VERSION: u32 = 1;
pub const REMAP_HOTKEY_BASE: u64 = 50_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredRemap {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub trigger: String,
    pub action: String,
    #[serde(default)]
    pub process: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub consume: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemapDocument {
    pub schema_version: u32,
    pub mappings: Vec<StoredRemap>,
}

impl Default for RemapDocument {
    fn default() -> Self {
        Self {
            schema_version: REMAP_SCHEMA_VERSION,
            mappings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeRemap {
    pub hotkey: RegisteredHotkey,
    pub mapping_id: String,
}

#[derive(Clone)]
pub struct RemapState {
    document: Arc<Mutex<RemapDocument>>,
    path: PathBuf,
}

impl RemapState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("failed to resolve config directory: {error}"))?;
        let path = dir.join("remaps.json");
        let mut document = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            serde_json::from_slice::<RemapDocument>(&bytes)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
        } else {
            RemapDocument::default()
        };
        document.schema_version = REMAP_SCHEMA_VERSION;
        for mapping in &document.mappings {
            validate_remap(mapping)?;
        }
        let state = Self {
            document: Arc::new(Mutex::new(document)),
            path,
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
            .map_err(|_| "remap document mutex poisoned".to_string())?
            .clone();
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("failed to serialize remaps: {error}"))?;
        fs::write(&self.path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }

    pub fn runtime_bindings(
        &self,
        foreground_process: Option<&str>,
    ) -> Result<Vec<RuntimeRemap>, String> {
        let foreground = foreground_process.map(normalize_process_name);
        let mappings = self
            .document
            .lock()
            .map_err(|_| "remap document mutex poisoned".to_string())?
            .mappings
            .clone();
        let mut result = Vec::new();
        for (index, mapping) in mappings
            .into_iter()
            .filter(|mapping| mapping.enabled)
            .enumerate()
        {
            if !mapping.process.trim().is_empty()
                && foreground.as_deref() != Some(normalize_process_name(&mapping.process).as_str())
            {
                continue;
            }
            let binding = HotkeyBinding::parse(&mapping.trigger)?.with_consume(mapping.consume);
            result.push(RuntimeRemap {
                hotkey: RegisteredHotkey {
                    id: REMAP_HOTKEY_BASE + index as u64,
                    binding,
                },
                mapping_id: mapping.id,
            });
        }
        Ok(result)
    }

    pub fn execute(
        &self,
        mapping_id: &str,
        phase: HotkeyPhase,
        macros: &MacroState,
    ) -> Result<(), String> {
        let mapping = self
            .document
            .lock()
            .map_err(|_| "remap document mutex poisoned".to_string())?
            .mappings
            .iter()
            .find(|mapping| mapping.id == mapping_id)
            .cloned()
            .ok_or_else(|| format!("unknown remap '{mapping_id}'"))?;
        execute_action(&mapping.action, phase, macros)
    }
}

#[tauri::command]
pub fn remaps_snapshot(state: State<'_, RemapState>) -> Result<RemapDocument, String> {
    state
        .document
        .lock()
        .map_err(|_| "remap document mutex poisoned".to_string())
        .map(|document| document.clone())
}

#[tauri::command]
pub fn save_remap(
    mut mapping: StoredRemap,
    state: State<'_, RemapState>,
) -> Result<StoredRemap, String> {
    if mapping.id.trim().is_empty() {
        mapping.id = make_remap_id(&mapping.name);
    }
    validate_remap(&mapping)?;
    mapping.process = normalize_optional_process(&mapping.process);
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "remap document mutex poisoned".to_string())?;
        if let Some(existing) = document
            .mappings
            .iter_mut()
            .find(|item| item.id == mapping.id)
        {
            *existing = mapping.clone();
        } else {
            document.mappings.push(mapping.clone());
        }
    }
    state.persist()?;
    Ok(mapping)
}

#[tauri::command]
pub fn delete_remap(id: String, state: State<'_, RemapState>) -> Result<(), String> {
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "remap document mutex poisoned".to_string())?;
        let before = document.mappings.len();
        document.mappings.retain(|mapping| mapping.id != id);
        if before == document.mappings.len() {
            return Err(format!("unknown remap '{id}'"));
        }
    }
    state.persist()
}

fn execute_action(action: &str, phase: HotkeyPhase, macros: &MacroState) -> Result<(), String> {
    let action = action.trim();
    if let Some(macro_id) = action
        .strip_prefix("Macro:")
        .or_else(|| action.strip_prefix("macro:"))
    {
        return macros.handle_hotkey(macro_id.trim(), phase);
    }

    if let Some(button) = parse_mouse_action(action) {
        return match phase {
            HotkeyPhase::Pressed => WindowsInput.mouse_down(button),
            HotkeyPhase::Released => WindowsInput.mouse_up(button),
        };
    }

    let keys = action
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            key_name_to_vk(part).ok_or_else(|| format!("unsupported remap action key '{part}'"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Err("remap action cannot be empty".into());
    }
    match phase {
        HotkeyPhase::Pressed => {
            for key in keys {
                WindowsInput.key_down(key)?;
            }
        }
        HotkeyPhase::Released => {
            for key in keys.into_iter().rev() {
                WindowsInput.key_up(key)?;
            }
        }
    }
    Ok(())
}

fn validate_remap(mapping: &StoredRemap) -> Result<(), String> {
    let name = mapping.name.trim();
    if name.is_empty() || name.len() > 96 {
        return Err("remap name must contain 1-96 characters".into());
    }
    HotkeyBinding::parse(&mapping.trigger)?;
    if mapping.action.trim().is_empty() {
        return Err("remap action cannot be empty".into());
    }
    if let Some(macro_id) = mapping
        .action
        .strip_prefix("Macro:")
        .or_else(|| mapping.action.strip_prefix("macro:"))
    {
        if macro_id.trim().is_empty() {
            return Err("macro remap action must include a macro id".into());
        }
        return Ok(());
    }
    if parse_mouse_action(&mapping.action).is_some() {
        return Ok(());
    }
    for key in mapping
        .action
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if key_name_to_vk(key).is_none() {
            return Err(format!("unsupported remap action key '{key}'"));
        }
    }
    Ok(())
}

fn parse_mouse_action(value: &str) -> Option<MouseButton> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" | "left click" => Some(MouseButton::Left),
        "right" | "right click" => Some(MouseButton::Right),
        "middle" | "middle click" => Some(MouseButton::Middle),
        "mouse 4" | "x1" => Some(MouseButton::X1),
        "mouse 5" | "x2" => Some(MouseButton::X2),
        _ => None,
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
    if let Some(number) = upper
        .strip_prefix('F')
        .and_then(|value| value.parse::<u16>().ok())
    {
        if (1..=24).contains(&number) {
            return Some(0x70 + number - 1);
        }
    }
    match upper.as_str() {
        "SPACE" => Some(0x20),
        "ENTER" | "RETURN" => Some(0x0D),
        "TAB" => Some(0x09),
        "ESC" | "ESCAPE" => Some(0x1B),
        "SHIFT" | "LSHIFT" => Some(0xA0),
        "RSHIFT" => Some(0xA1),
        "CTRL" | "CONTROL" | "LCTRL" => Some(0xA2),
        "RCTRL" => Some(0xA3),
        "ALT" | "LALT" => Some(0xA4),
        "RALT" => Some(0xA5),
        "CAPS LOCK" | "CAPSLOCK" => Some(0x14),
        "LEFT" | "ARROW LEFT" => Some(0x25),
        "UP" | "ARROW UP" => Some(0x26),
        "RIGHT" | "ARROW RIGHT" => Some(0x27),
        "DOWN" | "ARROW DOWN" => Some(0x28),
        _ => None,
    }
}

fn normalize_process_name(value: &str) -> String {
    Path::new(value.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value.trim())
        .to_ascii_lowercase()
}

fn normalize_optional_process(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        normalize_process_name(value)
    }
}

fn make_remap_id(name: &str) -> String {
    let slug = name
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
    format!("{}-{millis}", if slug.is_empty() { "remap" } else { &slug })
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{StoredRemap, key_name_to_vk, validate_remap};

    #[test]
    fn validates_key_and_mouse_actions() {
        for action in ["F", "Ctrl+C", "Mouse 5"] {
            let mapping = StoredRemap {
                id: "test".into(),
                name: "Test".into(),
                trigger: "F6".into(),
                action: action.into(),
                process: String::new(),
                enabled: true,
                consume: true,
            };
            assert!(validate_remap(&mapping).is_ok(), "{action}");
        }
    }

    #[test]
    fn key_parser_supports_right_modifiers() {
        assert_eq!(key_name_to_vk("RCTRL"), Some(0xA3));
        assert_eq!(key_name_to_vk("F12"), Some(0x7B));
    }
}
