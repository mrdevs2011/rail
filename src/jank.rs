//! Main-loop jank counter.
//!
//! The 100 ms watchdog timer in `main.rs` reports how late it fired. That lateness is how long the UI
//! thread was busy when the timer was due, so every value above ~16 ms is a frame that could not have
//! been drawn on time. This module only aggregates those samples (no allocation, no I/O per sample) and
//! appends one summary line per window to `jank.log`, so a normal session on real hardware yields numbers
//! instead of guesses.
//!
//! What it measures: main-loop responsiveness. It does NOT measure GPU/compositor frame times.

use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

/// One summary line is written per window.
const WINDOW: Duration = Duration::from_secs(60);
/// `jank.log` is truncated once it grows past this many bytes.
const MAX_LOG_BYTES: u64 = 256 * 1024;

pub struct Jank {
    window_start: Instant,
    ticks: u32,
    over16: u32,
    over33: u32,
    over100: u32,
    max_ms: u64,
}

impl Jank {
    pub fn new() -> Self {
        Self {
            window_start: Instant::now(),
            ticks: 0,
            over16: 0,
            over33: 0,
            over100: 0,
            max_ms: 0,
        }
    }

    /// `late` = how much later than scheduled the watchdog timer fired.
    pub fn record(&mut self, late: Duration) {
        let ms = late.as_millis() as u64;
        self.ticks += 1;
        if ms > 16 {
            self.over16 += 1;
        }
        if ms > 33 {
            self.over33 += 1;
        }
        if ms > 100 {
            self.over100 += 1;
        }
        self.max_ms = self.max_ms.max(ms);
    }

    fn summary(&self, unix_secs: u64) -> String {
        format!(
            "{} window={}s ticks={} late>16ms={} late>33ms={} late>100ms={} max={}ms",
            unix_secs,
            WINDOW.as_secs(),
            self.ticks,
            self.over16,
            self.over33,
            self.over100,
            self.max_ms
        )
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    /// Writes the summary and starts a new window once the current one is full. Cheap when it is not.
    pub fn maybe_report(&mut self, data_dir: &Path) {
        if self.window_start.elapsed() < WINDOW {
            return;
        }
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = data_dir.join("jank.log");
        if std::fs::metadata(&path)
            .map(|m| m.len() > MAX_LOG_BYTES)
            .unwrap_or(false)
        {
            let _ = std::fs::write(&path, "");
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{}", self.summary(secs));
        }
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_cumulative_and_max_is_tracked() {
        let mut j = Jank::new();
        for ms in [0u64, 5, 16, 17, 33, 34, 100, 101, 250] {
            j.record(Duration::from_millis(ms));
        }
        assert_eq!(j.ticks, 9);
        assert_eq!(j.over16, 6); // 17, 33, 34, 100, 101, 250
        assert_eq!(j.over33, 4); // 34, 100, 101, 250
        assert_eq!(j.over100, 2); // 101, 250
        assert_eq!(j.max_ms, 250);
    }

    #[test]
    fn summary_format_and_reset() {
        let mut j = Jank::new();
        j.record(Duration::from_millis(40));
        assert_eq!(
            j.summary(1),
            "1 window=60s ticks=1 late>16ms=1 late>33ms=1 late>100ms=0 max=40ms"
        );
        j.reset();
        assert_eq!(j.ticks, 0);
        assert_eq!(j.max_ms, 0);
    }
}
