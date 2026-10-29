//! The niri-style 2D tiled layout: a custom GTK layout manager that places
//! columns horizontally and rows (workspaces) vertically, sliding by (offset_x, offset_y).

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{graphene, gsk};
use gtk4 as gtk;

pub const GAP: f64 = 6.0;

/// Space between columns and rows, in strip coordinates.
/// Must be **constant** (not a function of zoom): otherwise every animation frame
/// reflows the grid and the overview looks like it slides sideways.
pub fn gap(_zoom: f64) -> f64 {
    gap_px()
}

/// Fullscreen (F11) da faqat kontent: tab oralig'i 0. Bu faqat fullscreen almashganda o'zgaradi,
/// animatsiya kadrlarida emas — shuning uchun "gap doimiy" qoidasi buzilmaydi.
static CONTENT_ONLY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_content_only(on: bool) {
    CONTENT_ONLY.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn gap_px() -> f64 {
    if CONTENT_ONLY.load(std::sync::atomic::Ordering::Relaxed) {
        0.0
    } else {
        GAP
    }
}

/// Minimum width for a column to be usable.
pub const MIN_COL_W: f64 = 480.0;

/// Column width calculation
pub fn col_width(full: bool, viewport: f64) -> f64 {
    if full || viewport < (MIN_COL_W * 2.0 + gap_px()) {
        viewport.max(280.0)
    } else {
        ((viewport - gap_px()) / 2.0).floor()
    }
}

/// Smooth animated column width calculation (progress 0.0: normal, 1.0: full)
pub fn col_width_animated(progress: f64, viewport: f64) -> f64 {
    let p = progress.clamp(0.0, 1.0);
    if viewport < (MIN_COL_W * 2.0 + gap_px()) {
        viewport.max(280.0)
    } else {
        let normal_w = ((viewport - gap_px()) / 2.0).floor();
        let full_w = viewport.max(280.0);
        normal_w + (full_w - normal_w) * p
    }
}

/// Height of one window inside a column that stacks `k` windows (niri-style column).
pub fn stack_height(vh: f64, k: usize, g: f64) -> f64 {
    if k <= 1 {
        return vh;
    }
    ((vh - (k as f64 - 1.0) * g) / k as f64).max(80.0)
}

/// Consecutive equal ids form one column: returns (start, len) of every column.
pub fn group_ranges(ids: &[u64]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < ids.len() {
        let mut j = i + 1;
        while j < ids.len() && ids[j] == ids[i] {
            j += 1;
        }
        out.push((i, j - i));
        i = j;
    }
    out
}

mod imp {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Default)]
    pub struct StripLayout {
        /// Left edge of the visible region (X), in unscaled coordinates.
        pub offset_x: Cell<f64>,
        /// Top edge of the visible region (Y), in unscaled coordinates.
        pub offset_y: Cell<f64>,
        /// 1.0 normally; < 1.0 in overview, where the 2D grid is scaled down.
        pub zoom: Cell<f64>,
        pub width: Cell<i32>,
        pub height: Cell<i32>,
        pub on_resize: RefCell<Option<Rc<dyn Fn()>>>,
        /// Called with (child, now_visible) right BEFORE a column is hidden (so the app can grab a last
        /// frame) and right after it is shown again.
        pub on_visibility: RefCell<Option<Rc<dyn Fn(&gtk::Widget, bool)>>>,
        /// For each row: list of is_full flags for each column
        pub grid: RefCell<Vec<Vec<bool>>>,
        /// Target progress (0.0 for normal, 1.0 for full)
        pub target_grid: RefCell<Vec<Vec<f64>>>,
        /// Current animated progress (0.0 .. 1.0)
        pub current_grid: RefCell<Vec<Vec<f64>>>,
        /// Custom widths per column in each row
        pub custom_widths: RefCell<Vec<Vec<Option<f64>>>>,
        /// Custom heights per tab (stack splits); None = equal share
        pub custom_heights: RefCell<Vec<Vec<Option<f64>>>>,
        /// Per row: column id of every tab. Consecutive equal ids are stacked in one column.
        pub col_ids: RefCell<Vec<Vec<u64>>>,
        /// Sahifa elementi (video va h.k.) to'liq ekran: shu tab butun oynani egallaydi,
        /// qolgan tablar (split, yon tomondagilar) yashiriladi.
        pub solo: RefCell<Option<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StripLayout {
        const NAME: &'static str = "RailStripLayout";
        type Type = super::StripLayout;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for StripLayout {}

    impl StripLayout {
        /// Columns far outside the viewport are unmapped (`set_child_visible(false)`). WebKit derives page
        /// visibility from the widget's mapped state, so hidden pages stop painting and their timers /
        /// requestAnimationFrame are throttled — without this every off-screen tab runs at full speed.
        /// A margin keeps neighbours mapped so they are already painted when they slide into view, and the
        /// column holding keyboard focus is never hidden.
        #[allow(clippy::too_many_arguments)]
        fn apply_visibility(
            &self,
            c: &gtk::Widget,
            x: f64,
            y: f64,
            w: f64,
            h: f64,
            vw: f64,
            vh: f64,
            focus: Option<&gtk::Widget>,
        ) {
            if vw <= 1.0 || vh <= 1.0 {
                return;
            }
            let (mx, my) = (vw * 0.25, vh * 0.25);
            let mut vis = x + w > -mx && x < vw + mx && y + h > -my && y < vh + my;
            if !vis {
                if let Some(f) = focus {
                    if f == c || f.is_ancestor(c) {
                        vis = true;
                    }
                }
            }
            if vis == c.is_child_visible() {
                return;
            }
            let cb = self.on_visibility.borrow().clone();
            if !vis {
                if let Some(cb) = &cb {
                    cb(c, false);
                }
            }
            c.set_child_visible(vis);
            if vis {
                if let Some(cb) = &cb {
                    cb(c, true);
                }
            }
        }
    }

    impl LayoutManagerImpl for StripLayout {
        fn measure(
            &self,
            _w: &gtk::Widget,
            _o: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            (0, 0, -1, -1)
        }

        fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
            let resized =
                self.width.replace(width) != width || self.height.replace(height) != height;
            if resized {
                if let Some(cb) = self.on_resize.borrow().clone() {
                    glib::idle_add_local_once(move || cb());
                }
            }
            let vw = width as f64;
            let vh = height as f64;
            let (off_x, off_y, z) = (self.offset_x.get(), self.offset_y.get(), self.zoom.get());
            let g = gap(z);
            let grid = self.grid.borrow();
            let cur_prog = self.current_grid.borrow();
            let c_widths = self.custom_widths.borrow();
            let c_heights = self.custom_heights.borrow();
            let mut child = widget.first_child();
            let focus = widget.root().and_then(|r| r.focus());

            // Element to'liq ekran: faqat solo tab, butun oyna, hech qanday kamera siljishisiz
            let solo = self.solo.borrow().clone();
            if let Some(solo) = solo.filter(|s| s.parent().as_ref() == Some(widget)) {
                let cb = self.on_visibility.borrow().clone();
                while let Some(c) = child {
                    child = c.next_sibling();
                    if c == solo {
                        if !c.is_child_visible() {
                            c.set_child_visible(true);
                            if let Some(cb) = &cb {
                                cb(&c, true);
                            }
                        }
                        c.allocate(width, height, -1, None);
                    } else if c.is_child_visible() {
                        if let Some(cb) = &cb {
                            cb(&c, false);
                        }
                        c.set_child_visible(false);
                    }
                }
                return;
            }

            if grid.is_empty() {
                let mut x = 0.0;
                while let Some(c) = child {
                    let cw = col_width(c.has_css_class("full"), vw);
                    let (tx, ty) = (((x - off_x) * z) as f32, ((0.0 - off_y) * z) as f32);
                    let t = gsk::Transform::new()
                        .translate(&graphene::Point::new(tx, ty))
                        .scale(z as f32, z as f32);
                    self.apply_visibility(
                        &c,
                        tx as f64,
                        ty as f64,
                        cw * z,
                        height as f64 * z,
                        vw,
                        vh,
                        focus.as_ref(),
                    );
                    c.allocate(cw as i32, height, -1, Some(t));
                    x += cw + g;
                    child = c.next_sibling();
                }
                return;
            }

            let col_ids = self.col_ids.borrow();
            for (r_idx, row) in grid.iter().enumerate() {
                let row_y = r_idx as f64 * (vh + g);
                let ids = col_ids.get(r_idx).filter(|v| v.len() == row.len());
                let groups = match ids {
                    Some(v) => group_ranges(v),
                    None => (0..row.len()).map(|i| (i, 1)).collect(),
                };
                let mut col_x = 0.0;
                for (start, len) in groups {
                    let is_full = row[start];
                    let cw = if let Some(Some(cw_val)) =
                        c_widths.get(r_idx).and_then(|r| r.get(start))
                    {
                        cw_val.clamp(280.0, vw)
                    } else {
                        let prog = cur_prog
                            .get(r_idx)
                            .and_then(|r| r.get(start))
                            .copied()
                            .unwrap_or(if is_full { 1.0 } else { 0.0 });
                        col_width_animated(prog, vw)
                    };
                    // Per-tab heights inside a stacked column (drag-resizable)
                    let equal_h = stack_height(vh, len, g);
                    let mut hs: Vec<f64> = (0..len)
                        .map(|j| {
                            c_heights
                                .get(r_idx)
                                .and_then(|r| r.get(start + j))
                                .copied()
                                .flatten()
                                .unwrap_or(equal_h)
                        })
                        .collect();
                    if len > 1 {
                        let target = (vh - (len as f64 - 1.0) * g).max(80.0 * len as f64);
                        let sum: f64 = hs.iter().sum();
                        if sum > 1.0 && (sum - target).abs() > 0.5 {
                            let scale = target / sum;
                            for h in &mut hs {
                                *h = (*h * scale).max(80.0);
                            }
                        }
                    }
                    let mut y_off = 0.0f64;
                    for j in 0..len {
                        let Some(c) = child else { break };
                        let ch_f = if len == 1 { vh } else { hs[j] };
                        let tx = ((col_x - off_x) * z) as f32;
                        let ty = ((row_y + y_off - off_y) * z) as f32;
                        let t = gsk::Transform::new()
                            .translate(&graphene::Point::new(tx, ty))
                            .scale(z as f32, z as f32);
                        let ch = if len == 1 {
                            height
                        } else {
                            ch_f.round() as i32
                        };
                        self.apply_visibility(
                            &c,
                            tx as f64,
                            ty as f64,
                            cw.round() * z,
                            ch as f64 * z,
                            vw,
                            vh,
                            focus.as_ref(),
                        );
                        c.allocate(cw.round() as i32, ch, -1, Some(t));
                        y_off += ch_f + g;
                        child = c.next_sibling();
                    }
                    col_x += cw + g;
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct StripLayout(ObjectSubclass<imp::StripLayout>) @extends gtk::LayoutManager;
}

impl StripLayout {
    pub fn new() -> Self {
        let l: Self = glib::Object::new();
        l.imp().zoom.set(1.0);
        l
    }
    pub fn offset_x(&self) -> f64 {
        self.imp().offset_x.get()
    }
    pub fn set_offset_x(&self, v: f64) {
        self.imp().offset_x.set(v);
    }
    pub fn offset_y(&self) -> f64 {
        self.imp().offset_y.get()
    }
    pub fn set_offset_y(&self, v: f64) {
        self.imp().offset_y.set(v);
    }
    pub fn zoom(&self) -> f64 {
        self.imp().zoom.get()
    }
    pub fn set_zoom(&self, v: f64) {
        self.imp().zoom.set(v);
    }
    /// Element to'liq ekran rejimi: `Some(tab_container)` — faqat shu tab ko'rinadi, `None` — oddiy holat.
    pub fn set_solo(&self, w: Option<gtk::Widget>) {
        *self.imp().solo.borrow_mut() = w;
        self.layout_changed();
    }
    pub fn solo(&self) -> Option<gtk::Widget> {
        self.imp().solo.borrow().clone()
    }
    pub fn set_col_ids(&self, ids: Vec<Vec<u64>>) {
        *self.imp().col_ids.borrow_mut() = ids;
    }
    pub fn set_grid(&self, grid: Vec<Vec<bool>>) {
        let imp = self.imp();
        let mut target = Vec::with_capacity(grid.len());
        for row in grid.iter() {
            let mut target_row = Vec::with_capacity(row.len());
            for &is_full in row.iter() {
                target_row.push(if is_full { 1.0 } else { 0.0 });
            }
            target.push(target_row);
        }
        *imp.target_grid.borrow_mut() = target.clone();

        let mut current = imp.current_grid.borrow_mut();
        if current.len() != target.len()
            || current
                .iter()
                .zip(target.iter())
                .any(|(c, t)| c.len() != t.len())
        {
            *current = target;
        }
        *imp.grid.borrow_mut() = grid;
    }
    pub fn set_custom_widths(&self, widths: Vec<Vec<Option<f64>>>) {
        *self.imp().custom_widths.borrow_mut() = widths;
    }
    pub fn set_custom_heights(&self, heights: Vec<Vec<Option<f64>>>) {
        *self.imp().custom_heights.borrow_mut() = heights;
    }
    pub fn set_tab_height(&self, r_idx: usize, c_idx: usize, height: Option<f64>) {
        let imp = self.imp();
        let mut heights = imp.custom_heights.borrow_mut();
        while heights.len() <= r_idx {
            heights.push(Vec::new());
        }
        while heights[r_idx].len() <= c_idx {
            heights[r_idx].push(None);
        }
        heights[r_idx][c_idx] = height;
    }
    pub fn set_column_width(&self, r_idx: usize, c_idx: usize, width: Option<f64>) {
        let imp = self.imp();
        let mut widths = imp.custom_widths.borrow_mut();
        while widths.len() <= r_idx {
            widths.push(Vec::new());
        }
        while widths[r_idx].len() <= c_idx {
            widths[r_idx].push(None);
        }
        widths[r_idx][c_idx] = width;
    }
    pub fn grid_needs_animation(&self) -> bool {
        let imp = self.imp();
        let target = imp.target_grid.borrow();
        let current = imp.current_grid.borrow();
        for (r_idx, target_row) in target.iter().enumerate() {
            if let Some(cur_row) = current.get(r_idx) {
                for (c_idx, &t_val) in target_row.iter().enumerate() {
                    if let Some(&c_val) = cur_row.get(c_idx) {
                        if (c_val - t_val).abs() > 0.002 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    pub fn step_grid_progress(&self, k: f64) -> bool {
        let imp = self.imp();
        let target = imp.target_grid.borrow();
        let mut current = imp.current_grid.borrow_mut();
        let mut still_animating = false;

        for (r_idx, target_row) in target.iter().enumerate() {
            if let Some(cur_row) = current.get_mut(r_idx) {
                for (c_idx, &t_val) in target_row.iter().enumerate() {
                    if let Some(c_val) = cur_row.get_mut(c_idx) {
                        if (*c_val - t_val).abs() > 0.002 {
                            *c_val += (t_val - *c_val) * k;
                            if (*c_val - t_val).abs() <= 0.002 {
                                *c_val = t_val;
                            } else {
                                still_animating = true;
                            }
                        }
                    }
                }
            }
        }
        still_animating
    }
    pub fn connect_visibility(&self, f: impl Fn(&gtk::Widget, bool) + 'static) {
        *self.imp().on_visibility.borrow_mut() = Some(std::rc::Rc::new(f));
    }
    pub fn connect_resized(&self, f: impl Fn() + 'static) {
        *self.imp().on_resize.borrow_mut() = Some(std::rc::Rc::new(f));
    }
}

// ---------------------------------------------------------------------------
// Shell layout: [sidebar | strip]. The sidebar is always allocated at its full
// width and slid in/out with a transform, so it can only ever be fully open,
// fully closed, or mid-animation -- never a stuck half width.
// ---------------------------------------------------------------------------

pub const SIDE_W: i32 = 260;

mod shell_imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    pub struct ShellLayout {
        /// Currently visible sidebar width, 0..=SIDE_W.
        pub side: Cell<f64>,
        /// Whether sidebar acts as a floating overlay (Zen browser style) without resizing tab content.
        pub is_overlay: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ShellLayout {
        const NAME: &'static str = "RailShellLayout";
        type Type = super::ShellLayout;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for ShellLayout {}

    impl LayoutManagerImpl for ShellLayout {
        fn measure(
            &self,
            _w: &gtk::Widget,
            _o: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            (0, 0, -1, -1)
        }

        fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
            let sw = self.side.get().round() as i32;
            let Some(main) = widget.first_child() else {
                return;
            };
            let Some(side) = main.next_sibling() else {
                return;
            };

            if self.is_overlay.get() {
                // Floating overlay mode: tab content stays at full window width and never jitters.
                main.allocate(width, height, -1, None);
                if side.is_visible() {
                    let t = gsk::Transform::new()
                        .translate(&graphene::Point::new((sw - SIDE_W) as f32, 0.0));
                    side.allocate(SIDE_W, height, -1, Some(t));
                }
            } else {
                // Fixed pinned mode: sidebar docks alongside main content.
                if side.is_visible() {
                    let t = gsk::Transform::new()
                        .translate(&graphene::Point::new((sw - SIDE_W) as f32, 0.0));
                    side.allocate(SIDE_W, height, -1, Some(t));
                }
                let t = gsk::Transform::new().translate(&graphene::Point::new(sw as f32, 0.0));
                main.allocate((width - sw).max(0), height, -1, Some(t));
            }
        }
    }
}

glib::wrapper! {
    pub struct ShellLayout(ObjectSubclass<shell_imp::ShellLayout>) @extends gtk::LayoutManager;
}

impl ShellLayout {
    pub fn new() -> Self {
        glib::Object::new()
    }
    pub fn side(&self) -> f64 {
        self.imp().side.get()
    }
    pub fn set_side(&self, v: f64) {
        self.imp().side.set(v);
    }
    pub fn is_overlay(&self) -> bool {
        self.imp().is_overlay.get()
    }
    pub fn set_overlay(&self, v: bool) {
        self.imp().is_overlay.set(v);
    }
}

#[cfg(test)]
mod tests {
    use super::{group_ranges, stack_height};

    #[test]
    fn consecutive_equal_ids_make_one_column() {
        assert_eq!(
            group_ranges(&[1, 1, 2, 3, 3, 3]),
            vec![(0, 2), (2, 1), (3, 3)]
        );
        assert_eq!(group_ranges(&[7]), vec![(0, 1)]);
        assert!(group_ranges(&[]).is_empty());
        // equal ids that are not adjacent stay separate columns
        assert_eq!(group_ranges(&[1, 2, 1]), vec![(0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn stacked_windows_share_the_column_height() {
        assert_eq!(stack_height(900.0, 1, 6.0), 900.0);
        assert_eq!(stack_height(900.0, 2, 6.0), 447.0);
        let h3 = stack_height(900.0, 3, 6.0);
        assert!((3.0 * h3 + 2.0 * 6.0 - 900.0).abs() < 1e-9);
    }
}
