//! The sidebar tab list.

use crate::*;

impl App {
    // ---- sidebar ---------------------------------------------------------

    pub(crate) fn tab_label(t: &Tab) -> String {
        match t.view.title().filter(|s| !s.is_empty()) {
            Some(s) => s.to_string(),
            None => {
                let u = t.real_uri();
                if u.is_empty() || u.starts_with("about:") {
                    tr("New tab").into()
                } else {
                    host_of(&u).unwrap_or_else(|| u.clone())
                }
            }
        }
    }

    pub(crate) fn tab_icon(t: &Tab, size: i32) -> gtk::Widget {
        if t.view.is_loading() {
            let s = gtk::Spinner::new();
            s.set_size_request(size, size);
            s.start();
            return s.upcast();
        }
        if let Some(f) = t.view.favicon() {
            let img = gtk::Image::from_paintable(Some(&f));
            img.set_pixel_size(size);
            return img.upcast();
        }
        let label = Self::tab_label(t);
        let letter = label.chars().find(|c| c.is_alphanumeric()).unwrap_or('R');
        let seed = label
            .bytes()
            .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
        let badge = gtk::Label::new(Some(&letter.to_uppercase().to_string()));
        badge.add_css_class("fav");
        badge.add_css_class(&format!("fav-{}", seed % 8));
        badge.set_size_request(size, size);
        badge.set_valign(gtk::Align::Center);
        badge.set_halign(gtk::Align::Center);
        badge.upcast()
    }

    /// The UI thread was blocked for `gap` (watchdog in main loop). Append one line to
    /// stalls.log with what was going on, so freezes can be traced instead of guessed.
    pub(crate) fn log_stall(&self, gap: Duration) {
        use std::io::Write;
        let (mut live, mut loading, mut total) = (0usize, 0usize, 0usize);
        let mut loading_hosts: Vec<String> = Vec::new();
        if let Ok(rows) = self.rows.try_borrow() {
            for row in rows.iter() {
                for t in row.tabs.iter() {
                    total += 1;
                    if !t.suspended.get() {
                        live += 1;
                    }
                    if t.view.is_loading() {
                        loading += 1;
                        if loading_hosts.len() < 4 {
                            loading_hosts.push(host_of(&t.real_uri()).unwrap_or_default());
                        }
                    }
                }
            }
        }
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = rail_data_dir().join("stalls.log");
        if std::fs::metadata(&path)
            .map(|m| m.len() > 256 * 1024)
            .unwrap_or(false)
        {
            let _ = std::fs::write(&path, "");
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(
                f,
                "{} stall={}ms tabs={} live={} loading={} overview={} [{}]",
                secs,
                gap.as_millis().saturating_sub(100),
                total,
                live,
                loading,
                self.ov.get(),
                loading_hosts.join(", ")
            );
        }
    }

