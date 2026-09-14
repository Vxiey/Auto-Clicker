use std::collections::HashSet;

pub const MAX_MAIN_KEYS: usize = 5;

const VK_LBUTTON: u16 = 0x01;
const VK_RBUTTON: u16 = 0x02;
const VK_MBUTTON: u16 = 0x04;
const VK_XBUTTON1: u16 = 0x05;
const VK_XBUTTON2: u16 = 0x06;
const VK_BACK: u16 = 0x08;
const VK_TAB: u16 = 0x09;
const VK_RETURN: u16 = 0x0D;
const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_MENU: u16 = 0x12;
const VK_PAUSE: u16 = 0x13;
const VK_CAPITAL: u16 = 0x14;
const VK_ESCAPE: u16 = 0x1B;
const VK_SPACE: u16 = 0x20;
const VK_PRIOR: u16 = 0x21;
const VK_NEXT: u16 = 0x22;
const VK_END: u16 = 0x23;
const VK_HOME: u16 = 0x24;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
const VK_SNAPSHOT: u16 = 0x2C;
const VK_INSERT: u16 = 0x2D;
const VK_DELETE: u16 = 0x2E;
const VK_LWIN: u16 = 0x5B;
const VK_RWIN: u16 = 0x5C;
const VK_NUMPAD0: u16 = 0x60;
const VK_F1: u16 = 0x70;
const VK_NUMLOCK: u16 = 0x90;
const VK_SCROLL: u16 = 0x91;
const VK_LSHIFT: u16 = 0xA0;
const VK_RSHIFT: u16 = 0xA1;
const VK_LCONTROL: u16 = 0xA2;
const VK_RCONTROL: u16 = 0xA3;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierMatchMode {
    /// Required modifiers must be down; unrelated extra modifiers are allowed.
    Inclusive,
    /// The active modifier set must exactly match the binding.
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSourcePolicy {
    /// Physical input and remaps emitted by other software are accepted.
    PhysicalOrExternalInjected,
    /// Only input reported by the Windows low-level hook as physical is accepted.
    PhysicalOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierKind {
    Ctrl,
    Alt,
    Shift,
    Super,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierSide {
    Any,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModifierRequirement {
    pub kind: ModifierKind,
    pub side: ModifierSide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyBinding {
    canonical: String,
    modifiers: Vec<ModifierRequirement>,
    main_keys: Vec<u16>,
    pub modifier_match: ModifierMatchMode,
    pub source_policy: InputSourcePolicy,
    /// When true, the native hook backend may consume the trigger instead of
    /// passing it through to the foreground application.
    pub consume: bool,
}

impl HotkeyBinding {
    pub fn parse(value: &str) -> Result<Self, String> {
        let raw_tokens: Vec<_> = value
            .split('+')
            .map(normalize_token)
            .filter(|token| !token.is_empty())
            .collect();

        if raw_tokens.is_empty() {
            return Err("hotkey cannot be empty".into());
        }

        let only_modifiers = raw_tokens.iter().all(|token| parse_modifier(token).is_some());
        let mut modifiers = Vec::new();
        let mut mains = Vec::<(u16, String)>::new();
        let mut modifier_groups = HashSet::new();

        for token in raw_tokens {
            if let Some(requirement) = parse_modifier(&token) {
                if only_modifiers || mains.is_empty() {
                    validate_modifier_requirement(&modifiers, requirement, value)?;
                    modifiers.push(requirement);
                    modifier_groups.insert(requirement.kind);
                    continue;
                }
            }

            let vk = parse_main_key(&token)?;
            if mains.len() >= MAX_MAIN_KEYS {
                return Err(format!(
                    "hotkey supports at most {MAX_MAIN_KEYS} main keys"
                ));
            }
            if mains.iter().any(|(existing, _)| *existing == vk) {
                return Err(format!("duplicate key '{token}' in hotkey '{value}'"));
            }
            mains.push((vk, token));
        }

        if mains.is_empty() && modifiers.is_empty() {
            return Err(format!("hotkey '{value}' has no usable keys"));
        }

        // A generic modifier and a side-specific requirement for the same group
        // would make exact matching ambiguous, so reject that at parse time.
        for kind in modifier_groups {
            let group: Vec<_> = modifiers.iter().filter(|modifier| modifier.kind == kind).collect();
            if group.iter().any(|modifier| modifier.side == ModifierSide::Any)
                && group.iter().any(|modifier| modifier.side != ModifierSide::Any)
            {
                return Err(format!(
                    "hotkey '{value}' mixes a generic modifier with a left/right-specific variant"
                ));
            }
        }

        mains.sort_by(|left, right| left.1.cmp(&right.1));
        modifiers.sort_by_key(|modifier| modifier_sort_key(*modifier));
        let canonical = format_binding(&modifiers, &mains);

        Ok(Self {
            canonical,
            modifiers,
            main_keys: mains.into_iter().map(|(vk, _)| vk).collect(),
            modifier_match: ModifierMatchMode::Inclusive,
            source_policy: InputSourcePolicy::PhysicalOrExternalInjected,
            consume: false,
        })
    }

    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    pub fn modifiers(&self) -> &[ModifierRequirement] {
        &self.modifiers
    }

    pub fn main_keys(&self) -> &[u16] {
        &self.main_keys
    }

    pub fn with_consume(mut self, consume: bool) -> Self {
        self.consume = consume;
        self
    }

    pub fn with_modifier_match(mut self, mode: ModifierMatchMode) -> Self {
        self.modifier_match = mode;
        self
    }

    pub fn with_source_policy(mut self, policy: InputSourcePolicy) -> Self {
        self.source_policy = policy;
        self
    }

    /// Evaluates a binding against an externally maintained key-state table.
    /// The Windows hook backend can therefore use atomics and avoid allocating
    /// or locking simply to match a hotkey.
    pub fn is_satisfied_with<F>(&self, is_down: F) -> bool
    where
        F: Fn(u16) -> bool,
    {
        if !self
            .modifiers
            .iter()
            .copied()
            .all(|modifier| modifier_is_down(modifier, &is_down))
        {
            return false;
        }

        if !self.main_keys.iter().copied().all(&is_down) {
            return false;
        }

        if self.modifier_match == ModifierMatchMode::Exact {
            for kind in [
                ModifierKind::Ctrl,
                ModifierKind::Alt,
                ModifierKind::Shift,
                ModifierKind::Super,
            ] {
                let left = modifier_vk(kind, ModifierSide::Left);
                let right = modifier_vk(kind, ModifierSide::Right);
                let any_down = is_down(left) || is_down(right);
                if !any_down {
                    continue;
                }

                let requirements: Vec<_> = self
                    .modifiers
                    .iter()
                    .copied()
                    .filter(|modifier| modifier.kind == kind)
                    .collect();
                let main_uses_group = self
                    .main_keys
                    .iter()
                    .copied()
                    .any(|vk| modifier_kind_for_vk(vk) == Some(kind));

                if requirements.is_empty() && !main_uses_group {
                    return false;
                }
                if requirements.iter().any(|item| item.side == ModifierSide::Left)
                    && is_down(right)
                    && !requirements.iter().any(|item| item.side == ModifierSide::Right)
                    && !main_uses_group
                {
                    return false;
                }
                if requirements.iter().any(|item| item.side == ModifierSide::Right)
                    && is_down(left)
                    && !requirements.iter().any(|item| item.side == ModifierSide::Left)
                    && !main_uses_group
                {
                    return false;
                }
            }
        }

        true
    }

    pub fn contains_virtual_key(&self, vk: u16) -> bool {
        self.main_keys.contains(&vk)
            || self.modifiers.iter().copied().any(|modifier| match modifier.side {
                ModifierSide::Any => {
                    vk == modifier_vk(modifier.kind, ModifierSide::Left)
                        || vk == modifier_vk(modifier.kind, ModifierSide::Right)
                        || vk == generic_modifier_vk(modifier.kind)
                }
                side => vk == modifier_vk(modifier.kind, side),
            })
    }
}

fn validate_modifier_requirement(
    existing: &[ModifierRequirement],
    incoming: ModifierRequirement,
    original: &str,
) -> Result<(), String> {
    if existing.contains(&incoming) {
        return Err(format!("duplicate modifier in hotkey '{original}'"));
    }
    if existing.iter().any(|modifier| {
        modifier.kind == incoming.kind
            && (modifier.side == ModifierSide::Any || incoming.side == ModifierSide::Any)
            && modifier.side != incoming.side
    }) {
        return Err(format!(
            "hotkey '{original}' mixes generic and side-specific modifiers"
        ));
    }
    Ok(())
}

fn modifier_is_down<F>(modifier: ModifierRequirement, is_down: &F) -> bool
where
    F: Fn(u16) -> bool,
{
    match modifier.side {
        ModifierSide::Any => {
            is_down(modifier_vk(modifier.kind, ModifierSide::Left))
                || is_down(modifier_vk(modifier.kind, ModifierSide::Right))
                || is_down(generic_modifier_vk(modifier.kind))
        }
        side => is_down(modifier_vk(modifier.kind, side)),
    }
}

fn parse_modifier(token: &str) -> Option<ModifierRequirement> {
    let (kind, side) = match token {
        "ctrl" | "control" => (ModifierKind::Ctrl, ModifierSide::Any),
        "leftctrl" | "lctrl" | "ctrlleft" => (ModifierKind::Ctrl, ModifierSide::Left),
        "rightctrl" | "rctrl" | "ctrlright" => (ModifierKind::Ctrl, ModifierSide::Right),
        "alt" | "option" => (ModifierKind::Alt, ModifierSide::Any),
        "leftalt" | "lalt" | "altleft" => (ModifierKind::Alt, ModifierSide::Left),
        "rightalt" | "ralt" | "altright" | "altgr" => {
            (ModifierKind::Alt, ModifierSide::Right)
        }
        "shift" => (ModifierKind::Shift, ModifierSide::Any),
        "leftshift" | "lshift" | "shiftleft" => (ModifierKind::Shift, ModifierSide::Left),
        "rightshift" | "rshift" | "shiftright" => (ModifierKind::Shift, ModifierSide::Right),
        "win" | "super" | "meta" | "command" | "cmd" => {
            (ModifierKind::Super, ModifierSide::Any)
        }
        "leftwin" | "lwin" | "leftsuper" | "superleft" => {
            (ModifierKind::Super, ModifierSide::Left)
        }
        "rightwin" | "rwin" | "rightsuper" | "superright" => {
            (ModifierKind::Super, ModifierSide::Right)
        }
        _ => return None,
    };
    Some(ModifierRequirement { kind, side })
}

fn parse_main_key(token: &str) -> Result<u16, String> {
    if token == "fn" || token == "function" {
        return Err(
            "Fn is usually handled by keyboard firmware/OEM software and is not reliably exposed as a Windows key"
                .into(),
        );
    }

    if token.len() == 1 {
        let byte = token.as_bytes()[0];
        if byte.is_ascii_alphabetic() {
            return Ok(byte.to_ascii_uppercase() as u16);
        }
        if byte.is_ascii_digit() {
            return Ok(byte as u16);
        }
    }

    if let Some(number) = token.strip_prefix('f').and_then(|value| value.parse::<u16>().ok()) {
        if (1..=24).contains(&number) {
            return Ok(VK_F1 + number - 1);
        }
    }

    if let Some(number) = token
        .strip_prefix("numpad")
        .and_then(|value| value.parse::<u16>().ok())
    {
        if number <= 9 {
            return Ok(VK_NUMPAD0 + number);
        }
    }

    let vk = match token {
        "leftmouse" | "mouse1" | "lmb" => VK_LBUTTON,
        "rightmouse" | "mouse2" | "rmb" => VK_RBUTTON,
        "middlemouse" | "mouse3" | "mmb" => VK_MBUTTON,
        "mouse4" | "x1" | "backmouse" => VK_XBUTTON1,
        "mouse5" | "x2" | "forwardmouse" => VK_XBUTTON2,
        "backspace" | "back" => VK_BACK,
        "tab" => VK_TAB,
        "enter" | "return" => VK_RETURN,
        "pause" | "break" | "pausebreak" => VK_PAUSE,
        "capslock" => VK_CAPITAL,
        "escape" | "esc" => VK_ESCAPE,
        "space" | "spacebar" => VK_SPACE,
        "pageup" | "pgup" => VK_PRIOR,
        "pagedown" | "pgdn" => VK_NEXT,
        "end" => VK_END,
        "home" => VK_HOME,
        "left" | "leftarrow" => VK_LEFT,
        "up" | "uparrow" => VK_UP,
        "right" | "rightarrow" => VK_RIGHT,
        "down" | "downarrow" => VK_DOWN,
        "printscreen" | "prtsc" | "snapshot" => VK_SNAPSHOT,
        "insert" | "ins" => VK_INSERT,
        "delete" | "del" => VK_DELETE,
        "numlock" => VK_NUMLOCK,
        "scrolllock" => VK_SCROLL,
        "leftshift" | "lshift" | "shiftleft" => VK_LSHIFT,
        "rightshift" | "rshift" | "shiftright" => VK_RSHIFT,
        "leftctrl" | "lctrl" | "ctrlleft" => VK_LCONTROL,
        "rightctrl" | "rctrl" | "ctrlright" => VK_RCONTROL,
        "leftalt" | "lalt" | "altleft" => VK_LMENU,
        "rightalt" | "ralt" | "altright" | "altgr" => VK_RMENU,
        "leftwin" | "lwin" | "leftsuper" | "superleft" => VK_LWIN,
        "rightwin" | "rwin" | "rightsuper" | "superright" => VK_RWIN,
        _ => return Err(format!("unsupported hotkey token '{token}'")),
    };
    Ok(vk)
}

fn normalize_token(token: &str) -> String {
    token
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

fn modifier_vk(kind: ModifierKind, side: ModifierSide) -> u16 {
    match (kind, side) {
        (ModifierKind::Ctrl, ModifierSide::Left) => VK_LCONTROL,
        (ModifierKind::Ctrl, ModifierSide::Right) => VK_RCONTROL,
        (ModifierKind::Alt, ModifierSide::Left) => VK_LMENU,
        (ModifierKind::Alt, ModifierSide::Right) => VK_RMENU,
        (ModifierKind::Shift, ModifierSide::Left) => VK_LSHIFT,
        (ModifierKind::Shift, ModifierSide::Right) => VK_RSHIFT,
        (ModifierKind::Super, ModifierSide::Left) => VK_LWIN,
        (ModifierKind::Super, ModifierSide::Right) => VK_RWIN,
        (_, ModifierSide::Any) => generic_modifier_vk(kind),
    }
}

fn generic_modifier_vk(kind: ModifierKind) -> u16 {
    match kind {
        ModifierKind::Ctrl => VK_CONTROL,
        ModifierKind::Alt => VK_MENU,
        ModifierKind::Shift => VK_SHIFT,
        ModifierKind::Super => VK_LWIN,
    }
}

fn modifier_kind_for_vk(vk: u16) -> Option<ModifierKind> {
    match vk {
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Some(ModifierKind::Ctrl),
        VK_MENU | VK_LMENU | VK_RMENU => Some(ModifierKind::Alt),
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Some(ModifierKind::Shift),
        VK_LWIN | VK_RWIN => Some(ModifierKind::Super),
        _ => None,
    }
}

fn modifier_sort_key(modifier: ModifierRequirement) -> (u8, u8) {
    let kind = match modifier.kind {
        ModifierKind::Ctrl => 0,
        ModifierKind::Alt => 1,
        ModifierKind::Shift => 2,
        ModifierKind::Super => 3,
    };
    let side = match modifier.side {
        ModifierSide::Any => 0,
        ModifierSide::Left => 1,
        ModifierSide::Right => 2,
    };
    (kind, side)
}

fn format_binding(modifiers: &[ModifierRequirement], mains: &[(u16, String)]) -> String {
    let mut parts = Vec::with_capacity(modifiers.len() + mains.len());
    for modifier in modifiers {
        parts.push(
            match (modifier.kind, modifier.side) {
                (ModifierKind::Ctrl, ModifierSide::Any) => "ctrl",
                (ModifierKind::Ctrl, ModifierSide::Left) => "leftctrl",
                (ModifierKind::Ctrl, ModifierSide::Right) => "rightctrl",
                (ModifierKind::Alt, ModifierSide::Any) => "alt",
                (ModifierKind::Alt, ModifierSide::Left) => "leftalt",
                (ModifierKind::Alt, ModifierSide::Right) => "rightalt",
                (ModifierKind::Shift, ModifierSide::Any) => "shift",
                (ModifierKind::Shift, ModifierSide::Left) => "leftshift",
                (ModifierKind::Shift, ModifierSide::Right) => "rightshift",
                (ModifierKind::Super, ModifierSide::Any) => "win",
                (ModifierKind::Super, ModifierSide::Left) => "leftwin",
                (ModifierKind::Super, ModifierSide::Right) => "rightwin",
            }
            .to_string(),
        );
    }
    parts.extend(mains.iter().map(|(_, token)| token.clone()));
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        HotkeyBinding, ModifierMatchMode, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_RCONTROL,
    };

    #[test]
    fn parses_side_specific_modifier_and_chord() {
        let binding = HotkeyBinding::parse("LeftCtrl + Shift + B + A").unwrap();
        assert_eq!(binding.canonical(), "leftctrl+shift+a+b");
        assert_eq!(binding.main_keys().len(), 2);
    }

    #[test]
    fn supports_standalone_left_alt() {
        let binding = HotkeyBinding::parse("Left Alt").unwrap();
        let down = HashSet::from([VK_LMENU]);
        assert!(binding.is_satisfied_with(|vk| down.contains(&vk)));
    }

    #[test]
    fn inclusive_mode_allows_extra_modifier() {
        let binding = HotkeyBinding::parse("ctrl+a").unwrap();
        let down = HashSet::from([VK_LCONTROL, VK_LSHIFT, b'A' as u16]);
        assert!(binding.is_satisfied_with(|vk| down.contains(&vk)));
    }

    #[test]
    fn exact_mode_rejects_extra_modifier() {
        let binding = HotkeyBinding::parse("ctrl+a")
            .unwrap()
            .with_modifier_match(ModifierMatchMode::Exact);
        let down = HashSet::from([VK_LCONTROL, VK_LSHIFT, b'A' as u16]);
        assert!(!binding.is_satisfied_with(|vk| down.contains(&vk)));
    }

    #[test]
    fn exact_mode_rejects_wrong_modifier_side() {
        let binding = HotkeyBinding::parse("leftctrl+a")
            .unwrap()
            .with_modifier_match(ModifierMatchMode::Exact);
        let down = HashSet::from([VK_RCONTROL, b'A' as u16]);
        assert!(!binding.is_satisfied_with(|vk| down.contains(&vk)));
    }

    #[test]
    fn rejects_ambiguous_generic_and_side_specific_modifier() {
        let error = HotkeyBinding::parse("ctrl+leftctrl+a").unwrap_err();
        assert!(error.contains("generic"));
    }

    #[test]
    fn rejects_duplicate_main_keys() {
        assert!(HotkeyBinding::parse("ctrl+a+a").is_err());
    }

    #[test]
    fn supports_print_screen_pause_and_mouse_buttons() {
        for hotkey in ["printscreen", "pause", "mouse4", "mouse5"] {
            assert!(HotkeyBinding::parse(hotkey).is_ok(), "{hotkey}");
        }
    }

    #[test]
    fn fn_has_explicit_platform_limitation() {
        let error = HotkeyBinding::parse("fn").unwrap_err();
        assert!(error.contains("firmware"));
    }
}
