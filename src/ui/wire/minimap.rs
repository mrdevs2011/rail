//! Minimap / roadmap: buttons, drawing, pan, zoom and header dragging.

use crate::*;

pub(crate) fn wire_minimap(
    app: &Rc<App>,
    map_close_btn: gtk::Button,
    map_container: gtk::Box,
    map_drawing_area: gtk::DrawingArea,
    map_header_row: gtk::Box,
    map_location_btn: gtk::Button,
    overlay: gtk::Overlay,
) {
    let a_close = app.clone();
    map_close_btn.connect_clicked(move |_| a_close.hide_map());

    let a_loc = app.clone();
    map_location_btn.connect_clicked(move |_| {
        {
            let mut st = a_loc.map_state.borrow_mut();
            st.target_offset_x = 0.0;
            st.target_offset_y = 0.0;
            st.vel_x = 0.0;
            st.vel_y = 0.0;
            st.user_panned = false;
        }
        a_loc.map_location_btn.set_visible(false);
        a_loc.trigger_map_animation();
    });

    let a_draw = app.clone();
    map_drawing_area.set_draw_func(move |_area, cr, width, height| {
        a_draw.draw_minimap(cr, width as f64, height as f64);
    });

    let drag_dist = Rc::new(Cell::new(0.0));
    let is_pinching = Rc::new(Cell::new(false));
    let pinch_end_time = Rc::new(Cell::new(0i64));
    let last_drag_pos = Rc::new(Cell::new((0.0f64, 0.0f64)));
    let last_drag_t = Rc::new(Cell::new(0i64));

    let drag = gtk::GestureDrag::new();
    drag.connect_drag_begin({
        let a = app.clone();
        let drag_dist = drag_dist.clone();
        let last_pos = last_drag_pos.clone();
        let last_t = last_drag_t.clone();
        move |_gesture, start_x, start_y| {
            drag_dist.set(0.0);
            last_pos.set((start_x, start_y));
            last_t.set(glib::monotonic_time());
            let mut st = a.map_state.borrow_mut();
            st.drag_start_offset_x = st.target_offset_x;
            st.drag_start_offset_y = st.target_offset_y;
            st.vel_x = 0.0;
            st.vel_y = 0.0;
        }
    });

    drag.connect_drag_update({
        let a = app.clone();
        let drag_dist = drag_dist.clone();
        let is_pinching = is_pinching.clone();
        let pinch_end_time = pinch_end_time.clone();
        let last_t = last_drag_t.clone();
        move |_gesture, offset_x, offset_y| {
            if is_pinching.get() || (glib::monotonic_time() - pinch_end_time.get()) < 200_000 {
                return;
            }
            drag_dist.set(offset_x.hypot(offset_y));

            let now = glib::monotonic_time();
            let dt = ((now - last_t.get()) as f64 / 1e6).max(0.001);

            let mut st = a.map_state.borrow_mut();
            let new_target_ox = st.drag_start_offset_x - offset_x / st.zoom;
            let new_target_oy = st.drag_start_offset_y - offset_y / st.zoom;

            let instant_vx = (new_target_ox - st.target_offset_x) / dt;
            let instant_vy = (new_target_oy - st.target_offset_y) / dt;
            st.vel_x = st.vel_x * 0.3 + instant_vx.clamp(-3000.0, 3000.0) * 0.7;
            st.vel_y = st.vel_y * 0.3 + instant_vy.clamp(-3000.0, 3000.0) * 0.7;

            st.target_offset_x = new_target_ox;
            st.target_offset_y = new_target_oy;
            st.offset_x = new_target_ox;
            st.offset_y = new_target_oy;
            st.user_panned = true;

            last_t.set(now);
            drop(st);
            a.update_map_location_btn_visibility();
            a.map_drawing_area.queue_draw();
        }
    });

    drag.connect_drag_end({
        let a = app.clone();
        let drag_dist = drag_dist.clone();
        move |_gesture, _offset_x, _offset_y| {
            if drag_dist.get() > 4.0 {
                a.trigger_map_animation();
            }
        }
    });
    map_drawing_area.add_controller(drag);

    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_PRIMARY);
    let a_click = app.clone();
    let drag_dist_for_click = drag_dist.clone();
    let is_pinching_click = is_pinching.clone();
    let pinch_end_time_click = pinch_end_time.clone();
    click.connect_released(move |_gesture, _n_press, x, y| {
        if is_pinching_click.get()
            || (glib::monotonic_time() - pinch_end_time_click.get()) < 250_000
        {
            return;
        }
        if drag_dist_for_click.get() < 6.0 {
            a_click.handle_minimap_click(x, y);
        }
    });
    map_drawing_area.add_controller(click);

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    let a_scroll = app.clone();
    scroll.connect_scroll(move |_ctrl, _dx, dy| {
        if dy.abs() > 0.001 {
            let mut st = a_scroll.map_state.borrow_mut();
            let factor = 1.0 - (dy * 0.12).clamp(-0.35, 0.35);
            st.target_zoom = (st.target_zoom * factor).clamp(0.2, 4.0);
            drop(st);
            a_scroll.trigger_map_animation();
        }
        glib::Propagation::Stop
    });
    map_drawing_area.add_controller(scroll);

    let zoom_gesture = gtk::GestureZoom::new();
    let prev_scale = Rc::new(Cell::new(1.0f64));

    {
        let prev_scale = prev_scale.clone();
        let is_pinching = is_pinching.clone();
        zoom_gesture.connect_begin(move |_gesture, _seq| {
            prev_scale.set(1.0);
            is_pinching.set(true);
        });
    }

    {
        let is_pinching = is_pinching.clone();
        let pinch_end_time = pinch_end_time.clone();
        let a = app.clone();
        zoom_gesture.connect_end(move |_gesture, _seq| {
            is_pinching.set(false);
            pinch_end_time.set(glib::monotonic_time());
            a.trigger_map_animation();
        });
    }

    {
        let a = app.clone();
        let prev_scale = prev_scale.clone();
        zoom_gesture.connect_scale_changed(move |_gesture, scale| {
            let p = prev_scale.get();
            if p > 0.0 && scale > 0.0 {
                let factor = scale / p;
                let mut st = a.map_state.borrow_mut();
                st.target_zoom = (st.target_zoom * factor).clamp(0.2, 4.0);
                st.zoom = st.target_zoom;
                drop(st);
                a.update_map_location_btn_visibility();
                a.map_drawing_area.queue_draw();
            }
            prev_scale.set(scale);
        });
    }
    map_drawing_area.add_controller(zoom_gesture);

    // ---- Minimap Header Dragging (reposition floating roadmap window) ----
    let header_drag = gtk::GestureDrag::new();
    header_drag.set_button(gdk::BUTTON_PRIMARY);

    let header_start_margin_end = Rc::new(Cell::new(24));
    let header_start_margin_bottom = Rc::new(Cell::new(24));
    let header_start_root_x = Rc::new(Cell::new(0.0f64));
    let header_start_root_y = Rc::new(Cell::new(0.0f64));

    {
        let start_e = header_start_margin_end.clone();
        let start_b = header_start_margin_bottom.clone();
        let start_rx = header_start_root_x.clone();
        let start_ry = header_start_root_y.clone();
        let map_container = map_container.clone();
        let map_header_row = map_header_row.clone();
        header_drag.connect_drag_begin(move |gesture, _start_x, _start_y| {
            start_e.set(map_container.margin_end());
            start_b.set(map_container.margin_bottom());
            if let Some(ev) = gesture.current_event() {
                if let Some((rx, ry)) = ev.position() {
                    start_rx.set(rx);
                    start_ry.set(ry);
                }
            }
            map_header_row.set_cursor_from_name(Some("grabbing"));
        });
    }

    {
        let start_e = header_start_margin_end.clone();
        let start_b = header_start_margin_bottom.clone();
        let start_rx = header_start_root_x.clone();
        let start_ry = header_start_root_y.clone();
        let map_container = map_container.clone();
        let overlay = overlay.clone();
        header_drag.connect_drag_update(move |gesture, offset_x, offset_y| {
            let (dx, dy) = if let Some(ev) = gesture.current_event() {
                if let Some((rx, ry)) = ev.position() {
                    (rx - start_rx.get(), ry - start_ry.get())
                } else {
                    (offset_x, offset_y)
                }
            } else {
                (offset_x, offset_y)
            };

            let new_m_end = start_e.get() - (dx.round() as i32);
            let new_m_bottom = start_b.get() - (dy.round() as i32);

            let overlay_w = overlay.width().max(600);
            let overlay_h = overlay.height().max(400);
            let map_w = map_container.width().max(380);
            let map_h = map_container.height().max(260);

            let max_margin_e = (overlay_w - map_w - 8).max(8);
            let max_margin_b = (overlay_h - map_h - 8).max(8);

            map_container.set_margin_end(new_m_end.clamp(8, max_margin_e));
            map_container.set_margin_bottom(new_m_bottom.clamp(8, max_margin_b));
        });
    }

    {
        let map_header_row = map_header_row.clone();
        header_drag.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
            map_header_row.set_cursor_from_name(Some("grab"));
        });
    }
    map_header_row.add_controller(header_drag);
}
