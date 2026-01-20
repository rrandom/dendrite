use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in milliseconds
pub fn now() -> u64 {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}
