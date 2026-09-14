# Key Remapping

VxClick uses a native `Input → Rule → Action` remap runtime.

## Supported mapping types

- Key → Key
- Mouse → Key
- Key → Mouse
- Mouse → Mouse
- Input → Macro
- Chord → Action

Mappings can be global or restricted to a foreground process such as `game.exe`.

## Consume / pass-through

A mapping can consume the source input so Windows/the foreground app does not also receive it, or pass it through while triggering the mapped action.

## Recursive trigger protection

VxClick marks its own synthetic `SendInput` events with a dedicated `dwExtraInfo` tag. The low-level hook filters those events so an output does not recursively trigger its own mapping.

External remapping tools are not automatically treated as VxClick-generated input; this keeps integrations such as mouse-software remaps usable unless a rule explicitly requires physical-only input.
