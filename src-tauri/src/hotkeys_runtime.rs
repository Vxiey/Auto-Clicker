use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use auto_clicker_core::hotkeys::HotkeyBinding;
use auto_clicker_core::platform::windows::{
    GlobalInputRecorder, HotkeyPhase, RegisteredHotkey, WindowsHotkeyManager,
};
use tauri::{AppHandle, Manager};

use crate::diagnostics::{DiagnosticsState, LogLevel};
use crate::{EngineState, start_clicker_inner, stop_clicker_inner};

const TOGGLE_CLICKER_ID: u64 = 1;
const EMERGENCY_STOP_ID: u64 = 2;

pub struct HotkeyRuntime {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl HotkeyRuntime {
    pub fn start(app: AppHandle) -> Result<Self, String> {
        let bindings = vec![
            RegisteredHotkey {
                id: TOGGLE_CLICKER_ID,
                binding: HotkeyBinding::parse("F6")?.with_consume(true),
            },
            RegisteredHotkey {
                id: EMERGENCY_STOP_ID,
                binding: HotkeyBinding::parse("F8")?.with_consume(true),
            },
        ];
        WindowsHotkeyManager::replace_bindings(bindings)?;
        let events = WindowsHotkeyManager::subscribe()?;
        let (recorder, _captured) = GlobalInputRecorder::start()?;

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("vxclick-hotkey-runtime".into())
            .spawn(move || {
                let _recorder = recorder;
                while !worker_stop.load(Ordering::Acquire) {
                    let event = match events.recv_timeout(Duration::from_millis(250)) {
                        Ok(event) => event,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if event.phase != HotkeyPhase::Pressed {
                        continue;
                    }

                    let engine = app.state::<EngineState>();
                    let diagnostics = app.state::<DiagnosticsState>();
                    match event.id {
                        TOGGLE_CLICKER_ID => {
                            if engine.running.load(Ordering::Acquire) {
                                stop_clicker_inner(&engine, &diagnostics);
                            } else {
                                let cps = f64::from_bits(
                                    engine.target_cps_bits.load(Ordering::Relaxed),
                                );
                                let button = engine
                                    .button
                                    .lock()
                                    .map(|button| *button)
                                    .unwrap_or_default();
                                if let Err(error) =
                                    start_clicker_inner(cps, button, &engine, &diagnostics)
                                {
                                    diagnostics.log(
                                        LogLevel::Error,
                                        "hotkeys",
                                        &format!("F6 start failed: {error}"),
                                    );
                                }
                            }
                        }
                        EMERGENCY_STOP_ID => {
                            diagnostics.log(
                                LogLevel::Warn,
                                "hotkeys",
                                "emergency stop triggered",
                            );
                            stop_clicker_inner(&engine, &diagnostics);
                        }
                        _ => {}
                    }
                }

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

impl Drop for HotkeyRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut thread) = self.thread.lock()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
    }
}
