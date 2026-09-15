use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use auto_clicker_core::engine::MouseButton;
use auto_clicker_core::platform::windows::{CapturedInput, CapturedInputKind};
use serde::Serialize;

const CAPTURE_EVENT_SUPPRESSION: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct HotkeyCaptureResult {
    pub request_id: String,
    pub binding: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CaptureOutcome {
    NotCapturing,
    Capturing,
    Completed(HotkeyCaptureResult),
}

#[derive(Debug, Default, Clone, Copy)]
struct ModifierState {
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
}

impl ModifierState {
    fn set(&mut self, virtual_key: u16, down: bool) -> bool {
        match virtual_key {
            0x11 | 0xA2 | 0xA3 => self.ctrl = down,
            0x12 | 0xA4 | 0xA5 => self.alt = down,
            0x10 | 0xA0 | 0xA1 => self.shift = down,
            0x5B | 0x5C => self.win = down,
            _ => return false,
        }
        true
    }

    fn binding(&self, main: Option<&str>) -> Option<String> {
        let mut parts = Vec::with_capacity(5);
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.win {
            parts.push("Win".to_string());
        }
        if let Some(main) = main {
            parts.push(main.to_string());
        }
        (!parts.is_empty()).then(|| parts.join(" + "))
    }
}

#[derive(Debug, Default)]
struct CaptureSession {
    request_id: Option<String>,
    modifiers: ModifierState,
    suppress_until: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
pub struct HotkeyCaptureController {
    inner: Arc<Mutex<CaptureSession>>,
}

impl HotkeyCaptureController {
    pub fn start(&self, request_id: String) -> Result<(), String> {
        let request_id = request_id.trim();
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("hotkey capture request id must contain 1-128 characters".into());
        }
        let mut session = self
            .inner
            .lock()
            .map_err(|_| "hotkey capture mutex poisoned".to_string())?;
        if session.request_id.is_some() {
            return Err("another hotkey capture is already active".into());
        }
        session.request_id = Some(request_id.to_string());
        session.modifiers = ModifierState::default();
        session.suppress_until = None;
        Ok(())
    }

    pub fn cancel(&self, request_id: &str) -> Result<(), String> {
        let mut session = self
            .inner
            .lock()
            .map_err(|_| "hotkey capture mutex poisoned".to_string())?;
        if session.request_id.as_deref() == Some(request_id) {
            session.request_id = None;
            session.modifiers = ModifierState::default();
        }
        Ok(())
    }

    pub fn blocks_hotkey_execution(&self) -> bool {
        let Ok(mut session) = self.inner.lock() else {
            return false;
        };
        if session.request_id.is_some() {
            return true;
        }
        if let Some(until) = session.suppress_until {
            if Instant::now() < until {
                return true;
            }
            session.suppress_until = None;
        }
        false
    }

    pub fn process_input(&self, input: &CapturedInput) -> CaptureOutcome {
        let Ok(mut session) = self.inner.lock() else {
            return CaptureOutcome::NotCapturing;
        };
        if session.request_id.is_none() {
            return CaptureOutcome::NotCapturing;
        }

        match input.kind {
            CapturedInputKind::KeyDown { virtual_key, .. } => {
                if virtual_key == 0x1B {
                    return complete(&mut session, None);
                }
                if session.modifiers.set(virtual_key, true) {
                    return CaptureOutcome::Capturing;
                }
                let Some(main) = virtual_key_token(virtual_key) else {
                    return CaptureOutcome::Capturing;
                };
                let Some(binding) = session.modifiers.binding(Some(&main)) else {
                    return CaptureOutcome::Capturing;
                };
                complete(&mut session, Some(binding))
            }
            CapturedInputKind::KeyUp { virtual_key, .. } => {
                if is_modifier(virtual_key) {
                    let binding = session.modifiers.binding(None);
                    if let Some(binding) = binding {
                        return complete(&mut session, Some(binding));
                    }
                    session.modifiers.set(virtual_key, false);
                }
                CaptureOutcome::Capturing
            }
            CapturedInputKind::MouseDown(button) => {
                let Some(binding) = session.modifiers.binding(Some(mouse_button_token(button)))
                else {
                    return CaptureOutcome::Capturing;
                };
                complete(&mut session, Some(binding))
            }
            CapturedInputKind::MouseUp(_)
            | CapturedInputKind::MouseMove { .. }
            | CapturedInputKind::MouseWheel { .. } => CaptureOutcome::Capturing,
        }
    }
}

fn complete(session: &mut CaptureSession, binding: Option<String>) -> CaptureOutcome {
    let Some(request_id) = session.request_id.take() else {
        return CaptureOutcome::NotCapturing;
    };
    session.modifiers = ModifierState::default();
    session.suppress_until = Some(Instant::now() + CAPTURE_EVENT_SUPPRESSION);
    CaptureOutcome::Completed(HotkeyCaptureResult {
        request_id,
        binding,
    })
}

fn is_modifier(virtual_key: u16) -> bool {
    matches!(
        virtual_key,
        0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5
    )
}

fn mouse_button_token(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "Mouse1",
        MouseButton::Right => "Mouse2",
        MouseButton::Middle => "Mouse3",
        MouseButton::X1 => "Mouse4",
        MouseButton::X2 => "Mouse5",
    }
}

