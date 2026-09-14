use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Mutex, OnceLock, RwLock};

use crate::engine::MouseButton;
use crate::hotkeys::{HotkeyBinding, InputSourcePolicy};

const PHYSICAL_DOWN: u8 = 0b01;
const EXTERNAL_INJECTED_DOWN: u8 = 0b10;
const HOTKEY_EVENT_QUEUE_CAPACITY: usize = 256;

const VK_LBUTTON: u16 = 0x01;
const VK_RBUTTON: u16 = 0x02;
const VK_MBUTTON: u16 = 0x04;
const VK_XBUTTON1: u16 = 0x05;
const VK_XBUTTON2: u16 = 0x06;
const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_MENU: u16 = 0x12;
const VK_LSHIFT: u16 = 0xA0;
const VK_RSHIFT: u16 = 0xA1;
const VK_LCONTROL: u16 = 0xA2;
const VK_RCONTROL: u16 = 0xA3;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;

static KEY_STATE: OnceLock<[AtomicU8; 256]> = OnceLock::new();
static BINDINGS: OnceLock<RwLock<Vec<RegisteredHotkey>>> = OnceLock::new();
static EVENT_SENDER: OnceLock<Mutex<Option<SyncSender<HotkeyEvent>>>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredHotkey {
    pub id: u64,
    pub binding: HotkeyBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyPhase {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyEvent {
    pub id: u64,
    pub phase: HotkeyPhase,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsHotkeyManager;

impl WindowsHotkeyManager {
    pub fn subscribe() -> Result<Receiver<HotkeyEvent>, String> {
        let (sender, receiver) = mpsc::sync_channel(HOTKEY_EVENT_QUEUE_CAPACITY);
        let slot = EVENT_SENDER.get_or_init(|| Mutex::new(None));
        let mut guard = slot
            .lock()
            .map_err(|_| "hotkey event sender mutex was poisoned".to_string())?;
        if guard.is_some() {
            return Err("hotkey event receiver is already registered".into());
        }
        *guard = Some(sender);
        Ok(receiver)
    }

    pub fn replace_bindings(bindings: Vec<RegisteredHotkey>) -> Result<(), String> {
        validate_bindings(&bindings)?;
        let registry = BINDINGS.get_or_init(|| RwLock::new(Vec::new()));
        let mut guard = registry
            .write()
            .map_err(|_| "hotkey binding registry was poisoned".to_string())?;
        *guard = bindings;
        Ok(())
    }

    pub fn clear_bindings() {
        if let Some(registry) = BINDINGS.get()
            && let Ok(mut guard) = registry.write()
        {
            guard.clear();
        }
    }

    pub fn unsubscribe() {
        if let Some(slot) = EVENT_SENDER.get()
            && let Ok(mut guard) = slot.lock()
        {
            *guard = None;
        }
    }
}

pub(crate) fn normalize_keyboard_vk(virtual_key: u16, scan_code: u16, extended: bool) -> u16 {
    match virtual_key {
        VK_SHIFT => {
            if scan_code == 0x36 {
                VK_RSHIFT
            } else {
                VK_LSHIFT
            }
        }
        VK_CONTROL => {
            if extended {
                VK_RCONTROL
            } else {
                VK_LCONTROL
            }
        }
        VK_MENU => {
            if extended {
                VK_RMENU
            } else {
                VK_LMENU
            }
        }
        _ => virtual_key,
    }
}

pub(crate) fn mouse_button_virtual_key(button: MouseButton) -> u16 {
    match button {
        MouseButton::Left => VK_LBUTTON,
        MouseButton::Right => VK_RBUTTON,
        MouseButton::Middle => VK_MBUTTON,
        MouseButton::X1 => VK_XBUTTON1,
        MouseButton::X2 => VK_XBUTTON2,
    }
}

/// Updates source-aware key state, emits hotkey edges and returns whether the
/// originating Windows event should be consumed.
pub(crate) fn handle_transition(virtual_key: u16, down: bool, physical: bool) -> bool {
    let index = virtual_key as usize;
    if index >= 256 {
        return false;
    }

    let states = key_states();
    let source_bit = if physical {
        PHYSICAL_DOWN
    } else {
        EXTERNAL_INJECTED_DOWN
    };
    let old_state = if down {
        states[index].fetch_or(source_bit, Ordering::AcqRel)
    } else {
        states[index].fetch_and(!source_bit, Ordering::AcqRel)
    };

    let Some(registry) = BINDINGS.get() else {
        return false;
    };
    let Ok(bindings) = registry.read() else {
        return false;
    };

    let mut consume = false;
    for registered in bindings.iter() {
        if !registered.binding.contains_virtual_key(virtual_key) {
            continue;
        }
        if !physical && registered.binding.source_policy == InputSourcePolicy::PhysicalOnly {
            continue;
        }

        let policy = registered.binding.source_policy;
        let before = registered.binding.is_satisfied_with(|vk| {
            if vk == virtual_key {
                state_is_down(old_state, policy)
            } else {
                load_state(states, vk, policy)
            }
        });
        let after = registered
            .binding
            .is_satisfied_with(|vk| load_state(states, vk, policy));

        if !before && after {
            publish_hotkey(HotkeyEvent {
                id: registered.id,
                phase: HotkeyPhase::Pressed,
            });
        } else if before && !after {
            publish_hotkey(HotkeyEvent {
                id: registered.id,
                phase: HotkeyPhase::Released,
            });
        }

        if registered.binding.consume && (before || after) {
            consume = true;
        }
    }

    consume
}

fn key_states() -> &'static [AtomicU8; 256] {
    KEY_STATE.get_or_init(|| std::array::from_fn(|_| AtomicU8::new(0)))
}

fn load_state(states: &[AtomicU8; 256], virtual_key: u16, policy: InputSourcePolicy) -> bool {
    let Some(state) = states.get(virtual_key as usize) else {
        return false;
    };
    state_is_down(state.load(Ordering::Acquire), policy)
}

fn state_is_down(state: u8, policy: InputSourcePolicy) -> bool {
    match policy {
        InputSourcePolicy::PhysicalOrExternalInjected => state != 0,
        InputSourcePolicy::PhysicalOnly => (state & PHYSICAL_DOWN) != 0,
    }
}

fn validate_bindings(bindings: &[RegisteredHotkey]) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut hotkeys = HashSet::new();
    for registered in bindings {
        if !ids.insert(registered.id) {
            return Err(format!("duplicate hotkey id {}", registered.id));
        }
        if !hotkeys.insert(registered.binding.canonical().to_string()) {
            return Err(format!(
                "conflicting hotkey '{}'",
                registered.binding.canonical()
            ));
        }
    }
    Ok(())
}

fn publish_hotkey(event: HotkeyEvent) {
    let Some(slot) = EVENT_SENDER.get() else {
        return;
    };
    let Ok(guard) = slot.lock() else {
        return;
    };
    let Some(sender) = guard.as_ref() else {
        return;
    };
    match sender.try_send(event) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HotkeyPhase, RegisteredHotkey, WindowsHotkeyManager, handle_transition,
        normalize_keyboard_vk,
    };
    use crate::hotkeys::{HotkeyBinding, InputSourcePolicy, ModifierMatchMode};

    fn reset() {
        WindowsHotkeyManager::clear_bindings();
        for state in super::key_states() {
            state.store(0, std::sync::atomic::Ordering::Release);
        }
    }

    #[test]
    fn normalizes_right_control_from_extended_flag() {
        assert_eq!(normalize_keyboard_vk(0x11, 0x1D, true), 0xA3);
        assert_eq!(normalize_keyboard_vk(0x11, 0x1D, false), 0xA2);
    }

    #[test]
    fn external_remap_is_accepted_by_default() {
        reset();
        let binding = HotkeyBinding::parse("num9").unwrap_err();
        assert!(binding.contains("unsupported"));

        let binding = HotkeyBinding::parse("9").unwrap();
        WindowsHotkeyManager::replace_bindings(vec![RegisteredHotkey { id: 1, binding }]).unwrap();
        assert!(!handle_transition(b'9' as u16, true, false));
        assert!(!handle_transition(b'9' as u16, false, false));
    }

    #[test]
    fn physical_only_does_not_trigger_from_external_injection() {
        reset();
        let binding = HotkeyBinding::parse("f6")
            .unwrap()
            .with_source_policy(InputSourcePolicy::PhysicalOnly);
        WindowsHotkeyManager::replace_bindings(vec![RegisteredHotkey { id: 2, binding }]).unwrap();
        assert!(!handle_transition(0x75, true, false));
        assert!(!handle_transition(0x75, false, false));
    }

    #[test]
    fn consumed_single_key_suppresses_down_and_up() {
        reset();
        let binding = HotkeyBinding::parse("f5").unwrap().with_consume(true);
        WindowsHotkeyManager::replace_bindings(vec![RegisteredHotkey { id: 3, binding }]).unwrap();
        assert!(handle_transition(0x74, true, true));
        assert!(handle_transition(0x74, false, true));
    }

    #[test]
    fn exact_modifier_mode_rejects_extra_shift() {
        reset();
        let binding = HotkeyBinding::parse("ctrl+k")
            .unwrap()
            .with_modifier_match(ModifierMatchMode::Exact);
        WindowsHotkeyManager::replace_bindings(vec![RegisteredHotkey { id: 4, binding }]).unwrap();
        handle_transition(0xA2, true, true);
        handle_transition(0xA0, true, true);
        assert!(!handle_transition(b'K' as u16, true, true));
    }

    #[test]
    fn hotkey_phase_enum_is_stable_for_dispatch_contract() {
        assert_ne!(HotkeyPhase::Pressed, HotkeyPhase::Released);
    }
}
