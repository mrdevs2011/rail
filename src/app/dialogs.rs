//! Script dialogs and the page context menu.

use crate::*;

impl App {
    pub(crate) fn confirm_modal_open(&self) -> bool {
        self.confirm_modal.borrow().is_some()
    }

    pub(crate) fn hide_confirm_modal(self: &Rc<Self>) {
        if let Some(backdrop) = self.confirm_modal.borrow_mut().take()
            && let Some(parent) = backdrop.parent()
            && let Ok(overlay) = parent.downcast::<gtk::Overlay>()
        {
            overlay.remove_overlay(&backdrop);
        }
    }

    /// Centralized confirmation modal for irreversible/destructive actions (Clear History, Clear Cache, Clear Downloads, etc.)
    pub(crate) fn show_confirm_dialog<F>(
        self: &Rc<Self>,
        title: &str,
        message: &str,
        confirm_label: &str,
        is_danger: bool,
        on_confirm: F,
    ) where
        F: FnOnce() + 'static,
    {
        self.hide_confirm_modal();

        let root_overlay = match self
            .win
            .child()
            .and_then(|c| c.downcast::<gtk::Overlay>().ok())
        {
            Some(ov) => ov,
            None => return,
        };

        // Full-window dim backdrop
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("confirm-backdrop");
        backdrop.set_hexpand(true);
        backdrop.set_vexpand(true);
        backdrop.set_halign(gtk::Align::Fill);
        backdrop.set_valign(gtk::Align::Fill);
        backdrop.set_can_target(true);

        // Center card with sharp minimalist styling
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("confirm-card");
        card.set_halign(gtk::Align::Center);
        card.set_valign(gtk::Align::Center);
        card.set_can_target(true);

        // Topbar: title + close '✕' button
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        header.add_css_class("confirm-header");

        let title_lbl = gtk::Label::new(Some(title));
        title_lbl.add_css_class("confirm-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);
        header.append(&title_lbl);

        let close_btn = gtk::Button::new();
        close_btn.add_css_class("confirm-close-btn");
        close_btn.set_has_frame(false);
        close_btn.set_focus_on_click(false);
        let x_lbl = gtk::Label::new(Some("✕"));
        close_btn.set_child(Some(&x_lbl));
        let a_close = Rc::downgrade(self);
        close_btn.connect_clicked(move |_| {
            if let Some(app) = a_close.upgrade() {
                app.hide_confirm_modal();
            }
        });
        header.append(&close_btn);
        card.append(&header);

        // Description / message
        let msg_lbl = gtk::Label::new(Some(message));
        msg_lbl.add_css_class("confirm-message");
        msg_lbl.set_xalign(0.0);
        msg_lbl.set_wrap(true);
        msg_lbl.set_max_width_chars(44);
        card.append(&msg_lbl);

        // Action buttons (Cancel & Confirm)
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        actions.add_css_class("confirm-actions");
        actions.set_halign(gtk::Align::End);

        let cancel_btn = gtk::Button::with_label(tr("Cancel"));
        cancel_btn.add_css_class("confirm-btn-cancel");
        cancel_btn.set_cursor_from_name(Some("pointer"));
        let a_cancel = Rc::downgrade(self);
        cancel_btn.connect_clicked(move |_| {
            if let Some(app) = a_cancel.upgrade() {
                app.hide_confirm_modal();
            }
        });
        actions.append(&cancel_btn);

        let confirm_btn = gtk::Button::with_label(confirm_label);
        if is_danger {
            confirm_btn.add_css_class("confirm-btn-danger");
        } else {
            confirm_btn.add_css_class("confirm-btn-primary");
        }
        confirm_btn.set_cursor_from_name(Some("pointer"));

        let a_confirm = Rc::downgrade(self);
        let on_confirm_cell = Rc::new(RefCell::new(Some(on_confirm)));
        confirm_btn.connect_clicked(move |_| {
            if let Some(app) = a_confirm.upgrade() {
                app.hide_confirm_modal();
                if let Some(cb) = on_confirm_cell.borrow_mut().take() {
                    cb();
                }
            }
        });
        actions.append(&confirm_btn);
        card.append(&actions);

        backdrop.append(&card);

        // Click outside card closes modal
        let click = gtk::GestureClick::new();
        let c_ref = card.clone();
        let bd_ref = backdrop.clone();
        let a_click = Rc::downgrade(self);
        click.connect_pressed(move |_, _, x, y| {
            if let Some(c) = c_ref.parent()
                && let Some(bounds) = c.compute_bounds(&bd_ref)
            {
                let bx = bounds.x() as f64;
                let by = bounds.y() as f64;
                let bw = bounds.width() as f64;
                let bh = bounds.height() as f64;
                if (x < bx || x > bx + bw || y < by || y > by + bh)
                    && let Some(app) = a_click.upgrade()
                {
                    app.hide_confirm_modal();
                }
            }
        });
        backdrop.add_controller(click);

        // Escape key to close
        let key = gtk::EventControllerKey::new();
        let a_key = Rc::downgrade(self);
        key.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                if let Some(app) = a_key.upgrade() {
                    app.hide_confirm_modal();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        backdrop.add_controller(key);

        root_overlay.add_overlay(&backdrop);
        *self.confirm_modal.borrow_mut() = Some(backdrop);
    }

    /// Show content as a top-center card that slides down from the top and slides back up on close.
    /// Returns a closer closure.
    pub(crate) fn show_top_dialog(
        self: &Rc<Self>,
        content: gtk::Box,
    ) -> impl Fn() + Clone + 'static {
        let revealer = gtk::Revealer::new();
        revealer.add_css_class("top-dialog-revealer");
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(180);
        revealer.set_halign(gtk::Align::Center);
        revealer.set_valign(gtk::Align::Start);
        revealer.set_margin_top(8);
        revealer.set_reveal_child(false);
        revealer.set_can_target(true);

        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("top-dialog-card");
        card.append(&content);
        revealer.set_child(Some(&card));

        // Attach to the main window overlay
        if let Some(overlay) = self
            .win
            .child()
            .and_then(|c| c.downcast::<gtk::Overlay>().ok())
        {
            overlay.add_overlay(&revealer);
        }

        // Animate in
        revealer.set_reveal_child(true);

        let revealer_c = revealer.clone();

        move || {
            revealer_c.set_reveal_child(false);
            let r = revealer_c.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
                if let Some(parent) = r.parent()
                    && let Ok(overlay) = parent.downcast::<gtk::Overlay>()
                {
                    overlay.remove_overlay(&r);
                }
            });
        }
    }

    pub(crate) fn show_script_dialog(self: &Rc<Self>, view: &WebView, dialog: &ScriptDialog) {
        // JavaScript dialogs belong to the WebView that emitted the signal. Make that
        // owner the visible tab before presenting the browser-level dialog; otherwise
        // GTK focus restoration can leave the user on whichever tab was focused earlier.
        if let Some((row, col)) = self.find_view_coords(view) {
            self.focus_coords(row, col);
        }
        let owner_view = view.downgrade();
        let app_weak = Rc::downgrade(self);
        let dialog_type = dialog.dialog_type();
        let message = dialog.message().map(|s| s.to_string()).unwrap_or_default();
        // Only prompt dialogs carry default text; calling this on alert/confirm trips a
        // WebKit g_return_if_fail CRITICAL.
        let default_prompt = if dialog_type == ScriptDialogType::Prompt {
            dialog
                .prompt_get_default_text()
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let host = view
            .uri()
            .and_then(|u| glib::Uri::parse(&u, glib::UriFlags::NONE).ok())
            .and_then(|u| u.host().map(|h| h.to_string()))
            .unwrap_or_else(|| tr("This page").to_string());

        let title_text = match dialog_type {
            ScriptDialogType::Alert => trf("{} says:", &[&host]),
            ScriptDialogType::Confirm => trf("Confirm from {}", &[&host]),
            ScriptDialogType::Prompt => trf("Prompt from {}", &[&host]),
            ScriptDialogType::BeforeUnloadConfirm => tr("Leave Site?").to_string(),
            _ => trf("Message from {}", &[&host]),
        };

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 14);
        vbox.add_css_class("js-dialog-window");
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let header_lbl = gtk::Label::new(Some(&title_text));
        header_lbl.add_css_class("js-dialog-title");
        header_lbl.set_xalign(0.0);
        vbox.append(&header_lbl);

        let msg_text = if dialog_type == ScriptDialogType::BeforeUnloadConfirm && message.is_empty()
        {
            tr("Changes you made may not be saved. Are you sure you want to leave?").to_string()
        } else {
            message
        };
        let msg_lbl = gtk::Label::new(Some(&msg_text));
        msg_lbl.add_css_class("js-dialog-message");
        msg_lbl.set_wrap(true);
        msg_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        msg_lbl.set_selectable(true);
        msg_lbl.set_xalign(0.0);
        msg_lbl.set_max_width_chars(50);
        vbox.append(&msg_lbl);

        let prompt_entry = if dialog_type == ScriptDialogType::Prompt {
            let entry = gtk::Entry::new();
            entry.add_css_class("js-dialog-entry");
            entry.set_text(&default_prompt);
            vbox.append(&entry);
            Some(entry)
        } else {
            None
        };

        let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        btn_row.set_halign(gtk::Align::End);
        btn_row.set_margin_top(6);

        let handled = Rc::new(Cell::new(false));

        let close_dialog = {
            let handled = handled.clone();
            let dialog = dialog.clone();
            let prompt_entry = prompt_entry.clone();
            move |confirmed: bool| {
                if !handled.get() {
                    handled.set(true);
                    match dialog_type {
                        ScriptDialogType::Alert => {
                            dialog.close();
                        }
                        ScriptDialogType::Confirm | ScriptDialogType::BeforeUnloadConfirm => {
                            dialog.confirm_set_confirmed(confirmed);
                            dialog.close();
                        }
                        ScriptDialogType::Prompt => {
                            if confirmed {
                                let text = prompt_entry
                                    .as_ref()
                                    .map(|e| e.text().to_string())
                                    .unwrap_or_default();
                                dialog.prompt_set_text(&text);
                                dialog.confirm_set_confirmed(true);
                            } else {
                                dialog.confirm_set_confirmed(false);
                            }
                            dialog.close();
                        }
                        _ => {
                            dialog.close();
                        }
                    }
                    // closer will be called below
                }
            }
        };

        let cancel_btn = if dialog_type != ScriptDialogType::Alert {
            let cancel_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm {
                tr("Stay")
            } else {
                tr("Cancel")
            };
            let cancel_btn = gtk::Button::with_label(cancel_label);
            cancel_btn.add_css_class("js-dialog-btn-cancel");
            btn_row.append(&cancel_btn);
            Some(cancel_btn)
        } else {
            None
        };

        let ok_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm {
            tr("Leave")
        } else {
            tr("OK")
        };
        let ok_btn = gtk::Button::with_label(ok_label);
        ok_btn.add_css_class("js-dialog-btn-ok");
        btn_row.append(&ok_btn);

        vbox.append(&btn_row);

        let vbox_for_dialog = vbox.clone();
        let closer = self.show_top_dialog(vbox_for_dialog);

        // Wire closer into the existing close_dialog
        let close_with_anim = {
            let closer = closer.clone();
            let close_dialog = close_dialog.clone();
            let owner_view = owner_view.clone();
            let app_weak = app_weak.clone();
            move |confirmed: bool| {
                close_dialog(confirmed);
                closer();

                // The overlay temporarily owns GTK focus. Restore focus to the exact
                // WebView that opened the JS dialog after its exit animation, not to
                // a stale global/current-tab reference.
                let owner_view = owner_view.clone();
                let app_weak = app_weak.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(220), move || {
                    if let (Some(app), Some(view)) = (app_weak.upgrade(), owner_view.upgrade())
                        && let Some((row, col)) = app.find_view_coords(&view)
                    {
                        app.focus_coords(row, col);
                        view.grab_focus();
                    }
                });
            }
        };

        if let Some(cancel_btn) = &cancel_btn {
            let cd = close_with_anim.clone();
            cancel_btn.connect_clicked(move |_| cd(false));
        }
        {
            let cd = close_with_anim.clone();
            ok_btn.connect_clicked(move |_| cd(true));
        }

        // Re-bind buttons to the animated closer
        if let Some(entry) = &prompt_entry {
            let cd = close_with_anim.clone();
            entry.connect_activate(move |_| cd(true));
        }

        let key_ctl = gtk::EventControllerKey::new();
        let cd_key = close_with_anim.clone();
        key_ctl.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                cd_key(false);
                glib::Propagation::Stop
            } else if keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter {
                cd_key(true);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        vbox.add_controller(key_ctl);

        if let Some(entry) = &prompt_entry {
            entry.grab_focus();
        } else {
            ok_btn.grab_focus();
        }
    }

    pub(crate) fn customize_context_menu(
        self: &Rc<Self>,
        view: &WebView,
        menu: &ContextMenu,
        hit: &HitTestResult,
    ) {
        let is_link = hit.context_is_link();
        let link_uri = hit.link_uri().map(|s| s.to_string());
        let image_uri = hit.image_uri().map(|s| s.to_string());
        let is_image = hit.context_is_image() || image_uri.is_some();
        let is_media = hit.context_is_media();
        let is_editable = hit.context_is_editable();
        let is_selection = hit.context_is_selection();

        menu.remove_all();

        // 1. Link items
        if is_link && let Some(uri) = &link_uri {
            let uri_t = uri.clone();
            let app_t = self.clone();
            let v_t = view.clone();
            let act_tab = gio::SimpleAction::new("ctx-open-tab", None);
            act_tab.connect_activate(move |_, _| {
                app_t.open_link_in_new_tab(&v_t, &uri_t);
            });
            self.win.add_action(&act_tab);
            menu.append(&ContextMenuItem::from_gaction(
                &act_tab,
                tr("Open Link in New Tab"),
                None,
            ));

            let uri_s = uri.clone();
            let app_s = self.clone();
            let v_s = view.clone();
            let act_split = gio::SimpleAction::new("ctx-open-split", None);
            act_split.connect_activate(move |_, _| {
                app_s.open_link_in_split_view(&v_s, &uri_s);
            });
            self.win.add_action(&act_split);
            menu.append(&ContextMenuItem::from_gaction(
                &act_split,
                tr("Open Link in Split View"),
                None,
            ));

            let uri_c = uri.clone();
            let act_copy = gio::SimpleAction::new("ctx-copy-link", None);
            act_copy.connect_activate(move |_, _| {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&uri_c);
                }
            });
            self.win.add_action(&act_copy);
            menu.append(&ContextMenuItem::from_gaction(
                &act_copy,
                tr("Copy Link Address"),
                None,
            ));

            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::DownloadLinkToDisk,
                tr("Save Link As..."),
            ));

            menu.append(&ContextMenuItem::new_separator());
        }

        // 2. Image items
        if is_image && let Some(img_url) = &image_uri {
            let img_t = img_url.clone();
            let app_t = self.clone();
            let v_t = view.clone();
            let act_img = gio::SimpleAction::new("ctx-open-img", None);
            act_img.connect_activate(move |_, _| {
                app_t.open_image_in_new_tab(&v_t, &img_t);
            });
            self.win.add_action(&act_img);
            menu.append(&ContextMenuItem::from_gaction(
                &act_img,
                tr("Open Image in New Tab"),
                None,
            ));

            let img_c = img_url.clone();
            let act_copy_img = gio::SimpleAction::new("ctx-copy-img-url", None);
            act_copy_img.connect_activate(move |_, _| {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&img_c);
                }
            });
            self.win.add_action(&act_copy_img);
            menu.append(&ContextMenuItem::from_gaction(
                &act_copy_img,
                tr("Copy Image Address"),
                None,
            ));

            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::CopyImageToClipboard,
                tr("Copy Image"),
            ));

            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::DownloadImageToDisk,
                tr("Save Image As..."),
            ));

            menu.append(&ContextMenuItem::new_separator());
        }

        // 3. Media items
        if is_media {
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::OpenVideoInNewWindow,
                tr("Open Media in New Tab"),
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::CopyVideoLinkToClipboard,
                tr("Copy Media Address"),
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::DownloadVideoToDisk,
                tr("Save Media As..."),
            ));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 4. Editable text
        if is_editable {
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::Cut,
                tr("Cut"),
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::Copy,
                tr("Copy"),
            ));
            if self.clipboard_has_image() {
                let app_p = self.clone();
                let v_p = view.clone();
                let act_paste = gio::SimpleAction::new("ctx-paste-image", None);
                act_paste.connect_activate(move |_, _| app_p.paste_clipboard_image(v_p.clone()));
                self.win.add_action(&act_paste);
                menu.append(&ContextMenuItem::from_gaction(
                    &act_paste,
                    tr("Paste"),
                    None,
                ));
            } else {
                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::Paste,
                    tr("Paste"),
                ));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::PasteAsPlainText,
                tr("Paste as Plain Text"),
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::SelectAll,
                tr("Select All"),
            ));
            menu.append(&ContextMenuItem::new_separator());
        } else if is_selection {
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::Copy,
                tr("Copy"),
            ));

            let app_sel = self.clone();
            let v_sel = view.clone();
            let act_search = gio::SimpleAction::new("ctx-search-sel", None);
            act_search.connect_activate(move |_, _| {
                let app_s = app_sel.clone();
                v_sel.evaluate_javascript(
                    "window.getSelection() ? window.getSelection().toString() : ''",
                    None,
                    None,
                    None::<&gio::Cancellable>,
                    move |res| {
                        if let Ok(js_val) = res {
                            let text = js_val.to_string();
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                let search_url = format!(
                                    "https://www.google.com/search?q={}",
                                    glib::Uri::escape_string(trimmed, None, false)
                                );
                                app_s.new_tab_with_url(&search_url);
                            }
                        }
                    },
                );
            });
            self.win.add_action(&act_search);
            menu.append(&ContextMenuItem::from_gaction(
                &act_search,
                tr("Search Google for Selection"),
                None,
            ));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 5. Page navigation
        if !is_link && !is_editable && !is_selection {
            if view.can_go_back() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::GoBack,
                    tr("Back"),
                ));
            }
            if view.can_go_forward() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::GoForward,
                    tr("Forward"),
                ));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::Reload,
                tr("Reload"),
            ));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 5b. Translate page (Google Translate)
        self.append_translate_item(view, menu);

        // 5c. Install as Desktop App (PWA)
        if !is_link && !is_editable && !is_selection {
            let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
            if cur_uri.starts_with("http://") || cur_uri.starts_with("https://") {
                let app_pwa = self.clone();
                let v_pwa = view.clone();
                let act_pwa = gio::SimpleAction::new("ctx-install-pwa", None);
                act_pwa.connect_activate(move |_, _| {
                    app_pwa.handle_pwa_install_message("", &v_pwa);
                });
                self.win.add_action(&act_pwa);
                menu.append(&ContextMenuItem::from_gaction(
                    &act_pwa,
                    tr("Install Page as Desktop App..."),
                    None,
                ));
                menu.append(&ContextMenuItem::new_separator());
            }
        }

        // 6. Developer Tools: ONE item, always last. "Inspect Element" opens the same
        // devtools as F12 / View Source (see signals.rs), so a separate View Source entry is redundant.
        menu.append(&ContextMenuItem::from_stock_action_with_label(
            ContextMenuAction::InspectElement,
            &sc_apply(tr("Inspect Element  (F12)"), &["devtools"]),
        ));

        // WebKit's ContextMenuItem has no icon API: decorate the popover's GMenu after it is built.
        Self::decorate_context_menu_icons(view);
    }
}