    /// WebKit notifications (title / favicon / uri / loading) fire in bursts, especially while many
    /// tabs load. Rebuilding the whole sidebar for each one froze the UI, so they are coalesced
    /// into at most one rebuild per ~60 ms. User actions still call refresh_sidebar() directly.
    /// Cheap sidebar update: only moves the active-workspace / active-tab highlight to the
    /// current focus (used while navigating in overview, where a full rebuild would be slow).
    pub(crate) fn sync_sidebar_active(&self) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        SIDEBAR_ITEMS.with(|s| {
            let s = s.borrow();
            for (ri, b) in &s.0 {
                if *ri == cur_r {
                    b.add_css_class("active-workspace");
                } else {
                    b.remove_css_class("active-workspace");
                }
            }
            for (ri, ci, b) in &s.1 {
                if *ri == cur_r && *ci == cur_c {
                    b.add_css_class("active-tab");
                } else {
                    b.remove_css_class("active-tab");
                }
            }
        });
    }

    pub(crate) fn refresh_sidebar_soon(self: &Rc<Self>) {
        thread_local! { static SIDEBAR_PENDING: Cell<bool> = Cell::new(false); }
        if SIDEBAR_PENDING.with(|p| p.replace(true)) {
            return;
        }
        let app = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(60), move || {
            SIDEBAR_PENDING.with(|p| p.set(false));
            app.refresh_sidebar();
        });
    }

    pub(crate) fn refresh_sidebar(self: &Rc<Self>) {
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        let rows = self.rows.borrow();
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let tab_items_registry: Rc<RefCell<Vec<(usize, usize, gtk::Box)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let mut ws_headers: Vec<(usize, gtk::Box)> = Vec::new();

        for (ri, row) in rows.iter().enumerate() {
            // Workspace Header
            let ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            ws_box.add_css_class("workspace-header");
            ws_box.set_overflow(gtk::Overflow::Hidden);
            if ri == cur_r {
                ws_box.add_css_class("active-workspace");
            }
            ws_headers.push((ri, ws_box.clone()));

            let ws_title = gtk::Label::new(Some(&trf("Workspace {}", &[&(ri + 1).to_string()])));
            ws_title.add_css_class("workspace-title");
            ws_title.set_xalign(0.0);
            ws_title.set_hexpand(true);
            ws_title.set_width_chars(1);
            ws_title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            ws_box.append(&ws_title);

            let count_text = if row.tabs.len() == 1 {
                tr("1 tab").to_string()
            } else {
                trf("{} tabs", &[&row.tabs.len().to_string()])
            };
            let count_lbl = gtk::Label::new(Some(&count_text));
            count_lbl.add_css_class("workspace-tab-count");
            count_lbl.set_width_chars(1);
            count_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            ws_box.append(&count_lbl);

            let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
            add_btn.add_css_class("btn-ws-add");
            add_btn.set_tooltip_text(Some(&trf(
                "Add tab to Workspace {}",
                &[&(ri + 1).to_string()],
            )));
            let app = self.clone();
            add_btn.connect_clicked(move |_| {
                app.focus_row(ri);
                app.open_new_tab_palette();
            });
            ws_box.append(&add_btn);

            let app = self.clone();
            let ws_click = gtk::GestureClick::new();
            ws_click.connect_released(move |_, _, _, _| {
                app.focus_row(ri);
            });
            ws_box.add_controller(ws_click);

            self.list.append(&ws_box);

            // Tabs inside this workspace
            for (ci, t) in row.tabs.iter().enumerate() {
                let item = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                item.add_css_class("tabrow");
                item.set_overflow(gtk::Overflow::Hidden);
                if ri == cur_r && ci == cur_c {
                    item.add_css_class("active-tab");
                }
                tab_items_registry.borrow_mut().push((ri, ci, item.clone()));

                let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                content_box.set_hexpand(true);
                content_box.set_overflow(gtk::Overflow::Hidden);
                content_box.append(&Self::tab_icon(t, 16));

                let mgr = self.profiles.borrow();
                if mgr.profiles().len() > 1 {
                    if let Some(p) = mgr.get_profile(&t.profile_id) {
                        let p_dot = gtk::Label::new(None);
                        let fg = p.text_color();
                        p_dot.set_markup(&format!(
                            "<span foreground='{}' font_weight='800'>{}</span>",
                            fg,
                            glib::markup_escape_text(&p.initial_letter())
                        ));
                        p_dot.add_css_class("profile-badge-mini");
                        p_dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
                        p_dot.set_valign(gtk::Align::Center);
                        p_dot.set_halign(gtk::Align::Center);
                        p_dot.set_width_request(18);
                        p_dot.set_height_request(16);
                        p_dot.set_tooltip_text(Some(&trf("Profile: {}", &[&p.display_name()])));
                        content_box.append(&p_dot);
                    }
                }
                drop(mgr);

                let index_lbl = gtk::Label::new(Some(&format!("{}", ci + 1)));
                index_lbl.add_css_class("tab-index");
                index_lbl.set_valign(gtk::Align::Center);
                content_box.append(&index_lbl);

                let title_lbl = gtk::Label::new(Some(&Self::tab_label(t)));
                title_lbl.add_css_class("tab-title");
                title_lbl.set_xalign(0.0);
                title_lbl.set_hexpand(true);
                title_lbl.set_width_chars(1);
                title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                content_box.append(&title_lbl);

                // Ovoz tugmasi: shu tab uchun ovozni o'chiradi / yoqadi (toggle).
                // Holat bosilgan paytdagi jonli `is_muted()` dan olinadi — eskirgan closure holatiga emas.
                let muted_now = t.view.is_muted();
                if muted_now || t.view.is_playing_audio() {
                    let audio_btn = gtk::Button::from_icon_name(if muted_now {
                        "audio-volume-muted-symbolic"
                    } else {
                        "audio-volume-high-symbolic"
                    });
                    audio_btn.add_css_class("flat-icon");
                    audio_btn.set_tooltip_text(Some(tr(if muted_now {
                        "Muted (click to unmute)"
                    } else {
                        "Playing audio (click to mute)"
                    })));
                    let v_audio = t.view.clone();
                    audio_btn.connect_clicked(move |b| {
                        let mute = !v_audio.is_muted();
                        v_audio.set_is_muted(mute);
                        b.set_icon_name(if mute {
                            "audio-volume-muted-symbolic"
                        } else {
                            "audio-volume-high-symbolic"
                        });
                        b.set_tooltip_text(Some(tr(if mute {
                            "Muted (click to unmute)"
                        } else {
                            "Playing audio (click to mute)"
                        })));
                    });
                    content_box.append(&audio_btn);
                }

                let app = self.clone();
                let drag = gtk::GestureDrag::new();
                drag.set_button(gdk::BUTTON_PRIMARY);

                let item_drag = item.clone();
                let drag_dist = Rc::new(Cell::new(0.0f64));

                drag.connect_drag_begin({
                    let item_drag = item_drag.clone();
                    let drag_dist = drag_dist.clone();
                    move |_gesture, _start_x, _start_y| {
                        drag_dist.set(0.0);
                        item_drag.add_css_class("dragging");
                        item_drag.set_cursor_from_name(Some("grabbing"));
                    }
                });

                let tab_reg_update = tab_items_registry.clone();
                let app_update = self.clone();
                drag.connect_drag_update({
                    let drag_dist = drag_dist.clone();
                    let tab_reg_update = tab_reg_update.clone();
                    let app_update = app_update.clone();
                    move |_gesture, offset_x, offset_y| {
                        let dist = offset_x.hypot(offset_y);
                        drag_dist.set(dist);

                        if dist < 4.0 {
                            let items = tab_reg_update.borrow();
                            for &(_, _, ref b) in items.iter() {
                                b.remove_css_class("drop-target-above");
                                b.remove_css_class("drop-target-below");
                            }
                            return;
                        }

                        let tab_h = 32.0f64;
                        let delta = (offset_y / tab_h).round() as isize;
                        let rows_borrow = app_update.rows.borrow();
                        let cur_tabs_len = rows_borrow.get(ri).map_or(0, |r| r.tabs.len());
                        let total_rows = rows_borrow.len();

                        let target = if delta == 0 {
                            None
                        } else {
                            let new_col_idx = (ci as isize) + delta;
                            if new_col_idx >= 0 && (new_col_idx as usize) < cur_tabs_len {
                                Some((ri, new_col_idx as usize))
                            } else if new_col_idx < 0 && ri > 0 {
                                let tr = ri - 1;
                                let target_len = rows_borrow.get(tr).map_or(0, |r| r.tabs.len());
                                Some((tr, target_len.saturating_sub(1)))
                            } else if new_col_idx >= cur_tabs_len as isize && ri + 1 < total_rows {
                                Some((ri + 1, 0))
                            } else {
                                Some((
                                    ri,
                                    if delta > 0 {
                                        cur_tabs_len.saturating_sub(1)
                                    } else {
                                        0
                                    },
                                ))
                            }
                        };
                        drop(rows_borrow);

                        let items = tab_reg_update.borrow();
                        for &(r, c, ref b) in items.iter() {
                            if let Some((tr, tc)) = target {
                                if (r, c) == (tr, tc) && (tr, tc) != (ri, ci) {
                                    if delta < 0 {
                                        b.add_css_class("drop-target-above");
                                        b.remove_css_class("drop-target-below");
                                    } else {
                                        b.add_css_class("drop-target-below");
                                        b.remove_css_class("drop-target-above");
                                    }
                                } else {
                                    b.remove_css_class("drop-target-above");
                                    b.remove_css_class("drop-target-below");
                                }
                            } else {
                                b.remove_css_class("drop-target-above");
                                b.remove_css_class("drop-target-below");
                            }
                        }
                    }
                });

                let item_end = item.clone();
                let app_end = app.clone();
                let drag_dist_end = drag_dist.clone();
                let tab_reg_end = tab_items_registry.clone();

                drag.connect_drag_end(move |_gesture, _offset_x, offset_y| {
                    item_end.remove_css_class("dragging");
                    item_end.set_cursor_from_name(Some("grab"));

                    let items = tab_reg_end.borrow();
                    for &(_, _, ref b) in items.iter() {
                        b.remove_css_class("drop-target-above");
                        b.remove_css_class("drop-target-below");
                    }
                    drop(items);

                    let d = drag_dist_end.get();
                    if d < 6.0 {
                        app_end.focus_coords(ri, ci);
                        return;
                    }

                    let tab_h = 32.0f64;
                    let delta = (offset_y / tab_h).round() as isize;
                    let rows_borrow = app_end.rows.borrow();
                    let cur_tabs_len = rows_borrow.get(ri).map_or(0, |r| r.tabs.len());
                    let total_rows = rows_borrow.len();
                    drop(rows_borrow);

                    if delta != 0 {
                        let new_col_idx = (ci as isize) + delta;
                        if new_col_idx >= 0 && (new_col_idx as usize) < cur_tabs_len {
                            app_end.reorder_tab_in_row(ri, ci, new_col_idx as usize);
                        } else if new_col_idx < 0 && ri > 0 {
                            let target_r = ri - 1;
                            let rows_borrow = app_end.rows.borrow();
                            let target_len = rows_borrow.get(target_r).map_or(0, |r| r.tabs.len());
                            drop(rows_borrow);
                            app_end.move_tab_to_row_pos(ri, ci, target_r, target_len);
                        } else if new_col_idx >= cur_tabs_len as isize && ri + 1 < total_rows {
                            let target_r = ri + 1;
                            app_end.move_tab_to_row_pos(ri, ci, target_r, 0);
                        }
                    }
                });

                content_box.add_controller(drag);

                let close = gtk::Button::from_icon_name("window-close-symbolic");
                close.add_css_class("tab-close");
                close.set_valign(gtk::Align::Center);
                close.set_cursor_from_name(Some("pointer"));
                close.set_hexpand(false);
                close.set_halign(gtk::Align::End);
                let (app, v) = (self.clone(), t.view.clone());
                close.connect_clicked(move |_| {
                    app.close_view(&v);
                });

                item.append(&content_box);
                item.append(&close);

                self.list.append(&item);
            }
        }
        drop(rows);

        // Bottom "+ New Workspace" button
        let new_ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        new_ws_box.add_css_class("btn-new-workspace");
        new_ws_box.set_halign(gtk::Align::Fill);
        new_ws_box.set_overflow(gtk::Overflow::Hidden);
        let new_ws_lbl = gtk::Label::new(None);
        sc_bind_text(
            &new_ws_lbl,
            "+ New Workspace (Alt+Shift+V)",
            &["new_workspace"],
        );
        new_ws_lbl.set_xalign(0.0);
        new_ws_lbl.set_hexpand(true);
        new_ws_lbl.set_width_chars(1);
        new_ws_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        new_ws_box.append(&new_ws_lbl);
        let app = self.clone();
        let new_ws_click = gtk::GestureClick::new();
        new_ws_click.connect_released(move |_, _, _, _| {
            app.split_vertical();
        });
        new_ws_box.add_controller(new_ws_click);
        self.list.append(&new_ws_box);

        SIDEBAR_ITEMS.with(|s| *s.borrow_mut() = (ws_headers, tab_items_registry.borrow().clone()));
        let disp_uri = self.focused_display_uri();
        self.pill_label.set_text(&disp_uri);
        let is_web = disp_uri.starts_with("http://") || disp_uri.starts_with("https://");
        self.pill_pwa_btn.set_visible(is_web);
    }

    pub(crate) fn prompt_install_current_pwa(self: &Rc<Self>) {
        if let Some(view) = self.focused_view() {
            let u = view.uri().map(|s| s.to_string()).unwrap_or_default();
            let t = view
                .title()
                .map(|s| s.to_string())
                .unwrap_or_else(|| u.clone());
            if !u.is_empty() && (u.starts_with("http://") || u.starts_with("https://")) {
                let info = crate::pwa::PwaAppInfo {
                    name: t,
                    url: u,
                    icon_url: None,
                    theme_color: None,
                };
                crate::pwa::prompt_install_pwa_dialog(self, &info);
            }
        }
    }

    pub(crate) fn toggle_sidebar(self: &Rc<Self>) {
        // Alt+B and the header button do exactly the same thing: cycle fixed ↔ auto.
        self.cycle_sidebar_mode();
    }

    /// Alt+B / header button: cycle fixed ↔ auto. Auto starts closed; fixed starts open.

    /// Rail logo is always visible: fixed mode and the hover-opened (auto) sidebar.
    pub(crate) fn sync_brand_logo(&self) {
        self.brand_logo.set_reveal_child(true);
    }
    pub(crate) fn cycle_sidebar_mode(self: &Rc<Self>) {
        if let Some(source) = self.side_open_timer.borrow_mut().take() {
            source.remove();
        }
        if let Some(source) = self.side_close_timer.borrow_mut().take() {
            source.remove();
        }
        let auto = !self.sidebar_auto.get();
        self.sidebar_auto.set(auto);
        self.shell_layout.set_overlay(auto);
        let mut s = load_app_settings();
        s.sidebar_mode = if auto { "auto".into() } else { "fixed".into() };
        save_app_settings(&s);
        if auto {
            // auto: hide until intentional edge hover
            self.set_sidebar(false);
        } else {
            self.set_sidebar(true);
        }
        self.sync_brand_logo();
    }

    pub(crate) fn on_shell_pointer_x(self: &Rc<Self>, x: f64) {
        if !self.sidebar_auto.get() {
            return;
        }
        if self.sidebar_locked.get() {
            if let Some(source) = self.side_close_timer.borrow_mut().take() {
                source.remove();
            }
            return;
        }

        // Cancel pending close if pointer is back inside or near sidebar
        if x <= (SIDE_W as f64) + 16.0 {
            if let Some(source) = self.side_close_timer.borrow_mut().take() {
                source.remove();
            }
        }

        if !self.side_open.get() {
            // Sidebar is closed.
            // Only initiate opening if cursor is at the extreme left edge (x <= 5.0).
            // A short dwell (160ms) ensures casual sweeps across the edge do not flap open.
            if x <= 5.0 {
                if self.side_open_timer.borrow().is_none() {
                    let app = self.clone();
                    let source =
                        glib::timeout_add_local_once(Duration::from_millis(160), move || {
                            app.side_open_timer.borrow_mut().take();
                            if app.sidebar_auto.get() && !app.side_open.get() {
                                app.set_sidebar(true);
                            }
                        });
                    *self.side_open_timer.borrow_mut() = Some(source);
                }
            } else {
                // Moved away from the extreme edge before dwell completed: cancel immediately
                if let Some(source) = self.side_open_timer.borrow_mut().take() {
                    source.remove();
                }
            }
        } else {
            // Sidebar is open.
            // If pointer moves past sidebar width plus grace margin, start debounce close timer.
            if x > (SIDE_W as f64) + 24.0 {
                if self.side_close_timer.borrow().is_none() {
                    let app = self.clone();
                    let source =
                        glib::timeout_add_local_once(Duration::from_millis(200), move || {
                            app.side_close_timer.borrow_mut().take();
                            if app.sidebar_auto.get() && app.side_open.get() {
                                app.set_sidebar(false);
                            }
                        });
                    *self.side_close_timer.borrow_mut() = Some(source);
                }
            }
        }
    }

    pub(crate) fn on_shell_pointer_leave(self: &Rc<Self>, last_x: f64) {
        if !self.sidebar_auto.get() {
            return;
        }
        if self.sidebar_locked.get() {
            if let Some(source) = self.side_close_timer.borrow_mut().take() {
                source.remove();
            }
            return;
        }
        // Cancel pending open dwell if cursor leaves the window
        if let Some(source) = self.side_open_timer.borrow_mut().take() {
            source.remove();
        }

        if !self.side_open.get() {
            return;
        }

        // Pointer left the window: give a short grace period (160ms) before closing
        // so quick flicks to window borders don't abruptly snap the sidebar away.
        if last_x > 4.0 {
            if self.side_close_timer.borrow().is_none() {
                let app = self.clone();
                let source = glib::timeout_add_local_once(Duration::from_millis(160), move || {
                    app.side_close_timer.borrow_mut().take();
                    if app.sidebar_auto.get() && app.side_open.get() {
                        app.set_sidebar(false);
                    }
                });
                *self.side_close_timer.borrow_mut() = Some(source);
            }
        }
    }

    /// Open rail://downloads in a **new tab** (target=_blank style).
    pub(crate) fn open_downloads(self: &Rc<Self>) {
        self.exit_overview();
        self.hide_settings_modal();
        self.hide_palette();
        self.open_internal_new_tab("rail://downloads");
    }

    /// Always open an internal page in a new tab and show the real rail:// URL.
    pub(crate) fn open_internal_new_tab(self: &Rc<Self>, uri: &str) {
        let uri = normalize_internal_uri(uri);
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        load_internal_page(&view, &uri);
        self.add_view(view.clone(), prof_id, false);
        self.set_view_saved_uri(&view, &uri);
        self.pill_label.set_text(&uri);
        if uri.starts_with("rail://settings") {
            // UI for settings is shown by open_settings_tab / focus sync
        } else {
            self.hide_settings_modal();
        }
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    /// Adapt the dock to Niri's narrow default half-width columns without hiding the page.
    /// Below 600 logical pixels the sidebar floats over the page; when closed, a small
    /// launcher remains available instead of leaving the WebView with zero width.
    pub(crate) fn sync_responsive_sidebar(&self) {
        let narrow = self.win.width() > 0 && self.win.width() < 600;
        self.responsive_layout.set(narrow);

        if narrow {
            self.shell_layout.set_overlay(true);
            // A resize can happen while the dock's close animation is in flight.
            // Force one deterministic closed state so the launcher is never stranded.
            self.side.set_visible(false);
            self.shell_layout.set_side(0.0);
            self.responsive_sidebar_toggle.set_visible(true);
        } else {
            self.shell_layout.set_overlay(self.sidebar_auto.get());
            let open = self.side_open.get();
            self.side.set_visible(open);
            self.shell_layout
                .set_side(if open { SIDE_W as f64 } else { 0.0 });
            self.responsive_sidebar_toggle.set_visible(false);
        }
        self.shell.queue_allocate();
    }

    pub(crate) fn set_sidebar(self: &Rc<Self>, open: bool) {
        if open {
            if let Some(source) = self.side_close_timer.borrow_mut().take() {
                source.remove();
            }
        } else {
            if let Some(source) = self.side_open_timer.borrow_mut().take() {
                source.remove();
            }
        }
        self.side_open.set(open);
        if self.responsive_layout.get() {
            self.responsive_sidebar_toggle.set_visible(!open);
        }
        self.schedule_save_session();
        if self.win.is_fullscreen() {
            // fullscreen'da faqat kontent: tanlov saqlanadi, fullscreen'dan chiqqanda qo'llanadi
            return;
        }
        let epoch = self.side_gen.get().wrapping_add(1);
        self.side_gen.set(epoch);
        if open {
            self.side.set_visible(true);
        }
        let start = self.shell_layout.side();
        let end = if open { SIDE_W as f64 } else { 0.0 };
        let app = self.clone();
        let t0 = Cell::new(None::<i64>);
        self.shell.add_tick_callback(move |shell, clock| {
            if app.side_gen.get() != epoch {
                return glib::ControlFlow::Break;
            }
            let now = clock.frame_time();
            let begin = t0.get().unwrap_or_else(|| {
                t0.set(Some(now));
                now
            });
            let p = ((now - begin) as f64 / 200_000.0).min(1.0);
            let e = 1.0 - (1.0 - p).powi(3);
            app.shell_layout.set_side(start + (end - start) * e);
            shell.queue_allocate();
            if p >= 1.0 {
                if !open {
                    app.side.set_visible(false);
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }
}
