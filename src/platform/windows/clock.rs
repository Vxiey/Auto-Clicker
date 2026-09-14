use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

#[derive(Debug, Clone, Copy)]
pub struct QpcClock {
    frequency: i64,
}

impl QpcClock {
    pub fn new() -> Result<Self, String> {
        let mut frequency = 0_i64;
        let ok = unsafe { QueryPerformanceFrequency(&mut frequency) };
        if ok == 0 || frequency <= 0 {
            return Err("QueryPerformanceFrequency failed".into());
        }
        Ok(Self { frequency })
    }

    #[inline(always)]
    pub fn now_ticks(&self) -> i64 {
        let mut value = 0_i64;
        unsafe {
            QueryPerformanceCounter(&mut value);
        }
        value
    }

    #[inline(always)]
    pub fn frequency(&self) -> i64 {
        self.frequency
    }

    #[inline(always)]
    pub fn ticks_to_micros(&self, ticks: i64) -> f64 {
        ticks as f64 * 1_000_000.0 / self.frequency as f64
    }
}
