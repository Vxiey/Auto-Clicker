use std::collections::HashSet;
use std::mem::{size_of, zeroed};
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC_EX, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    MapVirtualKeyW, SendInput,
};

use crate::engine::MouseButton;

pub const INJECTED_INPUT_TAG: usize = 0x4155_434C_4943_4B52;
const XBUTTON1_DATA: u32 = 1;
const XBUTTON2_DATA: u32 = 2;
const MAX_BURST_CLICKS: usize = 16;

#[derive(Default)]
struct HeldInputState {
    keys: HashSet<u16>,
    buttons: HashSet<MouseButton>,
}

static HELD_INPUT: OnceLock<Mutex<HeldInputState>> = OnceLock::new();

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsInput;

impl WindowsInput {
    #[inline]
    pub fn click(&self, button: MouseButton) -> Result<(), String> {
        let (down_flags, up_flags, data) = mouse_button_flags(button);
        let inputs = [
            mouse_input(0, 0, down_flags, data),
            mouse_input(0, 0, up_flags, data),
        ];
        let result = send(&inputs);
        if result.is_err() {
            let _ = send(&[mouse_input(0, 0, up_flags, data)]);
        }
        result
    }

    #[inline]
    pub fn click_burst(&self, button: MouseButton, clicks: u32) -> Result<u32, String> {
        let clicks = clicks.clamp(1, MAX_BURST_CLICKS as u32) as usize;
        let (down_flags, up_flags, data) = mouse_button_flags(button);
        let mut inputs: [INPUT; MAX_BURST_CLICKS * 2] =
            std::array::from_fn(|_| unsafe { zeroed() });
        for index in 0..clicks {
            inputs[index * 2] = mouse_input(0, 0, down_flags, data);
            inputs[index * 2 + 1] = mouse_input(0, 0, up_flags, data);
        }
        let result = send(&inputs[..clicks * 2]);
        if result.is_err() {
            let _ = send(&[mouse_input(0, 0, up_flags, data)]);
        }
        result.map(|_| clicks as u32)
    }

    #[inline]
    pub fn mouse_down(&self, button: MouseButton) -> Result<(), String> {
        let (flags, _, data) = mouse_button_flags(button);
        send(&[mouse_input(0, 0, flags, data)])?;
        track_button(button, true);
        Ok(())
    }

    #[inline]
    pub fn mouse_up(&self, button: MouseButton) -> Result<(), String> {
        let (_, flags, data) = mouse_button_flags(button);
        send(&[mouse_input(0, 0, flags, data)])?;
        track_button(button, false);
        Ok(())
    }

    #[inline]
    pub fn move_relative(&self, dx: i32, dy: i32) -> Result<(), String> {
        send(&[mouse_input(dx, dy, MOUSEEVENTF_MOVE, 0)])
    }

    #[inline]
    pub fn wheel(&self, delta: i32) -> Result<(), String> {
        send(&[mouse_input(0, 0, MOUSEEVENTF_WHEEL, delta as u32)])
    }

    #[inline]
    pub fn key_down(&self, virtual_key: u16) -> Result<(), String> {
        send(&[keyboard_input(virtual_key, false)])?;
        track_key(virtual_key, true);
        Ok(())
    }

    #[inline]
    pub fn key_up(&self, virtual_key: u16) -> Result<(), String> {
        send(&[keyboard_input(virtual_key, true)])?;
        track_key(virtual_key, false);
        Ok(())
    }

