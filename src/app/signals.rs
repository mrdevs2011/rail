//! Per-view and per-tab signal wiring.

use crate::*;

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
                if let Some((r, c)) = app.find_view_coords(&v) {
                    if app.focus_row.get() != r || app.focus_col.get() != c {
                        app.focus_coords(r, c);
                    }
                }
            }
        });
        view.add_controller(click);

        // Web page horizontal scroll via mouse wheel (Shift + wheel)
        let page_scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        page_scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let v_sc = view.downgrade();
        let a_sc = self.clone();
        page_scroll.connect_scroll(move |ctl, dx, dy| {
            if a_sc.ov.get() {
                return glib::Propagation::Proceed;
            }
            let is_shift = ctl.current_event_state().contains(gdk::ModifierType::SHIFT_MASK);
            if is_shift && dy != 0.0 && dx == 0.0 {
                if let Some(v) = v_sc.upgrade() {
                    let px = dy * 60.0;
                    let js = format!("window.scrollBy({{ left: {}, behavior: 'auto' }});", px);
                    v.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        view.add_controller(page_scroll);

        // Track developer tools window state & auto-dock to per-tab container
        if let Some(insp) = view.inspector() {
            let app_insp = self.clone();
            let view_weak = view.downgrade();
            insp.connect_open_window(move |_| {
                if let Some(v) = view_weak.upgrade() {
                    app_insp.show_view_devtools(&v);
                }
                true
            });
            let app_detach = self.clone();
            let v_detach = view.downgrade();
            insp.connect_detach(move |_| {
                if let Some(v) = v_detach.upgrade() {
                    app_detach.show_view_devtools(&v);
                }
                true
            });
            let app_attach = self.clone();
            let v_attach = view.downgrade();
            insp.connect_attach(move |_| {
                if let Some(v) = v_attach.upgrade() {
                    app_attach.show_view_devtools(&v);
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
                    app_front.show_view_devtools(&v);
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
            dialog.set_title(&tr("Select Color"));
            dialog.set_with_alpha(false);
            let initial = request.rgba();
            let req = request.clone();
            let cancellable = gio::Cancellable::new();
            let c_clone = cancellable.clone();
            request.connect_finished(move |_| {
                c_clone.cancel();
            });
            dialog.choose_rgba(Some(&app.win), Some(&initial), Some(&cancellable), move |res| {
                if let Ok(rgba) = res {
                    req.set_rgba(&rgba);
                    req.finish();
                } else {
                    req.cancel();
                }
            });
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
                if let Some((r, c)) = app.find_view_coords(&v) {
                    if app.focus_row.get() != r || app.focus_col.get() != c {
                        app.focus_coords(r, c);
                    }
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
        view.connect_load_failed(move |_v, event, uri, error| {
            eprintln!("rail: load failed for {} ({:?}): {}", uri, event, error);
            false
        });
        view.connect_load_failed_with_tls_errors(move |v, failing_uri, cert, _errors| {
            if let Some(session) = v.network_session() {
                if let Ok(uri_obj) = glib::Uri::parse(failing_uri, glib::UriFlags::NONE) {
                    if let Some(host) = uri_obj.host() {
                        session.allow_tls_certificate_for_host(cert, &host);
                        v.load_uri(failing_uri);
                        return true;
                    }
                }
            }
            false
        });
        let app = self.clone();
        view.connect_create(move |v, nav| {
            // Chrome/Brave parity: plain left-click navigates in the same tab.
            // Only middle-click, Ctrl/Cmd+click, or scripted window.open() open a new tab.
            let btn = nav.mouse_button();
            let mods = gdk::ModifierType::from_bits_truncate(nav.modifiers());
            let want_new = btn == 2
                || mods.contains(gdk::ModifierType::CONTROL_MASK)
                || mods.contains(gdk::ModifierType::META_MASK)
                || (btn == 0 && nav.is_user_gesture());

            if !want_new {
                if let Some(req) = nav.request() {
                    if let Some(uri) = req.uri() {
                        let u = uri.to_string();
                        if !u.is_empty() && u != "about:blank" {
                            v.load_uri(&u);
                        }
                    }
                }
                return None;
            }

            let parent_prof = app.find_view_profile(v).unwrap_or_else(|| app.profiles.borrow().active_profile().id);
            let nv = WebView::builder().related_view(v).build();
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
            let too_soon = LAST.with(|c| {
                match c.get() {
                    Some(t) if now.duration_since(t) < Duration::from_secs(4) => true,
                    _ => { c.set(Some(now)); false }
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
            } else if saved.starts_with("rail://downloads") {
                load_internal_page(v, "rail://downloads");
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

        // Per-tab DevTools inspector vertical resize drag gesture
        let dt_drag = gtk::GestureDrag::new();
        dt_drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let start_h = Rc::new(Cell::new(0i32));
        let dt_box_weak = dt_box.downgrade();
        let app_dt = self.clone();
        let v_dt = view.downgrade();
        let s_h_begin = start_h.clone();
        let b_weak_begin = dt_box_weak.clone();
        dt_drag.connect_drag_begin(move |g, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
            if let Some(b) = b_weak_begin.upgrade() {
                s_h_begin.set(b.height());
            }
        });
        let s_h_update = start_h.clone();
        let b_weak_update = dt_box_weak.clone();
        let v_weak_update = v_dt.clone();
        let app_dt_update = app_dt.clone();
        dt_drag.connect_drag_update(move |_, _, offset_y| {
            let Some(b) = b_weak_update.upgrade() else { return };
            let Some(v) = v_weak_update.upgrade() else { return };
            let base = s_h_update.get() as f64;
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
        }

        let app = self.clone();
        let v = view.downgrade();
        let click = gtk::GestureClick::new();
        click.set_button(0);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            if !app.ov.get() {
                let Some(v) = v.upgrade() else { return };
                if let Some((r, c)) = app.find_view_coords(&v) {
                    if app.focus_row.get() != r || app.focus_col.get() != c {
                        app.focus_coords(r, c);
                    }
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
                if let Some(ev) = gesture.current_event() {
                    if let Some((rx, _)) = ev.position() {
                        start_root_x.set(rx);
                    }
                }
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let (c, next_c) = app.col_head_and_next(r, c);
                    let vw = app.strip.width() as f64;
                    let rows = app.rows.borrow();
                    if let Some(row) = rows.get(r) {
                        let w0 = row.tabs.get(c).and_then(|t| t.custom_width.get()).unwrap_or_else(|| {
                            let is_full = row.tabs.get(c).map(|t| t.container.has_css_class("full")).unwrap_or(false);
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
                    let min_w = 280.0;
                    let max_w = vw; // Fullscreen max: left edge to right edge
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
            if n_press == 2 {
                if let Some((r, c)) = v_dbl.upgrade().and_then(|v| app_dbl.find_view_coords(&v)) {
                    let rows = app_dbl.rows.borrow();
                    if let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(row.col_span(c).0)) {
                        tab.custom_width.set(None);
                    }
                    drop(rows);
                    app_dbl.sync_strip();
                    app_dbl.schedule_save_session();
                }
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
                if let Some(ev) = gesture.current_event() {
                    if let Some((_, ry)) = ev.position() {
                        start_root_y.set(ry);
                    }
                }
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let vh = app.strip.height() as f64;
                    let rows = app.rows.borrow();
                    let Some(row) = rows.get(r) else { return };
                    let Some(next) = row.tabs.get(c + 1).filter(|t| t.col_id.get() == row.tabs[c].col_id.get()) else {
                        return;
                    };
                    // Equal share baseline if unset
                    let id = row.tabs[c].col_id.get();
                    let k = row.tabs.iter().filter(|t| t.col_id.get() == id).count().max(1);
                    let equal = stack_height(vh, k, gap(1.0));
                    let h0 = row.tabs[c].custom_height.get().unwrap_or(equal);
                    let h1 = next.custom_height.get().unwrap_or(equal);
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
                    let min_h = 80.0;
                    let pair = start_h0.get() + start_h1.get();
                    let new0 = (start_h0.get() + dy).clamp(min_h, (pair - min_h).max(min_h));
                    let new1 = (pair - new0).max(min_h);
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
