//! Animation timing and tab suspension policy (memory pressure aware).

/// Physical overview scale at the user's 100% zoom setting.
pub(crate) const OV_ZOOM: f64 = 0.4;
/// User-facing page and overview zoom range, independent of the overview's physical base scale.
pub(crate) const PAGE_ZOOM_MIN: f64 = 0.2;
pub(crate) const PAGE_ZOOM_MAX: f64 = 5.0;
pub(crate) const OV_ZOOM_MIN: f64 = 0.2;
pub(crate) const OV_ZOOM_MAX: f64 = 5.0;
/// Fast page zoom steps (Ctrl +/-/0), 20%..500%.
pub(crate) const PAGE_ZOOM_STEPS: [f64; 16] = [
    0.20, 0.25, 0.33, 0.50, 0.67, 0.75, 0.80, 1.0, 1.25, 1.50, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0,
];

/// Next page zoom step above (`dir > 0`) or below (`dir < 0`) `cur`; clamped to 20%..500%.
pub(crate) fn next_page_zoom(cur: f64, dir: i32) -> f64 {
    const EPS: f64 = 0.005;
    let min = PAGE_ZOOM_MIN;
    let max = PAGE_ZOOM_MAX;
    let cur = if cur.is_finite() {
        cur.clamp(min, max)
    } else {
        1.0
    };
    if dir > 0 {
        PAGE_ZOOM_STEPS
            .iter()
            .copied()
            .find(|z| *z > cur + EPS)
            .unwrap_or(max)
    } else {
        PAGE_ZOOM_STEPS
            .iter()
            .rev()
            .copied()
            .find(|z| *z < cur - EPS)
            .unwrap_or(min)
    }
}

/// Apply a continuous zoom ratio without depending on frame rate or key-repeat cadence.
pub(crate) fn scale_zoom(current: f64, factor: f64, min: f64, max: f64) -> f64 {
    if !min.is_finite() || !max.is_finite() || min <= 0.0 || max < min {
        return 1.0;
    }
    let current = if current.is_finite() { current } else { 1.0 }.clamp(min, max);
    if !factor.is_finite() || factor <= 0.0 {
        return current;
    }
    (current * factor).clamp(min, max)
}

/// Overview's user-facing percentage (1.0 = 100%) converted to the physical strip scale.
pub(crate) fn overview_scale(user_zoom: f64) -> f64 {
    let normalized = if user_zoom.is_finite() {
        user_zoom
    } else {
        1.0
    };
    OV_ZOOM * normalized.clamp(OV_ZOOM_MIN, OV_ZOOM_MAX)
}

/// Overview zoom in/out duration (fixed, ease-out): zoom, pan and grid land together.
pub(crate) const OV_ANIM_SECS: f64 = 0.18;

/// Severe memory pressure only: reclaim a small number of inactive pages quickly.
pub(crate) const SUSPEND_AFTER_SECS: u64 = 60;
pub(crate) const MAX_LIVE_TABS: usize = 4;

/// Healthy-memory mode: keep a background page alive for an hour before reclaiming it.
pub(crate) const ROOMY_SUSPEND_AFTER_SECS: u64 = 3600;

/// Moderate memory pressure: preserve a useful working set for at least ten minutes.
pub(crate) const TIGHT_SUSPEND_AFTER_SECS: u64 = 600;
pub(crate) const TIGHT_MAX_LIVE_TABS: usize = 8;

/// Bo'sh start-page tab — tez uxlatiladi.
pub(crate) const BLANK_TAB_SUSPEND_SECS: u64 = 20;

/// Memory pressure from PSI (`/proc/pressure/memory`, "some avg10").
pub(crate) fn memory_pressure_avg10() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/pressure/memory").ok()?;
    let line = text.lines().find(|l| l.starts_with("some"))?;
    line.split_whitespace()
        .find_map(|t| t.strip_prefix("avg10="))?
        .parse()
        .ok()
}

/// MemAvailable (MiB) from /proc/meminfo.
pub(crate) fn mem_available_mb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

/// Total physical memory (MiB) from /proc/meminfo.
pub(crate) fn mem_total_mb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

/// Scale the healthy-memory cap to installed RAM (roughly one live tab per GiB,
/// bounded to avoid unbounded WebKit process growth on very large workstations).
pub(crate) fn roomy_max_live_tabs(total_mb: Option<u64>) -> usize {
    let total_mb = total_mb.unwrap_or(8 * 1024);
    total_mb.saturating_add(1023).div_euclid(1024).clamp(6, 20) as usize
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
            total = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("SwapFree:") {
            free = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }
    if total < 1024 {
        return false;
    } // no meaningful swap
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
        _ => (
            ROOMY_SUSPEND_AFTER_SECS,
            roomy_max_live_tabs(mem_total_mb()),
        ),
    }
}
