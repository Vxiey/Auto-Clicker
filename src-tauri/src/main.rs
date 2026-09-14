mod diagnostics;
mod profiles;
mod updater;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::platform::windows::{LiveClickerConfig, PrecisionClicker};
use diagnostics::{
    DiagnosticsState, LogLevel, clear_diagnostics, diagnostics_client_log, diagnostics_snapshot,
};
use profiles::{
    ProfileState, activate_profile, create_profile, delete_profile, foreground_process,
    profiles_snapshot, save_profile, set_profile_auto_switch,
};
use serde::Serialize;
use tauri::{Manager, State};
use updater::{check_for_updates, stage_patch};

struct SampleState {
    at: Instant,
    clicks: u64,
    actual_cps: f64,
}

struct EngineState {
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    clicks: Arc<AtomicU64>,
    target_cps_bits: AtomicU64,
    sample: Mutex<SampleState>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            clicks: Arc::new(AtomicU64::new(0)),
            target_cps_bits: AtomicU64::new(250.0_f64.to_bits()),
            sample: Mutex::new(SampleState {
                at: Instant::now(),
                clicks: 0,
                actual_cps: 0.0,
            }),
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

#[tauri::command]
fn start_clicker(
    cps: f64,
    button: String,
    state: State<'_, EngineState>,
    diagnostics: State<'_, DiagnosticsState>,
) -> Result<(), String> {
    if !cps.is_finite() || !(1.0..=20_000.0).contains(&cps) {
        diagnostics.log(LogLevel::Warn, "clicker", "rejected invalid CPS value");
        return Err("CPS must be between 1 and 20,000".into());
    }
    let button = MouseButton::parse(&button).ok_or_else(|| {
        diagnostics.log(LogLevel::Warn, "clicker", "rejected invalid mouse button");
        "button must be left, right, middle, x1, or x2".to_string()
    })?;

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
    if let Ok(mut sample) = state.sample.lock() {
        sample.at = Instant::now();
        sample.clicks = state.clicks.load(Ordering::Relaxed);
        sample.actual_cps = 0.0;
    }

    diagnostics.log(
        LogLevel::Info,
        "clicker",
        &format!("starting target_cps={cps:.3} button={button:?}"),
    );

    let running = Arc::clone(&state.running);
    let stop = Arc::clone(&state.stop);
    let clicks = Arc::clone(&state.clicks);
    let diagnostics = diagnostics.inner().clone();
    let worker_diagnostics = diagnostics.clone();
    let clicks_at_start = clicks.load(Ordering::Relaxed);

    std::thread::Builder::new()
        .name("vxclick-precision-worker".into())
        .spawn(move || {
            let started = Instant::now();
            let config = LiveClickerConfig {
                cps,
                button,
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
                        "stopped elapsed_s={elapsed:.3} clicks={produced} actual_cps={actual:.3}"
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
                    "target_cps={cps:.3} actual_cps={actual:.3} elapsed_s={elapsed:.3} clicks={produced}"
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

#[tauri::command]
fn stop_clicker(state: State<'_, EngineState>, diagnostics: State<'_, DiagnosticsState>) {
    diagnostics.log(LogLevel::Info, "clicker", "stop requested");
    state.stop.store(true, Ordering::Release);
}

fn main() {
    tauri::Builder::default()
        .manage(EngineState::default())
        .setup(|app| {
            let diagnostics =
                DiagnosticsState::initialize(app.handle()).map_err(std::io::Error::other)?;
            diagnostics.install_panic_hook();
            app.manage(diagnostics.clone());

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
            diagnostics.log(LogLevel::Info, "app", "Tauri setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_status,
            start_clicker,
            stop_clicker,
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
            check_for_updates,
            stage_patch
        ])
        .run(tauri::generate_context!())
        .expect("error while running VxClick");
}
