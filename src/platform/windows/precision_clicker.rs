use std::hint::spin_loop;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, HIGH_PRIORITY_CLASS, SetPriorityClass, SetThreadPriority,
    THREAD_PRIORITY_HIGHEST,
};

use crate::engine::{BenchmarkStats, MouseButton};

use super::{QpcClock, WindowsInput};

#[derive(Debug, Clone, Copy)]
pub struct ClickerConfig {
    pub cps: f64,
    pub duration: Duration,
    pub button: MouseButton,
    pub coarse_wait_threshold_us: f64,
    pub spin_window_us: f64,
}

impl Default for ClickerConfig {
    fn default() -> Self {
        Self {
            cps: 100.0,
            duration: Duration::from_secs(5),
            button: MouseButton::Left,
            coarse_wait_threshold_us: 2_000.0,
            spin_window_us: 350.0,
        }
    }
}

pub struct PrecisionClicker {
    clock: QpcClock,
    input: WindowsInput,
}

impl PrecisionClicker {
    pub fn new() -> Result<Self, String> {
        tune_current_worker();
        Ok(Self {
            clock: QpcClock::new()?,
            input: WindowsInput,
        })
    }

    pub fn run(&self, config: ClickerConfig) -> Result<BenchmarkStats, String> {
        validate(config)?;

        let frequency = self.clock.frequency() as f64;
        let interval_ticks = frequency / config.cps;
        let run_ticks = config.duration.as_secs_f64() * frequency;
        let start = self.clock.now_ticks();
        let stop_at = start as f64 + run_ticks;
        let mut deadline = start as f64;
        let expected_clicks = (config.cps * config.duration.as_secs_f64()).ceil() as usize;
        let mut click_timestamps_us = Vec::with_capacity(expected_clicks.min(2_000_000));
        let mut deadline_errors_us = Vec::with_capacity(expected_clicks.min(2_000_000));
        let mut missed_deadlines = 0_u64;

        while deadline < stop_at {
            wait_until(
                &self.clock,
                deadline.round() as i64,
                config.coarse_wait_threshold_us,
                config.spin_window_us,
            );

            let before_send = self.clock.now_ticks();
            let late_ticks = before_send - deadline.round() as i64;
            let late_us = self.clock.ticks_to_micros(late_ticks);
            if late_us > (1_000_000.0 / config.cps) {
                missed_deadlines += 1;
            }

            self.input.click(config.button)?;
            let after_send = self.clock.now_ticks();
            click_timestamps_us.push(self.clock.ticks_to_micros(after_send - start));
            deadline_errors_us.push(late_us);

            deadline += interval_ticks;
        }

        let end = self.clock.now_ticks();
        let elapsed_seconds = (end - start) as f64 / frequency;
        Ok(BenchmarkStats::from_samples(
            config.cps,
            elapsed_seconds,
            &click_timestamps_us,
            &deadline_errors_us,
            missed_deadlines,
        ))
    }
}

fn validate(config: ClickerConfig) -> Result<(), String> {
    if !config.cps.is_finite() || config.cps <= 0.0 {
        return Err("CPS must be a positive finite value".into());
    }
    if config.cps > 20_000.0 {
        return Err(
            "v0.1 safety cap is 20,000 CPS; raise it only after benchmark validation".into(),
        );
    }
    if config.duration.is_zero() {
        return Err("duration must be greater than zero".into());
    }
    Ok(())
}

#[inline]
fn wait_until(clock: &QpcClock, deadline: i64, coarse_threshold_us: f64, spin_window_us: f64) {
    loop {
        let now = clock.now_ticks();
        if now >= deadline {
            return;
        }

        let remaining_us = clock.ticks_to_micros(deadline - now);
        if remaining_us > coarse_threshold_us {
            let sleep_us = (remaining_us - spin_window_us).max(100.0) as u64;
            thread::sleep(Duration::from_micros(sleep_us.min(1_000)));
        } else if remaining_us > spin_window_us {
            thread::yield_now();
        } else {
            while clock.now_ticks() < deadline {
                spin_loop();
            }
            return;
        }
    }
}

fn tune_current_worker() {
    unsafe {
        let _ = SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
    }
}
