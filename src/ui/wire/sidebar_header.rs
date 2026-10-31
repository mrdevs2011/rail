use crate::*;

pub(crate) fn wire_sidebar_header(
    app: &Rc<App>,
    head_back: gtk::Button,
    head_forward: gtk::Button,
    head_reload: gtk::Button,
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
    head_forward.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            if v.can_go_forward() {
                v.go_forward();
            }
        }
    });

    let a = app.clone();
    head_reload.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            reload_or_start(&v);
        }
    });

    // Cycle: fixed open ↔ auto (edge hover). Persisted to settings.json.
    let a = app.clone();
    head_mode.connect_clicked(move |_| {
        if a.responsive_layout.get() {
            a.set_sidebar(false);
        } else {
            a.cycle_sidebar_mode();
        }
    });

    // Edge-hover for auto mode: motion on the window captured before WebKit consumes it.
    let motion = gtk::EventControllerMotion::new();
    motion.set_propagation_phase(gtk::PropagationPhase::Capture);
    let last_x = std::rc::Rc::new(std::cell::Cell::new(0.0f64));

    let a_enter = app.clone();
    let last_x_enter = last_x.clone();
    motion.connect_enter(move |_, x, _y| {
        last_x_enter.set(x);
        a_enter.on_shell_pointer_x(x);
    });

    let a_motion = app.clone();
    let last_x_motion = last_x.clone();
    motion.connect_motion(move |_, x, _y| {
        last_x_motion.set(x);
        a_motion.on_shell_pointer_x(x);
    });

    let a_leave = app.clone();
    let last_x_leave = last_x.clone();
    motion.connect_leave(move |_| {
        a_leave.on_shell_pointer_leave(last_x_leave.get());
    });
    app.win.add_controller(motion);

    let a_active = app.clone();
    app.win.connect_is_active_notify(move |w| {
        if !w.is_active()
            && a_active.sidebar_auto.get()
            && a_active.side_open.get()
            && !a_active.sidebar_locked.get()
        {
            a_active.set_sidebar(false);
        }
    });
}
