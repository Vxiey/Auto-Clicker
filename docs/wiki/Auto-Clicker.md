# Auto Clicker

VxClick's click engine uses native Windows `SendInput` and a high-resolution scheduler built around absolute deadlines.

## Modes

- **Toggle** — press the configured hotkey to start and press it again to stop.
- **Hold** — click while the hotkey remains pressed.
- **Once** — issue a single click when the hotkey is pressed.

## Buttons

Left, right, middle, X1 / Mouse 4 and X2 / Mouse 5 are supported.

## Timing

Set the target using CPS or interval. Normal playback uses absolute deadlines to avoid cumulative drift. Randomization can vary the interval around the selected target. Burst mode sends small groups of clicks when throughput matters more than even spacing.

## Position modes

- **Current cursor** — click wherever the cursor currently is.
- **Fixed** — use one X,Y coordinate.
- **Multi-point** — cycle through a semicolon-separated list of coordinates.

## Safety

Always configure the emergency-stop hotkey before running long or unattended automation. Emergency Stop requests clicker/macro shutdown and releases VxClick-held synthetic inputs on a best-effort basis.
