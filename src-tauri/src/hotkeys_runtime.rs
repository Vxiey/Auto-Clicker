use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::hotkeys::HotkeyBinding;
use auto_clicker_core::platform::windows::{
    GlobalInputRecorder, HotkeyPhase, RegisteredHotkey, WindowsHotkeyManager, WindowsInput,
};
use tauri::{AppHandle, Emitter, Manager};

use crate::diagnostics::{DiagnosticsState, LogLevel};
use crate::macros::{
    MacroEventRecord, MacroState, StoredMacro, save_macro, start_macro_recording,
    stop_macro_recording,
};
use crate::profiles::{ProfileState, profiles_snapshot};
use crate::remaps::RemapState;
use crate::{
    ClickerRuntimeOptions, EngineState, start_clicker_with_options_inner, stop_clicker_inner,
};

const CLICKER_HOTKEY_ID: u64 = 1;
const EMERGENCY_STOP_ID: u64 = 2;
const MACRO_RECORD_START_ID: u64 = 3;
const MACRO_RECORD_STOP_ID: u64 = 4;
const MACRO_HOTKEY_BASE: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveMode {
    Toggle,
    Hold,
    Once,
}

#[derive(Debug, Clone)]
struct ActiveProfileConfig {
    signature: String,
    cps: f64,
    button: MouseButton,
    mode: ActiveMode,
    randomize: bool,
    burst: bool,
    macro_by_hotkey: HashMap<u64, String>,
    remap_by_hotkey: HashMap<u64, String>,
}

