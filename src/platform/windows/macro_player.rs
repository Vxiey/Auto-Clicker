use std::collections::HashSet;
use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

use crate::engine::MouseButton;
use crate::macro_engine::InputAction;

use super::{QpcClock, WindowsInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroPlaybackStats {
    pub actions_executed: u64,
    pub elapsed_micros: u64,
}

pub struct MacroPlayer {
    clock: QpcClock,
    input: WindowsInput,
}

impl MacroPlayer {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            clock: QpcClock::new()?,
            input: WindowsInput,
        })
    }

    pub fn play(
        &self,
        actions: &[InputAction],
        stop: &AtomicBool,
        speed: f64,
    ) -> Result<MacroPlaybackStats, String> {
        if !speed.is_finite() || !(0.1..=10.0).contains(&speed) {
            return Err("macro playback speed must be between 0.1x and 10x".into());
        }

        let started = self.clock.now_ticks();
        let mut deadline = started;
        let mut executed = 0_u64;
        let mut held_keys = HashSet::new();
        let mut held_buttons = HashSet::new();

        let result: Result<(), String> = (|| {
            for action in actions {
                if stop.load(Ordering::Acquire) {
                    break;
                }

                match *action {
                    InputAction::WaitMicros(micros) => {
                        deadline = deadline.saturating_add(self.micros_to_ticks(micros, speed));
                        self.wait_until(deadline, stop);
                    }
                    InputAction::KeyDown(key) => {
                        self.input.key_down(key)?;
                        held_keys.insert(key);
                        executed += 1;
                    }
                    InputAction::KeyUp(key) => {
                        self.input.key_up(key)?;
                        held_keys.remove(&key);
                        executed += 1;
                    }
                    InputAction::MouseDown(button) => {
                        self.input.mouse_down(button)?;
                        held_buttons.insert(button);
                        executed += 1;
                    }
                    InputAction::MouseUp(button) => {
                        self.input.mouse_up(button)?;
                        held_buttons.remove(&button);
                        executed += 1;
                    }
                    InputAction::MouseMoveRelative { dx, dy } => {
                        self.input.move_relative(dx, dy)?;
                        executed += 1;
                    }
                    InputAction::MouseMoveAbsolute { x, y } => {
                        if unsafe { SetCursorPos(x, y) } == 0 {
                            return Err("SetCursorPos failed during macro playback".into());
                        }
                        executed += 1;
                    }
                    InputAction::MouseWheel { delta } => {
                        self.input.wheel(delta)?;
                        executed += 1;
                    }
                }
            }
            Ok(())
        })();

        release_held(&self.input, &mut held_keys, &mut held_buttons);
        result?;

        let elapsed = self.clock.now_ticks().saturating_sub(started);
        Ok(MacroPlaybackStats {
            actions_executed: executed,
            elapsed_micros: self.clock.ticks_to_micros(elapsed).max(0.0) as u64,
        })
    }

    fn micros_to_ticks(&self, micros: u64, speed: f64) -> i64 {
        ((micros as f64 / speed) * self.clock.frequency() as f64 / 1_000_000.0)
            .round()
            .clamp(0.0, i64::MAX as f64) as i64
    }

    fn wait_until(&self, deadline: i64, stop: &AtomicBool) {
        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }
            let now = self.clock.now_ticks();
            let remaining = deadline.saturating_sub(now);
            if remaining <= 0 {
                return;
            }

            let remaining_us = self.clock.ticks_to_micros(remaining);
            if remaining_us > 2_000.0 {
                let sleep_us = (remaining_us - 750.0).max(1.0) as u64;
                std::thread::sleep(Duration::from_micros(sleep_us));
            } else {
                spin_loop();
            }
        }
    }
}

fn release_held(input: &WindowsInput, keys: &mut HashSet<u16>, buttons: &mut HashSet<MouseButton>) {
    for key in keys.drain() {
        let _ = input.key_up(key);
    }
    for button in buttons.drain() {
        let _ = input.mouse_up(button);
    }
}

#[cfg(test)]
mod tests {
    use super::MacroPlayer;

    #[test]
    fn rejects_invalid_speed() {
        let player = MacroPlayer::new().expect("QPC should be available on Windows");
        let stop = std::sync::atomic::AtomicBool::new(false);
        assert!(player.play(&[], &stop, 0.0).is_err());
        assert!(player.play(&[], &stop, 10.1).is_err());
    }

    #[test]
    fn empty_macro_is_valid() {
        let player = MacroPlayer::new().expect("QPC should be available on Windows");
        let stop = std::sync::atomic::AtomicBool::new(false);
        let stats = player
            .play(&[], &stop, 1.0)
            .expect("empty macro should play");
        assert_eq!(stats.actions_executed, 0);
    }
}
