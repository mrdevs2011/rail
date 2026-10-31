//! Strip scrolling, camera animation and slides.

use crate::*;

/// Preserve the existing viewport center across an overview zoom, then reveal the
/// focused tile only as much as necessary. This mirrors Niri's
/// `center-focused-column "never"` behavior: focus changes never snap a column
/// to the center, including columns that are exactly half the viewport wide.
fn overview_axis_offset(
    current_offset: f64,
    from_zoom: f64,
    to_zoom: f64,
    viewport: f64,
    focused_span: Option<(f64, f64)>,
) -> f64 {
    if viewport <= 0.0 || from_zoom <= 0.0 || to_zoom <= 0.0 {
        return current_offset;
    }

    let world_center = current_offset + viewport / (2.0 * from_zoom);
    let visible = viewport / to_zoom;
    let mut offset = world_center - visible / 2.0;

    if let Some((start, end)) = focused_span {
        let size = (end - start).max(0.0);
        if size >= visible {
            // An oversized tile cannot fit; keep its leading edge visible.
            offset = start;
        } else if start < offset {
            offset = start;
        } else if end > offset + visible {
            offset = end - visible;
        }
    }

    offset
}

impl App {
    // ---- strip scrolling -------------------------------------------------

    /// Camera offsets that preserve the current viewport center across zoom changes,
    /// revealing the focused tile only when it would otherwise leave the visible area.
    pub(crate) fn overview_offsets_at_zoom(&self, z: f64) -> (f64, f64) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        if vw <= 0.0 || vh <= 0.0 || z <= 0.01 {
            return (self.target_x.get(), self.target_y.get());
        }

        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let g = gap(z);
        let mut focused_x = None;
        let mut focused_y = None;

        {
            let rows = self.rows.borrow();
            if let Some(row) = rows.get(cur_r) {
                for (start, len, x, w) in row_cols(row, vw, g) {
                    if cur_c < start || cur_c >= start + len {
                        continue;
                    }

                    focused_x = Some((x, x + w));

                    // Keep overview hit framing identical to StripLayout's ratio-based height allocation.
                    let weights: Vec<f64> = (0..len)
                        .map(|j| {
                            row.tabs[start + j]
                                .custom_height
                                .get()
                                .unwrap_or(1.0 / len.max(1) as f64)
                        })
                        .collect();
                    let heights = if len == 1 {
                        vec![vh]
                    } else {
                        let available = (vh - (len as f64 - 1.0) * g).max(1.0);
                        distribute_height_weights(&weights, available, MIN_WINDOW_HEIGHT)
                    };
                    let index = cur_c - start;
                    let y_in_stack: f64 = heights[..index].iter().map(|h| h + g).sum();
                    let tile_h = heights[index];
                    let y = cur_r as f64 * (vh + g) + y_in_stack;
                    focused_y = Some((y, y + tile_h));
                    break;
                }
            }
        }

        let from_zoom = self.target_zoom.get().max(0.01);
        let tx = overview_axis_offset(self.target_x.get(), from_zoom, z, vw, focused_x);
        let ty = overview_axis_offset(self.target_y.get(), from_zoom, z, vh, focused_y);
        (tx, ty)
    }

    /// Normal mode: minimally reveal the focused row and column.
    /// Overview: preserve the camera and minimally reveal the focused tile, Niri-style.
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
            let dt = last
                .replace(Some(now))
                .map_or(1.0 / 60.0, |l| (now - l) as f64 / 1e6);
            let k = (1.0 - (-dt / 0.038).exp()).clamp(0.04, 1.0);

            let oxc = app.layout.offset_x();
            let oyc = app.layout.offset_y();
            let zc = app.layout.zoom();
            let oxt = app.target_x.get();
            let oyt = app.target_y.get();
            let zt = app.target_zoom.get();

            let vw = strip.width() as f64;
            let vh = strip.height() as f64;
            let in_overview = app.ov.get() || app.ov_exiting.get() || zc < 0.999 || zt < 0.999;
            // Any overview zoom transition (in or out), staying active until the timed ease settles.
            let ov_motion = vw > 1.0
                && vh > 1.0
                && in_overview
                && (zanim.get().is_some() || !near(zc, zt, 0.001));

            let nz: f64;
            let mut nx: f64;
            let mut ny: f64;
            if ov_motion {
                let zc_safe = zc.max(0.01);
                let zt_safe = zt.max(0.01);
                let target_wcx = oxt + vw / (2.0 * zt_safe);
                let target_wcy = oyt + vh / (2.0 * zt_safe);

                let (t0, z0, wcx0, wcy0, wcxt, wcyt) = match zanim.get() {
                    Some(a) if (a.2 - zt).abs() < 1e-9 => (a.0, a.1, a.3, a.4, a.5, a.6),
                    _ => {
                        let wcx_start = oxc + vw / (2.0 * zc_safe);
                        let wcy_start = oyc + vh / (2.0 * zc_safe);
                        let mut twcx = target_wcx;
                        let mut twcy = target_wcy;
                        // Subpixel alignment: if centers match within 1px, lock to eliminate micro-drift/jitter
                        if (twcx - wcx_start).abs() < 1.0 {
                            twcx = wcx_start;
                        }
                        if (twcy - wcy_start).abs() < 1.0 {
                            twcy = wcy_start;
                        }
                        let a = (now, zc, zt, wcx_start, wcy_start, twcx, twcy);
                        zanim.set(Some(a));
                        (a.0, a.1, a.3, a.4, twcx, twcy)
                    }
                };
                let zoom_ratio = (zt / z0.max(0.01)).ln().abs();
                // Full overview open/close uses OV_ANIM_SECS; small incremental pinch steps use 0.06s so gestures stay smooth
                let anim_secs = (zoom_ratio * 0.20).clamp(0.06, OV_ANIM_SECS);
                let u = (((now - t0) as f64 / 1e6) / anim_secs).clamp(0.0, 1.0);
                // Ease-out quad: responsive, fast, and flat/even (tekkiz) without sluggish tail
                let e = 1.0 - (1.0 - u) * (1.0 - u);
                if u >= 1.0 {
                    nz = zt;
                    nx = oxt;
                    ny = oyt;
                    zanim.set(None);
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
            } else {
                zanim.set(None);
                let mut z = zc + (zt - zc) * k;
                if near(zt, z, 0.002) {
                    z = zt;
                }
                nz = z;
                // Niri-style critically damped spring on the real frame time (no per-frame lerp)
                let (sx, _) = step_cam_axis(&app.cam_spring_x, oxc, oxt, now);
                let (sy, _) = step_cam_axis(&app.cam_spring_y, oyc, oyt, now);
                nx = sx;
                ny = sy;
            }
            if near(oxt, nx, 0.5) && nz == zt {
                nx = oxt;
            }
            if near(oyt, ny, 0.5) && nz == zt {
                ny = oyt;
            }

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
}
