use crate::macro_engine::{InputAction, Trigger};

#[derive(Debug, Clone)]
pub struct RemapBinding {
    pub name: String,
    pub trigger: Trigger,
    pub actions: Vec<InputAction>,
    pub enabled: bool,
}

impl RemapBinding {
    pub fn simple(name: impl Into<String>, trigger: Trigger, action: InputAction) -> Self {
        Self {
            name: name.into(),
            trigger,
            actions: vec![action],
            enabled: true,
        }
    }
}
