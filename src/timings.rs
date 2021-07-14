use std::fmt;
use std::time::Instant;
use tracing::field::{display, DisplayValue};

pub struct Timings {
    idle: u64,
    busy: u64,
    last: Instant,
}

// This type is basically copied from:
// https://docs.rs/tracing-subscriber/0.2.19/src/tracing_subscriber/fmt/fmt_layer.rs.html#898
impl Timings {
    pub fn new() -> Self {
        Timings {
            idle: 0,
            busy: 0,
            last: Instant::now(),
        }
    }

    pub fn now_idle(&mut self) {
        let now = Instant::now();
        self.busy += (now - self.last).as_nanos() as u64;
        self.last = now;
    }

    // Returns how long it was idle for, if it was started
    pub fn now_busy(&mut self) {
        let now = Instant::now();
        self.idle += (now - self.last).as_nanos() as u64;
        self.last = now;
    }

    pub fn idle(&self) -> u64 {
        self.idle
    }

    pub fn busy(&self) -> u64 {
        self.busy
    }

    pub fn display_busy(&self) -> DisplayValue<TimingDisplay> {
        display(TimingDisplay(self.busy()))
    }

    pub fn display_idle(&self) -> DisplayValue<TimingDisplay> {
        display(TimingDisplay(self.idle()))
    }
}

// Thanks for making this private :(, guess I'll just copy paste
// https://github.com/tokio-rs/tracing/blob/c848820fc62c274d3df1be61303d97f3b6802673/tracing-subscriber/src/fmt/format/mod.rs#L1229-L1245
pub struct TimingDisplay(u64);
impl fmt::Display for TimingDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut t = self.0 as f64;
        for unit in ["ns", "µs", "ms", "s"].iter() {
            if t < 10.0 {
                return write!(f, "{:.2}{}", t, unit);
            } else if t < 100.0 {
                return write!(f, "{:.1}{}", t, unit);
            } else if t < 1000.0 {
                return write!(f, "{:.0}{}", t, unit);
            }
            t /= 1000.0;
        }
        write!(f, "{:.0}s", t * 1000.0)
    }
}
