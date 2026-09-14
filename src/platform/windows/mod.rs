mod clock;
mod hooks;
mod input;
mod precision_clicker;

pub use clock::QpcClock;
pub use hooks::{CapturedInput, CapturedInputKind, CapturedInputSource, GlobalInputRecorder};
pub use input::WindowsInput;
pub use precision_clicker::{ClickerConfig, LiveClickerConfig, PrecisionClicker};
