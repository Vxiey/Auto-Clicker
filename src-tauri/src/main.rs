#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod benchmark;
mod diagnostics;
mod hotkey_capture;
mod hotkeys_runtime;
mod lua_scripts;
mod macros;
mod profiles;
mod recoil;
mod remaps;
mod updater;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::platform::windows::{LiveClickerConfig, PrecisionClicker};
use benchmark::run_precision_benchmark;
use diagnostics::{
    DiagnosticsState, LogLevel, clear_diagnostics, diagnostics_client_log, diagnostics_snapshot,
};
use hotkeys_runtime::{HotkeyRuntime, cancel_hotkey_capture, start_hotkey_capture};
use lua_scripts::{
    LuaScriptState, delete_lua_script, duplicate_lua_script, lua_scripts_snapshot,
    rename_lua_script, save_lua_script,
};
use macros::{
    MacroState, delete_macro, macros_snapshot, play_macro, run_lua_script, save_macro,
    start_macro_recording, stop_macro, stop_macro_recording, validate_lua_script,
};
use profiles::{
    ProfileState, activate_profile, create_profile, delete_profile, foreground_process,
    profiles_snapshot, save_profile, set_profile_auto_switch,
};
use recoil::{
    RecoilState, activate_recoil_game, create_recoil_game, recoil_snapshot, recoil_start,
    recoil_stop, save_recoil_game, save_recoil_preset, set_recoil_slot,
};
use remaps::{RemapState, delete_remap, remaps_snapshot, save_remap};
use serde::Serialize;
use tauri::{
    Manager, State,
    menu::{Menu, MenuItem},
    tray::{MouseButton as TrayMouseButton, MouseButtonState, TrayIconEvent},
};
use updater::{check_for_updates, stage_patch};

const MIN_CLICKER_CPS: f64 = 1.0 / 604_800.0; // one click per week
const MAX_CLICKER_CPS: f64 = 20_000.0;
const VXCLICK_GITHUB_URL: &str = "https://github.com/Vxiey/VxClick";
const TRAY_OPEN_ID: &str = "tray-open";
const TRAY_EXIT_ID: &str = "tray-exit";

struct SampleState {
    at: Instant,
    clicks: u64,
    actual_cps: f64,
}

