use std::mem::{size_of, zeroed};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, SendInput,
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
        let inputs = [mouse_input(down_flags, data), mouse_input(up_flags, data)];
        send(&inputs)
    }

    #[inline]
    pub fn mouse_down(&self, button: MouseButton) -> Result<(), String> {
        let (flags, _, data) = mouse_button_flags(button);
        send(&[mouse_input(flags, data)])
    }

    #[inline]
    pub fn mouse_up(&self, button: MouseButton) -> Result<(), String> {
        let (_, flags, data) = mouse_button_flags(button);
        send(&[mouse_input(flags, data)])
    }

    #[inline]
    pub fn key_down(&self, virtual_key: u16) -> Result<(), String> {
        send(&[keyboard_input(virtual_key, 0)])
    }

    #[inline]
    pub fn key_up(&self, virtual_key: u16) -> Result<(), String> {
        send(&[keyboard_input(virtual_key, KEYEVENTF_KEYUP)])
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

fn mouse_input(flags: u32, data: u32) -> INPUT {
    let mut input: INPUT = unsafe { zeroed() };
    input.r#type = INPUT_MOUSE;
    input.Anonymous.mi = MOUSEINPUT {
        dx: 0,
        dy: 0,
        mouseData: data,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: INJECTED_INPUT_TAG,
    };
    input
}

fn keyboard_input(virtual_key: u16, flags: u32) -> INPUT {
    let mut input: INPUT = unsafe { zeroed() };
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: virtual_key,
        wScan: 0,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: INJECTED_INPUT_TAG,
    };
    input
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
