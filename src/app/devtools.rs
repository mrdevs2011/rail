//! Docked / undocked Web Inspector.

use crate::*;

impl App {
    // ---- DevTools (Alt+Shift+I / F12 / Alt+Shift+D) ----

    pub(crate) fn devtools_open(&self) -> bool {
        self.devtools_open.get()
    }

    pub(crate) fn dt_view(&self) -> Option<WebView> {
        DT_VIEW.with(|v| v.borrow().clone()).or_else(|| self.focused_view())
    }

    /// Close the undocked DevTools window without holding the RefCell borrow (close-request re-enters it).
    pub(crate) fn dt_close_window(&self) {
        let win = self.devtools_window.borrow_mut().take();
        if let Some(win) = win {
            DT_CLOSING_WIN.with(|f| f.set(true));
            win.set_child(None::<&gtk::Widget>);
            win.close();
            DT_CLOSING_WIN.with(|f| f.set(false));
        }
    }

    pub(crate) fn toggle_devtools(self: &Rc<Self>) {
        if self.devtools_open() {
            // F12 on another tab than the one being inspected: move DevTools to this tab instead of closing the wrong inspector.
            let other_tab = match (DT_VIEW.with(|v| v.borrow().clone()), self.focused_view()) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            };
            self.hide_devtools();
            if other_tab {
                self.show_devtools();
            }
        } else {
            self.show_devtools();
        }
    }

    pub(crate) fn cycle_devtools_dock(self: &Rc<Self>) {
        if !self.devtools_open() {
            self.show_devtools();
            return;
        }
        let next = match self.devtools_dock_side.get() {
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
        // Oxirgi dock holatini eslab qolamiz: keyingi safar DevTools shu joyda ochiladi.
        // Undocked saqlanmaydi (alohida oyna butun ekran bo'lib ochilib qoladi), faqat Bottom/Left/Right.
        if prev != side && side != DevToolsDockSide::Undocked {
            let mut st = load_app_settings();
            let name = devtools_dock_to_str(side);
            if st.devtools_dock != name {
                st.devtools_dock = name.to_string();
                save_app_settings(&st);
            }
        }
        let Some(view) = self.dt_view() else { return };
        let Some(insp) = view.inspector() else { return };
        let Some(w) = insp.web_view() else { return };
        let widget: gtk::Widget = w.upcast();

        // Already docked on this side with the inspector widget in place: do not reparent.
        // Reparenting on every WebKit attach signal steals clicks and makes the panel feel "closed".
        if prev == side && self.devtools_open.get() {
            let already_in_panel = matches!(
                side,
                DevToolsDockSide::Bottom | DevToolsDockSide::Left | DevToolsDockSide::Right
            ) && widget
                .parent()
                .map(|p| p == self.devtools_content_box.upcast_ref::<gtk::Widget>().clone())
                .unwrap_or(false);
            let already_undocked = side == DevToolsDockSide::Undocked
                && self.devtools_window.borrow().is_some();
            if already_in_panel || already_undocked {
                return;
            }
        }

        match side {
            DevToolsDockSide::Bottom => {
                self.dt_close_window();
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                if widget.parent().is_some() {
                    widget.unparent();
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);

                self.devtools_panel.remove_css_class("dock-left");
                self.devtools_panel.remove_css_class("dock-right");
                self.devtools_panel.add_css_class("dock-bottom");

                self.devtools_panel.set_orientation(gtk::Orientation::Vertical);
                while let Some(c) = self.devtools_panel.first_child() {
                    self.devtools_panel.remove(&c);
                }
                self.devtools_panel.append(&self.devtools_resize_handle);
                self.devtools_panel.append(&self.devtools_content_box);

                self.devtools_panel.set_halign(gtk::Align::Fill);
                self.devtools_panel.set_valign(gtk::Align::End);
                self.devtools_panel.set_width_request(-1);
                self.devtools_panel.set_height_request(400);
                self.devtools_panel.set_visible(true);
                self.devtools_open.set(true);
            }
            DevToolsDockSide::Left => {
                self.dt_close_window();
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                if widget.parent().is_some() {
                    widget.unparent();
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);

                self.devtools_panel.remove_css_class("dock-bottom");
                self.devtools_panel.remove_css_class("dock-right");
                self.devtools_panel.add_css_class("dock-left");

                self.devtools_panel.set_orientation(gtk::Orientation::Horizontal);
                while let Some(c) = self.devtools_panel.first_child() {
                    self.devtools_panel.remove(&c);
                }
                self.devtools_panel.append(&self.devtools_content_box);
                self.devtools_panel.append(&self.devtools_resize_handle);

                self.devtools_panel.set_halign(gtk::Align::Start);
                self.devtools_panel.set_valign(gtk::Align::Fill);
                self.devtools_panel.set_height_request(-1);
                self.devtools_panel.set_width_request(560);
                self.devtools_panel.set_visible(true);
                self.devtools_open.set(true);
            }
            DevToolsDockSide::Right => {
                self.dt_close_window();
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                if widget.parent().is_some() {
                    widget.unparent();
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);

                self.devtools_panel.remove_css_class("dock-bottom");
                self.devtools_panel.remove_css_class("dock-left");
                self.devtools_panel.add_css_class("dock-right");

                self.devtools_panel.set_orientation(gtk::Orientation::Horizontal);
                while let Some(c) = self.devtools_panel.first_child() {
                    self.devtools_panel.remove(&c);
                }
                self.devtools_panel.append(&self.devtools_resize_handle);
                self.devtools_panel.append(&self.devtools_content_box);

                self.devtools_panel.set_halign(gtk::Align::End);
                self.devtools_panel.set_valign(gtk::Align::Fill);
                self.devtools_panel.set_height_request(-1);
                self.devtools_panel.set_width_request(560);
                self.devtools_panel.set_visible(true);
                self.devtools_open.set(true);
            }
            DevToolsDockSide::Undocked => {
                self.devtools_panel.set_visible(false);
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }

                let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
                let host = host_of(&cur_uri).unwrap_or_else(|| "DevTools".to_string());
                let mut win_opt = self.devtools_window.borrow_mut();
                if let Some(win) = win_opt.as_ref() {
                    if let Some(child) = win.child() {
                        if let Some(box_container) = child.downcast_ref::<gtk::Box>() {
                            if widget.parent().as_ref() != Some(box_container.upcast_ref()) {
                                if widget.parent().is_some() {
                                    widget.unparent();
                                }
                                while let Some(c) = box_container.first_child() {
                                    box_container.remove(&c);
                                }
                                widget.set_vexpand(true);
                                widget.set_hexpand(true);
                                box_container.append(&widget);
                            }
                        }
                    }
                    win.set_title(Some(&trf("Rail DevTools - {}", &[&host])));
                    win.present();
                } else {
                    let win_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    win_box.add_css_class("devtools-window-box");
                    if widget.parent().is_some() {
                        widget.unparent();
                    }
                    widget.set_vexpand(true);
                    widget.set_hexpand(true);
                    win_box.append(&widget);


                    let win = gtk::Window::builder()
                        .title(&trf("Rail DevTools - {}", &[&host]))
                        .default_width(1050)
                        .default_height(720)
                        // no transient_for: niri opens windows that have a parent as floating
                        .child(&win_box)
                        .build();
                    win.add_css_class("devtools-window");

                    let app_win = self.clone();
                    let win_box_ref = win_box.clone();
                    win.connect_close_request(move |w| {
                        if DT_CLOSING_WIN.with(|f| f.get()) {
                            return glib::Propagation::Proceed;
                        }
                        while let Some(c) = win_box_ref.first_child() {
                            win_box_ref.remove(&c);
                        }
                        w.set_child(None::<&gtk::Widget>);
                        app_win.devtools_window.borrow_mut().take();
                        app_win.hide_devtools();
                        glib::Propagation::Proceed
                    });

                    win.present();
                    *win_opt = Some(win);
                }
                self.devtools_open.set(true);
            }
        }
    }

    pub(crate) fn show_devtools(self: &Rc<Self>) {
        let Some(view) = self.focused_view() else { return };
        let Some(insp) = view.inspector() else { return };

        if IN_SHOW_DEVTOOLS.with(|f| f.replace(true)) {
            return;
        }
        DT_VIEW.with(|v| *v.borrow_mut() = Some(view.clone()));

        // Always ensure DevTools docks into the bottom panel (or saved side) by default, never float
        let side = match self.devtools_dock_side.get() {
            DevToolsDockSide::Undocked => DevToolsDockSide::Bottom,
            s => s,
        };
        self.devtools_dock_side.set(side);
        self.set_devtools_dock_side(side);

        insp.show();

        // Update URL and host badge in devtools header if ever needed
        let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_else(|| "about:blank".to_string());
        let host = host_of(&cur_uri).unwrap_or_else(|| "localhost".to_string());
        self.devtools_url_lbl.set_markup(&format!("<span foreground='#fafafa'>{}</span> <span color='#64748b'>•</span> <span color='#94a3b8'>{}</span>", glib::markup_escape_text(&host), glib::markup_escape_text(&cur_uri)));

        IN_SHOW_DEVTOOLS.with(|f| f.set(false));
    }

    pub(crate) fn hide_devtools(&self) {
        self.hide_devtools_impl(true);
    }

    /// `close_inspector=false` is used from the inspector's own `closed` signal: it is already closing, don't re-enter WebKit.
    pub(crate) fn hide_devtools_impl(&self, close_inspector: bool) {
        if !self.devtools_open.get() {
            return;
        }
        self.devtools_open.set(false);

        let dt_view = DT_VIEW.with(|v| v.borrow_mut().take()).or_else(|| self.focused_view());
        if close_inspector {
            if let Some(view) = dt_view {
                if let Some(insp) = view.inspector() {
                    if let Some(w) = insp.web_view() {
                        let widget: gtk::Widget = w.upcast();
                        if widget.parent().is_some() {
                            widget.unparent();
                        }
                    }
                    insp.close();
                }
            }
        }
        self.dt_close_window();
        while let Some(c) = self.devtools_content_box.first_child() {
            self.devtools_content_box.remove(&c);
        }
        self.devtools_panel.set_visible(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    pub(crate) fn toggle_devtools_undocked(self: &Rc<Self>) {
        if self.devtools_dock_side.get() == DevToolsDockSide::Undocked {
            self.set_devtools_dock_side(DevToolsDockSide::Bottom);
        } else {
            self.set_devtools_dock_side(DevToolsDockSide::Undocked);
        }
    }
}
