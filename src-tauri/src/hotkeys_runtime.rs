use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::hotkeys::HotkeyBinding;
use auto_clicker_core::platform::windows::{
    GlobalInputRecorder, HotkeyPhase, RegisteredHotkey, WindowsHotkeyManager, WindowsInput,
};
use tauri::{AppHandle, Manager};

use crate::diagnostics::{DiagnosticsState, LogLevel};
use crate::profiles::{ProfileState, profiles_snapshot};
use crate::{EngineState, start_clicker_inner, stop_clicker_inner};

const CLICKER_HOTKEY_ID: u64 = 1;
const EMERGENCY_STOP_ID: u64 = 2;

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
}

pub struct HotkeyRuntime {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl HotkeyRuntime {
    pub fn start(app: AppHandle) -> Result<Self, String> {
        WindowsHotkeyManager::clear_bindings();
        let events = WindowsHotkeyManager::subscribe()?;
        let (recorder, _captured) = GlobalInputRecorder::start()?;

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("vxclick-hotkey-runtime".into())
            .spawn(move || {
                let _recorder = recorder;
                let mut active: Option<ActiveProfileConfig> = None;

                while !worker_stop.load(Ordering::Acquire) {
                    if let Err(error) = sync_profile(&app, &mut active) {
                        app.state::<DiagnosticsState>().log(
                            LogLevel::Error,
                            "hotkeys",
                            &format!("profile hotkey sync failed: {error}"),
                        );
                    }

                    let event = match events.recv_timeout(Duration::from_millis(250)) {
                        Ok(event) => event,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let Some(config) = active.as_ref() else {
                        continue;
                    };

                    let engine = app.state::<EngineState>();
                    let diagnostics = app.state::<DiagnosticsState>();
                    match event.id {
                        CLICKER_HOTKEY_ID => match (config.mode, event.phase) {
                            (ActiveMode::Toggle, HotkeyPhase::Pressed) => {
                                if engine.running.load(Ordering::Acquire) {
                                    stop_clicker_inner(&engine, &diagnostics);
                                } else if let Err(error) = start_clicker_inner(
                                    config.cps,
                                    config.button,
                                    &engine,
                                    &diagnostics,
                                ) {
                                    diagnostics.log(
                                        LogLevel::Error,
                                        "hotkeys",
                                        &format!("toggle start failed: {error}"),
                                    );
                                }
                            }
                            (ActiveMode::Hold, HotkeyPhase::Pressed) => {
                                if !engine.running.load(Ordering::Acquire)
                                    && let Err(error) = start_clicker_inner(
                                        config.cps,
                                        config.button,
                                        &engine,
                                        &diagnostics,
                                    )
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
                            diagnostics.log(
                                LogLevel::Warn,
                                "hotkeys",
                                "emergency stop triggered",
                            );
                            stop_clicker_inner(&engine, &diagnostics);
                            if let Err(error) = WindowsInput.release_all() {
                                diagnostics.log(
                                    LogLevel::Error,
                                    "safety",
                                    &format!("emergency input release failed: {error}"),
                                );
                            }
                        }
                        _ => {}
                    }
                }

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

fn sync_profile(app: &AppHandle, active: &mut Option<ActiveProfileConfig>) -> Result<(), String> {
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
    let signature = format!(
        "{}|{}|{}|{:.6}|{}|{}",
        profile.id,
        profile.clicker.start_hotkey,
        profile.clicker.emergency_stop_hotkey,
        profile.clicker.cps,
        profile.clicker.button,
        profile.clicker.mode
    );

    if active.as_ref().is_some_and(|current| current.signature == signature) {
        return Ok(());
    }

    let start = HotkeyBinding::parse(&profile.clicker.start_hotkey)?.with_consume(true);
    let emergency =
        HotkeyBinding::parse(&profile.clicker.emergency_stop_hotkey)?.with_consume(true);
    WindowsHotkeyManager::replace_bindings(vec![
        RegisteredHotkey {
            id: CLICKER_HOTKEY_ID,
            binding: start,
        },
        RegisteredHotkey {
            id: EMERGENCY_STOP_ID,
            binding: emergency,
        },
    ])?;

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
            "profile={} start={} emergency={} mode={}",
            profile.name,
            profile.clicker.start_hotkey,
            profile.clicker.emergency_stop_hotkey,
            profile.clicker.mode
        ),
    );

    *active = Some(ActiveProfileConfig {
        signature,
        cps: profile.clicker.cps,
        button,
        mode,
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
