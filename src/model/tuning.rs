//! Animation timing and tab suspension policy (memory pressure aware).

/// How far the strip zooms out in overview (niri uses a similar factor).
pub(crate) const OV_ZOOM: f64 = 0.4;
/// Overview zoom in/out duration (fixed, ease-out): zoom, pan and grid land together.
pub(crate) const OV_ANIM_SECS: f64 = 0.24;

/// Xotira tanqis (yoki RAIL_LOW_MEM=1 yoki swap to'liq): tez uxlatish, 3 ta tirik tab.
pub(crate) const SUSPEND_AFTER_SECS: u64 = 60;
pub(crate) const MAX_LIVE_TABS: usize = 3;

/// Xotira yetarli: 10 daqiqa, 6 ta tirik tab (har biri ~80–200 MB web process).
pub(crate) const ROOMY_SUSPEND_AFTER_SECS: u64 = 600;
pub(crate) const ROOMY_MAX_LIVE_TABS: usize = 6;

/// O'rta pog'ona: 3 daqiqa, 4 ta tirik tab.
pub(crate) const TIGHT_SUSPEND_AFTER_SECS: u64 = 180;
pub(crate) const TIGHT_MAX_LIVE_TABS: usize = 4;

/// Bo'sh start-page tab — tez uxlatiladi.
pub(crate) const BLANK_TAB_SUSPEND_SECS: u64 = 20;

/// Memory pressure from PSI (`/proc/pressure/memory`, "some avg10").
pub(crate) fn memory_pressure_avg10() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/pressure/memory").ok()?;
    let line = text.lines().find(|l| l.starts_with("some"))?;
    line.split_whitespace().find_map(|t| t.strip_prefix("avg10="))?.parse().ok()
}

/// MemAvailable (MiB) from /proc/meminfo.
pub(crate) fn mem_available_mb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

/// Swap nearly full? System is paging — suspend aggressively even if MemAvailable looks OK.
pub(crate) fn swap_under_pressure() -> bool {
    let text = match std::fs::read_to_string("/proc/meminfo") {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut total = 0u64;
    let mut free = 0u64;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("SwapTotal:") {
            total = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("SwapFree:") {
            free = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    if total < 1024 { return false; } // no meaningful swap
    // <15% free or <256 MiB free while most swap is used
    free * 100 / total < 15 || (total - free > 512 * 1024 && free < 256 * 1024)
}

/// (suspend_after_secs, max_live_tabs) for the current memory situation.
pub(crate) fn suspend_policy() -> (u64, usize) {
    if std::env::var_os("RAIL_LOW_MEM").is_some_and(|v| v != "0") {
        return (SUSPEND_AFTER_SECS, MAX_LIVE_TABS);
    }
    // Swap full = system is thrashing; do not trust MemAvailable alone.
    if swap_under_pressure() {
        return (SUSPEND_AFTER_SECS, MAX_LIVE_TABS);
    }
    let pressure = memory_pressure_avg10().unwrap_or(0.0);
    match mem_available_mb() {
        Some(m) if m < 1500 => (SUSPEND_AFTER_SECS, MAX_LIVE_TABS),
        _ if pressure >= 8.0 => (SUSPEND_AFTER_SECS, MAX_LIVE_TABS),
        Some(m) if m < 3000 => (TIGHT_SUSPEND_AFTER_SECS, TIGHT_MAX_LIVE_TABS),
        _ if pressure >= 1.0 => (TIGHT_SUSPEND_AFTER_SECS, TIGHT_MAX_LIVE_TABS),
        _ => (ROOMY_SUSPEND_AFTER_SECS, ROOMY_MAX_LIVE_TABS),
    }
}