pub(crate) struct EngineState {
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    clicks: Arc<AtomicU64>,
    target_cps_bits: AtomicU64,
    button: Mutex<MouseButton>,
    sample: Mutex<SampleState>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            clicks: Arc::new(AtomicU64::new(0)),
            target_cps_bits: AtomicU64::new(250.0_f64.to_bits()),
            button: Mutex::new(MouseButton::Left),
            sample: Mutex::new(SampleState {
                at: Instant::now(),
                clicks: 0,
                actual_cps: 0.0,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClickerRuntimeOptions {
    pub(crate) randomize_percent: f64,
    pub(crate) burst_size: u32,
    pub(crate) positions: Vec<(i32, i32)>,
}

impl Default for ClickerRuntimeOptions {
    fn default() -> Self {
        Self {
            randomize_percent: 0.0,
            burst_size: 1,
            positions: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct EngineStatus {
    running: bool,
    clicks: u64,
    actual_cps: f64,
    target_cps: f64,
}

#[tauri::command]
fn engine_status(state: State<'_, EngineState>) -> EngineStatus {
    let running = state.running.load(Ordering::Acquire);
    let clicks = state.clicks.load(Ordering::Relaxed);
    let target_cps = f64::from_bits(state.target_cps_bits.load(Ordering::Relaxed));

    let mut sample = state.sample.lock().expect("engine sample mutex poisoned");
    let elapsed = sample.at.elapsed().as_secs_f64();
    if elapsed >= 0.25 {
        sample.actual_cps = if running {
            clicks.saturating_sub(sample.clicks) as f64 / elapsed
        } else {
            0.0
        };
        sample.at = Instant::now();
        sample.clicks = clicks;
    }

    EngineStatus {
        running,
        clicks,
        actual_cps: sample.actual_cps,
        target_cps,
    }
}

pub(crate) fn start_clicker_with_options_inner(
    cps: f64,
    button: MouseButton,
    options: ClickerRuntimeOptions,
    state: &EngineState,
    diagnostics: &DiagnosticsState,
) -> Result<(), String> {
    if !cps.is_finite() || !(MIN_CLICKER_CPS..=MAX_CLICKER_CPS).contains(&cps) {
        diagnostics.log(LogLevel::Warn, "clicker", "rejected invalid CPS value");
        return Err(
            "CPS must be positive, no slower than one click per week, and no higher than 20,000"
                .into(),
        );
    }
    if !options.randomize_percent.is_finite() || !(0.0..=50.0).contains(&options.randomize_percent)
    {
        return Err("randomization must be between 0 and 50 percent".into());
    }
    if !(1..=16).contains(&options.burst_size) {
        return Err("burst size must be between 1 and 16 clicks".into());
    }

    if state.running.swap(true, Ordering::AcqRel) {
        diagnostics.log(
            LogLevel::Debug,
            "clicker",
            "start ignored because engine is already running",
        );
        return Ok(());
    }

    state.stop.store(false, Ordering::Release);
    state
        .target_cps_bits
        .store(cps.to_bits(), Ordering::Release);
    if let Ok(mut configured_button) = state.button.lock() {
        *configured_button = button;
    }
    if let Ok(mut sample) = state.sample.lock() {
        sample.at = Instant::now();
        sample.clicks = state.clicks.load(Ordering::Relaxed);
        sample.actual_cps = 0.0;
    }

    diagnostics.log(
        LogLevel::Info,
        "clicker",
        &format!(
            "starting target_cps={cps:.6} button={button:?} randomize={:.1}% burst={} positions={}",
            options.randomize_percent,
            options.burst_size,
            options.positions.len()
        ),
    );

    let running = Arc::clone(&state.running);
    let stop = Arc::clone(&state.stop);
    let clicks = Arc::clone(&state.clicks);
    let diagnostics = diagnostics.clone();
    let worker_diagnostics = diagnostics.clone();
    let clicks_at_start = clicks.load(Ordering::Relaxed);

    std::thread::Builder::new()
        .name("vxclick-precision-worker".into())
        .spawn(move || {
            let started = Instant::now();
            let config = LiveClickerConfig {
                cps,
                button,
                randomize_percent: options.randomize_percent,
                burst_size: options.burst_size,
                positions: options.positions,
                ..LiveClickerConfig::default()
            };
            let result = PrecisionClicker::new()
                .and_then(|engine| engine.run_until_stopped(config, &stop, &clicks));
            let elapsed = started.elapsed().as_secs_f64();
            let produced = clicks
                .load(Ordering::Relaxed)
                .saturating_sub(clicks_at_start);
            let actual = if elapsed > 0.0 {
                produced as f64 / elapsed
            } else {
                0.0
            };

            match result {
                Ok(()) => worker_diagnostics.log(
                    LogLevel::Info,
                    "clicker",
                    &format!(
                        "stopped elapsed_s={elapsed:.3} clicks={produced} actual_cps={actual:.6}"
                    ),
                ),
                Err(error) => worker_diagnostics.log(
                    LogLevel::Error,
                    "clicker",
                    &format!("precision worker failed: {error}"),
                ),
            }
            worker_diagnostics.performance(
                "clicker-run",
                &format!(
                    "target_cps={cps:.6} actual_cps={actual:.6} elapsed_s={elapsed:.3} clicks={produced}"
                ),
            );
            running.store(false, Ordering::Release);
        })
        .map_err(|error| {
            state.running.store(false, Ordering::Release);
            diagnostics.log(
                LogLevel::Error,
                "clicker",
                &format!("failed to start precision worker: {error}"),
            );
            format!("failed to start precision worker: {error}")
        })?;

    Ok(())
}

pub(crate) fn stop_clicker_inner(state: &EngineState, diagnostics: &DiagnosticsState) {
    diagnostics.log(LogLevel::Info, "clicker", "stop requested");
    state.stop.store(true, Ordering::Release);
}

#[tauri::command]
fn start_clicker(
    cps: f64,
    button: String,
    randomize: bool,
    burst: bool,
    position_mode: String,
    positions: String,
    state: State<'_, EngineState>,
    diagnostics: State<'_, DiagnosticsState>,
) -> Result<(), String> {
    let button = MouseButton::parse(&button).ok_or_else(|| {
        diagnostics.log(LogLevel::Warn, "clicker", "rejected invalid mouse button");
        "button must be left, right, middle, x1, or x2".to_string()
    })?;
    let positions = parse_positions(&position_mode, &positions)?;
    let options = ClickerRuntimeOptions {
        randomize_percent: if randomize { 5.0 } else { 0.0 },
        burst_size: if burst { 4 } else { 1 },
        positions,
    };
    start_clicker_with_options_inner(cps, button, options, &state, &diagnostics)
}

#[tauri::command]
fn stop_clicker(state: State<'_, EngineState>, diagnostics: State<'_, DiagnosticsState>) {
    stop_clicker_inner(&state, &diagnostics);
}

fn is_allowed_external_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix(VXCLICK_GITHUB_URL) else {
        return false;
    };
    rest.is_empty() || rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('#')
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if !is_allowed_external_url(url) {
        return Err("external URL is not an approved VxClick GitHub link".into());
    }
    std::process::Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open external link: {error}"))
}

fn show_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn parse_positions(mode: &str, value: &str) -> Result<Vec<(i32, i32)>, String> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "cursor" | "" => Ok(Vec::new()),
        "fixed" => {
            let point = parse_point(value)?;
            Ok(vec![point])
        }
        "multi" => {
            let points = value
                .split(';')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(parse_point)
                .collect::<Result<Vec<_>, _>>()?;
            if points.is_empty() {
                return Err("multi-point mode needs at least one X,Y coordinate".into());
            }
            if points.len() > 256 {
                return Err("multi-point mode supports at most 256 coordinates".into());
            }
            Ok(points)
        }
        _ => Err("position mode must be cursor, fixed, or multi".into()),
    }
}

fn parse_point(value: &str) -> Result<(i32, i32), String> {
    let mut parts = value.split(',').map(str::trim);
    let x = parts
        .next()
        .ok_or_else(|| "coordinate must be X,Y".to_string())?
        .parse::<i32>()
        .map_err(|_| "coordinate X is invalid".to_string())?;
    let y = parts
        .next()
        .ok_or_else(|| "coordinate must be X,Y".to_string())?
        .parse::<i32>()
        .map_err(|_| "coordinate Y is invalid".to_string())?;
    if parts.next().is_some() {
        return Err("coordinate must contain exactly X,Y".into());
    }
    Ok((x, y))
}

fn main() {
    tauri::Builder::default()
        .manage(EngineState::default())
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_OPEN_ID => show_main_window(app),
            TRAY_EXIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|app, event| {
            if let TrayIconEvent::Click {
                button: TrayMouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(app);
            }
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let diagnostics =
                DiagnosticsState::initialize(app.handle()).map_err(std::io::Error::other)?;
            diagnostics.install_panic_hook();
            app.manage(diagnostics.clone());

            if let Some(tray) = app.tray_by_id("main") {
                let open_item =
                    MenuItem::with_id(app, TRAY_OPEN_ID, "Open VxClick", true, None::<&str>)?;
                let exit_item =
                    MenuItem::with_id(app, TRAY_EXIT_ID, "Exit VxClick", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&open_item, &exit_item])?;
                tray.set_menu(Some(menu))?;
            }

            let profiles = match ProfileState::load(app.handle()) {
                Ok(profiles) => profiles,
                Err(error) => {
                    diagnostics.log(
                        LogLevel::Error,
                        "profiles",
                        &format!("failed to load profile state: {error}"),
                    );
                    return Err(std::io::Error::other(error).into());
                }
            };
            profiles.start_watcher(app.handle().clone());
            app.manage(profiles);

            let macros = MacroState::load(app.handle()).map_err(|error| {
                diagnostics.log(LogLevel::Error, "macros", &error);
                std::io::Error::other(error)
            })?;
            app.manage(macros);

            let lua_scripts = LuaScriptState::load(app.handle()).map_err(|error| {
                diagnostics.log(LogLevel::Error, "lua-scripts", &error);
                std::io::Error::other(error)
            })?;
            app.manage(lua_scripts);

            let remaps = RemapState::load(app.handle()).map_err(|error| {
                diagnostics.log(LogLevel::Error, "remap", &error);
                std::io::Error::other(error)
            })?;
            app.manage(remaps);

            let recoil = RecoilState::load(app.handle()).map_err(|error| {
                diagnostics.log(LogLevel::Error, "recoil", &error);
                std::io::Error::other(error)
            })?;
            app.manage(recoil);

            let hotkeys = HotkeyRuntime::start(app.handle().clone()).map_err(|error| {
                diagnostics.log(LogLevel::Error, "hotkeys", &error);
                std::io::Error::other(error)
            })?;
            app.manage(hotkeys);

            diagnostics.log(LogLevel::Info, "app", "Tauri setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_status,
            start_clicker,
            stop_clicker,
            open_external_url,
            start_hotkey_capture,
            cancel_hotkey_capture,
            run_precision_benchmark,
            diagnostics_snapshot,
            diagnostics_client_log,
            clear_diagnostics,
            profiles_snapshot,
            create_profile,
            save_profile,
            activate_profile,
            delete_profile,
            set_profile_auto_switch,
            foreground_process,
            macros_snapshot,
            save_macro,
            delete_macro,
            play_macro,
            stop_macro,
            start_macro_recording,
            stop_macro_recording,
            validate_lua_script,
            run_lua_script,
            lua_scripts_snapshot,
            save_lua_script,
            rename_lua_script,
            duplicate_lua_script,
            delete_lua_script,
            remaps_snapshot,
            save_remap,
            delete_remap,
            recoil_snapshot,
            create_recoil_game,
            save_recoil_game,
            activate_recoil_game,
            set_recoil_slot,
            save_recoil_preset,
            recoil_start,
            recoil_stop,
            check_for_updates,
            stage_patch
        ])
        .run(tauri::generate_context!())
        .expect("error while running VxClick");
}

#[cfg(test)]
mod tests {
    use super::{ClickerRuntimeOptions, is_allowed_external_url, parse_positions};

    #[test]
    fn safe_runtime_options_have_single_click_burst() {
        assert_eq!(ClickerRuntimeOptions::default().burst_size, 1);
    }

    #[test]
    fn parses_fixed_and_multi_positions() {
        assert_eq!(parse_positions("fixed", "10, 20").unwrap(), vec![(10, 20)]);
        assert_eq!(
            parse_positions("multi", "10,20;30,40").unwrap(),
            vec![(10, 20), (30, 40)]
        );
        assert!(parse_positions("fixed", "10").is_err());
    }

    #[test]
    fn external_links_are_limited_to_the_vxclick_github_repository() {
        assert!(is_allowed_external_url("https://github.com/Vxiey/VxClick"));
        assert!(is_allowed_external_url(
            "https://github.com/Vxiey/VxClick/blob/main/TERMS.md"
        ));
        assert!(!is_allowed_external_url("https://example.com"));
        assert!(!is_allowed_external_url(
            "https://github.com/Vxiey/VxClick-malicious"
        ));
    }
}
