//! The niri-style 2D tiled layout: a custom GTK layout manager that places
//! columns horizontally and rows (workspaces) vertically, sliding by (offset_x, offset_y).

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{graphene, gsk};
use gtk4 as gtk;

pub const GAP: f64 = 6.0;

/// Space between columns and rows, in strip coordinates. It grows as the strip zooms
/// out so scaled-down tabs keep a readable gap instead of nearly touching.
pub fn gap(zoom: f64) -> f64 {
    GAP + (1.0 - zoom) * 40.0
}

/// Minimum width for a column to be usable.
pub const MIN_COL_W: f64 = 480.0;

/// Column width calculation
pub fn col_width(full: bool, viewport: f64) -> f64 {
    if full || viewport < (MIN_COL_W * 2.0 + GAP) {
        viewport.max(280.0)
    } else {
        ((viewport - GAP) / 2.0).floor()
    }
}

/// Smooth animated column width calculation (progress 0.0: normal, 1.0: full)
pub fn col_width_animated(progress: f64, viewport: f64) -> f64 {
    let p = progress.clamp(0.0, 1.0);
    if viewport < (MIN_COL_W * 2.0 + GAP) {
        viewport.max(280.0)
    } else {
        let normal_w = ((viewport - GAP) / 2.0).floor();
        let full_w = viewport.max(280.0);
        normal_w + (full_w - normal_w) * p
    }
}


/// Auto-fit grid used by the "tab grid" view: every tab becomes an equal tile with the
/// same aspect ratio as the viewport, packed into the rows/columns that give the biggest tiles.
pub struct TileGrid {
    pub cols: usize,
    pub scale: f64,
    /// (x, y, w, h) of every tile in strip coordinates, in tab order.
    pub rects: Vec<(f64, f64, f64, f64)>,
}

