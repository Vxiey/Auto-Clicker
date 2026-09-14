use std::mem::{size_of, zeroed};

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
        send(&inputs)
    }

    #[inline]
    pub fn mouse_down(&self, button: MouseButton) -> Result<(), String> {
        let (flags, _, data) = mouse_button_flags(button);
        send(&[mouse_input(0, 0, flags, data)])
    }

    #[inline]
    pub fn mouse_up(&self, button: MouseButton) -> Result<(), String> {
        let (_, flags, data) = mouse_button_flags(button);
        send(&[mouse_input(0, 0, flags, data)])
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
        send(&[keyboard_input(virtual_key, false)])
    }

    #[inline]
    pub fn key_up(&self, virtual_key: u16) -> Result<(), String> {
        send(&[keyboard_input(virtual_key, true)])
    }
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
        // Some special/OEM keys do not have a useful scan-code mapping. Keep a
        // virtual-key fallback instead of silently emitting an invalid event.
        (virtual_key, 0, base_flags)
    } else {
        let scan = (raw_scan & 0xff) as u16;
        // MAPVK_VK_TO_VSC_EX normally returns 0xE0/0xE1 in the high byte for
        // extended keys. Keep an explicit VK fallback because some Windows
        // environments/drivers only return the base scan code.
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
        0x21 // VK_PRIOR / Page Up
            | 0x22 // VK_NEXT / Page Down
            | 0x23 // VK_END
            | 0x24 // VK_HOME
            | 0x25 // VK_LEFT
            | 0x26 // VK_UP
            | 0x27 // VK_RIGHT
            | 0x28 // VK_DOWN
            | 0x2D // VK_INSERT
            | 0x2E // VK_DELETE
            | 0x6F // VK_DIVIDE
            | 0x90 // VK_NUMLOCK
            | 0xA3 // VK_RCONTROL
            | 0xA5 // VK_RMENU / Right Alt
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