// ---------------------------------------------------------------------------
// Site permission prompts (microphone, camera, geolocation, notifications)
// ---------------------------------------------------------------------------

thread_local! {
    /// Persistent decisions keyed by (host, permission kind), loaded once per app thread.
    static PERMISSION_DECISIONS: std::cell::RefCell<std::collections::HashMap<(String, &'static str), bool>> =
        std::cell::RefCell::new(load_permission_decisions());
}

fn permission_decisions_path() -> std::path::PathBuf {
    rail_data_dir().join("site-permissions.tsv")
}

fn load_permission_decisions() -> std::collections::HashMap<(String, &'static str), bool> {
    let mut out = std::collections::HashMap::new();
    let Ok(text) = std::fs::read_to_string(permission_decisions_path()) else {
        return out;
    };
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(host), Some(kind), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let kind: &'static str = match kind {
            "mic" => "mic",
            "camera" => "camera",
            "camera+mic" => "camera+mic",
            "geolocation" => "geolocation",
            "notifications" => "notifications",
            _ => continue,
        };
        if !host.is_empty() && !host.chars().any(char::is_control) {
            out.insert((host.to_string(), kind), value == "allow");
        }
    }
    out
}

fn save_permission_decisions(decisions: &std::collections::HashMap<(String, &'static str), bool>) {
    let path = permission_decisions_path();
    let Some(dir) = path.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let mut entries: Vec<_> = decisions.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut data = String::new();
    for ((host, kind), allowed) in entries {
        if host.chars().any(char::is_control) {
            continue;
        }
        data.push_str(host);
        data.push('\t');
        data.push_str(kind);
        data.push('\t');
        data.push_str(if *allowed { "allow" } else { "deny" });
        data.push('\n');
    }
    let tmp = path.with_extension("tsv.tmp");
    if std::fs::write(&tmp, data).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

/// Permission kinds Rail asks about. Anything else keeps WebKit's default handling.
fn permission_kind(req: &webkit6::PermissionRequest) -> Option<&'static str> {
    if let Some(um) = req.downcast_ref::<webkit6::UserMediaPermissionRequest>() {
        return match (um.is_for_audio_device(), um.is_for_video_device()) {
            (true, true) => Some("camera+mic"),
            (true, false) => Some("mic"),
            (false, true) => Some("camera"),
            (false, false) => None,
        };
    }
    if req.is::<webkit6::GeolocationPermissionRequest>() {
        return Some("geolocation");
    }
    if req.is::<webkit6::NotificationPermissionRequest>() {
        return Some("notifications");
    }
    None
}

fn permission_label(kind: &str) -> &'static str {
    match kind {
        "camera+mic" => tr("Use your camera and microphone"),
        "mic" => tr("Use your microphone"),
        "camera" => tr("Use your camera"),
        "geolocation" => tr("Know your location"),
        _ => tr("Show notifications"),
    }
}

