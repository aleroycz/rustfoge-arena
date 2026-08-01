use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Time {
    startup: Instant,
    last_update: Instant,
    delta: Duration,
    elapsed: Duration,

    pub fixed_delta: Duration,

    accumulator: Duration,
}

impl Time {
    pub fn new(fixed_hz: f64) -> Self {
        let now = Instant::now();
        Self {
            startup: now,
            last_update: now,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
            fixed_delta: Duration::from_secs_f64(1.0 / fixed_hz),
            accumulator: Duration::ZERO,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = now.duration_since(self.last_update);

        if self.delta > Duration::from_millis(250) {
            self.delta = Duration::from_millis(250);
        }
        self.last_update = now;
        self.elapsed = now.duration_since(self.startup);
        self.accumulator += self.delta;
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn consume_fixed_step(&mut self) -> bool {
        if self.accumulator >= self.fixed_delta {
            self.accumulator -= self.fixed_delta;
            true
        } else {
            false
        }
    }

    pub fn fixed_alpha(&self) -> f32 {
        self.accumulator.as_secs_f32() / self.fixed_delta.as_secs_f32()
    }
}
