use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const LUA_SCRIPT_SCHEMA_VERSION: u32 = 1;
const MAX_LUA_SCRIPTS: usize = 256;
const MAX_LUA_SCRIPT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredLuaScript {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub script: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LuaScriptDocument {
    pub schema_version: u32,
    pub scripts: Vec<StoredLuaScript>,
}

impl Default for LuaScriptDocument {
    fn default() -> Self {
        Self {
            schema_version: LUA_SCRIPT_SCHEMA_VERSION,
            scripts: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct LuaScriptState {
    document: Arc<Mutex<LuaScriptDocument>>,
    path: PathBuf,
}

impl LuaScriptState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("failed to resolve config directory: {error}"))?;
        let path = dir.join("lua-scripts.json");
        let mut document = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            serde_json::from_slice::<LuaScriptDocument>(&bytes)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
        } else {
            LuaScriptDocument::default()
        };
        if document.schema_version > LUA_SCRIPT_SCHEMA_VERSION {
            return Err(format!(
                "Lua script schema {} is newer than supported schema {}",
                document.schema_version, LUA_SCRIPT_SCHEMA_VERSION
            ));
        }
        document.schema_version = LUA_SCRIPT_SCHEMA_VERSION;
        if document.scripts.len() > MAX_LUA_SCRIPTS {
            return Err(format!(
                "Lua library supports at most {MAX_LUA_SCRIPTS} scripts"
            ));
        }
        for script in &document.scripts {
            validate_lua_entry(script)?;
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
            .map_err(|_| "Lua script document mutex poisoned".to_string())?
            .clone();
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("failed to serialize Lua library: {error}"))?;
        fs::write(&self.path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }
}

#[tauri::command]
pub fn lua_scripts_snapshot(state: State<'_, LuaScriptState>) -> Result<LuaScriptDocument, String> {
    state
        .document
        .lock()
        .map_err(|_| "Lua script document mutex poisoned".to_string())
        .map(|document| document.clone())
}

#[tauri::command]
pub fn save_lua_script(
    mut script_def: StoredLuaScript,
    state: State<'_, LuaScriptState>,
) -> Result<StoredLuaScript, String> {
    script_def.name = clean_lua_name(&script_def.name)?;
    if script_def.id.trim().is_empty() {
        script_def.id = make_lua_id(&script_def.name);
    }
    validate_lua_entry(&script_def)?;

    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "Lua script document mutex poisoned".to_string())?;
        if let Some(existing) = document
            .scripts
            .iter_mut()
            .find(|item| item.id == script_def.id)
        {
            *existing = script_def.clone();
        } else {
            if document.scripts.len() >= MAX_LUA_SCRIPTS {
                return Err(format!(
                    "Lua library supports at most {MAX_LUA_SCRIPTS} scripts"
                ));
            }
            document.scripts.push(script_def.clone());
        }
    }

    state.persist()?;
    Ok(script_def)
}

#[tauri::command]
pub fn rename_lua_script(
    id: String,
    name: String,
    state: State<'_, LuaScriptState>,
) -> Result<StoredLuaScript, String> {
    let name = clean_lua_name(&name)?;
    let renamed = {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "Lua script document mutex poisoned".to_string())?;
        let script = document
            .scripts
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| format!("unknown Lua script '{id}'"))?;
        script.name = name;
        script.clone()
    };
    state.persist()?;
    Ok(renamed)
}

#[tauri::command]
pub fn duplicate_lua_script(
    id: String,
    state: State<'_, LuaScriptState>,
) -> Result<StoredLuaScript, String> {
    let original = {
        let document = state
            .document
            .lock()
            .map_err(|_| "Lua script document mutex poisoned".to_string())?;
        document
            .scripts
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .ok_or_else(|| format!("unknown Lua script '{id}'"))?
    };
    let mut copy = original;
    copy.name = clean_lua_name(&format!("{} Copy", copy.name))?;
    copy.id = make_lua_id(&copy.name);
    save_lua_script(copy, state)
}

#[tauri::command]
pub fn delete_lua_script(id: String, state: State<'_, LuaScriptState>) -> Result<(), String> {
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "Lua script document mutex poisoned".to_string())?;
        let before = document.scripts.len();
        document.scripts.retain(|item| item.id != id);
        if before == document.scripts.len() {
            return Err(format!("unknown Lua script '{id}'"));
        }
    }
    state.persist()
}

fn validate_lua_entry(script: &StoredLuaScript) -> Result<(), String> {
    if script.id.trim().is_empty() || script.id.len() > 128 {
        return Err("invalid Lua script id".into());
    }
    clean_lua_name(&script.name)?;
    if script.script.len() > MAX_LUA_SCRIPT_BYTES {
        return Err(format!(
            "Lua script cannot exceed {} KiB",
            MAX_LUA_SCRIPT_BYTES / 1024
        ));
    }
    Ok(())
}

fn clean_lua_name(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 96 {
        return Err("Lua script name must contain 1-96 characters".into());
    }
    Ok(value.to_string())
}

fn make_lua_id(name: &str) -> String {
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
    format!(
        "lua-{}-{millis}",
        if slug.is_empty() { "script" } else { &slug }
    )
}
