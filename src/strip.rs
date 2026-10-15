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
