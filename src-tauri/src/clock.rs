use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch. Returns 0 if the system clock is set
/// before 1970 and saturates instead of wrapping.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
