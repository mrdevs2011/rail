//! Touchpad notification card: dismiss and hover freeze.

use crate::*;

pub(crate) fn wire_toast(app: &Rc<App>, tp_close: gtk::Button) {
    // Notification card: X dismiss + hover freeze (clicks do not pass through)
    {
        let a = app.clone();
        tp_close.connect_clicked(move |_| {
            a.touchpad_toast.set_reveal_child(false);
        });
    }
    {
        let motion = gtk::EventControllerMotion::new();
        let a_enter = app.clone();
        motion.connect_enter(move |_, _, _| {
            a_enter.touchpad_toast_hover.set(true);
        });
        let a_leave = app.clone();
        motion.connect_leave(move |_| {
            a_leave.touchpad_toast_hover.set(false);
            let token = a_leave.touchpad_toast_gen.get();
            a_leave.schedule_touchpad_toast_hide(token);
        });
        // controller on the revealer so the whole card area counts
        app.touchpad_toast.add_controller(motion);
    }
}
