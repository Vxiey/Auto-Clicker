use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};

use windows_sys::Win32::System::Performance::QueryPerformanceCounter;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLMHF_INJECTED, MSLLHOOKSTRUCT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN,
    WM_XBUTTONUP,
};

use crate::engine::MouseButton;
use crate::platform::windows::input::INJECTED_INPUT_TAG;

const EVENT_QUEUE_CAPACITY: usize = 16_384;
const XBUTTON1_DATA: u32 = 1;
const XBUTTON2_DATA: u32 = 2;

static EVENT_SENDER: OnceLock<Mutex<Option<SyncSender<CapturedInput>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturedInputSource {
    Physical,
    /// Input injected by another application or device utility. VxClick keeps
    /// this distinct from its own SendInput events so remaps can be accepted.
    ExternalInjected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedInput {
    pub qpc_ticks: i64,
    pub source: CapturedInputSource,
    pub kind: CapturedInputKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapturedInputKind {
    KeyDown {
        virtual_key: u16,
        scan_code: u16,
        extended: bool,
    },
    KeyUp {
        virtual_key: u16,
        scan_code: u16,
        extended: bool,
    },
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseWheel {
        delta: i16,
    },
}

pub struct GlobalInputRecorder {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl GlobalInputRecorder {
    pub fn start() -> Result<(Self, Receiver<CapturedInput>), String> {
        let (event_tx, event_rx) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let slot = EVENT_SENDER.get_or_init(|| Mutex::new(None));
        {
            let mut sender = slot
                .lock()
                .map_err(|_| "global input sender mutex was poisoned".to_string())?;
            if sender.is_some() {
                return Err("global input recorder is already running".into());
            }
            *sender = Some(event_tx);
        }

        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<u32, String>>(1);
        let thread = thread::Builder::new()
            .name("vxclick-input-recorder".into())
            .spawn(move || hook_thread(ready_tx))
            .map_err(|error| {
                clear_sender();
                format!("failed to create input recorder thread: {error}")
            })?;

        match ready_rx.recv() {
            Ok(Ok(thread_id)) => Ok((
                Self {
                    thread_id,
                    thread: Some(thread),
                },
                event_rx,
            )),
            Ok(Err(error)) => {
                let _ = thread.join();
                clear_sender();
                Err(error)
            }
            Err(error) => {
                let _ = thread.join();
                clear_sender();
                Err(format!(
                    "input recorder failed before initialization: {error}"
                ))
            }
        }
    }

    pub fn stop(&mut self) {
        if self.thread_id != 0 {
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            self.thread_id = 0;
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        clear_sender();
    }
}

impl Drop for GlobalInputRecorder {
    fn drop(&mut self) {
        self.stop();
    }
}

fn hook_thread(ready_tx: SyncSender<Result<u32, String>>) {
    let thread_id = unsafe { GetCurrentThreadId() };

    // Ensure this thread owns a message queue before another thread tries to post WM_QUIT.
    let mut message: MSG = unsafe { zeroed() };
    unsafe {
        PeekMessageW(&mut message, null_mut(), 0, 0, PM_NOREMOVE);
    }

    let keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), null_mut(), 0) };
    if keyboard.is_null() {
        let _ = ready_tx.send(Err("SetWindowsHookExW(WH_KEYBOARD_LL) failed".into()));
        return;
    }

    let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), null_mut(), 0) };
    if mouse.is_null() {
        unsafe {
            UnhookWindowsHookEx(keyboard);
        }
        let _ = ready_tx.send(Err("SetWindowsHookExW(WH_MOUSE_LL) failed".into()));
        return;
    }

    if ready_tx.send(Ok(thread_id)).is_err() {
        unsafe {
            UnhookWindowsHookEx(mouse);
            UnhookWindowsHookEx(keyboard);
        }
        return;
    }

    unsafe {
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {}
        UnhookWindowsHookEx(mouse);
        UnhookWindowsHookEx(keyboard);
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        let data = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if data.dwExtraInfo != INJECTED_INPUT_TAG {
            let source = if (data.flags & LLKHF_INJECTED) != 0 {
                CapturedInputSource::ExternalInjected
            } else {
                CapturedInputSource::Physical
            };
            let extended = (data.flags & LLKHF_EXTENDED) != 0;
            let kind = match wparam as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(CapturedInputKind::KeyDown {
                    virtual_key: data.vkCode as u16,
                    scan_code: data.scanCode as u16,
                    extended,
                }),
                WM_KEYUP | WM_SYSKEYUP => Some(CapturedInputKind::KeyUp {
                    virtual_key: data.vkCode as u16,
                    scan_code: data.scanCode as u16,
                    extended,
                }),
                _ => None,
            };
            if let Some(kind) = kind {
                publish(source, kind);
            }
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        let data = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        if data.dwExtraInfo != INJECTED_INPUT_TAG {
            let source = if (data.flags & LLMHF_INJECTED) != 0 {
                CapturedInputSource::ExternalInjected
            } else {
                CapturedInputSource::Physical
            };
            let xbutton = (data.mouseData >> 16) & 0xffff;
            let kind = match wparam as u32 {
                WM_MOUSEMOVE => Some(CapturedInputKind::MouseMove {
                    x: data.pt.x,
                    y: data.pt.y,
                }),
                WM_LBUTTONDOWN => Some(CapturedInputKind::MouseDown(MouseButton::Left)),
                WM_LBUTTONUP => Some(CapturedInputKind::MouseUp(MouseButton::Left)),
                WM_RBUTTONDOWN => Some(CapturedInputKind::MouseDown(MouseButton::Right)),
                WM_RBUTTONUP => Some(CapturedInputKind::MouseUp(MouseButton::Right)),
                WM_MBUTTONDOWN => Some(CapturedInputKind::MouseDown(MouseButton::Middle)),
                WM_MBUTTONUP => Some(CapturedInputKind::MouseUp(MouseButton::Middle)),
                WM_XBUTTONDOWN => {
                    mouse_button_from_xdata(xbutton).map(CapturedInputKind::MouseDown)
                }
                WM_XBUTTONUP => mouse_button_from_xdata(xbutton).map(CapturedInputKind::MouseUp),
                WM_MOUSEWHEEL => Some(CapturedInputKind::MouseWheel {
                    delta: ((data.mouseData >> 16) & 0xffff) as u16 as i16,
                }),
                _ => None,
            };
            if let Some(kind) = kind {
                publish(source, kind);
            }
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

fn mouse_button_from_xdata(value: u32) -> Option<MouseButton> {
    match value {
        XBUTTON1_DATA => Some(MouseButton::X1),
        XBUTTON2_DATA => Some(MouseButton::X2),
        _ => None,
    }
}

fn publish(source: CapturedInputSource, kind: CapturedInputKind) {
    let mut qpc_ticks = 0_i64;
    unsafe {
        QueryPerformanceCounter(&mut qpc_ticks);
    }

    let Some(slot) = EVENT_SENDER.get() else {
        return;
    };
    let Ok(sender) = slot.lock() else {
        return;
    };
    let Some(sender) = sender.as_ref() else {
        return;
    };

    match sender.try_send(CapturedInput {
        qpc_ticks,
        source,
        kind,
    }) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}

fn clear_sender() {
    if let Some(slot) = EVENT_SENDER.get() {
        if let Ok(mut sender) = slot.lock() {
            *sender = None;
        }
    }
}
