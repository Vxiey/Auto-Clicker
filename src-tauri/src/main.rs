use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::platform::windows::{LiveClickerConfig, PrecisionClicker};
use serde::Serialize;
use tauri::State;

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
fn start_clicker(cps: f64, button: String, state: State<'_, EngineState>) -> Result<(), String> {
    if !cps.is_finite() || !(1.0..=20_000.0).contains(&cps) {
        return Err("CPS must be between 1 and 20,000".into());
    }
    let button = MouseButton::parse(&button)
        .ok_or_else(|| "button must be left, right, middle, x1, or x2".to_string())?;

    if state.running.swap(true, Ordering::AcqRel) {
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

    let running = Arc::clone(&state.running);
    let stop = Arc::clone(&state.stop);
    let clicks = Arc::clone(&state.clicks);

    std::thread::Builder::new()
        .name("vxclick-precision-worker".into())
        .spawn(move || {
            let config = LiveClickerConfig {
                cps,
                button,
                ..LiveClickerConfig::default()
            };
            let result = PrecisionClicker::new()
                .and_then(|engine| engine.run_until_stopped(config, &stop, &clicks));
            if let Err(error) = result {
                eprintln!("precision worker stopped: {error}");
            }
            running.store(false, Ordering::Release);
        })
        .map_err(|error| {
            state.running.store(false, Ordering::Release);
            format!("failed to start precision worker: {error}")
        })?;

    Ok(())
}

#[tauri::command]
fn stop_clicker(state: State<'_, EngineState>) {
    state.stop.store(true, Ordering::Release);
}

fn main() {
    tauri::Builder::default()
        .manage(EngineState::default())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            start_clicker,
            stop_clicker
        ])
        .run(tauri::generate_context!())
        .expect("error while running VxClick");
}
