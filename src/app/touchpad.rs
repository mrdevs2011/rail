//! Touchpad canvas navigation mode.

use crate::*;

impl App {
    pub(crate) fn toggle_touchpad_mode(self: &Rc<Self>) {
        let new_state = !self.touchpad_mode.get();
        self.touchpad_mode.set(new_state);
        self.tp_switch.set_active(new_state);
        self.schedule_save_session();
        self.refresh_page_scrollable();
        self.show_touchpad_toast(new_state);
    }

    /// Asks the focused page whether it can scroll (either axis) and caches the answer.
    /// Touchpad slide only takes over the gesture when the page has nothing to scroll.
    pub(crate) fn refresh_page_scrollable(self: &Rc<Self>) {
        let Some(view) = self.focused_view() else {
            self.page_scrollable.set(false);
            return;
        };
        let app = self.clone();
        let js = "(function(){try{var d=document,se=d.scrollingElement||d.documentElement,b=d.body;\
            var ok=function(o){return o!=='hidden'&&o!=='clip';};\
            var cs=getComputedStyle(se),cb=b?getComputedStyle(b):cs;\
            var v=se.scrollHeight-se.clientHeight>2&&ok(cs.overflowY)&&ok(cb.overflowY);\
            var h=se.scrollWidth-se.clientWidth>2&&ok(cs.overflowX)&&ok(cb.overflowX);\
            return (v||h)?'1':'0';}catch(e){return '0';}})()";
        view.evaluate_javascript(js, None, None, None::<&gio::Cancellable>, move |res| {
            if let Ok(val) = res {
                app.page_scrollable.set(val.to_string() == "1");
            }
        });
    }

    /// Bottom-right card (same style as the update card) announcing Touchpad Slide ON / OFF.
    /// Hover freezes auto-dismiss; leave resumes a fresh timeout. Clicks stay on the card.
    pub(crate) fn show_touchpad_toast(self: &Rc<Self>, enabled: bool) {
        if enabled {
            self.touchpad_toast_state.set_markup(&format!("<span foreground='#fafafa'>{}</span>", tr("ON")));
            self.touchpad_toast_sub.set_label(tr("2-finger swipe slides tabs on pages that don't scroll"));
        } else {
            self.touchpad_toast_state.set_markup(&format!("<span foreground='#8a8a8a'>{}</span>", tr("OFF")));
            self.touchpad_toast_sub.set_label(tr("Standard page scroll"));
        }
        let token = self.touchpad_toast_gen.get().wrapping_add(1);
        self.touchpad_toast_gen.set(token);
        self.touchpad_toast_hover.set(false);
        self.touchpad_toast.set_reveal_child(true);
        self.schedule_touchpad_toast_hide(token);
    }

    pub(crate) fn schedule_touchpad_toast_hide(self: &Rc<Self>, token: u32) {
        let app = self.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(2200), move || {
            if app.touchpad_toast_gen.get() != token {
                return;
            }
            if app.touchpad_toast_hover.get() {
                return; // frozen while hovered
            }
            app.touchpad_toast.set_reveal_child(false);
        });
    }
}
