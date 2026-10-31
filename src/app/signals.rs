//! Per-view and per-tab signal wiring.

use crate::*;

thread_local! {
    /// Tracks WebViews where the website called `preventDefault()` on `contextmenu` (or has a custom menu).
    static LAST_CONTEXT_PREVENTED: std::cell::RefCell<std::collections::HashMap<usize, std::time::Instant>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

impl App {
    pub(crate) fn setup_view_signals(self: &Rc<Self>, view: &WebView) {
        let app = self.clone();
        let v = view.downgrade();
        let click = gtk::GestureClick::new();
        click.set_button(0); // Listen to any mouse click
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            if app.sidebar_auto.get() && app.side_open.get() {
                app.set_sidebar(false);
            }
            if !app.ov.get() {
                let Some(v) = v.upgrade() else { return };
                if let Some((r, c)) = app.find_view_coords(&v)
                    && (app.focus_row.get() != r || app.focus_col.get() != c)
                {
                    app.focus_coords(r, c);
                }
            }
        });
        view.add_controller(click);

        // WebKit's hit-test signal provides the real destination of the hovered anchor,
        // including links whose visible text is different from their href.
        let app_hover = self.clone();
        let hovered_view = view.downgrade();
        view.connect_mouse_target_changed(move |_, hit, _| {
            let Some(v) = hovered_view.upgrade() else {
                return;
            };
            if app_hover.focused_view().as_ref() != Some(&v) {
                app_hover.hovered_link_label.set_visible(false);
                return;
            }
            if hit.context_is_link()
                && let Some(uri) = hit.link_uri().filter(|u| !u.is_empty())
            {
                app_hover.hovered_link_label.set_text(uri.as_str());
                app_hover
                    .hovered_link_label
                    .set_tooltip_text(Some(uri.as_str()));
                app_hover.hovered_link_label.set_visible(true);
                return;
            }
            app_hover.hovered_link_label.set_text("");
            app_hover.hovered_link_label.set_tooltip_text(None);
            app_hover.hovered_link_label.set_visible(false);
        });

        // Leave WebKitGTK's native pinch gesture and zoom-level handling enabled.
        // WebKit arbitrates native page zoom against page-level touch/pointer handlers;
        // Rail must not remove the gesture or force the zoom level back to a stale value.

        // Track developer tools window state & auto-dock to per-tab container
        if let Some(insp) = view.inspector() {
            let app_insp = self.clone();
            let view_weak = view.downgrade();
            let insp_weak = insp.downgrade();
            insp.connect_open_window(move |_| {
                if let Some(v) = view_weak.upgrade() {
                    let side = app_insp.devtools_dock_side.get();
                    if side == DevToolsDockSide::Undocked {
                        if let Some(i) = insp_weak.upgrade()
                            && let Some((r, c)) = app_insp.find_view_coords(&v)
                        {
                            let rows = app_insp.rows.borrow();
                            if let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) {
                                app_insp.show_floating_devtools(&v, tab, &i);
                                return true;
                            }
                        }
                    } else {
                        app_insp.show_view_devtools(&v);
                        return true;
                    }
                }
                false
            });

            let app_detach = self.clone();
            let v_detach = view.downgrade();
            insp.connect_detach(move |_| {
                if let Some(_v) = v_detach.upgrade() {
                    let target = LAST_DT_TARGET
                        .with(|c| c.take())
                        .unwrap_or(DevToolsDockSide::Undocked);
                    app_detach.set_devtools_dock_side(target);
                }
                true
            });

            let app_attach = self.clone();
            let v_attach = view.downgrade();
            insp.connect_attach(move |_| {
                if let Some(_v) = v_attach.upgrade() {
                    let target = LAST_DT_TARGET.with(|c| c.take()).unwrap_or_else(|| {
                        let cur = app_attach.devtools_dock_side.get();
                        if cur == DevToolsDockSide::Undocked {
                            DevToolsDockSide::Bottom
                        } else {
                            cur
                        }
                    });
                    app_attach.set_devtools_dock_side(target);
                }
                true
            });

            let a_closed = self.clone();
            let v_closed = view.downgrade();
            insp.connect_closed(move |_| {
                if let Some(v) = v_closed.upgrade() {
                    a_closed.hide_view_devtools(&v);
                }
            });

            let app_front = self.clone();
            let v_front = view.downgrade();
            insp.connect_bring_to_front(move |_| {
                if let Some(v) = v_front.upgrade() {
                    if app_front.devtools_dock_side.get() == DevToolsDockSide::Undocked
                        && let Some(win) = app_front.devtools_window.borrow().as_ref()
                    {
                        win.present();
                        return true;
                    }
                    if let Some((r, c)) = app_front.find_view_coords(&v) {
                        app_front.focus_coords(r, c);
                    }
                }
                true
            });
        }

        // JavaScript dialogs (alert, confirm, prompt, beforeunload)
        let app = self.clone();
        view.connect_script_dialog(move |v, dialog| {
            app.show_script_dialog(v, dialog);
            true
        });

        // HTML <input type="color"> color chooser dialog
        let app = self.clone();
        view.connect_run_color_chooser(move |_v, request| {
            let dialog = gtk::ColorDialog::new();
            dialog.set_modal(true);
            dialog.set_title(tr("Select Color"));
            dialog.set_with_alpha(false);
            let initial = request.rgba();
            let req = request.clone();
            let cancellable = gio::Cancellable::new();
            let c_clone = cancellable.clone();
            request.connect_finished(move |_| {
                c_clone.cancel();
            });
            dialog.choose_rgba(
                Some(&app.win),
                Some(&initial),
                Some(&cancellable),
                move |res| {
                    if let Ok(rgba) = res {
                        req.set_rgba(&rgba);
                        req.finish();
                    } else {
                        req.cancel();
                    }
                },
            );
            true
        });

        // Developer context menu (Right-click & 2-finger touchpad tap)
        let app = self.clone();
        view.connect_context_menu(move |v, menu, hit| {
            // Railning o'z sahifalarida (settings, start, downloads va boshqa rail://) kontekst menyu ochilmasin
            let uri = v.uri().map(|s| s.to_string()).unwrap_or_default();
            if uri.starts_with("rail://") {
                return true;
            }

            // Shift+RightClick forces Rail's browser menu (Chrome / Firefox parity for Inspect / View Source)
            let is_shift = gdk::Display::default()
                .and_then(|d| d.default_seat())
                .and_then(|s| s.keyboard())
                .map(|k| k.modifier_state().contains(gdk::ModifierType::SHIFT_MASK))
                .unwrap_or(false);

            let ptr = v.as_ptr() as usize;
            let recently_prevented = LAST_CONTEXT_PREVENTED.with(|m| {
                m.borrow_mut()
                    .remove(&ptr)
                    .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(400))
            });

            if recently_prevented && !is_shift {
                // The website provided its own custom context menu or called preventDefault().
                // Suppress Rail's menu completely so the website's menu displays cleanly without overlap.
                return true;
            }

            if App::is_page_level_hit(hit) {
                app.show_page_menu(v);
                return true;
            }
            app.customize_context_menu(v, menu, hit);
            false
        });

        let app = self.clone();
        let v = view.downgrade();
        let focus = gtk::EventControllerFocus::new();
        focus.connect_enter(move |_| {
            if !app.ov.get() {
                let Some(v) = v.upgrade() else { return };
                if let Some((r, c)) = app.find_view_coords(&v)
                    && (app.focus_row.get() != r || app.focus_col.get() != c)
                {
                    app.focus_coords(r, c);
                }
            }
        });
        view.add_controller(focus);

        let app = self.clone();
        view.connect_title_notify(move |v| {
            app.refresh_sidebar_soon();
            app.update_window_title();
            if let Some(u) = v.uri() {
                let title = v.title().unwrap_or_default().to_string();
                app.history.borrow_mut().record_visit(&u, &title);
            }
        });
        let app = self.clone();
        view.connect_uri_notify(move |v| {
            app.remember_uri(v);
            app.refresh_sidebar_soon();
            app.update_window_title();
            app.schedule_save_session();
            if let Some(u) = v.uri() {
                let us = u.as_str();
                // Settings chrome follows the tab. Ignore about:blank mid load_html
                // (otherwise the panel vanishes the moment settings opens).
                if app.settings_modal_open()
                    && !us.starts_with("rail://settings")
                    && !us.is_empty()
                    && !us.starts_with("about:")
                {
                    app.settings_modal.set_visible(false);
                }
                let title = v.title().unwrap_or_default().to_string();
                app.history.borrow_mut().record_visit(&u, &title);
            }
        });
        let app = self.clone();
        view.connect_favicon_notify(move |_| app.refresh_sidebar_soon());
        let app = self.clone();
        view.connect_is_loading_notify(move |v| {
            app.refresh_sidebar_soon();
            app.update_load_bar();
            // Freeze faqat yuklash tugagach ochiladi (uxlagan tab uyg'onganda)
            if !v.is_loading() {
                app.clear_tab_freeze(v);
                // Mark as no longer suspended once content is back
                if let Ok(rows) = app.rows.try_borrow() {
                    for row in rows.iter() {
                        for tab in &row.tabs {
                            if &tab.view == v {
                                tab.suspended.set(false);
                            }
                        }
                    }
                }
                // Welcome page: always re-inject omnibar state after load (avoids empty bar race)
                let mut u = v.uri().map(|s| s.to_string()).unwrap_or_default();
                if u.is_empty() || u.starts_with("about:") {
                    u = app.saved_uri_of(v);
                }
                if u.starts_with("rail://start") {
                    app.push_start_state(v);
                    if app.focused_view().as_ref() == Some(v) {
                        app.focus_welcome_input(false);
                    }
                }
            }
        });
        // Site permission prompts (microphone, camera, geolocation, notifications)
        let app = self.clone();
        view.connect_permission_request(move |v, req| app.handle_permission_request(v, req));
        let app = self.clone();
        view.connect_estimated_load_progress_notify(move |_| app.update_load_bar());
        view.connect_load_failed(move |v, _event, uri, error| {
            let err_msg = error.message().to_string();
            if err_msg.contains("Load request cancelled") {
                return false;
            }
            let html = crate::web::format_error_html(uri, &err_msg);
            v.load_alternate_html(&html, uri, None);
            true
        });
        let app_fs = self.clone();
        view.connect_enter_fullscreen(move |v| {
            // o'sha tab butun oynani egallaydi, boshqa tablar (split) yashirinadi
            app_fs.enter_solo_fullscreen(v);
            app_fs.win.set_fullscreened(true);
            false
        });
        let app_unfs = self.clone();
        view.connect_leave_fullscreen(move |_| {
            app_unfs.leave_solo_fullscreen(false);
            app_unfs.win.set_fullscreened(false);
            false
        });
        let app_audio = self.clone();
        view.connect_is_playing_audio_notify(move |_| {
            app_audio.refresh_sidebar_soon();
        });
        let app_muted = self.clone();
        view.connect_is_muted_notify(move |_| {
            app_muted.refresh_sidebar_soon();
        });
        view.connect_load_failed_with_tls_errors(move |v, failing_uri, cert, _errors| {
            if let Some(session) = v.network_session()
                && let Ok(uri_obj) = glib::Uri::parse(failing_uri, glib::UriFlags::NONE)
                && let Some(host) = uri_obj.host()
            {
                let h = host.as_str();
                if h == "localhost" || h == "127.0.0.1" || h.ends_with(".local") {
                    session.allow_tls_certificate_for_host(cert, &host);
                    v.load_uri(failing_uri);
                    return true;
                }
            }
            let html = crate::web::format_error_html(
                failing_uri,
                "net::ERR_CERT_AUTHORITY_INVALID: SSL / TLS certificate validation failed.",
            );
            v.load_alternate_html(&html, failing_uri, None);
            true
        });
        let app = self.clone();
        view.connect_create(move |v, nav| {
            let parent_prof = app
                .find_view_profile(v)
                .unwrap_or_else(|| app.profiles.borrow().active_profile().id);
            let nv = App::create_related_web_view(v);
            if let Some(req) = nav.request()
                && let Some(uri) = req.uri()
            {
                let u = uri.to_string();
                if !u.is_empty() && u != "about:blank" {
                    app.set_view_saved_uri(&nv, &u);
                }
            }
            app.add_view(nv.clone(), parent_prof, false);
            app.mark_keep_alive(v);
            app.mark_keep_alive(&nv);
            Some(nv.upcast())
        });
        let app = self.clone();
        view.connect_close(move |v| app.close_view(v));
        let app_term = self.clone();
        view.connect_web_process_terminated(move |v, _reason| {
            // Background tabs are intentionally suspended (terminate) to free RAM.
            // Only the focused tab auto-reloads — rate-limited so OOM thrash cannot loop.
            if app_term.focused_view().as_ref() != Some(v) {
                return;
            }
            thread_local! {
                static LAST: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
            }
            let now = std::time::Instant::now();
            let too_soon = LAST.with(|c| match c.get() {
                Some(t) if now.duration_since(t) < Duration::from_secs(4) => true,
                _ => {
                    c.set(Some(now));
                    false
                }
            });
            if too_soon {
                return;
            }
            let saved = app_term.saved_uri_of(v);
            if saved.is_empty() || saved.starts_with("rail://start") {
                v.load_html(&start_page(), Some("rail://start/"));
            } else if saved.starts_with("rail://settings") {
                v.load_html(&settings_shell_page(), Some(&saved));
                app_term.sync_settings_chrome();
            } else if is_internal_uri(&saved) {
                load_internal_page(v, &saved);
            } else {
                v.load_uri(&saved);
            }
        });
    }

    pub(crate) fn setup_tab_signals(
        self: &Rc<Self>,
        container: &gtk::Box,
        view: &WebView,
        resize_handle: &gtk::Box,
        height_handle: &gtk::Box,
        dt_handle: &gtk::Box,
        dt_box: &gtk::Box,
    ) {
        self.setup_view_signals(view);

        // Per-tab DevTools inspector resize drag gesture (handles Bottom, Left, and Right docking)
        let dt_drag = gtk::GestureDrag::new();
        dt_drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let start_dim = Rc::new(Cell::new(0i32));
        let dt_box_weak = dt_box.downgrade();
        let app_dt = self.clone();
        let v_dt = view.downgrade();
        let s_dim_begin = start_dim.clone();
        let b_weak_begin = dt_box_weak.clone();
        let app_dt_begin = app_dt.clone();
        dt_drag.connect_drag_begin(move |g, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
            if let Some(b) = b_weak_begin.upgrade() {
                let side = app_dt_begin.devtools_dock_side.get();
                let cur = match side {
                    DevToolsDockSide::Bottom => b.height(),
                    DevToolsDockSide::Left | DevToolsDockSide::Right => b.width(),
                    _ => 0,
                };
                s_dim_begin.set(cur);
            }
        });
        let s_dim_update = start_dim.clone();
        let b_weak_update = dt_box_weak.clone();
        let v_weak_update = v_dt.clone();
        let app_dt_update = app_dt.clone();
        dt_drag.connect_drag_update(move |_, offset_x, offset_y| {
            let Some(b) = b_weak_update.upgrade() else {
                return;
            };
            let Some(v) = v_weak_update.upgrade() else {
                return;
            };
            let side = app_dt_update.devtools_dock_side.get();
            let base = s_dim_update.get() as f64;
            match side {
                DevToolsDockSide::Bottom => {
                    let tab_total_h = (v.height() + b.height()) as f64;
                    let max_h = (tab_total_h - 100.0).max(160.0);
                    let nh = (base - offset_y).clamp(120.0, max_h) as i32;
                    b.set_height_request(nh);
                    if let Some((r, c)) = app_dt_update.find_view_coords(&v) {
                        let rows = app_dt_update.rows.borrow();
                        if let Some(t) = rows.get(r).and_then(|row| row.tabs.get(c)) {
                            t.dt_height.set(nh);
                        }
                    }
                    app_dt_update.default_devtools_height.set(nh);
                }
                DevToolsDockSide::Left => {
                    let tab_total_w = (v.width() + b.width()) as f64;
                    let max_w = (tab_total_w - 120.0).max(200.0);
                    let nw = (base + offset_x).clamp(200.0, max_w) as i32;
                    b.set_width_request(nw);
                    if let Some((r, c)) = app_dt_update.find_view_coords(&v) {
                        let rows = app_dt_update.rows.borrow();
                        if let Some(t) = rows.get(r).and_then(|row| row.tabs.get(c)) {
                            t.dt_width.set(nw);
                        }
                    }
                    app_dt_update.default_devtools_width.set(nw);
                }
                DevToolsDockSide::Right => {
                    let tab_total_w = (v.width() + b.width()) as f64;
                    let max_w = (tab_total_w - 120.0).max(200.0);
                    let nw = (base - offset_x).clamp(200.0, max_w) as i32;
                    b.set_width_request(nw);
                    if let Some((r, c)) = app_dt_update.find_view_coords(&v) {
                        let rows = app_dt_update.rows.borrow();
                        if let Some(t) = rows.get(r).and_then(|row| row.tabs.get(c)) {
                            t.dt_width.set(nw);
                        }
                    }
                    app_dt_update.default_devtools_width.set(nw);
                }
                _ => {}
            }
            b.queue_allocate();
        });
        dt_drag.connect_drag_end(move |g, _, _| {
            g.set_state(gtk::EventSequenceState::None);
        });
        dt_handle.add_controller(dt_drag);

        // Welcome page search box (a real <input> in the page) talks to us through this handler.
        // Connected here (not in add_view) so EVERY tab creation path gets it, including
        // session restore at startup; otherwise the page never receives its state (placeholder,
        // engine icon, hints, profile button).
        if let Some(ucm) = view.user_content_manager() {
            let app = self.clone();
            let wv = view.downgrade();
            ucm.connect_script_message_received(Some("railgo"), move |_, val| {
                let Some(v) = wv.upgrade() else { return };
                app.on_start_search(&v, val.to_str().as_str());
            });

            let app_ss = self.clone();
            let wv_ss = view.downgrade();
            ucm.connect_script_message_received(Some("railScreenshot"), move |_, val| {
                let Some(v) = wv_ss.upgrade() else { return };
                app_ss.on_screenshot_message(&v, val.to_str().as_str());
            });

            let app_downloads = self.clone();
            let wv_downloads = view.downgrade();
            ucm.connect_script_message_received(Some("railDownloads"), move |_, val| {
                if let Some(v) = wv_downloads.upgrade() {
                    app_downloads.handle_downloads_message(&v, val.to_str().as_str());
                }
            });

            let wv_ctx = view.downgrade();
            ucm.connect_script_message_received(Some("railContextMenu"), move |_, _val| {
                if let Some(v) = wv_ctx.upgrade() {
                    let ptr = v.as_ptr() as usize;
                    LAST_CONTEXT_PREVENTED.with(|m| {
                        let mut map = m.borrow_mut();
                        map.retain(|_, t| t.elapsed() < std::time::Duration::from_secs(5));
                        map.insert(ptr, std::time::Instant::now());
                    });
                }
            });
        }

        let app = self.clone();
        let v = view.downgrade();
        let click = gtk::GestureClick::new();
        click.set_button(0);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            if !app.ov.get() {
                let Some(v) = v.upgrade() else { return };
                if let Some((r, c)) = app.find_view_coords(&v)
                    && (app.focus_row.get() != r || app.focus_col.get() != c)
                {
                    app.focus_coords(r, c);
                }
            }
        });
        container.add_controller(click);

        // Interactive splitter resize drag gesture
        let drag = gtk::GestureDrag::new();
        // Weak refs: the controllers live on the handle itself, so a strong capture would be a
        // reference cycle that keeps the WebView (and its web process) alive after the tab closes.
        let app = self.clone();
        let v = view.downgrade();
        let h = resize_handle.downgrade();
        let start_w0 = Rc::new(Cell::new(0.0f64));
        let start_w1 = Rc::new(Cell::new(Option::<f64>::None));
        let start_root_x = Rc::new(Cell::new(0.0f64));

        drag.connect_drag_begin({
            let app = app.clone();
            let v = v.clone();
            let h = h.clone();
            let start_w0 = start_w0.clone();
            let start_w1 = start_w1.clone();
            let start_root_x = start_root_x.clone();
            move |gesture, _start_x, _start_y| {
                let Some(v) = v.upgrade() else { return };
                if let Some(h) = h.upgrade() {
                    h.add_css_class("active");
                }
                if let Some(ev) = gesture.current_event()
                    && let Some((rx, _)) = ev.position()
                {
                    start_root_x.set(rx);
                }
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let (c, next_c) = app.col_head_and_next(r, c);
                    let vw = app.strip.width() as f64;
                    let rows = app.rows.borrow();
                    if let Some(row) = rows.get(r) {
                        let w0 = row
                            .tabs
                            .get(c)
                            .and_then(|t| t.custom_width.get())
                            .unwrap_or_else(|| {
                                let is_full = row
                                    .tabs
                                    .get(c)
                                    .map(|t| t.container.has_css_class("full"))
                                    .unwrap_or(false);
                                col_width(is_full, vw)
                            });
                        start_w0.set(w0);

                        let w1 = row.tabs.get(next_c).map(|t| {
                            t.custom_width.get().unwrap_or_else(|| {
                                let is_full = t.container.has_css_class("full");
                                col_width(is_full, vw)
                            })
                        });
                        start_w1.set(w1);
                    }
                }
            }
        });

        drag.connect_drag_update({
            let app = app.clone();
            let v = v.clone();
            let start_w0 = start_w0.clone();
            let start_w1 = start_w1.clone();
            let start_root_x = start_root_x.clone();
            move |gesture, offset_x, _offset_y| {
                let Some(v) = v.upgrade() else { return };
                let dx = if let Some(ev) = gesture.current_event() {
                    if let Some((rx, _)) = ev.position() {
                        rx - start_root_x.get()
                    } else {
                        offset_x
                    }
                } else {
                    offset_x
                };

                if let Some((r, c)) = app.find_view_coords(&v) {
                    let (c, next_c) = app.col_head_and_next(r, c);
                    let vw = app.strip.width() as f64;
                    let min_w = MIN_COLUMN_WIDTH.min(vw.max(1.0));
                    let max_w = vw.max(min_w); // Fullscreen max: left edge to right edge
                    let initial0 = start_w0.get();
                    let new_w0 = (initial0 + dx).clamp(min_w, max_w);

                    if let Some(initial1) = start_w1.get() {
                        let total_pair = initial0 + initial1;
                        if total_pair <= vw + 15.0 {
                            let clamped_w0 = new_w0.clamp(min_w, (total_pair - min_w).min(max_w));
                            let new_w1 = (total_pair - clamped_w0).max(min_w);
                            app.set_tab_width(r, c, clamped_w0);
                            app.set_tab_width(r, next_c, new_w1);
                            return;
                        }
                    }

                    app.set_tab_width(r, c, new_w0);
                }
            }
        });

        drag.connect_drag_end({
            let app = app.clone();
            let h = h.clone();
            move |_gesture, _offset_x, _offset_y| {
                if let Some(h) = h.upgrade() {
                    h.remove_css_class("active");
                }
                app.schedule_save_session();
                app.reveal_focus();
            }
        });

        let dbl_click = gtk::GestureClick::new();
        dbl_click.set_button(gdk::BUTTON_PRIMARY);
        let app_dbl = self.clone();
        let v_dbl = view.downgrade();
        dbl_click.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2
                && let Some((r, c)) = v_dbl.upgrade().and_then(|v| app_dbl.find_view_coords(&v))
            {
                let rows = app_dbl.rows.borrow();
                if let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(row.col_span(c).0)) {
                    tab.custom_width.set(None);
                }
                drop(rows);
                app_dbl.sync_strip();
                app_dbl.schedule_save_session();
            }
        });

        resize_handle.add_controller(drag);
        resize_handle.add_controller(dbl_click);

        // Vertical height splitter between stacked tabs (Alt+[ / ])
        let hdrag = gtk::GestureDrag::new();
        let app = self.clone();
        let v = view.downgrade();
        let hh = height_handle.downgrade();
        let start_h0 = Rc::new(Cell::new(0.0f64));
        let start_h1 = Rc::new(Cell::new(0.0f64));
        let start_root_y = Rc::new(Cell::new(0.0f64));

        hdrag.connect_drag_begin({
            let app = app.clone();
            let v = v.clone();
            let hh = hh.clone();
            let start_h0 = start_h0.clone();
            let start_h1 = start_h1.clone();
            let start_root_y = start_root_y.clone();
            move |gesture, _, _| {
                let Some(v) = v.upgrade() else { return };
                if let Some(hh) = hh.upgrade() {
                    hh.add_css_class("active");
                }
                if let Some(ev) = gesture.current_event()
                    && let Some((_, ry)) = ev.position()
                {
                    start_root_y.set(ry);
                }
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let vh = app.strip.height() as f64;
                    let rows = app.rows.borrow();
                    let Some(row) = rows.get(r) else { return };
                    let Some(next) = row
                        .tabs
                        .get(c + 1)
                        .filter(|t| t.col_id.get() == row.tabs[c].col_id.get())
                    else {
                        return;
                    };
                    // Equal share baseline if unset
                    let id = row.tabs[c].col_id.get();
                    let k = row
                        .tabs
                        .iter()
                        .filter(|t| t.col_id.get() == id)
                        .count()
                        .max(1);
                    let available = (vh - (k as f64 - 1.0) * gap(1.0)).max(1.0);
                    let equal_ratio = 1.0 / k as f64;
                    let h0 = row.tabs[c].custom_height.get().unwrap_or(equal_ratio) * available;
                    let h1 = next.custom_height.get().unwrap_or(equal_ratio) * available;
                    start_h0.set(h0);
                    start_h1.set(h1);
                }
            }
        });

        hdrag.connect_drag_update({
            let app = app.clone();
            let v = v.clone();
            let start_h0 = start_h0.clone();
            let start_h1 = start_h1.clone();
            let start_root_y = start_root_y.clone();
            move |gesture, _ox, offset_y| {
                let Some(v) = v.upgrade() else { return };
                let dy = if let Some(ev) = gesture.current_event() {
                    if let Some((_, ry)) = ev.position() {
                        ry - start_root_y.get()
                    } else {
                        offset_y
                    }
                } else {
                    offset_y
                };
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let (available, stack_count) = {
                        let rows = app.rows.borrow();
                        let Some(row) = rows.get(r) else { return };
                        if c >= row.tabs.len() {
                            return;
                        }
                        let (start, end) = row.col_span(c);
                        let count = end - start;
                        let vh = app.strip.height() as f64;
                        ((vh - (count as f64 - 1.0) * gap(1.0)).max(1.0), count)
                    };
                    let min_h = MIN_WINDOW_HEIGHT.min(available / stack_count.max(1) as f64);
                    let pair = start_h0.get() + start_h1.get();
                    let lower = min_h.min(pair / 2.0);
                    let new0 = (start_h0.get() + dy).clamp(lower, (pair - lower).max(lower));
                    let new1 = (pair - new0).max(lower);
                    app.set_tab_stack_height(r, c, new0);
                    app.set_tab_stack_height(r, c + 1, new1);
                }
            }
        });

        hdrag.connect_drag_end({
            let app = app.clone();
            let hh = hh.clone();
            move |_, _, _| {
                if let Some(hh) = hh.upgrade() {
                    hh.remove_css_class("active");
                }
                app.schedule_save_session();
            }
        });

        height_handle.add_controller(hdrag);
    }
}
