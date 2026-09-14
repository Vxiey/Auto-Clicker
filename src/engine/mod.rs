mod stats;

pub use stats::{BenchmarkStats, SampleStats};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

impl MouseButton {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "left" | "l" => Some(Self::Left),
            "right" | "r" => Some(Self::Right),
            "middle" | "m" => Some(Self::Middle),
            "x1" | "back" => Some(Self::X1),
            "x2" | "forward" => Some(Self::X2),
            _ => None,
        }
    }
}
