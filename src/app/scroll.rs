//! Strip scrolling, camera animation and slides.

use crate::*;

impl App {
    // ---- strip scrolling -------------------------------------------------

    /// Camera offsets that keep the overview grid framed with equal margins
    /// (or focused on the active tab when the grid is larger than the view).
    /// Computed for an arbitrary zoom so animation frames stay centered — not skewed.
    pub(crate) fn overview_offsets_at_zoom(&self, z: f64) -> (f64, f64) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        if vw <= 0.0 || vh <= 0.0 || z <= 0.01 {
            return (self.target_x.get(), self.target_y.get());
        }
        let visible_w = vw / z;
        let visible_h = vh / z;
        let g = gap(z);
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();

        let rows = self.rows.borrow();
        if rows.is_empty() {
            return (0.0, 0.0);
        }

        let mut active_span: Option<(f64, f64)> = None;
        if let Some(row) = rows.get(cur_r) {
            let cols = row_cols(row, vw, g);
            active_span = cols
                .iter()
                .find(|&&(st, len, _, _)| cur_c >= st && cur_c < st + len)
                .map(|&(_, _, x, w)| (x, x + w));
        }
        drop(rows);

        // Niri-style: focused tab is always framed in the exact center of the screen
        let row_center_y = cur_r as f64 * (vh + g) + vh / 2.0;
        let col_center_x = active_span.map_or(vw / 2.0, |(l, r)| (l + r) / 2.0);
        let tx = col_center_x - visible_w / 2.0;
        let ty = row_center_y - visible_h / 2.0;
        (tx, ty)
    }

    /// Normal mode: scroll 2D minimum needed to show focused row and column.
    /// Overview: center the 2D grid in the zoomed-out plane (equal margins).
    pub(crate) fn reveal_focus(self: &Rc<Self>) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let z = self.target_zoom.get();
        let visible_w = vw / z;
        let g = gap(z);
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();

        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }

        let mut active_span: Option<(f64, f64)> = None;
        let mut active_row_total_w = 0.0;

        for (ri, row) in rows.iter().enumerate() {
            let cols = row_cols(row, vw, g);
            if ri == cur_r {
                active_span = cols
                    .iter()
                    .find(|&&(st, len, _, _)| cur_c >= st && cur_c < st + len)
                    .map(|&(_, _, x, w)| (x, x + w));
                active_row_total_w = cols.last().map_or(0.0, |&(_, _, x, w)| x + w);
            }
        }
        drop(rows);

        let (target_x, target_y) = if self.ov.get() {
            self.overview_offsets_at_zoom(z)
        } else {
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
            (tx, ty)
        };

        self.target_x.set(target_x);
        self.target_y.set(target_y);
        self.animate();
    }

    /// Ease scroll offsets + zoom toward targets (flat zoom, no mid-frame re-centering).
    pub(crate) fn animate(self: &Rc<Self>) {
        if self.animating.get() {
            return;
        }
        let near = |a: f64, b: f64, eps: f64| (a - b).abs() < eps;
        let cam_ready = near(self.target_x.get(), self.layout.offset_x(), 0.5)
            && near(self.target_y.get(), self.layout.offset_y(), 0.5)
            && near(self.target_zoom.get(), self.layout.zoom(), 0.002);

        if cam_ready && !self.layout.grid_needs_animation() {
            self.unfreeze_after_anim();
            self.layout.set_offset_x(self.target_x.get());
            self.layout.set_offset_y(self.target_y.get());
            self.layout.set_zoom(self.target_zoom.get());
            self.strip.queue_allocate();
            if self.ov_exiting.get() {
                self.finish_exit_overview();
            }
            return;
        }
        self.animating.set(true);
        self.freeze_for_anim();
        let app = self.clone();
        let last = Cell::new(None::<i64>);
        // Timed overview zoom: (start time us, start zoom, target zoom, start_wcx, start_wcy, target_wcx, target_wcy).
        // Anchoring on world camera centers eliminates the nonlinear horizontal drift/swing
        // that occurs when offset_x and zoom are interpolated independently.
        let zanim = Cell::new(None::<(i64, f64, f64, f64, f64, f64, f64)>);
        self.strip.add_tick_callback(move |strip, clock| {
            let now = clock.frame_time();
            let dt = last.replace(Some(now)).map_or(1.0 / 60.0, |l| (now - l) as f64 / 1e6);
            let k = (dt / 0.045).min(1.0);

            let oxc = app.layout.offset_x();
            let oyc = app.layout.offset_y();
            let zc = app.layout.zoom();
            let oxt = app.target_x.get();
            let oyt = app.target_y.get();
            let zt = app.target_zoom.get();

            let vw = strip.width() as f64;
            let vh = strip.height() as f64;
            let zoom_busy = !near(zc, zt, 0.002);
            // Any overview zoom transition (in or out), including delayed exit while ov still true.
            let ov_motion = vw > 1.0
                && vh > 1.0
                && zoom_busy
                && (app.ov.get() || app.ov_exiting.get() || zc < 0.99 || zt < 0.99);

            let nz: f64;
            let mut nx: f64;
            let mut ny: f64;
            if ov_motion {
                let zc_safe = zc.max(0.01);
                let zt_safe = zt.max(0.01);
                let target_wcx = oxt + vw / (2.0 * zt_safe);
                let target_wcy = oyt + vh / (2.0 * zt_safe);

                let (t0, z0, wcx0, wcy0, wcxt, wcyt) = match zanim.get() {
                    Some(a)
                        if (a.2 - zt).abs() < 1e-9
                            && (a.5 - target_wcx).abs() < 1.0
                            && (a.6 - target_wcy).abs() < 1.0 =>
                    {
                        (a.0, a.1, a.3, a.4, a.5, a.6)
                    }
                    _ => {
                        let wcx_start = oxc + vw / (2.0 * zc_safe);
                        let wcy_start = oyc + vh / (2.0 * zc_safe);
                        let a = (now, zc, zt, wcx_start, wcy_start, target_wcx, target_wcy);
                        zanim.set(Some(a));
                        (a.0, a.1, a.3, a.4, a.5, a.6)
                    }
                };
                let u = (((now - t0) as f64 / 1e6) / OV_ANIM_SECS).clamp(0.0, 1.0);
                // Ease-out quad: responsive, fast, and flat/even (tekkiz) without sluggish tail
                let e = 1.0 - (1.0 - u) * (1.0 - u);
                if u >= 1.0 {
                    nz = zt;
                    nx = oxt;
                    ny = oyt;
                } else {
                    // Logarithmic zoom for perceptually uniform field-of-view scaling
                    nz = (z0.max(0.01).ln() + (zt.max(0.01).ln() - z0.max(0.01).ln()) * e).exp();
                    // Coupled world center interpolation: guarantees zero horizontal/vertical wobble or drift!
                    let cur_wcx = wcx0 + (wcxt - wcx0) * e;
                    let cur_wcy = wcy0 + (wcyt - wcy0) * e;
                    let nz_safe = nz.max(0.01);
                    nx = cur_wcx - vw / (2.0 * nz_safe);
                    ny = cur_wcy - vh / (2.0 * nz_safe);
                }
                app.cam_spring_x.set(None);
                app.cam_spring_y.set(None);
                app.cam_v0_x.set(0.0);
            } else {
                zanim.set(None);
                let mut z = zc + (zt - zc) * k;
                if near(zt, z, 0.002) { z = zt; }
                nz = z;
                // Niri-style critically damped spring on the real frame time (no per-frame lerp)
                let (sx, _) = step_cam_axis(&app.cam_spring_x, oxc, oxt, &app.cam_v0_x, now);
                let (sy, _) = step_cam_axis(&app.cam_spring_y, oyc, oyt, &Cell::new(0.0), now);
                nx = sx;
                ny = sy;
            }
            if near(oxt, nx, 0.5) && nz == zt { nx = oxt; }
            if near(oyt, ny, 0.5) && nz == zt { ny = oyt; }

            let grid_busy = app.layout.step_grid_progress(k);

            app.layout.set_offset_x(nx);
            app.layout.set_offset_y(ny);
            app.layout.set_zoom(nz);
            strip.queue_allocate();

            if nx == oxt && ny == oyt && nz == zt && !grid_busy {
                app.cam_spring_x.set(None);
                app.cam_spring_y.set(None);
                app.animating.set(false);
                app.unfreeze_after_anim();
                // Zoom-in finished: drop overview chrome and run focus side-effects once.
                if app.ov_exiting.get() {
                    app.finish_exit_overview();
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }


    /// Live 1:1 horizontal pan (finger tracks content). Used by touchpad slide mode.
    /// Offsets update every event; strip allocate is coalesced to one per frame.
    pub(crate) fn pan_canvas_x(self: &Rc<Self>, dx: f64) {
        let vw = self.strip.width() as f64;
        if vw <= 0.0 || dx == 0.0 {
            return;
        }
        let g = gap(self.target_zoom.get());
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r >= rows.len() {
            return;
        }
        let row_w = row_cols(&rows[cur_r], vw, g).last().map_or(0.0, |&(_, _, x, w)| x + w);
        drop(rows);
        let max_x = (row_w - vw).max(0.0);
        let nx = (self.layout.offset_x() + dx).clamp(0.0, max_x);
        // A new swipe grabs the view: stop any running spring right where it is (like niri)
        self.cam_spring_x.set(None);
        self.cam_spring_y.set(None);
        self.cam_v0_x.set(0.0);
        self.layout.set_offset_x(nx);
        self.target_x.set(nx);
        // One allocate per display frame — per-event allocate freezes WebKit tabs
        if !self.pan_alloc_pending.get() {
            self.pan_alloc_pending.set(true);
            let app = self.clone();
            self.strip.add_tick_callback(move |strip, _clock| {
                app.pan_alloc_pending.set(false);
                strip.queue_allocate();
                glib::ControlFlow::Break
            });
        }
    }

    /// Finger lifted after a touchpad slide (niri `view_offset_gesture_end`): project where the
    /// swipe would coast to, snap to the nearest column edge there, focus the furthest column
    /// that is fully visible in the swipe direction, and let the spring continue from the
    /// release velocity so the motion never stops and restarts.
    pub(crate) fn finish_slide(self: &Rc<Self>, vel: f64) {
        let vw = self.strip.width() as f64;
        if vw <= 0.0 {
            return;
        }
        let g = gap(self.target_zoom.get());
        let cur_r = self.focus_row.get();
        let cur_off = self.layout.offset_x();
        let cols = {
            let rows = self.rows.borrow();
            if cur_r >= rows.len() || rows[cur_r].tabs.is_empty() {
                return;
            }
            row_cols(&rows[cur_r], vw, g)
        };
        if cols.is_empty() {
            return;
        }
        let row_w = cols.last().map_or(0.0, |&(_, _, x, w)| x + w);
        let max_x = (row_w - vw).max(0.0);

        // Where the swipe would stop after decelerating
        let proj = (cur_off + coast_distance(vel)).clamp(0.0, max_x);

        // Snap points: a column's left edge or right edge aligned with the view edge
        let mut snaps = vec![0.0, max_x];
        for &(_, _, x, w) in &cols {
            snaps.push(x.clamp(0.0, max_x));
            snaps.push((x + w - vw).clamp(0.0, max_x));
        }
        let snap = snaps
            .into_iter()
            .min_by(|a, b| (a - proj).abs().total_cmp(&(b - proj).abs()))
            .unwrap_or(cur_off);

        // Focus the furthest fully visible column in the direction of travel
        let visible: Vec<usize> = cols
            .iter()
            .enumerate()
            .filter(|&(_, &(_, _, x, w))| x >= snap - 0.5 && x + w <= snap + vw + 0.5)
            .map(|(i, _)| i)
            .collect();
        let pick = if visible.is_empty() {
            let center = snap + vw / 2.0;
            cols.iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    ((a.2 + a.3 / 2.0) - center).abs().total_cmp(&((b.2 + b.3 / 2.0) - center).abs())
                })
                .map_or(0, |(i, _)| i)
        } else if snap >= cur_off {
            *visible.last().unwrap()
        } else {
            visible[0]
        };
        let (start, len, _, _) = cols[pick];
        let cur_c = self.focus_col.get();
        let new_c = if cur_c >= start && cur_c < start + len { cur_c } else { start };

        self.target_x.set(snap);
        self.cam_v0_x.set(vel);
        if new_c != cur_c {
            self.focus_coords(cur_r, new_c);
        } else {
            self.reveal_focus();
        }
    }
}