impl App {
    pub(crate) fn handle_permission_request(
        self: &Rc<Self>,
        view: &WebView,
        req: &webkit6::PermissionRequest,
    ) -> bool {
        let Some(kind) = permission_kind(req) else {
            return false;
        };
        let host = view
            .uri()
            .and_then(|u| glib::Uri::parse(&u, glib::UriFlags::NONE).ok())
            .and_then(|u| u.host().map(|h| h.to_string()))
            .unwrap_or_else(|| tr("This page").to_string());
        let remembered =
            PERMISSION_DECISIONS.with(|m| m.borrow().get(&(host.clone(), kind)).copied());
        if let Some(allowed) = remembered {
            if allowed {
                req.allow();
            } else {
                req.deny();
            }
            return true;
        }
        self.show_permission_popup(host, kind, req.clone());
        true
    }

    #[allow(clippy::type_complexity)]
    fn show_permission_popup(
        self: &Rc<Self>,
        host: String,
        kind: &'static str,
        req: webkit6::PermissionRequest,
    ) {
        let is_notif = kind == "notifications";
        let title_text = if is_notif {
            tr("Turn on notifications").to_string()
        } else {
            trf("{} wants to", &[&host])
        };
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        vbox.add_css_class("js-dialog-window");
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let header_lbl = gtk::Label::new(Some(&title_text));
        header_lbl.add_css_class("js-dialog-title");
        header_lbl.set_xalign(0.0);
        vbox.append(&header_lbl);

        let msg_text = if is_notif {
            host.clone()
        } else {
            permission_label(kind).to_string()
        };
        let msg_lbl = gtk::Label::new(Some(&msg_text));
        msg_lbl.add_css_class("js-dialog-message");
        msg_lbl.set_xalign(0.0);
        msg_lbl.set_margin_bottom(6);
        vbox.append(&msg_lbl);

        let done = Rc::new(Cell::new(false));
        let closer_slot: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let finish = {
            let done = done.clone();
            let closer_slot = closer_slot.clone();
            move |allow: bool, remember: bool| {
                if done.replace(true) {
                    return;
                }
                if allow {
                    req.allow();
                } else {
                    req.deny();
                }
                if remember {
                    PERMISSION_DECISIONS.with(|m| {
                        let mut decisions = m.borrow_mut();
                        decisions.insert((host.clone(), kind), allow);
                        save_permission_decisions(&decisions);
                    });
                }
                if let Some(c) = closer_slot.borrow().as_ref() {
                    c();
                }
            }
        };

        let add_btn = |label: &'static str, class: &str, allow: bool, remember: bool| {
            let b = gtk::Button::with_label(label);
            b.add_css_class(class);
            let f = finish.clone();
            b.connect_clicked(move |_| f(allow, remember));
            vbox.append(&b);
        };
        if is_notif {
            // Site's own "enable notifications" button -> Edge-style prompt: Close / Turn on,
            // with "Turn on" already focused so Enter confirms it. Close is not remembered, so
            // the site can ask again later.
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_halign(gtk::Align::End);
            let close_btn = gtk::Button::with_label(tr("Close"));
            close_btn.add_css_class("js-dialog-btn-cancel");
            let f = finish.clone();
            close_btn.connect_clicked(move |_| f(false, false));
            let on_btn = gtk::Button::with_label(tr("Turn on"));
            on_btn.add_css_class("js-dialog-btn-ok");
            let f = finish.clone();
            on_btn.connect_clicked(move |_| f(true, true));
            row.append(&close_btn);
            row.append(&on_btn);
            vbox.append(&row);
            on_btn.grab_focus();
            let focus_btn = on_btn.clone();
            glib::idle_add_local_once(move || {
                focus_btn.grab_focus();
            });
        } else {
            add_btn(
                tr("Always allow on this site"),
                "js-dialog-btn-ok",
                true,
                true,
            );
            add_btn(tr("Allow this time"), "js-dialog-btn-cancel", true, false);
            add_btn(tr("Never allow"), "js-dialog-btn-cancel", false, true);
        }

        let vbox_for_dialog = vbox.clone();
        let closer = self.show_top_dialog(vbox_for_dialog);
        *closer_slot.borrow_mut() = Some(Box::new(closer));

        let key_ctl = gtk::EventControllerKey::new();
        let f_key = finish.clone();
        key_ctl.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                f_key(false, false);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        vbox.add_controller(key_ctl);
    }

    /// Open File dialog (Ctrl+O) to view local HTML, PDF, image, video, audio or text files
    pub(crate) fn open_file_dialog(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title(tr("Open File")).build();
        let app = self.clone();
        dialog.open(Some(&self.win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                let uri = file.uri();
                if !uri.is_empty() {
                    app.navigate(&uri);
                } else if let Some(path) = file.path() {
                    app.navigate(&format!("file://{}", path.display()));
                }
            }
        });
    }
}
