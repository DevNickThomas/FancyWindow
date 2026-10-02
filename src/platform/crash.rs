//! Writes panics to crash.log next to the executable, so a vanished window leaves a clue.

use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use super::storage::exe_dir;
use crate::model::crash_log_name;

/// Installs a panic hook that appends to `crash.log` (or `crash-<profile>.log`).
pub fn install(profile: Option<&str>) {
    let path = exe_dir().join(crash_log_name(profile));
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let backtrace = std::backtrace::Backtrace::force_capture();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "---\nUnix time: {seconds}\n{info}\n{backtrace}");
        }
        default_hook(info);
    }));
}
