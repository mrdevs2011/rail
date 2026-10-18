//! Page load progress bar.

use crate::*;

impl App {
    // ---- load feedback ---------------------------------------------------

    pub(crate) fn update_load_bar(self: &Rc<Self>) {
        let bar = &self.load_bar;
        if self.win.is_fullscreen() {
            bar.set_visible(false);
            return;
        }
        let Some(v) = self.focused_view() else {
            bar.set_visible(false);
            return;
        };
        if v.is_loading() {
            bar.set_visible(true);
            bar.set_fraction(v.estimated_load_progress().max(0.08));
        } else if gtk::prelude::WidgetExt::is_visible(bar) {
            bar.set_fraction(1.0);
            let app = self.clone();
            glib::timeout_add_local_once(Duration::from_millis(250), move || {
                if !app.focused_view().is_some_and(|v| v.is_loading()) {
                    app.load_bar.set_visible(false);
                }
            });
        }
    }
}
