use std::time::Instant;

pub struct Smooth {
    from: f32,
    to: f32,
}

impl Smooth {
    pub const fn new(value: f32) -> Self {
        Self {
            from: value,
            to: value,
        }
    }

    pub const fn interpolate(&self, t: f32) -> f32 {
        t * self.to + (1.0 - t) * self.from
    }

    pub const fn set(&mut self, to: f32) {
        self.to = to;
    }

    pub const fn stabilize(&mut self) {
        self.from = self.to;
    }

    pub const fn shift_set(&mut self, to: f32, t: f32) {
        self.from = self.interpolate(t);
        self.to = to;
    }
}

pub struct TimedSmooth {
    smooth: Smooth,
    last_modified: Instant,
    now_cached: Instant,
    transition_duration: f32,
}

impl TimedSmooth {
    pub fn new(value: f32, transition_duration: f32) -> Self {
        let now = Instant::now();

        Self {
            smooth: Smooth::new(value),
            last_modified: now,
            now_cached: now,
            transition_duration,
        }
    }

    fn elapsed(&self) -> f32 {
        (self.now_cached - self.last_modified).as_secs_f32()
    }

    fn ratio(&self) -> f32 {
        (self.elapsed() / self.transition_duration).min(1.0)
    }

    fn ratio_curved(&self) -> f32 {
        let r = self.ratio();
        r * (2.0 - r)
    }

    pub fn update(&mut self) {
        self.now_cached = Instant::now();
    }

    pub fn interpolate(&self) -> f32 {
        self.smooth.interpolate(self.ratio_curved())
    }

    pub fn shift_set(&mut self, to: f32) {
        self.smooth.shift_set(to, self.ratio_curved());
        self.last_modified = self.now_cached;
    }
}
