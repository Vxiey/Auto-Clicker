use crate::engine::MouseButton;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Trigger {
    Key(u16),
    Mouse(MouseButton),
    Chord(Vec<u16>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroMode {
    Once,
    WhileHeld,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    KeyDown(u16),
    KeyUp(u16),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseMoveAbsolute { x: i32, y: i32 },
    MouseWheel { delta: i32 },
    WaitMicros(u64),
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub trigger: Trigger,
    pub mode: MacroMode,
    pub actions: Vec<InputAction>,
    pub repeat_count: Option<u64>,
}

impl MacroDefinition {
    pub fn new(name: impl Into<String>, trigger: Trigger) -> Self {
        Self {
            name: name.into(),
            trigger,
            mode: MacroMode::Once,
            actions: Vec::new(),
            repeat_count: Some(1),
        }
    }
}
