//! Per-tab docked Web Inspector.
//! Each tab window contains its own DevTools console docked at the bottom,
//! preserving independent sizing and allowing multiple DevTools instances across tabs.

use crate::*;

thread_local! {
    static DT_CLOSING_WIN: Cell<bool> = const { Cell::new(false) };
    static IN_SHOW_DEVTOOLS: Cell<bool> = const { Cell::new(false) };
    pub(crate) static LAST_DT_SLOT: Cell<i32> = const { Cell::new(-1) };
    pub(crate) static DT_VIEW: RefCell<Option<WebView>> = const { RefCell::new(None) };
}

impl App {
    // ---- DevTools (Alt+Shift+I / F12 / Alt+Shift+D) ----

    pub(crate) fn devtools_open(&self) -> bool {
        let Some(view) = self.focused_view() else { return false };
        let Some((r, c)) = self.find_view_coords(&view) else { return false };
        let rows = self.rows.borrow();
        rows.get(r)
            .and_then(|row| row.tabs.get(c))
            .map(|t| t.dt_open.get())
            .unwrap_or(false)
    }

    pub(crate) fn dt_view(&self) -> Option<WebView> {
        self.focused_view()
    }

    pub(crate) fn dt_close_window(&self) {
        let win = self.devtools_window.borrow_mut().take();
        if let Some(win) = win {
            DT_CLOSING_WIN.with(|f| f.set(true));
            win.set_child(None::<&gtk::Widget>);
            win.close();
            DT_CLOSING_WIN.with(|f| f.set(false));
        }
    }

    pub(crate) fn show_view_devtools(&self, view: &WebView) {
        if IN_SHOW_DEVTOOLS.with(|f| f.replace(true)) {
            return;
        }
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                IN_SHOW_DEVTOOLS.with(|f| f.set(false));
            }
        }
        let _guard = Guard;

        let Some((r, c)) = self.find_view_coords(view) else { return };
        let rows = self.rows.borrow();
        let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) else { return };
        let Some(insp) = view.inspector() else { return };

        let mut h = tab.dt_height.get();
        if h < 140 {
            h = self.default_devtools_height.get().clamp(160, 600);
            tab.dt_height.set(h);
        }

        tab.dt_box.set_height_request(h);
        tab.dt_box.set_visible(true);
        tab.dt_handle.set_visible(true);
        tab.dt_open.set(true);

        insp.show();

        let attach_widget = {
            let dt_box = tab.dt_box.clone();
            let insp = insp.clone();
            move || {
                if let Some(w) = insp.web_view() {
                    let widget: gtk::Widget = w.upcast();
                    if widget.parent().as_ref() != Some(dt_box.upcast_ref()) {
                        if widget.parent().is_some() {
                            widget.unparent();
                        }
                        while let Some(ch) = dt_box.first_child() {
                            dt_box.remove(&ch);
                        }
                        widget.set_hexpand(true);
                        widget.set_vexpand(true);
                        dt_box.append(&widget);
                    }
                }
            }
        };

        attach_widget();
        glib::idle_add_local_once(attach_widget);

        let dt_box_poll = tab.dt_box.clone();
        let insp_poll = insp.clone();
        let retries = std::rc::Rc::new(std::cell::Cell::new(0));
        glib::timeout_add_local(std::time::Duration::from_millis(35), move || {
            let count = retries.get();
            if let Some(w) = insp_poll.web_view() {
                let widget: gtk::Widget = w.upcast();
                if widget.parent().as_ref() != Some(dt_box_poll.upcast_ref()) {
                    if widget.parent().is_some() {
                        widget.unparent();
                    }
                    while let Some(ch) = dt_box_poll.first_child() {
                        dt_box_poll.remove(&ch);
                    }
                    widget.set_hexpand(true);
                    widget.set_vexpand(true);
                    dt_box_poll.append(&widget);
                }
                return glib::ControlFlow::Break;
            }
            if count >= 35 {
                return glib::ControlFlow::Break;
            }
            retries.set(count + 1);
            glib::ControlFlow::Continue
        });
    }

    pub(crate) fn hide_view_devtools(&self, view: &WebView) {
        let Some((r, c)) = self.find_view_coords(view) else { return };
        let rows = self.rows.borrow();
        let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) else { return };

        tab.dt_open.set(false);
        tab.dt_box.set_visible(false);
        tab.dt_handle.set_visible(false);
        while let Some(ch) = tab.dt_box.first_child() {
            tab.dt_box.remove(&ch);
        }
        if let Some(insp) = view.inspector() {
            insp.close();
        }
        view.grab_focus();
    }

    pub(crate) fn show_devtools(self: &Rc<Self>) {
        if let Some(view) = self.focused_view() {
            self.show_view_devtools(&view);
        }
    }

    pub(crate) fn hide_devtools(&self) {
        if let Some(view) = self.focused_view() {
            self.hide_view_devtools(&view);
        }
    }

    pub(crate) fn toggle_devtools(self: &Rc<Self>) {
        let Some(view) = self.focused_view() else { return };
        let Some((r, c)) = self.find_view_coords(&view) else { return };
        let rows = self.rows.borrow();
        let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) else { return };

        if tab.dt_open.get() {
            self.hide_view_devtools(&view);
        } else {
            self.show_view_devtools(&view);
        }
    }

    pub(crate) fn cycle_devtools_dock(self: &Rc<Self>) {
        // Dock cycling preserved for shortcuts; per-tab devtools docks at bottom of each tab.
        self.toggle_devtools();
    }

    pub(crate) fn set_devtools_dock_side(self: &Rc<Self>, side: DevToolsDockSide) {
        self.devtools_dock_side.set(side);
        if let Some(view) = self.focused_view() {
            self.show_view_devtools(&view);
        }
    }

    pub(crate) fn hide_devtools_impl(&self, close_inspector: bool) {
        if let Some(view) = self.focused_view() {
            if close_inspector {
                self.hide_view_devtools(&view);
            } else {
                let Some((r, c)) = self.find_view_coords(&view) else { return };
                let rows = self.rows.borrow();
                let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) else { return };
                tab.dt_open.set(false);
                tab.dt_box.set_visible(false);
                tab.dt_handle.set_visible(false);
                view.grab_focus();
            }
        }
    }

    pub(crate) fn toggle_devtools_undocked(self: &Rc<Self>) {
        self.toggle_devtools();
    }
}
