# Macro Studio

Macro Studio records, edits, stores and plays keyboard/mouse automation.

## Recording

Recording uses the native Windows input hook, so capture continues while another application has focus. VxClick-injected events are tagged and ignored by the recorder to avoid feedback loops.

## Timeline

Recorded events are stored as keyboard down/up, mouse down/up, movement/wheel and delay events. Delays can be edited directly before playback.

## Playback modes

- No Repeat
- Repeat While Holding
- Toggle
- Sequence

Sequence macros can separate **On press**, **While holding** and **On release** actions.

## Timing

Playback uses the same high-resolution QPC-based scheduler as the click engine. Speed scaling changes the macro timeline without replacing absolute-deadline scheduling.

## Safety

Stopping playback triggers best-effort release of keys and mouse buttons still held by VxClick.
