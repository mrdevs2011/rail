//! Minimap / roadmap (Alt+M).

use crate::*;

impl App {
    // ---- Minimap / Roadmap (Alt+M) ----

    pub(crate) fn map_open(&self) -> bool {
        self.map_container.is_visible()
    }

    pub(crate) fn toggle_map(self: &Rc<Self>) {
        if self.map_open() {
            self.hide_map();
        } else {
            self.show_map();
        }
    }

    pub(crate) fn show_map(self: &Rc<Self>) {
        let rows = self.rows.borrow();
        let (cx, cy) = self.compute_active_tab_center(&rows);
        let (rx, ry, rw, rh) = self.compute_active_tab_rect(&rows);
        drop(rows);

        {
            let mut st = self.map_state.borrow_mut();
            st.cam_x = cx;
            st.cam_y = cy;
            st.offset_x = 0.0;
            st.offset_y = 0.0;
            st.target_offset_x = 0.0;
            st.target_offset_y = 0.0;
            st.zoom = 1.0;
            st.target_zoom = 1.0;
            st.user_panned = false;
            st.vel_x = 0.0;
            st.vel_y = 0.0;
            st.cursor_x = rx;
            st.cursor_y = ry;
            st.cursor_w = rw;
            st.cursor_h = rh;
            st.cursor_initialized = true;
            st.animating = false;
        }
        self.map_location_btn.set_visible(false);
        self.map_container.set_visible(true);
        self.map_drawing_area.queue_draw();
    }

    pub(crate) fn hide_map(&self) {
        self.map_container.set_visible(false);
    }

    pub(crate) fn update_map_location_btn_visibility(&self) {
        let st = self.map_state.borrow();
        let width = (self.map_drawing_area.width() as f64).max(380.0);
        let height = (self.map_drawing_area.height() as f64).max(260.0);
        let active_sx = width / 2.0 - st.target_offset_x * st.zoom;
        let active_sy = height / 2.0 - st.target_offset_y * st.zoom;
        let off_screen = active_sx < 20.0
            || active_sx > width - 20.0
            || active_sy < 20.0
            || active_sy > height - 20.0;
        let far_from_center =
            (st.target_offset_x * st.zoom).hypot(st.target_offset_y * st.zoom) > 40.0;
        let should_show = st.user_panned && (off_screen || far_from_center);
        self.map_location_btn.set_visible(should_show);
    }

    pub(crate) fn minimap_tab_width(&self, tab: &Tab) -> f64 {
        if let Some(cw) = tab.custom_width.get() {
            let vw = self.strip.width().max(600) as f64;
            (cw / vw * MAP_TAB_W_FULL).clamp(MAP_TAB_W * 0.7, MAP_TAB_W_FULL)
        } else if tab.container.has_css_class("full") {
            MAP_TAB_W_FULL
        } else {
            MAP_TAB_W
        }
    }

    pub(crate) fn compute_active_tab_center(&self, rows: &[Row]) -> (f64, f64) {
        let fr = self.focus_row.get();
        let fc = self.focus_col.get();
        if fr < rows.len() {
            let row = &rows[fr];
            if fc < row.tabs.len() {
                let y = fr as f64 * (MAP_TAB_H + MAP_GAP_Y);
                let mut x = 0.0;
                for (i, tab) in row.tabs.iter().enumerate() {
                    let w = self.minimap_tab_width(tab);
                    if i == fc {
                        return (x + w / 2.0, y + MAP_TAB_H / 2.0);
                    }
                    x += w + MAP_GAP_X;
                }
            }
        }
        (0.0, 0.0)
    }

    pub(crate) fn compute_active_tab_rect(&self, rows: &[Row]) -> (f64, f64, f64, f64) {
        let fr = self.focus_row.get();
        let fc = self.focus_col.get();
        if fr < rows.len() {
            let row = &rows[fr];
            if fc < row.tabs.len() {
                let y = fr as f64 * (MAP_TAB_H + MAP_GAP_Y);
                let mut x = 0.0;
                for (i, tab) in row.tabs.iter().enumerate() {
                    let w = self.minimap_tab_width(tab);
                    if i == fc {
                        return (x, y, w, MAP_TAB_H);
                    }
                    x += w + MAP_GAP_X;
                }
            }
        }
        (0.0, 0.0, MAP_TAB_W, MAP_TAB_H)
    }

