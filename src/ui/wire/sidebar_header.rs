//! Sidebar header buttons and fixed/auto mode.

use crate::*;

pub(crate) fn wire_sidebar_header(
    app: &Rc<App>,
    head_back: gtk::Button,
    head_reload: gtk::Button,
    head_overview: gtk::Button,
    head_more: gtk::Button,
    head_mode: gtk::Button,
) {
    let a = app.clone();
    head_back.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            if v.can_go_back() {
                v.go_back();
            }
        }
    });

    let a = app.clone();
    head_reload.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            reload_or_start(&v);
        }
    });

    let a = app.clone();
    head_overview.connect_clicked(move |_| {
        a.toggle_overview();
    });

    let a = app.clone();
    head_more.connect_clicked(move |_| {
        // Profiles manager — same as Alt+P
        a.toggle_profiles_modal();
    });

    // Cycle: fixed open ↔ auto (edge hover). Persisted to settings.json.
    let a = app.clone();
    head_mode.connect_clicked(move |_| {
        a.cycle_sidebar_mode();
    });

    // Edge-hover for auto mode: motion on the shell.
    let motion = gtk::EventControllerMotion::new();
    let a = app.clone();
    motion.connect_motion(move |_, x, _y| {
        a.on_shell_pointer_x(x);
    });
    let a = app.clone();
    motion.connect_leave(move |_| {
        a.on_shell_pointer_leave();
    });
    app.shell.add_controller(motion);
}
