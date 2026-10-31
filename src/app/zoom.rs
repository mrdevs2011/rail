//! Browser page zoom and shared zoom feedback.
//!
//! The percentage shown to the user is always normalized: 100% means the native
//! WebView size, and 100% in Spatial Overview means Rail's default overview scale.

use crate::*;

impl App {
    /// Show the current zoom percentage and restart its short fade-out timer.
    pub(crate) fn show_zoom_indicator(self: &Rc<Self>, percentage: f64) {
        // The readout belongs to browser page zoom only, never the spatial overview.
        if self.overview_open() {
            self.hide_zoom_indicator();
            return;
        }
        let value = if percentage.is_finite() {
            percentage.clamp(20.0, 500.0).round() as u32
        } else {
            100
        };
        self.zoom_indicator_label.set_text(&format!("{value}%"));
        self.zoom_indicator.set_visible(true);
        self.zoom_indicator.set_reveal_child(true);

        if let Some(timer) = self.zoom_indicator_timer.borrow_mut().take() {
            timer.remove();
        }
        let weak = Rc::downgrade(self);
        let timer = glib::timeout_add_local_once(Duration::from_millis(700), move || {
            if let Some(app) = weak.upgrade() {
                app.zoom_indicator.set_reveal_child(false);
                app.zoom_indicator_timer.borrow_mut().take();
                // GtkRevealer hides its child, not the overlay widget itself. Hide the
                // overlay after the fade so its own CSS background cannot remain visible.
                let indicator = app.zoom_indicator.clone();
                glib::timeout_add_local_once(Duration::from_millis(160), move || {
                    if !indicator.is_child_revealed() {
                        indicator.set_visible(false);
                    }
                });
            }
        });
        *self.zoom_indicator_timer.borrow_mut() = Some(timer);
    }

    pub(crate) fn hide_zoom_indicator(&self) {
        if let Some(timer) = self.zoom_indicator_timer.borrow_mut().take() {
            timer.remove();
        }
        self.zoom_indicator.set_reveal_child(false);
        self.zoom_indicator.set_visible(false);
    }

    /// One discrete browser-style page zoom step (or reset when `dir == 0`).
    pub(crate) fn page_zoom(self: &Rc<Self>, dir: i32) {
        let Some(view) = self.focused_view() else {
            return;
        };
        let next = if dir == 0 {
            1.0
        } else {
            next_page_zoom(view.zoom_level(), dir)
        };
        view.set_zoom_level(next);
        self.show_zoom_indicator(next * 100.0);
    }

    /// Shared discrete browser-style page zoom entry point.
    pub(crate) fn zoom_step(self: &Rc<Self>, dir: i32) {
        if !self.overview_open() {
            self.page_zoom(dir);
        }
    }
}
