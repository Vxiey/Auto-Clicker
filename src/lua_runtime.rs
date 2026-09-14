use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mlua::{Error as LuaError, HookTriggers, Lua, LuaOptions, StdLib, VmState};

use crate::engine::MouseButton;
use crate::macro_engine::InputAction;

const MAX_ACTIONS: usize = 100_000;
const MAX_INSTRUCTIONS: u64 = 5_000_000;
const HOOK_INTERVAL: u32 = 1_000;

#[derive(Debug, Default, Clone, Copy)]
pub struct LuaMacroCompiler;

impl LuaMacroCompiler {
    pub fn compile(script: &str) -> Result<Vec<InputAction>, String> {
        let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default())
            .map_err(|error| format!("failed to create Lua runtime: {error}"))?;

        // No file/process/network APIs are registered by VxClick. The script only builds
        // an InputAction timeline which is later executed by the native macro player.
        let actions = Arc::new(Mutex::new(Vec::<InputAction>::new()));
        register_api(&lua, Arc::clone(&actions))?;

        let instructions = Arc::new(AtomicU64::new(0));
        let instruction_counter = Arc::clone(&instructions);
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(HOOK_INTERVAL),
            move |_, _| {
                let total = instruction_counter.fetch_add(HOOK_INTERVAL as u64, Ordering::Relaxed)
                    + HOOK_INTERVAL as u64;
                if total > MAX_INSTRUCTIONS {
                    return Err(LuaError::RuntimeError(
                        "script instruction limit exceeded".to_string(),
                    ));
                }
                Ok(VmState::Continue)
            },
        )
        .map_err(|error| format!("failed to install Lua execution limit: {error}"))?;

        lua.load(script)
            .set_name("vxclick-script")
            .exec()
            .map_err(|error| format!("Lua error: {error}"))?;

        let actions = Arc::try_unwrap(actions)
            .map_err(|_| "Lua action buffer is still in use".to_string())?
            .into_inner()
            .map_err(|_| "Lua action buffer mutex was poisoned".to_string())?;

        Ok(actions)
    }
}

fn register_api(lua: &Lua, actions: Arc<Mutex<Vec<InputAction>>>) -> Result<(), String> {
    let globals = lua.globals();

    globals
        .set(
            "sleep",
            lua.create_function(with_actions(&actions, |actions, milliseconds: f64| {
                if !milliseconds.is_finite() || !(0.0..=60_000.0).contains(&milliseconds) {
                    return Err(LuaError::RuntimeError(
                        "sleep(ms) must be between 0 and 60000".into(),
                    ));
                }
                let micros = (milliseconds * 1_000.0).round() as u64;
                push_action(actions, InputAction::WaitMicros(micros))
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "move_mouse",
            lua.create_function(with_actions(&actions, |actions, (dx, dy): (i32, i32)| {
                push_action(actions, InputAction::MouseMoveRelative { dx, dy })
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "mouse_down",
            lua.create_function(with_actions(&actions, |actions, button: String| {
                push_action(actions, InputAction::MouseDown(parse_button(&button)?))
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "mouse_up",
            lua.create_function(with_actions(&actions, |actions, button: String| {
                push_action(actions, InputAction::MouseUp(parse_button(&button)?))
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "click",
            lua.create_function(with_actions(&actions, |actions, button: String| {
                let button = parse_button(&button)?;
                push_action(actions, InputAction::MouseDown(button))?;
                push_action(actions, InputAction::MouseUp(button))
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "key_down",
            lua.create_function(with_actions(&actions, |actions, virtual_key: u16| {
                push_action(actions, InputAction::KeyDown(virtual_key))
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "key_up",
            lua.create_function(with_actions(&actions, |actions, virtual_key: u16| {
                push_action(actions, InputAction::KeyUp(virtual_key))
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    globals
        .set(
            "wheel",
            lua.create_function(with_actions(&actions, |actions, delta: i32| {
                push_action(actions, InputAction::MouseWheel { delta })
            }))
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    Ok(())
}

fn with_actions<A, F>(
    actions: &Arc<Mutex<Vec<InputAction>>>,
    callback: F,
) -> impl Fn(&Lua, A) -> mlua::Result<()> + Send + 'static
where
    A: mlua::FromLuaMulti,
    F: Fn(&mut Vec<InputAction>, A) -> mlua::Result<()> + Send + 'static,
{
    let actions = Arc::clone(actions);
    move |_, args| {
        let mut guard = actions
            .lock()
            .map_err(|_| LuaError::RuntimeError("Lua action buffer mutex was poisoned".into()))?;
        callback(&mut guard, args)
    }
}

fn push_action(actions: &mut Vec<InputAction>, action: InputAction) -> mlua::Result<()> {
    if actions.len() >= MAX_ACTIONS {
        return Err(LuaError::RuntimeError(
            "script action limit exceeded".to_string(),
        ));
    }
    actions.push(action);
    Ok(())
}

fn parse_button(value: &str) -> mlua::Result<MouseButton> {
    MouseButton::parse(value).ok_or_else(|| {
        LuaError::RuntimeError("mouse button must be left, right, middle, x1, or x2".into())
    })
}

fn lua_error(error: LuaError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::LuaMacroCompiler;
    use crate::macro_engine::InputAction;

    #[test]
    fn compiles_basic_recoil_style_timeline() {
        let actions = LuaMacroCompiler::compile(
            r#"
            mouse_down("left")
            for _ = 1, 3 do
                move_mouse(0, 2)
                sleep(10)
            end
            mouse_up("left")
            "#,
        )
        .expect("script should compile");

        assert_eq!(actions.len(), 8);
        assert!(matches!(actions[0], InputAction::MouseDown(_)));
        assert!(matches!(
            actions[1],
            InputAction::MouseMoveRelative { dy: 2, .. }
        ));
        assert!(matches!(actions[2], InputAction::WaitMicros(10_000)));
        assert!(matches!(actions[7], InputAction::MouseUp(_)));
    }
}
