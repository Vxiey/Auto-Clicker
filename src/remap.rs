use std::collections::HashSet;

use crate::engine::MouseButton;
use crate::macro_engine::{InputAction, Trigger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemapScope {
    Global,
    Process(String),
}

#[derive(Debug, Clone)]
pub struct RemapBinding {
    pub name: String,
    pub trigger: Trigger,
    pub actions: Vec<InputAction>,
    pub enabled: bool,
    pub consume: bool,
    pub scope: RemapScope,
}

impl RemapBinding {
    pub fn simple(name: impl Into<String>, trigger: Trigger, action: InputAction) -> Self {
        Self {
            name: name.into(),
            trigger,
            actions: vec![action],
            enabled: true,
            consume: true,
            scope: RemapScope::Global,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemapInput {
    KeyDown(u16),
    KeyUp(u16),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
}

#[derive(Debug, Clone)]
pub struct RemapDecision {
    pub binding_name: String,
    pub actions: Vec<InputAction>,
    pub consume: bool,
}

#[derive(Debug, Default)]
pub struct RemapEngine {
    bindings: Vec<RemapBinding>,
    pressed_keys: HashSet<u16>,
    active_chords: HashSet<usize>,
}

impl RemapEngine {
    pub fn new(bindings: Vec<RemapBinding>) -> Result<Self, String> {
        validate_bindings(&bindings)?;
        Ok(Self {
            bindings,
            pressed_keys: HashSet::new(),
            active_chords: HashSet::new(),
        })
    }

    pub fn replace_bindings(&mut self, bindings: Vec<RemapBinding>) -> Result<(), String> {
        validate_bindings(&bindings)?;
        self.bindings = bindings;
        self.active_chords.clear();
        Ok(())
    }

    pub fn process(&mut self, input: RemapInput, foreground_process: Option<&str>) -> Vec<RemapDecision> {
        match input {
            RemapInput::KeyDown(key) => {
                self.pressed_keys.insert(key);
            }
            RemapInput::KeyUp(key) => {
                self.pressed_keys.remove(&key);
            }
            RemapInput::MouseDown(_) | RemapInput::MouseUp(_) => {}
        }

        let foreground = foreground_process.map(normalize_process_name);
        let mut decisions = Vec::new();

        for (index, binding) in self.bindings.iter().enumerate() {
            if !binding.enabled || !scope_matches(&binding.scope, foreground.as_deref()) {
                self.active_chords.remove(&index);
                continue;
            }

            let fired = match &binding.trigger {
                Trigger::Key(key) => matches!(input, RemapInput::KeyDown(candidate) if candidate == *key),
                Trigger::Mouse(button) => {
                    matches!(input, RemapInput::MouseDown(candidate) if candidate == *button)
                }
                Trigger::Chord(keys) => {
                    let satisfied = !keys.is_empty()
                        && keys.iter().all(|key| self.pressed_keys.contains(key));
                    let was_active = self.active_chords.contains(&index);
                    if satisfied {
                        self.active_chords.insert(index);
                    } else {
                        self.active_chords.remove(&index);
                    }
                    satisfied && !was_active
                }
            };

            if fired {
                decisions.push(RemapDecision {
                    binding_name: binding.name.clone(),
                    actions: binding.actions.clone(),
                    consume: binding.consume,
                });
            }
        }

        decisions
    }

    pub fn clear_input_state(&mut self) {
        self.pressed_keys.clear();
        self.active_chords.clear();
    }
}

fn validate_bindings(bindings: &[RemapBinding]) -> Result<(), String> {
    let mut names = HashSet::new();
    for binding in bindings {
        let name = binding.name.trim();
        if name.is_empty() {
            return Err("remap binding name cannot be empty".into());
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!("duplicate remap binding name '{name}'"));
        }
        if binding.actions.is_empty() {
            return Err(format!("remap binding '{name}' has no actions"));
        }
        if let Trigger::Chord(keys) = &binding.trigger {
            if keys.is_empty() {
                return Err(format!("remap binding '{name}' has an empty chord"));
            }
            let unique: HashSet<_> = keys.iter().copied().collect();
            if unique.len() != keys.len() {
                return Err(format!("remap binding '{name}' has duplicate chord keys"));
            }
        }
    }
    Ok(())
}

fn scope_matches(scope: &RemapScope, foreground: Option<&str>) -> bool {
    match scope {
        RemapScope::Global => true,
        RemapScope::Process(process) => foreground
            .map(|foreground| normalize_process_name(process) == foreground)
            .unwrap_or(false),
    }
}

fn normalize_process_name(value: &str) -> String {
    value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{RemapBinding, RemapEngine, RemapInput, RemapScope};
    use crate::macro_engine::{InputAction, Trigger};

    #[test]
    fn simple_key_mapping_fires_on_key_down() {
        let binding = RemapBinding::simple("caps-to-f", Trigger::Key(0x14), InputAction::KeyDown(0x46));
        let mut engine = RemapEngine::new(vec![binding]).expect("valid binding");
        let decisions = engine.process(RemapInput::KeyDown(0x14), None);
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].consume);
    }

    #[test]
    fn chord_fires_once_until_released() {
        let binding = RemapBinding::simple(
            "ctrl-k",
            Trigger::Chord(vec![0xA2, 0x4B]),
            InputAction::KeyDown(0x70),
        );
        let mut engine = RemapEngine::new(vec![binding]).expect("valid binding");
        assert!(engine.process(RemapInput::KeyDown(0xA2), None).is_empty());
        assert_eq!(engine.process(RemapInput::KeyDown(0x4B), None).len(), 1);
        assert!(engine.process(RemapInput::KeyDown(0x4B), None).is_empty());
        engine.process(RemapInput::KeyUp(0x4B), None);
        assert_eq!(engine.process(RemapInput::KeyDown(0x4B), None).len(), 1);
    }

    #[test]
    fn process_scope_is_respected() {
        let mut binding = RemapBinding::simple(
            "game-only",
            Trigger::Key(0x41),
            InputAction::KeyDown(0x42),
        );
        binding.scope = RemapScope::Process("ExampleGame.exe".into());
        let mut engine = RemapEngine::new(vec![binding]).expect("valid binding");
        assert!(engine.process(RemapInput::KeyDown(0x41), Some("notepad.exe")).is_empty());
        assert_eq!(engine.process(RemapInput::KeyDown(0x41), Some("C:\\Games\\ExampleGame.exe")).len(), 1);
    }
}
