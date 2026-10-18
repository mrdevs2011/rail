//! Zoomed-out 2D overview.

use crate::*;

impl App {
    // ---- overview (niri-style 2D grid: zoomed out) -----------------------

    pub(crate) fn toggle_overview(self: &Rc<Self>) {
        if self.ov.get() {
            self.exit_overview();
        } else {
            self.enter_overview();
        }
    }

    pub(crate) fn enter_overview(self: &Rc<Self>) {
        if self.ov.get() || self.rows.borrow().is_empty() {
            return;
        }
        self.palette.set_visible(false);
        // Settings rides on its tab overlay (like downloads HTML) — keep it visible
        // so the overview card shows real content, not a blank shell.
        self.ov.set(true);
        self.ov_return.set((self.focus_row.get(), self.focus_col.get()));
        self.ov_saved_x.set(self.target_x.get());
        self.ov_saved_y.set(self.target_y.get());
        self.strip.add_css_class("zoomed");
        self.shell.add_css_class("zoomed");
        self.ov_hint.set_visible(false);
        for row in self.rows.borrow().iter() {
            for t in row.tabs.iter() {
                t.view.set_can_target(false);
            }
        }
        self.settings_modal.set_can_target(false);
        // Overview is view-only: take keyboard focus away from whichever page/form had it.
        gtk::prelude::GtkWindowExt::set_focus(&self.win, None::<&gtk::Widget>);

        // Pure zoom-out from the current viewport center (no pan / rise).
        // Same world point stays under the screen center for the whole animation.
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        if vw <= 1.0 || vh <= 1.0 {
            self.target_zoom.set(OV_ZOOM);
            self.animate();
            return;
        }
        // Snap camera so the anchor is not mid-ease
        self.layout.set_offset_x(self.target_x.get());
        self.layout.set_offset_y(self.target_y.get());
        self.layout.set_zoom(1.0);
        self.strip.queue_allocate();

        let ox0 = self.target_x.get();
        let oy0 = self.target_y.get();
        // World point currently under the viewport center
        let anchor_x = ox0 + vw * 0.5;
        let anchor_y = oy0 + vh * 0.5;
        // Keep that point centered while zooming to OV_ZOOM
        let tx = anchor_x - (vw * 0.5) / OV_ZOOM;
        let ty = anchor_y - (vh * 0.5) / OV_ZOOM;

        // The zoom-out itself is anchored on the old camera center (see animate()); once the
        // zoom settles the camera glides to the framed position: whole grid centered when it
        // fits, otherwise the active tab centered (clamped at the grid edges).
        let _ = (tx, ty);
        let (fx, fy) = self.overview_offsets_at_zoom(OV_ZOOM);
        self.target_x.set(fx);
        self.target_y.set(fy);
        self.target_zoom.set(OV_ZOOM);
        self.animate();
    }

    pub(crate) fn exit_overview(self: &Rc<Self>) {
        if !self.ov.get() {
            return;
        }
        self.ov.set(false);
        self.strip.remove_css_class("zoomed");
        self.shell.remove_css_class("zoomed");
        self.ov_hint.set_visible(false);
        for row in self.rows.borrow().iter() {
            for t in row.tabs.iter() {
                t.view.set_can_target(true);
            }
        }
        self.target_zoom.set(1.0);
        // Full focus pipeline once (wakes a suspended tab, sidebar, title, session save, camera)
        let (fr, fc) = (self.focus_row.get(), self.focus_col.get());
        self.focus_coords(fr, fc);
        self.sync_settings_chrome();
        if let Some(v) = self.focused_view() {
            if !self.settings_modal_open() {
                v.grab_focus();
            }
        }
    }

    pub(crate) fn cancel_overview(self: &Rc<Self>) {
        let (r, c) = self.ov_return.get();
        self.focus_coords(r, c);
        self.exit_overview();
    }

    pub(crate) fn ov_click(self: &Rc<Self>, click_x: f64, click_y: f64) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;

        let z = self.layout.zoom();
        let off_x = self.layout.offset_x();
        let off_y = self.layout.offset_y();
        let g = gap(z);

        let unscaled_x = click_x / z + off_x;
        let unscaled_y = click_y / z + off_y;

        let rows = self.rows.borrow();
        let mut hit = None;
        for (r_idx, row) in rows.iter().enumerate() {
            let row_y = r_idx as f64 * (vh + g);
            if unscaled_y >= row_y && unscaled_y <= row_y + vh {
                for (start, len, x, w) in row_cols(row, vw, g) {
                    if unscaled_x >= x && unscaled_x <= x + w {
                        let h = stack_height(vh, len, g);
                        let j = (((unscaled_y - row_y) / (h + g)).floor().max(0.0) as usize).min(len - 1);
                        hit = Some((r_idx, start + j));
                        break;
                    }
                }
                if hit.is_some() {
                    break;
                }
            }
        }
        drop(rows);

        if let Some((r, c)) = hit {
            self.focus_coords(r, c);
        }
        self.exit_overview();
    }

    pub(crate) fn ov_key(self: &Rc<Self>, k: gdk::Key, m: gdk::ModifierType) -> bool {
        // Only bare keys act inside the overview. Modified keys fall through to the regular
        // shortcuts (e.g. Alt+H), so nothing is bound twice here.
        if !m.is_empty() {
            return false;
        }
        match k {
            gdk::Key::h | gdk::Key::Left | gdk::Key::KP_Left => self.focus_column_left(),
            gdk::Key::l | gdk::Key::Right | gdk::Key::KP_Right => self.focus_column_right(),
            gdk::Key::j | gdk::Key::Down | gdk::Key::KP_Down => self.focus_row_down(),
            gdk::Key::k | gdk::Key::Up | gdk::Key::KP_Up => self.focus_row_up(),
            gdk::Key::Return | gdk::Key::KP_Enter => self.exit_overview(),
            gdk::Key::x => {
                if let Some(v) = self.focused_view() {
                    self.close_view(&v);
                }
            }
            _ => {}
        }
        // Bare keys never leak through to the (hidden) page behind the overview.
        true
    }
}