    /// Best-effort safety release for every key or mouse button that VxClick
    /// has injected as down without a matching up event.
    pub fn release_all(&self) -> Result<(), String> {
        let (keys, buttons) = take_held_input();
        let mut first_error = None;

        for key in keys {
            if let Err(error) = send(&[keyboard_input(key, true)])
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        for button in buttons {
            let (_, up_flags, data) = mouse_button_flags(button);
            if let Err(error) = send(&[mouse_input(0, 0, up_flags, data)])
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }

    pub fn held_input_count(&self) -> usize {
        HELD_INPUT
            .get_or_init(|| Mutex::new(HeldInputState::default()))
            .lock()
            .map(|state| state.keys.len() + state.buttons.len())
            .unwrap_or(0)
    }
}

fn held_input() -> &'static Mutex<HeldInputState> {
    HELD_INPUT.get_or_init(|| Mutex::new(HeldInputState::default()))
}

fn track_key(key: u16, down: bool) {
    if let Ok(mut state) = held_input().lock() {
        if down {
            state.keys.insert(key);
        } else {
            state.keys.remove(&key);
        }
    }
}

fn track_button(button: MouseButton, down: bool) {
    if let Ok(mut state) = held_input().lock() {
        if down {
            state.buttons.insert(button);
        } else {
            state.buttons.remove(&button);
        }
    }
}

fn take_held_input() -> (Vec<u16>, Vec<MouseButton>) {
    let Ok(mut state) = held_input().lock() else {
        return (Vec::new(), Vec::new());
    };
    let keys = state.keys.drain().collect();
    let buttons = state.buttons.drain().collect();
    (keys, buttons)
}

fn mouse_button_flags(button: MouseButton) -> (u32, u32, u32) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
        MouseButton::X1 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON1_DATA),
        MouseButton::X2 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON2_DATA),
    }
}

fn mouse_input(dx: i32, dy: i32, flags: u32, data: u32) -> INPUT {
    let mut input: INPUT = unsafe { zeroed() };
    input.r#type = INPUT_MOUSE;
    input.Anonymous.mi = MOUSEINPUT {
        dx,
        dy,
        mouseData: data,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: INJECTED_INPUT_TAG,
    };
    input
}

fn keyboard_input(virtual_key: u16, key_up: bool) -> INPUT {
    let raw_scan = unsafe { MapVirtualKeyW(virtual_key as u32, MAPVK_VK_TO_VSC_EX) };
    let base_flags = if key_up { KEYEVENTF_KEYUP } else { 0 };

    let (w_vk, w_scan, flags) = if raw_scan == 0 {
        (virtual_key, 0, base_flags)
    } else {
        let scan = (raw_scan & 0xff) as u16;
        let extended = if (raw_scan >> 8) != 0 || is_extended_virtual_key(virtual_key) {
            KEYEVENTF_EXTENDEDKEY
        } else {
            0
        };
        (0, scan, base_flags | KEYEVENTF_SCANCODE | extended)
    };

    let mut input: INPUT = unsafe { zeroed() };
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: w_vk,
        wScan: w_scan,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: INJECTED_INPUT_TAG,
    };
    input
}

#[inline]
fn is_extended_virtual_key(virtual_key: u16) -> bool {
    matches!(
        virtual_key,
        0x21 | 0x22
            | 0x23
            | 0x24
            | 0x25
            | 0x26
            | 0x27
            | 0x28
            | 0x2D
            | 0x2E
            | 0x6F
            | 0x90
            | 0xA3
            | 0xA5
    )
}

#[inline]
fn send(inputs: &[INPUT]) -> Result<(), String> {
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!("SendInput sent {sent}/{} events", inputs.len()))
    }
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
    };

    use super::keyboard_input;

    #[test]
    fn key_up_keeps_scan_code_and_release_flags() {
        let input = keyboard_input(0x41, true);
        let keyboard = unsafe { input.Anonymous.ki };
        assert_ne!(keyboard.dwFlags & KEYEVENTF_KEYUP, 0);
        assert_ne!(keyboard.dwFlags & KEYEVENTF_SCANCODE, 0);
    }

    #[test]
    fn arrow_key_uses_extended_flag() {
        let input = keyboard_input(0x26, false);
        let keyboard = unsafe { input.Anonymous.ki };
        assert_ne!(keyboard.dwFlags & KEYEVENTF_SCANCODE, 0);
        assert_ne!(keyboard.dwFlags & KEYEVENTF_EXTENDEDKEY, 0);
    }
}
