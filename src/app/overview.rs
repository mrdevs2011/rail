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
        // Cancel an in-flight exit so we don't fight the zoom-in.
        self.ov_exiting.set(false);
        self.palette.set_visible(false);
        // Settings rides on its tab overlay (like downloads HTML) — keep it visible
        // so the overview card shows real content, not a blank shell.
        self.ov.set(true);
        self.ov_return
            .set((self.focus_row.get(), self.focus_col.get()));
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
            let z = self.ov_user_zoom.get().clamp(OV_ZOOM_MIN, OV_ZOOM_MAX);
            self.target_zoom.set(z);
            self.ensure_overview_previews();
            self.animate();
            return;
        }
        // Align normal camera targets to the focused tab and snap so anchor is precise
        self.set_normal_reveal_targets();
        self.layout.set_offset_x(self.target_x.get());
        self.layout.set_offset_y(self.target_y.get());
        self.layout.set_zoom(1.0);
        self.strip.queue_allocate();

        // The zoom-out itself is anchored on the old camera center (see animate()); once the
        // zoom settles the camera glides to the framed position: whole grid centered when it
        // fits, otherwise the active tab centered (clamped at the grid edges).
        let z = self.ov_user_zoom.get().clamp(OV_ZOOM_MIN, OV_ZOOM_MAX);
        let (fx, fy) = self.overview_offsets_at_zoom(z);
        self.target_x.set(fx);
        self.target_y.set(fy);
        self.target_zoom.set(z);
        self.ensure_overview_previews();
        self.animate();
    }

    /// Start zoom-in. CSS class and `ov` stay true until the camera lands (see `finish_exit_overview`).
    pub(crate) fn exit_overview(self: &Rc<Self>) {
        if !self.ov.get() {
            return;
        }
        if self.ov_exiting.get() {
            // Already zooming in — just retarget the focused tab.
            self.set_normal_reveal_targets();
            self.target_zoom.set(1.0);
            self.animate();
            return;
        }
        self.ov_exiting.set(true);
        // Keep "zoomed" CSS and ov=true during the animation so borders/layout don't snap.
        self.target_zoom.set(1.0);
        self.set_normal_reveal_targets();
        self.animate();
    }

    /// Called when zoom-in reaches 1:1 — restore interaction and run the full focus pipeline once.
    pub(crate) fn finish_exit_overview(self: &Rc<Self>) {
        if !self.ov_exiting.get() && !self.ov.get() {
            return;
        }
        self.ov_exiting.set(false);
        self.ov.set(false);
        self.strip.remove_css_class("zoomed");
        self.shell.remove_css_class("zoomed");
        self.ov_hint.set_visible(false);
        for row in self.rows.borrow().iter() {
            for t in row.tabs.iter() {
                t.view.set_can_target(true);
            }
        }
        self.settings_modal.set_can_target(true);
        // Ensure camera offsets are cleanly aligned to targets so focus pipeline doesn't start a micro-spring
        self.layout.set_offset_x(self.target_x.get());
        self.layout.set_offset_y(self.target_y.get());
        self.layout.set_zoom(1.0);
        // Full focus pipeline once the camera is still (wake tab, sidebar, title, session).
        let (fr, fc) = (self.focus_row.get(), self.focus_col.get());
        self.focus_coords(fr, fc);
        self.sync_settings_chrome();
        if let Some(v) = self.focused_view() {
            if !self.settings_modal_open() {
                v.grab_focus();
            }
        }
    }

    /// Camera targets for the focused tab at zoom 1.0 (normal strip mode), without side effects.
    pub(crate) fn set_normal_reveal_targets(&self) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let z = 1.0;
        let visible_w = vw / z;
        let g = gap(z);
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        if rows.is_empty() || cur_r >= rows.len() {
            return;
        }
        let cols = row_cols(&rows[cur_r], vw, g);
        let active_span = cols
            .iter()
            .find(|&&(st, len, _, _)| cur_c >= st && cur_c < st + len)
            .map(|&(_, _, x, w)| (x, x + w));
        let active_row_total_w = cols.last().map_or(0.0, |&(_, _, x, w)| x + w);
        drop(rows);

        let ty = cur_r as f64 * (vh + g);
        let tx = if let Some((l, r)) = active_span {
            let max_x = (active_row_total_w - visible_w).max(0.0);
            let mut t = self.target_x.get().clamp(0.0, max_x);
            if l < t {
                t = l;
            } else if r > t + visible_w {
                t = r - visible_w;
            }
            t.clamp(0.0, max_x)
        } else {
            0.0
        };
        self.target_x.set(tx);
        self.target_y.set(ty);
    }

    /// Overview: keep live cards visible; if a tab was already suspended, show its freeze frame.
    pub(crate) fn ensure_overview_previews(self: &Rc<Self>) {
        let Ok(rows) = self.rows.try_borrow() else {
            return;
        };
        for row in rows.iter() {
            for tab in row.tabs.iter() {
                if tab.suspended.get() {
                    // Show freeze only when a real picture exists.
                    tab.freeze.set_visible(tab.freeze.paintable().is_some());
                    continue;
                }
                // Live tab: always leave the live view visible, hide any leftover freeze
                tab.freeze.set_visible(false);
            }
        }
    }

    /// Ctrl+/- or pinch: change overview scale (clamped), keep framing on the focused tab.
    pub(crate) fn overview_zoom_by(self: &Rc<Self>, factor: f64) {
        if !self.ov.get() || self.ov_exiting.get() {
            return;
        }
        let z0 = self.target_zoom.get();
        let z1 = (z0 * factor).clamp(OV_ZOOM_MIN, OV_ZOOM_MAX);
        if (z1 - z0).abs() < 0.001 {
            return;
        }
        self.ov_user_zoom.set(z1);
        self.target_zoom.set(z1);
        let (fx, fy) = self.overview_offsets_at_zoom(z1);
        self.target_x.set(fx);
        self.target_y.set(fy);
        self.animate();
    }

    pub(crate) fn cancel_overview(self: &Rc<Self>) {
        let (r, c) = self.ov_return.get();
        // Move focus first (while still in overview — lightweight path)
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
                        let j = (((unscaled_y - row_y) / (h + g)).floor().max(0.0) as usize)
                            .min(len - 1);
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