pub struct HotkeyRuntime {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl HotkeyRuntime {
    pub fn start(app: AppHandle) -> Result<Self, String> {
        WindowsHotkeyManager::clear_bindings();
        let events = WindowsHotkeyManager::subscribe()?;
        let (recorder, captured) = GlobalInputRecorder::start()?;

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("vxclick-hotkey-runtime".into())
            .spawn(move || {
                let _recorder = recorder;
                let mut active: Option<ActiveProfileConfig> = None;

                while !worker_stop.load(Ordering::Acquire) {
                    if let Err(error) = sync_runtime(&app, &mut active) {
                        app.state::<DiagnosticsState>().log(
                            LogLevel::Error,
                            "hotkeys",
                            &format!("runtime hotkey sync failed: {error}"),
                        );
                    }

                    for input in captured.try_iter() {
                        app.state::<MacroState>().capture(&input);
                    }

                    let event = match events.recv_timeout(Duration::from_millis(25)) {
                        Ok(event) => event,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let diagnostics = app.state::<DiagnosticsState>();

                    if event.id == MACRO_RECORD_START_ID && event.phase == HotkeyPhase::Pressed {
                        match start_macro_recording(
                            true,
                            None,
                            "main".into(),
                            app.state::<MacroState>(),
                        ) {
                            Ok(()) => diagnostics.log(
                                LogLevel::Info,
                                "macros",
                                "global macro recording started with F1",
                            ),
                            Err(error) if error.contains("already active") => {}
                            Err(error) => diagnostics.log(
                                LogLevel::Error,
                                "macros",
                                &format!("F1 recording start failed: {error}"),
                            ),
                        }
                        continue;
                    }

                    if event.id == MACRO_RECORD_STOP_ID && event.phase == HotkeyPhase::Pressed {
                        match stop_macro_recording(app.state::<MacroState>()) {
                            Ok(mut recorded) => {
                                trim_record_stop_hotkey(&mut recorded);
                                if !recorded.is_empty() {
                                    let draft = StoredMacro {
                                        id: String::new(),
                                        name: "Recorded Macro".into(),
                                        trigger: String::new(),
                                        macro_type: "no-repeat".into(),
                                        repeat_delay_ms: 25,
                                        speed: 1.0,
                                        events: recorded,
                                    };
                                    match save_macro(draft, app.state::<MacroState>()) {
                                        Ok(saved) => {
                                            diagnostics.log(
                                                LogLevel::Info,
                                                "macros",
                                                &format!(
                                                    "global macro recording stopped with F2 and saved id={}",
                                                    saved.id
                                                ),
                                            );
                                            let _ = app.emit("macro-recorded", &saved);
                                        }
                                        Err(error) => diagnostics.log(
                                            LogLevel::Error,
                                            "macros",
                                            &format!("failed to save F1/F2 recording: {error}"),
                                        ),
                                    }
                                }
                            }
                            Err(error) => diagnostics.log(
                                LogLevel::Error,
                                "macros",
                                &format!("F2 recording stop failed: {error}"),
                            ),
                        }
                        continue;
                    }

                    let Some(config) = active.as_ref() else {
                        continue;
                    };
                    let engine = app.state::<EngineState>();
                    match event.id {
                        CLICKER_HOTKEY_ID => match (config.mode, event.phase) {
                            (ActiveMode::Toggle, HotkeyPhase::Pressed) => {
                                if engine.running.load(Ordering::Acquire) {
                                    stop_clicker_inner(&engine, &diagnostics);
                                } else if let Err(error) =
                                    start_profile_clicker(config, &engine, &diagnostics)
                                {
                                    diagnostics.log(
                                        LogLevel::Error,
                                        "hotkeys",
                                        &format!("toggle start failed: {error}"),
                                    );
                                }
                            }
                            (ActiveMode::Hold, HotkeyPhase::Pressed) => {
                                if !engine.running.load(Ordering::Acquire)
                                    && let Err(error) =
                                        start_profile_clicker(config, &engine, &diagnostics)
                                {
                                    diagnostics.log(
                                        LogLevel::Error,
                                        "hotkeys",
                                        &format!("hold start failed: {error}"),
                                    );
                                }
                            }
                            (ActiveMode::Hold, HotkeyPhase::Released) => {
                                stop_clicker_inner(&engine, &diagnostics);
                            }
                            (ActiveMode::Once, HotkeyPhase::Pressed) => {
                                match WindowsInput.click(config.button) {
                                    Ok(()) => {
                                        engine.clicks.fetch_add(1, Ordering::Relaxed);
                                    }
                                    Err(error) => diagnostics.log(
                                        LogLevel::Error,
                                        "hotkeys",
                                        &format!("single click failed: {error}"),
                                    ),
                                }
                            }
                            _ => {}
                        },
                        EMERGENCY_STOP_ID if event.phase == HotkeyPhase::Pressed => {
                            diagnostics.log(LogLevel::Warn, "hotkeys", "emergency stop triggered");
                            stop_clicker_inner(&engine, &diagnostics);
                            let _ = app.state::<MacroState>().stop_playback();
                            if let Err(error) = WindowsInput.release_all() {
                                diagnostics.log(
                                    LogLevel::Error,
                                    "safety",
                                    &format!("emergency input release failed: {error}"),
                                );
                            }
                        }
                        id if config.macro_by_hotkey.contains_key(&id) => {
                            if let Some(macro_id) = config.macro_by_hotkey.get(&id)
                                && let Err(error) = app
                                    .state::<MacroState>()
                                    .handle_hotkey(macro_id, event.phase)
                            {
                                diagnostics.log(
                                    LogLevel::Error,
                                    "macros",
                                    &format!("macro hotkey failed: {error}"),
                                );
                            }
                        }
                        id if config.remap_by_hotkey.contains_key(&id) => {
                            if let Some(mapping_id) = config.remap_by_hotkey.get(&id)
                                && let Err(error) = app.state::<RemapState>().execute(
                                    mapping_id,
                                    event.phase,
                                    &app.state::<MacroState>(),
                                )
                            {
                                diagnostics.log(
                                    LogLevel::Error,
                                    "remap",
                                    &format!("remap execution failed: {error}"),
                                );
                            }
                        }
                        _ => {}
                    }
                }

                let _ = app.state::<MacroState>().stop_playback();
                let _ = WindowsInput.release_all();
                WindowsHotkeyManager::clear_bindings();
                WindowsHotkeyManager::unsubscribe();
            })
            .map_err(|error| {
                WindowsHotkeyManager::clear_bindings();
                WindowsHotkeyManager::unsubscribe();
                format!("failed to start hotkey runtime: {error}")
            })?;

        Ok(Self {
            stop,
            thread: Mutex::new(Some(thread)),
        })
    }
}

fn trim_record_stop_hotkey(events: &mut Vec<MacroEventRecord>) {
    if events.last().is_some_and(|event| {
        event.event_type == "key-down" && event.label.eq_ignore_ascii_case("F2")
    }) {
        events.pop();
        if events.last().is_some_and(|event| event.event_type == "delay") {
            events.pop();
        }
    }
}

fn start_profile_clicker(
    config: &ActiveProfileConfig,
    engine: &EngineState,
    diagnostics: &DiagnosticsState,
) -> Result<(), String> {
    start_clicker_with_options_inner(
        config.cps,
        config.button,
        ClickerRuntimeOptions {
            randomize_percent: if config.randomize { 5.0 } else { 0.0 },
            burst_size: if config.burst { 4 } else { 1 },
            positions: Vec::new(),
        },
        engine,
        diagnostics,
    )
}

fn sync_runtime(app: &AppHandle, active: &mut Option<ActiveProfileConfig>) -> Result<(), String> {
    let snapshot = profiles_snapshot(app.state::<ProfileState>())?;
    let profile = snapshot
        .document
        .profiles
        .iter()
        .find(|profile| profile.id == snapshot.document.active_profile_id)
        .ok_or_else(|| "active profile is missing".to_string())?;
    let button = MouseButton::parse(&profile.clicker.button)
        .ok_or_else(|| "active profile has an invalid mouse button".to_string())?;
    let mode = match profile.clicker.mode.to_ascii_lowercase().as_str() {
        "hold" => ActiveMode::Hold,
        "once" => ActiveMode::Once,
        _ => ActiveMode::Toggle,
    };

    let macros = app.state::<MacroState>().runtime_macros()?;
    let runtime_remaps = app
        .state::<RemapState>()
        .runtime_bindings(snapshot.foreground_process.as_deref())?;

    let mut bindings = vec![
        RegisteredHotkey {
            id: MACRO_RECORD_START_ID,
            binding: HotkeyBinding::parse("F1")?.with_consume(false),
        },
        RegisteredHotkey {
            id: MACRO_RECORD_STOP_ID,
            binding: HotkeyBinding::parse("F2")?.with_consume(false),
        },
        RegisteredHotkey {
            id: CLICKER_HOTKEY_ID,
            binding: HotkeyBinding::parse(&profile.clicker.start_hotkey)?.with_consume(true),
        },
        RegisteredHotkey {
            id: EMERGENCY_STOP_ID,
            binding: HotkeyBinding::parse(&profile.clicker.emergency_stop_hotkey)?
                .with_consume(true),
        },
    ];

    let mut macro_by_hotkey = HashMap::new();
    for (index, macro_def) in macros.iter().filter(|item| !item.trigger.trim().is_empty()).enumerate() {
        let id = MACRO_HOTKEY_BASE + index as u64;
        bindings.push(RegisteredHotkey {
            id,
            binding: HotkeyBinding::parse(&macro_def.trigger)?.with_consume(false),
        });
        macro_by_hotkey.insert(id, macro_def.id.clone());
    }

    let mut remap_by_hotkey = HashMap::new();
    for remap in runtime_remaps {
        remap_by_hotkey.insert(remap.hotkey.id, remap.mapping_id);
        bindings.push(remap.hotkey);
    }

    let mut signature = format!(
        "{}|{}|{}|{:.6}|{}|{}|{}|{}|{}",
        profile.id,
        profile.clicker.start_hotkey,
        profile.clicker.emergency_stop_hotkey,
        profile.clicker.cps,
        profile.clicker.button,
        profile.clicker.mode,
        profile.clicker.randomize,
        profile.clicker.burst,
        snapshot.foreground_process.as_deref().unwrap_or("")
    );
    for binding in &bindings {
        signature.push('|');
        signature.push_str(&binding.id.to_string());
        signature.push(':');
        signature.push_str(binding.binding.canonical());
    }

    if active
        .as_ref()
        .is_some_and(|current| current.signature == signature)
    {
        return Ok(());
    }

    WindowsHotkeyManager::replace_bindings(bindings)?;

    let engine = app.state::<EngineState>();
    engine
        .target_cps_bits
        .store(profile.clicker.cps.to_bits(), Ordering::Release);
    if let Ok(mut current_button) = engine.button.lock() {
        *current_button = button;
    }

    app.state::<DiagnosticsState>().log(
        LogLevel::Info,
        "hotkeys",
        &format!(
            "profile={} start={} emergency={} mode={} randomize={} burst={} macros={} remaps={} recorder=F1/F2",
            profile.name,
            profile.clicker.start_hotkey,
            profile.clicker.emergency_stop_hotkey,
            profile.clicker.mode,
            profile.clicker.randomize,
            profile.clicker.burst,
            macro_by_hotkey.len(),
            remap_by_hotkey.len()
        ),
    );

    *active = Some(ActiveProfileConfig {
        signature,
        cps: profile.clicker.cps,
        button,
        mode,
        randomize: profile.clicker.randomize,
        burst: profile.clicker.burst,
        macro_by_hotkey,
        remap_by_hotkey,
    });
    Ok(())
}

impl Drop for HotkeyRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut thread) = self.thread.lock()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
        let _ = WindowsInput.release_all();
    }
}
