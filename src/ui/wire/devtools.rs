//! DevTools dock: toolbar slots, resize handle, close / detach; sidebar buttons.

use crate::*;

pub(crate) fn wire_devtools(app: &Rc<App>, devtools_close_btn: gtk::Button, devtools_content_box: gtk::Box, devtools_detach_btn: gtk::Button, devtools_resize_handle: gtk::Box, footer_downloads: gtk::Button, footer_settings: gtk::Button, head_close_side: gtk::Button) {
    // WebKit's own inspector toolbar handles close/dock/undock via closed/attach/detach signals.
    // The attach signal carries no side, so remember which toolbar slot was clicked (never claims the click).
    let dt_click = gtk::GestureClick::new();
    dt_click.set_propagation_phase(gtk::PropagationPhase::Capture);
    dt_click.connect_pressed(move |g, _, x, y| {
        g.set_state(gtk::EventSequenceState::None);
        if (0.0..=36.0).contains(&y) {
            LAST_DT_SLOT.with(|c| c.set((x / 24.0).floor() as i32));
        } else {
            LAST_DT_SLOT.with(|c| c.set(-1));
        }
    });
    devtools_content_box.add_controller(dt_click);

    // DevTools resize drag handle
    let dt_drag = gtk::GestureDrag::new();
    let app_drag = app.clone();
    let start_size = Rc::new(Cell::new(0i32));
    let st_sz_begin = start_size.clone();
    dt_drag.connect_drag_begin(move |_, _, _| {
        let side = app_drag.devtools_dock_side.get();
        let cur = match side {
            DevToolsDockSide::Bottom => app_drag.devtools_panel.height(),
            DevToolsDockSide::Left | DevToolsDockSide::Right => app_drag.devtools_panel.width(),
            _ => 0,
        };
        st_sz_begin.set(cur);
    });

    let app_drag_up = app.clone();
    let st_sz_update = start_size.clone();
    dt_drag.connect_drag_update(move |_, offset_x, offset_y| {
        let side = app_drag_up.devtools_dock_side.get();
        let base = st_sz_update.get() as f64;
        let win_w = app_drag_up.win.width() as f64;
        let win_h = app_drag_up.win.height() as f64;
        match side {
            DevToolsDockSide::Bottom => {
                let nh = (base - offset_y).clamp(180.0, (win_h - 100.0).max(200.0)) as i32;
                app_drag_up.devtools_panel.set_height_request(nh);
            }
            DevToolsDockSide::Left => {
                let nw = (base + offset_x).clamp(280.0, (win_w - 200.0).max(300.0)) as i32;
                app_drag_up.devtools_panel.set_width_request(nw);
            }
            DevToolsDockSide::Right => {
                let nw = (base - offset_x).clamp(280.0, (win_w - 200.0).max(300.0)) as i32;
                app_drag_up.devtools_panel.set_width_request(nw);
            }
            _ => {}
        }
    });
    devtools_resize_handle.add_controller(dt_drag);

    let a_dt_close = app.clone();
    devtools_close_btn.connect_clicked(move |_| a_dt_close.hide_devtools());

    let a_dt_detach = app.clone();
    devtools_detach_btn.connect_clicked(move |_| a_dt_detach.toggle_devtools_undocked());


    {
        let a = app.clone();
        head_close_side.connect_clicked(move |_| a.set_sidebar(false));
    }
    {
        let a = app.clone();
        footer_settings.connect_clicked(move |_| a.open_settings_tab(SettingsTab::General));
    }
    {
        let a = app.clone();
        footer_downloads.connect_clicked(move |_| a.open_downloads());
    }
}
