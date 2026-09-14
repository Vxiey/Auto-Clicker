use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use windows_sys::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

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

#[derive(Debug, Clone)]
pub struct LiveClickerConfig {
    pub cps: f64,
    pub button: MouseButton,
    pub coarse_wait_threshold_us: f64,
    pub spin_window_us: f64,
    pub randomize_percent: f64,
    pub burst_size: u32,
    /// Empty means current cursor. One point is fixed-position mode; multiple
    /// points are cycled deterministically without allocating in the hot loop.
    pub positions: Vec<(i32, i32)>,
}

impl Default for LiveClickerConfig {
    fn default() -> Self {
        Self {
            cps: 100.0,
            button: MouseButton::Left,
            coarse_wait_threshold_us: 2_000.0,
            spin_window_us: 350.0,
            randomize_percent: 0.0,
            burst_size: 1,
            positions: Vec::new(),
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
        validate_rate(config.cps)?;
        if config.duration.is_zero() {
            return Err("duration must be greater than zero".into());
        }

        let frequency = self.clock.frequency() as f64;
        let interval_ticks = frequency / config.cps;
        let run_ticks = config.duration.as_secs_f64() * frequency;
        let start = self.clock.now_ticks();
        let stop_at = start.saturating_add(run_ticks.ceil() as i64);
        let mut deadline = start as f64;
        let expected_clicks = (config.cps * config.duration.as_secs_f64()).ceil() as usize;
        let max_clicks = expected_clicks.saturating_add(1);
        let mut click_timestamps_us = Vec::with_capacity(expected_clicks.min(2_000_000));
        let mut deadline_errors_us = Vec::with_capacity(expected_clicks.min(2_000_000));
        let mut missed_deadlines = 0_u64;
        let spin_window_us = effective_spin_window(config.spin_window_us, config.cps);

        while deadline < stop_at as f64 && click_timestamps_us.len() < max_clicks {
            if self.clock.now_ticks() >= stop_at {
                break;
            }

            wait_until(
                &self.clock,
                (deadline.round() as i64).min(stop_at),
                config.coarse_wait_threshold_us,
                spin_window_us,
            );

            let before_send = self.clock.now_ticks();
            if before_send >= stop_at {
                break;
            }

            let late_ticks = before_send - deadline.round() as i64;
            let late_us = self.clock.ticks_to_micros(late_ticks);
            if late_us > (1_000_000.0 / config.cps) {
                missed_deadlines += 1;
            }

            self.input.click(config.button)?;
            let after_send = self.clock.now_ticks();
            click_timestamps_us.push(self.clock.ticks_to_micros(after_send - start));
            deadline_errors_us.push(late_us);

            deadline = next_deadline(deadline, interval_ticks, after_send as f64);
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

    pub fn run_until_stopped(
        &self,
        config: LiveClickerConfig,
        stop: &AtomicBool,
        click_counter: &AtomicU64,
    ) -> Result<(), String> {
        validate_live_config(&config)?;

        let frequency = self.clock.frequency() as f64;
        let base_interval_ticks = frequency / config.cps;
        let burst_size = config.burst_size.clamp(1, 16);
        let spin_window_us = effective_spin_window(config.spin_window_us, config.cps);
        let mut deadline = self.clock.now_ticks() as f64;
        let mut rng = (self.clock.now_ticks() as u64) ^ 0x9E37_79B9_7F4A_7C15;
        let mut position_index = 0_usize;

        while !stop.load(Ordering::Acquire) {
            if !wait_until_controlled(
                &self.clock,
                deadline.round() as i64,
                config.coarse_wait_threshold_us,
                spin_window_us,
                stop,
            ) {
                break;
            }

            if !config.positions.is_empty() {
                let (x, y) = config.positions[position_index % config.positions.len()];
                if unsafe { SetCursorPos(x, y) } == 0 {
                    return Err(format!("SetCursorPos failed for target {x},{y}"));
                }
                position_index = position_index.wrapping_add(1);
            }

            let produced = if burst_size > 1 {
                self.input.click_burst(config.button, burst_size)? as u64
            } else {
                self.input.click(config.button)?;
                1
            };
            click_counter.fetch_add(produced, Ordering::Relaxed);

            let factor = randomized_interval_factor(&mut rng, config.randomize_percent);
            let group_interval = base_interval_ticks * produced as f64 * factor;
            let now = self.clock.now_ticks() as f64;
            deadline = next_deadline(deadline, group_interval, now);
        }

        Ok(())
    }
}

fn validate_rate(cps: f64) -> Result<(), String> {
    if !cps.is_finite() || cps <= 0.0 {
        return Err("CPS must be a positive finite value".into());
    }
    if cps > 20_000.0 {
        return Err("safety cap is 20,000 CPS; raise it only after benchmark validation".into());
    }
    Ok(())
}

fn validate_live_config(config: &LiveClickerConfig) -> Result<(), String> {
    validate_rate(config.cps)?;
    if !config.randomize_percent.is_finite() || !(0.0..=50.0).contains(&config.randomize_percent) {
        return Err("randomization must be between 0 and 50 percent".into());
    }
    if !(1..=16).contains(&config.burst_size) {
        return Err("burst size must be between 1 and 16 clicks".into());
    }
    if config.positions.len() > 256 {
        return Err("multi-point mode supports at most 256 positions".into());
    }
    Ok(())
}

#[inline]
fn next_deadline(previous_deadline: f64, interval: f64, now: f64) -> f64 {
    let scheduled = previous_deadline + interval;
    if now >= scheduled {
        now + interval
    } else {
        scheduled
    }
}

#[inline]
fn effective_spin_window(configured_us: f64, cps: f64) -> f64 {
    let interval_us = 1_000_000.0 / cps;
    configured_us.min((interval_us * 0.25).clamp(50.0, 350.0))
}

#[inline]
fn controlled_sleep_cap_us(remaining_us: f64) -> u64 {
    if remaining_us > 50_000.0 {
        20_000
    } else if remaining_us > 10_000.0 {
        5_000
    } else {
        1_000
    }
}

#[inline]
fn randomized_interval_factor(state: &mut u64, percent: f64) -> f64 {
    if percent <= 0.0 {
        return 1.0;
    }
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    let sample = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
    let unit = (sample >> 11) as f64 / ((1_u64 << 53) as f64);
    let amplitude = percent / 100.0;
    1.0 + ((unit * 2.0) - 1.0) * amplitude
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

#[inline]
fn wait_until_controlled(
    clock: &QpcClock,
    deadline: i64,
    coarse_threshold_us: f64,
    spin_window_us: f64,
    stop: &AtomicBool,
) -> bool {
    loop {
        if stop.load(Ordering::Acquire) {
            return false;
        }

        let now = clock.now_ticks();
        if now >= deadline {
            return true;
        }

        let remaining_us = clock.ticks_to_micros(deadline - now);
        if remaining_us > coarse_threshold_us {
            let sleep_us = (remaining_us - spin_window_us).max(100.0) as u64;
            thread::sleep(Duration::from_micros(
                sleep_us.min(controlled_sleep_cap_us(remaining_us)),
            ));
        } else if remaining_us > spin_window_us {
            thread::yield_now();
        } else {
            let mut spins = 0_u32;
            while clock.now_ticks() < deadline {
                spin_loop();
                spins = spins.wrapping_add(1);
                if spins & 0x3f == 0 && stop.load(Ordering::Acquire) {
                    return false;
                }
            }
            return true;
        }
    }
}

fn tune_current_worker() {
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LiveClickerConfig, controlled_sleep_cap_us, effective_spin_window, next_deadline,
        randomized_interval_factor, validate_live_config,
    };

    #[test]
    fn randomization_stays_inside_requested_band() {
        let mut state = 123_u64;
        for _ in 0..10_000 {
            let factor = randomized_interval_factor(&mut state, 5.0);
            assert!((0.95..=1.05).contains(&factor));
        }
    }

    #[test]
    fn validates_burst_and_position_limits() {
        let invalid_burst = LiveClickerConfig {
            burst_size: 17,
            ..LiveClickerConfig::default()
        };
        assert!(validate_live_config(&invalid_burst).is_err());

        let too_many_positions = LiveClickerConfig {
            positions: vec![(0, 0); 257],
            ..LiveClickerConfig::default()
        };
        assert!(validate_live_config(&too_many_positions).is_err());
    }

    #[test]
    fn missed_deadline_never_schedules_catch_up_click() {
        assert_eq!(next_deadline(1_000.0, 100.0, 1_250.0), 1_350.0);
        assert_eq!(next_deadline(1_000.0, 100.0, 1_050.0), 1_100.0);
    }

    #[test]
    fn high_cps_reduces_busy_spin_window() {
        assert_eq!(effective_spin_window(350.0, 10_000.0), 50.0);
        assert_eq!(effective_spin_window(350.0, 250.0), 350.0);
    }

    #[test]
    fn long_intervals_reduce_wakeup_frequency() {
        assert_eq!(controlled_sleep_cap_us(100_000.0), 20_000);
        assert_eq!(controlled_sleep_cap_us(20_000.0), 5_000);
        assert_eq!(controlled_sleep_cap_us(5_000.0), 1_000);
    }
}
