//! Tabs, rows of columns and the camera spring that scrolls them.

use crate::*;

pub(crate) struct Tab {
    pub(crate) container: gtk::Box,
    pub(crate) view: WebView,
    /// Oxirgi kadr (freeze) — uxlaganda skrinshot o'rnida turadi
    pub(crate) freeze: gtk::Picture,
    /// Horizontal strip between stacked tabs — drag up/down to set height
    pub(crate) height_handle: gtk::Box,
    pub(crate) profile_id: String,
    pub(crate) custom_width: Cell<Option<f64>>,
    /// Height inside a stacked column (None = equal share)
    pub(crate) custom_height: Cell<Option<f64>>,
    /// Only meaningful on the first tab of a column: show one focused window with tabbed switching.
    pub(crate) tabbed: Cell<bool>,
    /// Consecutive tabs of a row with the same id share one column, stacked vertically (like niri)
    pub(crate) col_id: Cell<u64>,
    /// Oxirgi ma'lum http(s) manzil. WebKit web process to'xtatilganda uri() ni tozalab yuboradi,
    /// shuning uchun uyg'otish va sessiyani saqlash shu nusxadan foydalanadi.
    pub(crate) saved_uri: RefCell<String>,
    pub(crate) suspended: Cell<bool>,
    /// Popup (window.open) bilan bog'langan tablar bitta web process'ni bo'lishadi: ularni to'xtatib bo'lmaydi
    pub(crate) keep_alive: Cell<bool>,
    pub(crate) last_active: Cell<std::time::Instant>,
    /// Per-tab docked DevTools inspector container
    pub(crate) dt_box: gtk::Box,
    pub(crate) dt_handle: gtk::Box,
    pub(crate) tab_inner: gtk::Box,
    pub(crate) overlay: gtk::Overlay,
    pub(crate) dt_open: Cell<bool>,
    pub(crate) dt_height: Cell<i32>,
    pub(crate) dt_width: Cell<i32>,
    pub(crate) dt_dock: Cell<DevToolsDockSide>,
}

impl Tab {
    /// Haqiqiy manzil: jonli uri bo'sh bo'lsa (to'xtatilgan tab) saqlangan nusxa
    pub(crate) fn real_uri(&self) -> String {
        let live = self.view.uri().map(|s| s.to_string()).unwrap_or_default();
        // Start page is not a real destination — don't persist it.
        // Settings / downloads *are* real pages and must survive session restore.
        if live.starts_with("rail://start") {
            return String::new();
        }
        if live.starts_with("rail://settings") || live.starts_with("rail://downloads") {
            return live;
        }
        if is_internal_uri(&live) {
            return String::new();
        }
        if live.is_empty() {
            self.saved_uri.borrow().clone()
        } else {
            live
        }
    }
}

pub(crate) struct Row {
    pub(crate) tabs: Vec<Tab>,
}

