use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

const PROFILE_SCHEMA_VERSION: u32 = 1;
const WATCH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClickerProfileSettings {
    pub cps: f64,
    pub button: String,
    pub mode: String,
    pub randomize: bool,
    pub burst: bool,
    pub start_hotkey: String,
    pub emergency_stop_hotkey: String,
}

impl Default for ClickerProfileSettings {
    fn default() -> Self {
        Self {
            cps: 250.0,
            button: "left".into(),
            mode: "toggle".into(),
            randomize: false,
            burst: false,
            start_hotkey: "F6".into(),
            emergency_stop_hotkey: "F8".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub process_names: Vec<String>,
    pub auto_switch: bool,
    pub clicker: ClickerProfileSettings,
}

impl Profile {
    fn default_profile() -> Self {
        Self {
            id: "default".into(),
            name: "Default".into(),
            description: "General Windows automation".into(),
            process_names: Vec::new(),
            auto_switch: false,
            clicker: ClickerProfileSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileDocument {
    pub schema_version: u32,
    pub active_profile_id: String,
    pub auto_switch_enabled: bool,
    pub profiles: Vec<Profile>,
}

impl Default for ProfileDocument {
    fn default() -> Self {
        Self {
            schema_version: PROFILE_SCHEMA_VERSION,
            active_profile_id: "default".into(),
            auto_switch_enabled: true,
            profiles: vec![Profile::default_profile()],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProfilesSnapshot {
    pub document: ProfileDocument,
    pub foreground_process: Option<String>,
}

#[derive(Clone)]
pub struct ProfileState {
    document: Arc<Mutex<ProfileDocument>>,
    path: PathBuf,
}

impl ProfileState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("failed to resolve config directory: {error}"))?;
        let path = dir.join("profiles.json");

        let mut document = if path.exists() {
            let data = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            serde_json::from_slice::<ProfileDocument>(&data)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
        } else {
            ProfileDocument::default()
        };

        migrate_and_validate(&mut document)?;
        let state = Self {
            document: Arc::new(Mutex::new(document)),
            path,
        };
        state.persist()?;
        Ok(state)
    }

    pub fn start_watcher(&self, app: AppHandle) {
        let state = self.clone();
        thread::Builder::new()
            .name("vxclick-profile-watcher".into())
            .spawn(move || loop {
                if let Err(error) = state.auto_switch_tick(&app) {
                    eprintln!("profile watcher: {error}");
                }
                thread::sleep(WATCH_INTERVAL);
            })
            .expect("failed to start profile watcher");
    }

    fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let document = self
            .document
            .lock()
            .map_err(|_| "profile state mutex poisoned".to_string())?
            .clone();
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("failed to serialize profiles: {error}"))?;
        fs::write(&self.path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }

    fn auto_switch_tick(&self, app: &AppHandle) -> Result<(), String> {
        let Some(process) = foreground_process_name() else {
            return Ok(());
        };
        let process_lower = process.to_ascii_lowercase();

        let next_id = {
            let document = self
                .document
                .lock()
                .map_err(|_| "profile state mutex poisoned".to_string())?;
            if !document.auto_switch_enabled {
                return Ok(());
            }
            document
                .profiles
                .iter()
                .find(|profile| {
                    profile.auto_switch
                        && profile.process_names.iter().any(|candidate| {
                            normalize_process_name(candidate) == process_lower
                        })
                })
                .map(|profile| profile.id.clone())
        };

        let Some(next_id) = next_id else {
            return Ok(());
        };

        let changed = {
            let mut document = self
                .document
                .lock()
                .map_err(|_| "profile state mutex poisoned".to_string())?;
            if document.active_profile_id == next_id {
                false
            } else {
                document.active_profile_id = next_id.clone();
                true
            }
        };

        if changed {
            self.persist()?;
            let _ = app.emit("profile-changed", &next_id);
        }
        Ok(())
    }
}

#[tauri::command]
pub fn profiles_snapshot(state: State<'_, ProfileState>) -> Result<ProfilesSnapshot, String> {
    let document = state
        .document
        .lock()
        .map_err(|_| "profile state mutex poisoned".to_string())?
        .clone();
    Ok(ProfilesSnapshot {
        document,
        foreground_process: foreground_process_name(),
    })
}

#[tauri::command]
pub fn create_profile(
    name: String,
    process_name: Option<String>,
    state: State<'_, ProfileState>,
) -> Result<Profile, String> {
    let name = clean_name(&name)?;
    let id = make_profile_id(&name);
    let process_names = process_name
        .filter(|value| !value.trim().is_empty())
        .map(|value| vec![normalize_process_name(&value)])
        .unwrap_or_default();

    let profile = Profile {
        id,
        description: if process_names.is_empty() {
            "Custom automation profile".into()
        } else {
            format!("Auto-switch for {}", process_names[0])
        },
        auto_switch: !process_names.is_empty(),
        process_names,
        name,
        clicker: ClickerProfileSettings::default(),
    };

    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "profile state mutex poisoned".to_string())?;
        document.profiles.push(profile.clone());
    }
    state.persist()?;
    Ok(profile)
}

#[tauri::command]
pub fn save_profile(profile: Profile, state: State<'_, ProfileState>) -> Result<Profile, String> {
    validate_profile(&profile)?;
    let mut normalized = profile;
    normalized.process_names = normalized
        .process_names
        .iter()
        .map(|name| normalize_process_name(name))
        .filter(|name| !name.is_empty())
        .collect();

    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "profile state mutex poisoned".to_string())?;
        let existing = document
            .profiles
            .iter_mut()
            .find(|item| item.id == normalized.id)
            .ok_or_else(|| format!("unknown profile '{}'", normalized.id))?;
        *existing = normalized.clone();
    }
    state.persist()?;
    Ok(normalized)
}

