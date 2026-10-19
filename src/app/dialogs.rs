//! Script dialogs and the page context menu.

use crate::*;

impl App {
    pub(crate) fn show_script_dialog(self: &Rc<Self>, view: &WebView, dialog: &ScriptDialog) {
        let dialog_type = dialog.dialog_type();
        let message = dialog.message().map(|s| s.to_string()).unwrap_or_default();
        let default_prompt = dialog.prompt_get_default_text().map(|s| s.to_string()).unwrap_or_default();

        let host = view.uri()
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

        let dlg_win = gtk::Window::builder()
            .title(&title_text)
            .transient_for(&self.win)
            .modal(true)
            .resizable(false)
            .default_width(380)
            .build();
        dlg_win.add_css_class("js-dialog-window");

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 14);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let header_lbl = gtk::Label::new(Some(&title_text));
        header_lbl.add_css_class("js-dialog-title");
        header_lbl.set_xalign(0.0);
        vbox.append(&header_lbl);

        let msg_text = if dialog_type == ScriptDialogType::BeforeUnloadConfirm && message.is_empty() {
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
            let dlg_win = dlg_win.clone();
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
                                let text = prompt_entry.as_ref().map(|e| e.text().to_string()).unwrap_or_default();
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
                    dlg_win.close();
                }
            }
        };

        if dialog_type != ScriptDialogType::Alert {
            let cancel_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm { tr("Stay") } else { tr("Cancel") };
            let cancel_btn = gtk::Button::with_label(cancel_label);
            cancel_btn.add_css_class("js-dialog-btn-cancel");
            let cd = close_dialog.clone();
            cancel_btn.connect_clicked(move |_| cd(false));
            btn_row.append(&cancel_btn);
        }

        let ok_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm { tr("Leave") } else { tr("OK") };
        let ok_btn = gtk::Button::with_label(ok_label);
        ok_btn.add_css_class("js-dialog-btn-ok");
        let cd_ok = close_dialog.clone();
        ok_btn.connect_clicked(move |_| cd_ok(true));
        btn_row.append(&ok_btn);

        vbox.append(&btn_row);
        dlg_win.set_child(Some(&vbox));

        if let Some(entry) = &prompt_entry {
            let cd_ent = close_dialog.clone();
            entry.connect_activate(move |_| cd_ent(true));
        }

        let key_ctl = gtk::EventControllerKey::new();
        let cd_key = close_dialog.clone();
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
        dlg_win.add_controller(key_ctl);

        let cd_close = close_dialog.clone();
        dlg_win.connect_close_request(move |_| {
            cd_close(false);
            glib::Propagation::Proceed
        });

        dlg_win.present();
        if let Some(entry) = &prompt_entry {
            entry.grab_focus();
        } else {
            ok_btn.grab_focus();
        }
    }

    pub(crate) fn customize_context_menu(self: &Rc<Self>, view: &WebView, menu: &ContextMenu, hit: &HitTestResult) {
        let is_link = hit.context_is_link();
        let link_uri = hit.link_uri().map(|s| s.to_string());
        let is_image = hit.context_is_image();
        let image_uri = hit.image_uri().map(|s| s.to_string());
        let is_media = hit.context_is_media();
        let is_editable = hit.context_is_editable();
        let is_selection = hit.context_is_selection();

        menu.remove_all();

        // 1. Link items
        if is_link {
            if let Some(uri) = &link_uri {
                let uri_t = uri.clone();
                let app_t = self.clone();
                let act_tab = gio::SimpleAction::new("ctx-open-tab", None);
                act_tab.connect_activate(move |_, _| {
                    app_t.new_tab_with_url(&uri_t);
                });
                self.win.add_action(&act_tab);
                menu.append(&ContextMenuItem::from_gaction(&act_tab, tr("Open Link in New Tab"), None));

                let uri_s = uri.clone();
                let app_s = self.clone();
                let act_split = gio::SimpleAction::new("ctx-open-split", None);
                act_split.connect_activate(move |_, _| {
                    app_s.split_with_url(&uri_s);
                });
                self.win.add_action(&act_split);
                menu.append(&ContextMenuItem::from_gaction(&act_split, tr("Open Link in Split View"), None));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyLinkToClipboard,
                    tr("Copy Link Address"),
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::DownloadLinkToDisk,
                    tr("Save Link As..."),
                ));

                menu.append(&ContextMenuItem::new_separator());
            }
        }

        // 2. Image items
        if is_image {
            if let Some(img_url) = &image_uri {
                let img_t = img_url.clone();
                let app_t = self.clone();
                let act_img = gio::SimpleAction::new("ctx-open-img", None);
                act_img.connect_activate(move |_, _| {
                    app_t.new_tab_with_url(&img_t);
                });
                self.win.add_action(&act_img);
                menu.append(&ContextMenuItem::from_gaction(&act_img, tr("Open Image in New Tab"), None));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyImageUrlToClipboard,
                    tr("Copy Image Address"),
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
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Cut, tr("Cut")));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Copy, tr("Copy")));
            if self.clipboard_is_image_only() {
                let app_p = self.clone();
                let v_p = view.clone();
                let act_paste = gio::SimpleAction::new("ctx-paste-image", None);
                act_paste.connect_activate(move |_, _| app_p.paste_clipboard_image(v_p.clone()));
                self.win.add_action(&act_paste);
                menu.append(&ContextMenuItem::from_gaction(&act_paste, tr("Paste"), None));
            } else {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Paste, tr("Paste")));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::PasteAsPlainText, tr("Paste as Plain Text")));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::SelectAll, tr("Select All")));
            menu.append(&ContextMenuItem::new_separator());
        } else if is_selection {
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Copy, tr("Copy")));

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
                                let search_url = format!("https://www.google.com/search?q={}", glib::Uri::escape_string(trimmed, None, false));
                                app_s.new_tab_with_url(&search_url);
                            }
                        }
                    },
                );
            });
            self.win.add_action(&act_search);
            menu.append(&ContextMenuItem::from_gaction(&act_search, tr("Search Google for Selection"), None));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 5. Page navigation
        if !is_link && !is_editable && !is_selection {
            if view.can_go_back() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::GoBack, tr("Back")));
            }
            if view.can_go_forward() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::GoForward, tr("Forward")));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Reload, tr("Reload")));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 5b. Translate page (Google Translate)
        self.append_translate_item(view, menu);

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
    /// Per-app-session decisions: (host, permission kind) -> allowed.
    static PERMISSION_DECISIONS: std::cell::RefCell<std::collections::HashMap<(String, &'static str), bool>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
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
    pub(crate) fn handle_permission_request(self: &Rc<Self>, view: &WebView, req: &webkit6::PermissionRequest) -> bool {
        let Some(kind) = permission_kind(req) else { return false };
        let host = view.uri()
            .and_then(|u| glib::Uri::parse(&u, glib::UriFlags::NONE).ok())
            .and_then(|u| u.host().map(|h| h.to_string()))
            .unwrap_or_else(|| tr("This page").to_string());
        let remembered = PERMISSION_DECISIONS.with(|m| m.borrow().get(&(host.clone(), kind)).copied());
        if let Some(allowed) = remembered {
            if allowed { req.allow(); } else { req.deny(); }
            return true;
        }
        self.show_permission_popup(host, kind, req.clone());
        true
    }

    fn show_permission_popup(self: &Rc<Self>, host: String, kind: &'static str, req: webkit6::PermissionRequest) {
        let title_text = trf("{} wants to", &[&host]);
        let win = gtk::Window::builder()
            .title(&title_text)
            .transient_for(&self.win)
            .modal(true)
            .resizable(false)
            .default_width(380)
            .build();
        win.add_css_class("js-dialog-window");

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let header_lbl = gtk::Label::new(Some(&title_text));
        header_lbl.add_css_class("js-dialog-title");
        header_lbl.set_xalign(0.0);
        vbox.append(&header_lbl);

        let msg_lbl = gtk::Label::new(Some(permission_label(kind)));
        msg_lbl.add_css_class("js-dialog-message");
        msg_lbl.set_xalign(0.0);
        msg_lbl.set_margin_bottom(6);
        vbox.append(&msg_lbl);

        let done = Rc::new(Cell::new(false));
        let req_close = req.clone();
        let done_close = done.clone();
        let finish = {
            let win = win.clone();
            let done = done.clone();
            move |allow: bool, remember: bool| {
                if done.replace(true) {
                    return;
                }
                if allow { req.allow(); } else { req.deny(); }
                if remember {
                    PERMISSION_DECISIONS.with(|m| {
                        m.borrow_mut().insert((host.clone(), kind), allow);
                    });
                }
                win.close();
            }
        };

        let add_btn = |label: &'static str, class: &str, allow: bool, remember: bool| {
            let b = gtk::Button::with_label(label);
            b.add_css_class(class);
            let f = finish.clone();
            b.connect_clicked(move |_| f(allow, remember));
            vbox.append(&b);
        };
        add_btn(tr("Allow while visiting the site"), "js-dialog-btn-ok", true, true);
        add_btn(tr("Allow this time"), "js-dialog-btn-cancel", true, false);
        add_btn(tr("Never allow"), "js-dialog-btn-cancel", false, true);
        win.set_child(Some(&vbox));

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
        win.add_controller(key_ctl);

        // Closed with the window button: deny this one request, remember nothing.
        win.connect_close_request(move |_| {
            if !done_close.replace(true) {
                req_close.deny();
            }
            glib::Propagation::Proceed
        });

        win.present();
    }
}
