pub struct Note {
    ms: i32,
    lane: u8,
}

impl Note {
    pub const fn new(ms: i32, lane: u8) -> Self {
        Self { ms, lane }
    }

    pub const fn delta(&self, time: std::time::Duration) -> i32 {
        time.as_millis() as i32 - self.ms
    }
}