fn virtual_key_token(virtual_key: u16) -> Option<String> {
    if (b'A' as u16..=b'Z' as u16).contains(&virtual_key)
        || (b'0' as u16..=b'9' as u16).contains(&virtual_key)
    {
        return char::from_u32(virtual_key as u32).map(|value| value.to_string());
    }
    if (0x70..=0x87).contains(&virtual_key) {
        return Some(format!("F{}", virtual_key - 0x70 + 1));
    }
    if (0x60..=0x69).contains(&virtual_key) {
        return Some(format!("Numpad{}", virtual_key - 0x60));
    }
    let token = match virtual_key {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x13 => "Pause",
        0x14 => "CapsLock",
        0x20 => "Space",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2C => "PrintScreen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x90 => "NumLock",
        0x91 => "ScrollLock",
        _ => return None,
    };
    Some(token.to_string())
}

#[cfg(test)]
mod tests {
    use auto_clicker_core::platform::windows::{
        CapturedInput, CapturedInputKind, CapturedInputSource,
    };

    use super::{CaptureOutcome, HotkeyCaptureController, virtual_key_token};

    fn input(kind: CapturedInputKind) -> CapturedInput {
        CapturedInput {
            qpc_ticks: 0,
            source: CapturedInputSource::Physical,
            kind,
        }
    }

    #[test]
    fn captures_plain_keyboard_key() {
        let capture = HotkeyCaptureController::default();
        capture.start("plain".into()).unwrap();
        let result = capture.process_input(&input(CapturedInputKind::KeyDown {
            virtual_key: b'G' as u16,
            scan_code: 0x22,
            extended: false,
        }));
        assert_eq!(
            result,
            CaptureOutcome::Completed(super::HotkeyCaptureResult {
                request_id: "plain".into(),
                binding: Some("G".into()),
            })
        );
    }

    #[test]
    fn captures_modifier_chord() {
        let capture = HotkeyCaptureController::default();
        capture.start("test".into()).unwrap();
        assert_eq!(
            capture.process_input(&input(CapturedInputKind::KeyDown {
                virtual_key: 0xA2,
                scan_code: 0,
                extended: false,
            })),
            CaptureOutcome::Capturing
        );
        let result = capture.process_input(&input(CapturedInputKind::KeyDown {
            virtual_key: b'K' as u16,
            scan_code: 0,
            extended: false,
        }));
        assert_eq!(
            result,
            CaptureOutcome::Completed(super::HotkeyCaptureResult {
                request_id: "test".into(),
                binding: Some("Ctrl + K".into()),
            })
        );
    }

    #[test]
    fn escape_cancels_capture() {
        let capture = HotkeyCaptureController::default();
        capture.start("cancel".into()).unwrap();
        assert_eq!(
            capture.process_input(&input(CapturedInputKind::KeyDown {
                virtual_key: 0x1B,
                scan_code: 0,
                extended: false,
            })),
            CaptureOutcome::Completed(super::HotkeyCaptureResult {
                request_id: "cancel".into(),
                binding: None,
            })
        );
    }

    #[test]
    fn rejects_overlapping_capture_sessions() {
        let capture = HotkeyCaptureController::default();
        capture.start("first".into()).unwrap();
        assert!(capture.start("second".into()).is_err());
        capture.cancel("first").unwrap();
        assert!(capture.start("second".into()).is_ok());
    }

    #[test]
    fn key_tokens_match_supported_hotkey_parser_names() {
        assert_eq!(virtual_key_token(b'A' as u16).as_deref(), Some("A"));
        assert_eq!(virtual_key_token(0x7B).as_deref(), Some("F12"));
        assert_eq!(virtual_key_token(0x64).as_deref(), Some("Numpad4"));
        assert_eq!(virtual_key_token(0x20).as_deref(), Some("Space"));
    }
}
