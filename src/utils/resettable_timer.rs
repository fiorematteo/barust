use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct ResettableTimer {
    pub duration: Duration,
    timer: Instant,
}

impl ResettableTimer {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            timer: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        self.timer = Instant::now();
    }

    pub fn is_done(&self) -> bool {
        self.timer.elapsed() > self.duration
    }
}