#[tauri::command]
pub fn activate_profile(id: String, state: State<'_, ProfileState>) -> Result<(), String> {
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "profile state mutex poisoned".to_string())?;
        if !document.profiles.iter().any(|profile| profile.id == id) {
            return Err(format!("unknown profile '{id}'"));
        }
        document.active_profile_id = id;
    }
    state.persist()
}

#[tauri::command]
pub fn delete_profile(id: String, state: State<'_, ProfileState>) -> Result<(), String> {
    if id == "default" {
        return Err("the Default profile cannot be deleted".into());
    }
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "profile state mutex poisoned".to_string())?;
        let before = document.profiles.len();
        document.profiles.retain(|profile| profile.id != id);
        if document.profiles.len() == before {
            return Err(format!("unknown profile '{id}'"));
        }
        if document.active_profile_id == id {
            document.active_profile_id = "default".into();
        }
    }
    state.persist()
}

#[tauri::command]
pub fn set_profile_auto_switch(
    enabled: bool,
    state: State<'_, ProfileState>,
) -> Result<(), String> {
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "profile state mutex poisoned".to_string())?;
        document.auto_switch_enabled = enabled;
    }
    state.persist()
}

#[tauri::command]
pub fn foreground_process() -> Option<String> {
    foreground_process_name()
}

fn migrate_and_validate(document: &mut ProfileDocument) -> Result<(), String> {
    if document.schema_version > PROFILE_SCHEMA_VERSION {
        return Err(format!(
            "profile schema {} is newer than supported schema {}",
            document.schema_version, PROFILE_SCHEMA_VERSION
        ));
    }
    document.schema_version = PROFILE_SCHEMA_VERSION;
    if !document.profiles.iter().any(|profile| profile.id == "default") {
        document.profiles.insert(0, Profile::default_profile());
    }
    for profile in &document.profiles {
        validate_profile(profile)?;
    }
    if !document
        .profiles
        .iter()
        .any(|profile| profile.id == document.active_profile_id)
    {
        document.active_profile_id = "default".into();
    }
    Ok(())
}

fn validate_profile(profile: &Profile) -> Result<(), String> {
    if profile.id.trim().is_empty() || profile.id.len() > 96 {
        return Err("invalid profile id".into());
    }
    clean_name(&profile.name)?;
    if !profile.clicker.cps.is_finite() || !(1.0..=20_000.0).contains(&profile.clicker.cps) {
        return Err("profile CPS must be between 1 and 20,000".into());
    }
    if !matches!(
        profile.clicker.button.as_str(),
        "left" | "right" | "middle" | "x1" | "x2"
    ) {
        return Err("profile mouse button is invalid".into());
    }
    Ok(())
}

fn clean_name(name: &str) -> Result<String, String> {
    let value = name.trim();
    if value.is_empty() || value.len() > 64 {
        return Err("profile name must contain 1-64 characters".into());
    }
    Ok(value.to_string())
}

fn make_profile_id(name: &str) -> String {
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
    format!("{}-{millis}", if slug.is_empty() { "profile" } else { &slug })
}

fn normalize_process_name(value: &str) -> String {
    Path::new(value.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value.trim())
        .to_ascii_lowercase()
}

#[cfg(windows)]
fn foreground_process_name() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut pid = 0_u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }

        let mut buffer = vec![0_u16; 32_768];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        if ok == 0 || len == 0 {
            return None;
        }

        let full = String::from_utf16_lossy(&buffer[..len as usize]);
        Path::new(&full)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    }
}

#[cfg(not(windows))]
fn foreground_process_name() -> Option<String> {
    None
}
