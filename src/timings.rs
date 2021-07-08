use serde::Serialize;
use std::time::Instant;

#[derive(Default, Serialize)]
pub struct Timings {
    pub idle: u64,
    pub busy: u64,
}

pub struct Stopwatch {
    timings: Timings,
    last: Instant,
}

// This type is basically copied from:
// https://docs.rs/tracing-subscriber/0.2.19/src/tracing_subscriber/fmt/fmt_layer.rs.html#898
impl Stopwatch {
    pub fn new() -> Self {
        Stopwatch {
            timings: Timings::default(),
            last: Instant::now(),
        }
    }

    pub fn now_idle(&mut self) {
        let now = Instant::now();
        self.timings.busy += (now - self.last).as_nanos() as u64;
        self.last = now;
    }

    pub fn now_busy(&mut self) {
        let now = Instant::now();
        self.timings.idle += (now - self.last).as_nanos() as u64;
        self.last = now;
    }
}
