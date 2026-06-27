//! P5: Job management for the desktop client.
//!
//! Provides timestamp helpers used by the UI conversion flow.

pub(crate) fn chrono_now_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let mins = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    format!("{}d{:02}h{:02}m{:02}s", days, hours, mins, seconds)
}
