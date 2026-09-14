use std::time::Duration;

use auto_clicker_core::engine::{BenchmarkStats, MouseButton, SampleStats};
use auto_clicker_core::platform::windows::{ClickerConfig, PrecisionClicker};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SampleReport {
    mean_us: f64,
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    worst_us: f64,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkReport {
    target_cps: f64,
    actual_cps: f64,
    deviation_percent: f64,
    elapsed_seconds: f64,
    clicks: u64,
    missed_deadlines: u64,
    interval: SampleReport,
    jitter: SampleReport,
}

#[tauri::command]
pub async fn run_precision_benchmark(
    cps: f64,
    duration_ms: u64,
    button: String,
) -> Result<BenchmarkReport, String> {
    if !cps.is_finite() || !(1.0..=20_000.0).contains(&cps) {
        return Err("CPS must be between 1 and 20,000".into());
    }
    if !(100..=30_000).contains(&duration_ms) {
        return Err("benchmark duration must be between 100 ms and 30 seconds".into());
    }
    let button = MouseButton::parse(&button)
        .ok_or_else(|| "button must be left, right, middle, x1, or x2".to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        let engine = PrecisionClicker::new()?;
        let stats = engine.run(ClickerConfig {
            cps,
            duration: Duration::from_millis(duration_ms),
            button,
            ..ClickerConfig::default()
        })?;
        Ok(BenchmarkReport::from(stats))
    })
    .await
    .map_err(|error| format!("benchmark worker failed: {error}"))?
}

impl From<BenchmarkStats> for BenchmarkReport {
    fn from(stats: BenchmarkStats) -> Self {
        let deviation_percent = if stats.target_cps > 0.0 {
            (stats.actual_cps - stats.target_cps) / stats.target_cps * 100.0
        } else {
            0.0
        };
        Self {
            target_cps: stats.target_cps,
            actual_cps: stats.actual_cps,
            deviation_percent,
            elapsed_seconds: stats.elapsed_seconds,
            clicks: stats.clicks,
            missed_deadlines: stats.missed_deadlines,
            interval: stats.interval.into(),
            jitter: stats.jitter.into(),
        }
    }
}

impl From<SampleStats> for SampleReport {
    fn from(stats: SampleStats) -> Self {
        Self {
            mean_us: stats.mean_us,
            p50_us: stats.p50_us,
            p95_us: stats.p95_us,
            p99_us: stats.p99_us,
            worst_us: stats.worst_us,
        }
    }
}