pub fn tile_grid(n: usize, vw: f64, vh: f64) -> TileGrid {
    const PAD: f64 = 18.0;
    const MAX_SCALE: f64 = 0.7;
    if n == 0 || vw <= 0.0 || vh <= 0.0 {
        return TileGrid { cols: 1, scale: 1.0, rects: Vec::new() };
    }
    let mut cols = 1usize;
    let mut best = f64::MIN;
    for c in 1..=n {
        let r = (n + c - 1) / c;
        let sw = (vw - PAD * (c as f64 + 1.0)) / (c as f64 * vw);
        let sh = (vh - PAD * (r as f64 + 1.0)) / (r as f64 * vh);
        let sc = sw.min(sh);
        if sc > best + 1e-9 {
            best = sc;
            cols = c;
        }
    }
    let rows = (n + cols - 1) / cols;
    let scale = best.clamp(0.05, MAX_SCALE);
    let (tw, th) = (vw * scale, vh * scale);
    let total_h = rows as f64 * th + (rows as f64 - 1.0) * PAD;
    let y0 = ((vh - total_h) / 2.0).max(0.0);
    let mut rects = Vec::with_capacity(n);
    for i in 0..n {
        let (r, c) = (i / cols, i % cols);
        let in_row = if r + 1 == rows { n - r * cols } else { cols };
        let row_w = in_row as f64 * tw + (in_row as f64 - 1.0) * PAD;
        let x0 = ((vw - row_w) / 2.0).max(0.0);
        rects.push((x0 + c as f64 * (tw + PAD), y0 + r as f64 * (th + PAD), tw, th));
    }
    TileGrid { cols, scale, rects }
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
        /// For each row: list of is_full flags for each column
        pub grid: RefCell<Vec<Vec<bool>>>,
        /// Target progress (0.0 for normal, 1.0 for full)
        pub target_grid: RefCell<Vec<Vec<f64>>>,
        /// Current animated progress (0.0 .. 1.0)
        pub current_grid: RefCell<Vec<Vec<f64>>>,
        /// Custom widths per column in each row
        pub custom_widths: RefCell<Vec<Vec<Option<f64>>>>,
        /// Tab grid view: ignore rows/columns/zoom and tile every tab evenly.
        pub tile_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StripLayout {
        const NAME: &'static str = "RailStripLayout";
        type Type = super::StripLayout;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for StripLayout {}

    impl LayoutManagerImpl for StripLayout {
        fn measure(&self, _w: &gtk::Widget, _o: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (0, 0, -1, -1)
        }

        fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
            let resized = self.width.replace(width) != width || self.height.replace(height) != height;
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
            let mut child = widget.first_child();

            if self.tile_mode.get() {
                let mut kids: Vec<gtk::Widget> = Vec::new();
                while let Some(c) = child {
                    child = c.next_sibling();
                    kids.push(c);
                }
                let tg = tile_grid(kids.len(), vw, vh);
                for (c, &(x, y, _w, _h)) in kids.iter().zip(tg.rects.iter()) {
                    let t = gsk::Transform::new()
                        .translate(&graphene::Point::new(x as f32, y as f32))
                        .scale(tg.scale as f32, tg.scale as f32);
                    c.allocate(width, height, -1, Some(t));
                }
                return;
            }

            if grid.is_empty() {
                let mut x = 0.0;
                while let Some(c) = child {
                    let cw = col_width(c.has_css_class("full"), vw);
                    let t = gsk::Transform::new()
                        .translate(&graphene::Point::new(((x - off_x) * z) as f32, ((0.0 - off_y) * z) as f32))
                        .scale(z as f32, z as f32);
                    c.allocate(cw as i32, height, -1, Some(t));
                    x += cw + g;
                    child = c.next_sibling();
                }
                return;
            }

            for (r_idx, row) in grid.iter().enumerate() {
                let row_y = r_idx as f64 * (vh + g);
                let mut col_x = 0.0;
                for (c_idx, &is_full) in row.iter().enumerate() {
                    if let Some(c) = child {
                        let cw = if let Some(Some(cw_val)) = c_widths.get(r_idx).and_then(|r| r.get(c_idx)) {
                            cw_val.clamp(280.0, vw)
                        } else {
                            let prog = cur_prog
                                .get(r_idx)
                                .and_then(|r| r.get(c_idx))
                                .copied()
                                .unwrap_or(if is_full { 1.0 } else { 0.0 });
                            col_width_animated(prog, vw)
                        };
                        let tx = ((col_x - off_x) * z) as f32;
                        let ty = ((row_y - off_y) * z) as f32;
                        let t = gsk::Transform::new()
                            .translate(&graphene::Point::new(tx, ty))
                            .scale(z as f32, z as f32);
                        c.allocate(cw.round() as i32, height, -1, Some(t));
                        col_x += cw + g;
                        child = c.next_sibling();
                    }
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
    pub fn tile_mode(&self) -> bool {
        self.imp().tile_mode.get()
    }
    pub fn set_tile_mode(&self, v: bool) {
        self.imp().tile_mode.set(v);
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
        if current.len() != target.len() || current.iter().zip(target.iter()).any(|(c, t)| c.len() != t.len()) {
            *current = target;
        }
        *imp.grid.borrow_mut() = grid;
    }
    pub fn set_custom_widths(&self, widths: Vec<Vec<Option<f64>>>) {
        *self.imp().custom_widths.borrow_mut() = widths;
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ShellLayout {
        const NAME: &'static str = "RailShellLayout";
        type Type = super::ShellLayout;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for ShellLayout {}

    impl LayoutManagerImpl for ShellLayout {
        fn measure(&self, _w: &gtk::Widget, _o: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (0, 0, -1, -1)
        }

        fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
            let sw = self.side.get().round() as i32;
            let Some(side) = widget.first_child() else { return };
            if side.is_visible() {
                let t = gsk::Transform::new().translate(&graphene::Point::new((sw - SIDE_W) as f32, 0.0));
                side.allocate(SIDE_W, height, -1, Some(t));
            }
            if let Some(main) = side.next_sibling() {
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
}

#[cfg(test)]
mod tests {
    use super::tile_grid;

    #[test]
    fn six_tabs_make_three_by_two() {
        let tg = tile_grid(6, 1600.0, 900.0);
        assert_eq!(tg.cols, 3);
        assert_eq!(tg.rects.len(), 6);
        // same row -> same y, next row below
        assert_eq!(tg.rects[0].1, tg.rects[2].1);
        assert!(tg.rects[3].1 > tg.rects[0].1);
    }

    #[test]
    fn tiles_stay_inside_and_never_overlap() {
        for n in 1..=24 {
            let tg = tile_grid(n, 1280.0, 720.0);
            assert_eq!(tg.rects.len(), n);
            for (i, a) in tg.rects.iter().enumerate() {
                assert!(a.0 >= -0.01 && a.1 >= -0.01 && a.0 + a.2 <= 1280.01 && a.1 + a.3 <= 720.01, "n={n} tile {i} out of bounds");
                for b in tg.rects.iter().skip(i + 1) {
                    let apart = a.0 + a.2 <= b.0 + 0.01 || b.0 + b.2 <= a.0 + 0.01 || a.1 + a.3 <= b.1 + 0.01 || b.1 + b.3 <= a.1 + 0.01;
                    assert!(apart, "n={n} overlap");
                }
            }
        }
    }

    #[test]
    fn empty_is_safe() {
        assert!(tile_grid(0, 800.0, 600.0).rects.is_empty());
        assert!(tile_grid(3, 0.0, 0.0).rects.is_empty());
    }
}
