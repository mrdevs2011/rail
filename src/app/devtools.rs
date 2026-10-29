//! Per-tab docked Web Inspector (Bottom, Left, Right) and floating undocked window.
//! Preserves independent sizing and remembers preferred docking mode.

use crate::*;
use webkit6::WebInspector;

thread_local! {
    static DT_CLOSING_WIN: Cell<bool> = const { Cell::new(false) };
    static IN_SHOW_DEVTOOLS: Cell<bool> = const { Cell::new(false) };
    pub(crate) static LAST_DT_TARGET: Cell<Option<DevToolsDockSide>> = const { Cell::new(None) };
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

    pub(crate) fn show_view_devtools(self: &Rc<Self>, view: &WebView) {
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

        let side = self.devtools_dock_side.get();
        tab.dt_dock.set(side);

        if side == DevToolsDockSide::Undocked {
            tab.dt_box.set_visible(false);
            tab.dt_handle.set_visible(false);
            tab.dt_open.set(true);
            while let Some(ch) = tab.dt_box.first_child() {
                tab.dt_box.remove(&ch);
            }
            insp.show();
            self.show_floating_devtools(view, tab, &insp);
            return;
        }

        // Close any existing undocked floating window before docking into tab
        self.dt_close_window();

        // Reorder widgets in tab_inner according to dock side
        if tab.overlay.parent().is_some() {
            tab.overlay.unparent();
        }
        if tab.dt_handle.parent().is_some() {
            tab.dt_handle.unparent();
        }
        if tab.dt_box.parent().is_some() {
            tab.dt_box.unparent();
        }

        tab.dt_box.remove_css_class("dock-bottom");
        tab.dt_box.remove_css_class("dock-left");
        tab.dt_box.remove_css_class("dock-right");
        tab.dt_handle.remove_css_class("handle-horizontal");
        tab.dt_handle.remove_css_class("handle-vertical");

        match side {
            DevToolsDockSide::Bottom => {
                tab.tab_inner.set_orientation(gtk::Orientation::Vertical);
                tab.tab_inner.append(&tab.overlay);
                tab.tab_inner.append(&tab.dt_handle);
                tab.tab_inner.append(&tab.dt_box);

                tab.dt_box.add_css_class("dock-bottom");
                tab.dt_handle.add_css_class("handle-horizontal");
                tab.dt_handle.set_orientation(gtk::Orientation::Horizontal);
                tab.dt_handle.set_cursor_from_name(Some("ns-resize"));
                tab.dt_handle.set_size_request(-1, 6);

                let mut h = tab.dt_height.get();
                if h < 140 {
                    h = self.default_devtools_height.get().clamp(160, 600);
                    tab.dt_height.set(h);
                }
                tab.dt_box.set_hexpand(true);
                tab.dt_box.set_vexpand(false);
                tab.dt_box.set_width_request(-1);
                tab.dt_box.set_height_request(h);
            }
            DevToolsDockSide::Left => {
                tab.tab_inner.set_orientation(gtk::Orientation::Horizontal);
                tab.tab_inner.append(&tab.dt_box);
                tab.tab_inner.append(&tab.dt_handle);
                tab.tab_inner.append(&tab.overlay);

                tab.dt_box.add_css_class("dock-left");
                tab.dt_handle.add_css_class("handle-vertical");
                tab.dt_handle.set_orientation(gtk::Orientation::Vertical);
                tab.dt_handle.set_cursor_from_name(Some("col-resize"));
                tab.dt_handle.set_size_request(6, -1);

                let mut w = tab.dt_width.get();
                if w < 180 {
                    w = self.default_devtools_width.get().clamp(240, 800);
                    tab.dt_width.set(w);
                }
                tab.dt_box.set_hexpand(false);
                tab.dt_box.set_vexpand(true);
                tab.dt_box.set_height_request(-1);
                tab.dt_box.set_width_request(w);
            }
            DevToolsDockSide::Right => {
                tab.tab_inner.set_orientation(gtk::Orientation::Horizontal);
                tab.tab_inner.append(&tab.overlay);
                tab.tab_inner.append(&tab.dt_handle);
                tab.tab_inner.append(&tab.dt_box);

                tab.dt_box.add_css_class("dock-right");
                tab.dt_handle.add_css_class("handle-vertical");
                tab.dt_handle.set_orientation(gtk::Orientation::Vertical);
                tab.dt_handle.set_cursor_from_name(Some("col-resize"));
                tab.dt_handle.set_size_request(6, -1);

                let mut w = tab.dt_width.get();
                if w < 180 {
                    w = self.default_devtools_width.get().clamp(240, 800);
                    tab.dt_width.set(w);
                }
                tab.dt_box.set_hexpand(false);
                tab.dt_box.set_vexpand(true);
                tab.dt_box.set_height_request(-1);
                tab.dt_box.set_width_request(w);
            }
            _ => {}
        }

        tab.dt_box.set_visible(true);
        tab.dt_handle.set_visible(true);
        tab.dt_open.set(true);

        insp.show();

        let app_wire = self.clone();
        let view_wire = view.clone();
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
                        app_wire.wire_inspector_click_controller(&widget, &view_wire);
                    }
                }
            }
        };

        attach_widget();
        glib::idle_add_local_once(attach_widget);

        let dt_box_poll = tab.dt_box.clone();
        let insp_poll = insp.clone();
        let app_poll = self.clone();
        let view_poll = view.clone();
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
                    app_poll.wire_inspector_click_controller(&widget, &view_poll);
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

    pub(crate) fn show_floating_devtools(self: &Rc<Self>, view: &WebView, _tab: &Tab, insp: &WebInspector) {
        let app = self.clone();
        let v = view.clone();
        let win = if let Some(existing) = self.devtools_window.borrow().clone() {
            existing
        } else {
            let w = gtk::Window::new();
            let page_title = view.title().map(|s| s.to_string()).unwrap_or_default();
            w.set_title(Some(&format!("Developer Tools - {}", if page_title.is_empty() { "Web Inspector" } else { &page_title })));
            w.set_default_size(950, 680);
            w.add_css_class("devtools-window");
            let v_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            v_box.set_hexpand(true);
            v_box.set_vexpand(true);
            w.set_child(Some(&v_box));

            let app_close = app.clone();
            let v_close = v.clone();
            w.connect_close_request(move |_| {
                if !DT_CLOSING_WIN.with(|f| f.get()) {
                    app_close.hide_view_devtools(&v_close);
                }
                glib::Propagation::Proceed
            });

            *self.devtools_window.borrow_mut() = Some(w.clone());
            w
        };

        if let Some(w) = insp.web_view() {
            let widget: gtk::Widget = w.upcast();
            if let Some(v_box) = win.child().and_then(|c| c.downcast::<gtk::Box>().ok()) {
                if widget.parent().as_ref() != Some(v_box.upcast_ref()) {
                    if widget.parent().is_some() {
                        widget.unparent();
                    }
                    while let Some(ch) = v_box.first_child() {
                        v_box.remove(&ch);
                    }
                    widget.set_hexpand(true);
                    widget.set_vexpand(true);
                    v_box.append(&widget);
                    self.wire_inspector_click_controller(&widget, &v);
                }
            }
        }

        win.present();
    }

    pub(crate) fn wire_inspector_click_controller(self: &Rc<Self>, widget: &gtk::Widget, view: &WebView) {
        // Tag widget to prevent duplicate controllers
        let tag = "has_rail_dt_ctrl";
        if unsafe { widget.data::<bool>(tag).is_some() } {
            return;
        }
        unsafe {
            widget.set_data(tag, true);
        }

        let dt_click = gtk::GestureClick::new();
        dt_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let app_c = self.clone();
        let v_c = view.clone();
        dt_click.connect_pressed(move |g, _, x, y| {
            if g.current_button() != gdk::BUTTON_PRIMARY {
                return;
            }
            if y >= 0.0 && y <= 35.0 {
                let cur = app_c.devtools_dock_side.get();
                let slot = (x / 28.0).floor() as i32;
                let target_side: Option<DevToolsDockSide> = match cur {
                    DevToolsDockSide::Bottom => match slot {
                        0 => {
                            app_c.hide_view_devtools(&v_c);
                            None
                        }
                        1 => Some(DevToolsDockSide::Left),
                        2 => Some(DevToolsDockSide::Right),
                        3 => Some(DevToolsDockSide::Undocked),
                        _ => None,
                    },
                    DevToolsDockSide::Left => match slot {
                        0 => {
                            app_c.hide_view_devtools(&v_c);
                            None
                        }
                        1 => Some(DevToolsDockSide::Bottom),
                        2 => Some(DevToolsDockSide::Right),
                        3 => Some(DevToolsDockSide::Undocked),
                        _ => None,
                    },
                    DevToolsDockSide::Right => match slot {
                        0 => {
                            app_c.hide_view_devtools(&v_c);
                            None
                        }
                        1 => Some(DevToolsDockSide::Left),
                        2 => Some(DevToolsDockSide::Bottom),
                        3 => Some(DevToolsDockSide::Undocked),
                        _ => None,
                    },
                    DevToolsDockSide::Undocked => match slot {
                        0 => Some(DevToolsDockSide::Left),
                        1 => Some(DevToolsDockSide::Bottom),
                        2 => Some(DevToolsDockSide::Right),
                        _ => None,
                    },
                };

                if let Some(side) = target_side {
                    LAST_DT_TARGET.with(|c| c.set(Some(side)));
                    let app_change = app_c.clone();
                    glib::idle_add_local_once(move || {
                        app_change.set_devtools_dock_side(side);
                    });
                }
            }
        });
        widget.add_controller(dt_click);
    }

    pub(crate) fn hide_view_devtools(&self, view: &WebView) {
        let Some((r, c)) = self.find_view_coords(view) else { return };
        let rows = self.rows.borrow();
        let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) else { return };

        self.dt_close_window();

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
        if !self.devtools_open() {
            self.show_devtools();
            return;
        }
        let cur = self.devtools_dock_side.get();
        let next = match cur {
            DevToolsDockSide::Bottom => DevToolsDockSide::Right,
            DevToolsDockSide::Right => DevToolsDockSide::Left,
            DevToolsDockSide::Left => DevToolsDockSide::Undocked,
            DevToolsDockSide::Undocked => DevToolsDockSide::Bottom,
        };
        self.set_devtools_dock_side(next);
    }

    pub(crate) fn set_devtools_dock_side(self: &Rc<Self>, side: DevToolsDockSide) {
        let prev = self.devtools_dock_side.get();
        self.devtools_dock_side.set(side);

        // Save persistent dock side (Bottom, Left, Right, Undocked) in settings
        let mut st = load_app_settings();
        let name = devtools_dock_to_str(side);
        if st.devtools_dock != name {
            st.devtools_dock = name.to_string();
            save_app_settings(&st);
        }

        if let Some(view) = self.focused_view() {
            if prev != side || !self.devtools_open() {
                self.show_view_devtools(&view);
            }
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
                self.dt_close_window();
                tab.dt_open.set(false);
                tab.dt_box.set_visible(false);
                tab.dt_handle.set_visible(false);
                view.grab_focus();
            }
        }
    }

    pub(crate) fn toggle_devtools_undocked(self: &Rc<Self>) {
        if self.devtools_dock_side.get() == DevToolsDockSide::Undocked && self.devtools_open() {
            self.set_devtools_dock_side(DevToolsDockSide::Bottom);
        } else {
            self.set_devtools_dock_side(DevToolsDockSide::Undocked);
        }
    }
}
