mod clock;
mod hooks;
mod hotkeys;
mod input;
mod precision_clicker;

pub use clock::QpcClock;
pub use hooks::{CapturedInput, CapturedInputKind, CapturedInputSource, GlobalInputRecorder};
pub use hotkeys::{HotkeyEvent, HotkeyPhase, RegisteredHotkey, WindowsHotkeyManager};
pub use input::WindowsInput;
pub use precision_clicker::{ClickerConfig, LiveClickerConfig, PrecisionClicker};