    pub(crate) fn trigger_map_animation(self: &Rc<Self>) {
        if !self.map_open() {
            return;
        }
        {
            let mut st = self.map_state.borrow_mut();
            if st.animating {
                return;
            }
            st.animating = true;
        }

        let app = self.clone();
        let last_time = Cell::new(None::<i64>);
        let drawing_area = app.map_drawing_area.clone();

        drawing_area.add_tick_callback(move |area, clock| {
            if !app.map_open() {
                app.map_state.borrow_mut().animating = false;
                return glib::ControlFlow::Break;
            }

            let now = clock.frame_time();
            let dt = last_time
                .replace(Some(now))
                .map_or(1.0 / 60.0, |l| ((now - l) as f64 / 1e6).clamp(0.001, 0.1));

            let k_pos = 1.0 - (-dt / 0.065).exp();
            let k_zoom = 1.0 - (-dt / 0.075).exp();

            let rows = app.rows.borrow();
            let (target_active_cx, target_active_cy) = app.compute_active_tab_center(&rows);
            let (target_rect_x, target_rect_y, target_rect_w, target_rect_h) =
                app.compute_active_tab_rect(&rows);
            drop(rows);

            let mut st = app.map_state.borrow_mut();

            // Inertia decay
            if st.vel_x.abs() > 0.05 || st.vel_y.abs() > 0.05 {
                st.target_offset_x += st.vel_x * dt;
                st.target_offset_y += st.vel_y * dt;
                let friction = (-dt * 7.5).exp();
                st.vel_x *= friction;
                st.vel_y *= friction;
                if st.vel_x.abs() < 0.05 {
                    st.vel_x = 0.0;
                }
                if st.vel_y.abs() < 0.05 {
                    st.vel_y = 0.0;
                }
                st.user_panned = true;
            }

            let target_cam_x = target_active_cx + st.target_offset_x;
            let target_cam_y = target_active_cy + st.target_offset_y;

            let mut diff_cx = target_cam_x - st.cam_x;
            let mut diff_cy = target_cam_y - st.cam_y;
            if diff_cx.abs() < 0.2 {
                st.cam_x = target_cam_x;
                diff_cx = 0.0;
            } else {
                st.cam_x += diff_cx * k_pos;
            }
            if diff_cy.abs() < 0.2 {
                st.cam_y = target_cam_y;
                diff_cy = 0.0;
            } else {
                st.cam_y += diff_cy * k_pos;
            }

            let diff_ox = st.target_offset_x - st.offset_x;
            let diff_oy = st.target_offset_y - st.offset_y;
            if diff_ox.abs() < 0.2 {
                st.offset_x = st.target_offset_x;
            } else {
                st.offset_x += diff_ox * k_pos;
            }
            if diff_oy.abs() < 0.2 {
                st.offset_y = st.target_offset_y;
            } else {
                st.offset_y += diff_oy * k_pos;
            }

            let mut diff_zoom = st.target_zoom - st.zoom;
            if diff_zoom.abs() < 0.002 {
                st.zoom = st.target_zoom;
                diff_zoom = 0.0;
            } else {
                st.zoom += diff_zoom * k_zoom;
            }

            let mut diff_rx = target_rect_x - st.cursor_x;
            let mut diff_ry = target_rect_y - st.cursor_y;
            let mut diff_rw = target_rect_w - st.cursor_w;
            let mut diff_rh = target_rect_h - st.cursor_h;

            if diff_rx.abs() < 0.2 {
                st.cursor_x = target_rect_x;
                diff_rx = 0.0;
            } else {
                st.cursor_x += diff_rx * k_pos;
            }
            if diff_ry.abs() < 0.2 {
                st.cursor_y = target_rect_y;
                diff_ry = 0.0;
            } else {
                st.cursor_y += diff_ry * k_pos;
            }
            if diff_rw.abs() < 0.2 {
                st.cursor_w = target_rect_w;
                diff_rw = 0.0;
            } else {
                st.cursor_w += diff_rw * k_pos;
            }
            if diff_rh.abs() < 0.2 {
                st.cursor_h = target_rect_h;
                diff_rh = 0.0;
            } else {
                st.cursor_h += diff_rh * k_pos;
            }

            let still_animating = diff_cx != 0.0
                || diff_cy != 0.0
                || diff_zoom != 0.0
                || diff_rx != 0.0
                || diff_ry != 0.0
                || diff_rw != 0.0
                || diff_rh != 0.0
                || st.vel_x != 0.0
                || st.vel_y != 0.0;

            drop(st);

            app.update_map_location_btn_visibility();
            area.queue_draw();

            if still_animating {
                glib::ControlFlow::Continue
            } else {
                app.map_state.borrow_mut().animating = false;
                glib::ControlFlow::Break
            }
        });
    }

