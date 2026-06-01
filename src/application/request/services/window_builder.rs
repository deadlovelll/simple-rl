use std::time::{SystemTime, UNIX_EPOCH};

const WINDOW_SIZE: u64 = 60;

pub struct WindowBuilder {}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let window = now / WINDOW_SIZE;
        window
    }
}