pub(crate) static COL_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A tab with a fresh id lives in a column of its own.
pub(crate) fn new_col_id() -> u64 {
    COL_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn set_full_class(w: &gtk::Box, full: bool) {
    if full {
        w.add_css_class("full");
    } else {
        w.remove_css_class("full");
    }
}

impl Tab {
    /// Width of a brand-new tab: full width, no manual size.
    pub(crate) fn reset_col_props(&self) {
        self.custom_width.set(None);
        self.tabbed.set(false);
        set_full_class(&self.container, true);
    }
}

impl Row {
    pub(crate) fn ids(&self) -> Vec<u64> {
        self.tabs.iter().map(|t| t.col_id.get()).collect()
    }

    /// (start, end) of the column that holds tab `c`; `end` is exclusive.
    pub(crate) fn columns(&self) -> Vec<(usize, usize)> {
        group_ranges(&self.ids())
    }

    pub(crate) fn col_span(&self, c: usize) -> (usize, usize) {
        for (s, len) in self.columns() {
            if c >= s && c < s + len {
                return (s, s + len);
            }
        }
        (c, c + 1)
    }

    /// A column's width lives on its first tab; hands it over from one tab to another.
    pub(crate) fn move_col_props(&self, from: usize, to: usize) {
        let full = self.tabs[from].container.has_css_class("full");
        let tabbed = self.tabs[from].tabbed.get();
        self.tabs[to].tabbed.set(tabbed);
        self.tabs[to]
            .custom_width
            .set(self.tabs[from].custom_width.get());
        set_full_class(&self.tabs[to].container, full);
        self.tabs[from].reset_col_props();
    }

    /// Swaps two tabs but leaves the column width where it was (it belongs to the position).
    pub(crate) fn swap_keep_props(&mut self, i: usize, j: usize) {
        let (wi, wj) = (
            self.tabs[i].custom_width.get(),
            self.tabs[j].custom_width.get(),
        );
        let (ti, tj) = (self.tabs[i].tabbed.get(), self.tabs[j].tabbed.get());
        let (fi, fj) = (
            self.tabs[i].container.has_css_class("full"),
            self.tabs[j].container.has_css_class("full"),
        );
        self.tabs.swap(i, j);
        self.tabs[i].custom_width.set(wi);
        self.tabs[j].custom_width.set(wj);
        self.tabs[i].tabbed.set(ti);
        self.tabs[j].tabbed.set(tj);
        set_full_class(&self.tabs[i].container, fi);
        set_full_class(&self.tabs[j].container, fj);
    }

    /// Takes tab `c` out of the row. The tab ends up in a column of its own; if it was the
    /// first tab of a stack, the rest of the stack keeps the column width.
    pub(crate) fn detach(&mut self, c: usize) -> Tab {
        let (s, e) = self.col_span(c);
        if e - s > 1 {
            if c == s {
                self.move_col_props(s, s + 1);
            } else {
                self.tabs[c].reset_col_props();
            }
        }
        let tab = self.tabs.remove(c);
        tab.custom_height.set(None);
        tab.tabbed.set(false);
        tab.col_id.set(new_col_id());
        tab
    }
}

/// Columns of a row as (first tab, tab count, x, width).
/// One running camera spring (x or y). `start_us` is a GDK frame-clock time.
#[derive(Clone, Copy)]
pub(crate) struct CamSpring {
    pub(crate) spring: Spring,
    pub(crate) start_us: i64,
}

/// Advances one camera axis with a niri-style spring and returns `(value, at_rest)`.
/// Retargeting mid-flight keeps the current speed, so chained moves stay smooth.
/// `v0_hint` is a release velocity (touchpad flick) used when a new spring starts.
pub(crate) fn step_cam_axis(
    slot: &Cell<Option<CamSpring>>,
    cur: f64,
    target: f64,
    now_us: i64,
) -> (f64, bool) {
    let mut cs = slot.get();
    match cs {
        Some(c) if (c.spring.to - target).abs() > 0.01 => {
            let t = (now_us - c.start_us) as f64 / 1e6;
            let v = c.spring.velocity_at(t);
            cs = Some(CamSpring {
                spring: Spring::new(cur, target, v, SLIDE_DAMPING_RATIO, SLIDE_STIFFNESS),
                start_us: now_us,
            });
        }
        None if (cur - target).abs() > 0.01 => {
            cs = Some(CamSpring {
                spring: Spring::new(cur, target, 0.0, SLIDE_DAMPING_RATIO, SLIDE_STIFFNESS),
                start_us: now_us,
            });
        }
        _ => {}
    }
    let Some(c) = cs else {
        slot.set(None);
        return (target, true);
    };
    let t = (now_us - c.start_us) as f64 / 1e6;
    if c.spring.is_settled(t) {
        slot.set(None);
        return (target, true);
    }
    slot.set(Some(c));
    (c.spring.value_at(t), false)
}

pub(crate) fn row_cols(row: &Row, vw: f64, g: f64) -> Vec<(usize, usize, f64, f64)> {
    let mut out = Vec::new();
    let mut x = 0.0;
    for (start, len) in row.columns() {
        let head = &row.tabs[start];
        let w = clamp_column_width(
            head.custom_width
                .get()
                .unwrap_or_else(|| col_width(head.container.has_css_class("full"), vw)),
            vw,
        );
        out.push((start, len, x, w));
        x += w + g;
    }
    out
}