    pub(crate) fn draw_rounded_rect(
        cr: &gtk::cairo::Context,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: f64,
    ) {
        let r = r.min(w / 2.0).min(h / 2.0);
        cr.new_sub_path();
        cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(
            x + r,
            y + h - r,
            r,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        cr.arc(
            x + r,
            y + r,
            r,
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
        );
        cr.close_path();
    }

    pub(crate) fn draw_minimap(&self, cr: &gtk::cairo::Context, width: f64, height: f64) {
        // Rail monochrome canvas background (#080808)
        cr.set_source_rgb(0.03, 0.03, 0.03);
        let _ = cr.paint();

        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }

        let (cam_x, cam_y, zoom, cur_rx, cur_ry, cur_rw, cur_rh, cursor_initialized) = {
            let st = self.map_state.borrow();
            (
                st.cam_x,
                st.cam_y,
                st.zoom,
                st.cursor_x,
                st.cursor_y,
                st.cursor_w,
                st.cursor_h,
                st.cursor_initialized,
            )
        };

        for (r, row) in rows.iter().enumerate() {
            let y = r as f64 * (MAP_TAB_H + MAP_GAP_Y);
            let sy = height / 2.0 + (y - cam_y) * zoom;
            let sh = MAP_TAB_H * zoom;

            if sy + sh < -10.0 || sy > height + 10.0 {
                continue;
            }

            let mut x = 0.0;
            for (_c, tab) in row.tabs.iter().enumerate() {
                let w = self.minimap_tab_width(tab);
                let sx = width / 2.0 + (x - cam_x) * zoom;
                let sw = w * zoom;
                x += w + MAP_GAP_X;

                if sx + sw < -10.0 || sx > width + 10.0 {
                    continue;
                }

                let corner_r = (3.0 * zoom).clamp(1.5, 4.0);

                Self::draw_rounded_rect(cr, sx, sy, sw, sh, corner_r);

                // Subtle graphite tile (#141414)
                cr.set_source_rgb(0.08, 0.08, 0.08);
                let _ = cr.fill_preserve();
                // Subtle graphite border (#262626)
                cr.set_source_rgb(0.15, 0.15, 0.15);
                cr.set_line_width((1.2 * zoom).clamp(0.8, 1.8));
                let _ = cr.stroke();
            }
        }

        // Draw the silky-smooth gliding active tab reticle / cursor
        if cursor_initialized {
            let cur_sx = width / 2.0 + (cur_rx - cam_x) * zoom;
            let cur_sy = height / 2.0 + (cur_ry - cam_y) * zoom;
            let cur_sw = cur_rw * zoom;
            let cur_sh = cur_rh * zoom;
            let corner_r = (3.0 * zoom).clamp(1.5, 4.0);

            Self::draw_rounded_rect(cr, cur_sx, cur_sy, cur_sw, cur_sh, corner_r);

            // Dark fill
            cr.set_source_rgba(0.22, 0.22, 0.22, 0.95);
            let _ = cr.fill_preserve();

            // Bright white border
            cr.set_source_rgb(0.93, 0.93, 0.93);
            cr.set_line_width((2.0 * zoom).clamp(1.4, 2.8));
            let _ = cr.stroke();

            // Bright center core indicator
            let dot_r = (2.2 * zoom).clamp(1.5, 3.2);
            cr.arc(
                cur_sx + cur_sw / 2.0,
                cur_sy + cur_sh / 2.0,
                dot_r,
                0.0,
                std::f64::consts::PI * 2.0,
            );
            cr.set_source_rgb(1.0, 1.0, 1.0);
            let _ = cr.fill();
        }
    }

    pub(crate) fn handle_minimap_click(self: &Rc<Self>, click_x: f64, click_y: f64) {
        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }
        let (cam_x, cam_y, zoom) = {
            let st = self.map_state.borrow();
            (st.cam_x, st.cam_y, st.zoom)
        };

        let width = (self.map_drawing_area.width() as f64).max(380.0);
        let height = (self.map_drawing_area.height() as f64).max(260.0);
        let world_x = cam_x + (click_x - width / 2.0) / zoom;
        let world_y = cam_y + (click_y - height / 2.0) / zoom;

        for (r, row) in rows.iter().enumerate() {
            let y = r as f64 * (MAP_TAB_H + MAP_GAP_Y);
            if world_y < y || world_y > y + MAP_TAB_H {
                continue;
            }
            let mut x = 0.0;
            for (c, tab) in row.tabs.iter().enumerate() {
                let w = self.minimap_tab_width(tab);
                if world_x >= x && world_x <= x + w {
                    drop(rows);
                    self.focus_coords(r, c);
                    return;
                }
                x += w + MAP_GAP_X;
            }
        }
    }
}
