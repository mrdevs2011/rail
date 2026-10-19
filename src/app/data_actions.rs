//! Password prompt, cloud sync, export and import actions.

use crate::*;

impl App {
    pub(crate) fn show_password_dialog<F>(
        self: &Rc<Self>,
        title: &'static str,
        prompt_text: &'static str,
        btn_label: &'static str,
        need_confirm: bool,
        on_confirm: F,
    ) where
        F: Fn(String) + 'static,
    {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        vbox.add_css_class("settings-modal-window");
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let lbl_title = gtk::Label::new(Some(tr(title)));
        lbl_title.add_css_class("settings-card-title");
        lbl_title.set_xalign(0.0);
        vbox.append(&lbl_title);

        let lbl_desc = gtk::Label::new(Some(tr(prompt_text)));
        lbl_desc.add_css_class("data-section-desc");
        lbl_desc.set_xalign(0.0);
        lbl_desc.set_wrap(true);
        vbox.append(&lbl_desc);

        let entry_pass = gtk::PasswordEntry::new();
        entry_pass.add_css_class("profile-entry-box");
        entry_pass.set_show_peek_icon(true);
        entry_pass.set_placeholder_text(Some(tr("Password")));
        vbox.append(&entry_pass);

        let entry_confirm = if need_confirm {
            let e = gtk::PasswordEntry::new();
            e.add_css_class("profile-entry-box");
            e.set_show_peek_icon(true);
            e.set_placeholder_text(Some(tr("Confirm password (optional)")));
            vbox.append(&e);
            Some(e)
        } else {
            None
        };

        let err_lbl = gtk::Label::new(None);
        err_lbl.add_css_class("data-status-lbl");
        err_lbl.set_xalign(0.0);
        err_lbl.set_visible(false);
        vbox.append(&err_lbl);

        let btns_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        btns_row.set_halign(gtk::Align::End);
        btns_row.set_margin_top(8);

        let btn_cancel = gtk::Button::with_label(tr("Cancel"));
        btn_cancel.add_css_class("btn-data-action");

        let btn_ok = gtk::Button::with_label(tr(btn_label));
        btn_ok.add_css_class("btn-data-primary");

        btns_row.append(&btn_cancel);
        btns_row.append(&btn_ok);
        vbox.append(&btns_row);

        let vbox_for_dialog = vbox.clone();
        let closer = self.show_top_dialog(vbox_for_dialog);
        let closer_cancel = closer.clone();
        btn_cancel.connect_clicked(move |_| closer_cancel());

        let closer_ok = closer.clone();
        let on_confirm = std::rc::Rc::new(on_confirm);
        let ep = entry_pass.clone();
        let ec = entry_confirm.clone();
        let el = err_lbl.clone();
        let do_submit = move || {
            let p1 = ep.text().to_string();
            // Spec: 1-inputda kamida 1 belgi bo'lsa shifrlash ishlasin; 2-input bo'sh bo'lishi mumkin
            if p1.is_empty() {
                el.set_markup(&format!("<span foreground='#f87171'>{}</span>", tr("Password cannot be empty.")));
                el.set_visible(true);
                return;
            }
            if let Some(ref c) = ec {
                let p2 = c.text().to_string();
                if !p2.is_empty() && p1 != p2 {
                    el.set_markup(&format!("<span foreground='#f87171'>{}</span>", tr("Passwords do not match.")));
                    el.set_visible(true);
                    return;
                }
            }
            closer_ok();
            on_confirm(p1);
        };

        let ds1 = do_submit.clone();
        btn_ok.connect_clicked(move |_| ds1());

        // Enter on 1st field -> focus 2nd if present, else submit
        if let Some(ref conf) = entry_confirm {
            let conf2 = conf.clone();
            entry_pass.connect_activate(move |_| {
                conf2.grab_focus();
            });
            let ds3 = do_submit.clone();
            conf.connect_activate(move |_| ds3());
        } else {
            let ds2 = do_submit.clone();
            entry_pass.connect_activate(move |_| ds2());
        }

        entry_pass.grab_focus();
    }

    pub(crate) fn prompt_and_execute_sync(self: &Rc<Self>) {
        let app = self.clone();
        self.show_password_dialog(
            "Encryption Password",
            "Set a password to encrypt your data. Without this password, no one can restore or access your data.",
            "Encrypt & Sync",
            true,
            move |password| {
                app.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#38bdf8'><b>{}</b></span>",
                    tr("Syncing: Encrypting and uploading data to cloud...")
                ));

                app.save_session_immediate();
                let mut s = load_app_settings();
                s.search_engine = app.search_engine.borrow().clone();
                save_app_settings(&s);

                let data_dir = rail_data_dir();
                let server_url = sync::sync_server_url();
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let res = (|| -> Result<sync::SyncResult, String> {
                        let cipher_b64 = sync::pack_and_encrypt(&data_dir, &password)?;
                        sync::upload_sync_payload(&cipher_b64, &server_url)
                    })();
                    let _ = tx.send(res);
                });

                let app_check = app.clone();
                glib::timeout_add_local(Duration::from_millis(50), move || {
                    if let Ok(res) = rx.try_recv() {
                        match res {
                            Ok(r) => {
                                app_check.sync_url_result_entry.set_text(&r.url);
                                app_check.win.clipboard().set_text(&r.url);
                                app_check.data_modal_status_lbl.set_markup(&format!(
                                    "<span foreground='#4ade80'><b>{}</b> {} ({})</span>",
                                    tr("Sync Successful:"),
                                    tr("URL copied to clipboard! Remember your password to restore."),
                                    glib::markup_escape_text(&r.url)
                                ));
                            }
                            Err(e) => {
                                app_check.data_modal_status_lbl.set_markup(&format!(
                                    "<span foreground='#f87171'><b>{}</b> {}</span>",
                                    tr("Sync Failed:"),
                                    glib::markup_escape_text(&e)
                                ));
                            }
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            },
        );
    }

    pub(crate) fn prompt_and_execute_restore(self: &Rc<Self>) {
        let raw = self.sync_restore_entry.text().to_string();
        if raw.trim().is_empty() {
            self.data_modal_status_lbl.set_markup(&format!(
                "<span foreground='#f87171'><b>{}</b> {}</span>",
                tr("Error:"),
                tr("Please enter a valid Sync URL or ID.")
            ));
            return;
        }

        let app = self.clone();
        self.show_password_dialog(
            "Enter Decryption Password",
            "Enter the password used to encrypt this backup:",
            "Decrypt & Restore",
            false,
            move |password| {
                app.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#38bdf8'><b>{}</b></span>",
                    tr("Restoring: Downloading and decrypting cloud data...")
                ));

                let raw_input = raw.clone();
                let server_url = sync::sync_server_url();
                let rand_id = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 10000000).to_string();
                let temp_tar = std::env::temp_dir().join(format!("rail_cloud_restore_{}.tar.gz", rand_id));
                let temp_tar_thread = temp_tar.clone();
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let res = sync::download_and_decrypt(&raw_input, &password, &server_url, &temp_tar_thread);
                    let _ = tx.send(res);
                });

                let app_check = app.clone();
                glib::timeout_add_local(Duration::from_millis(50), move || {
                    if let Ok(res) = rx.try_recv() {
                        match res {
                            Ok(()) => {
                                let imp_res = import_data(&temp_tar);
                                let _ = std::fs::remove_file(&temp_tar);
                                match imp_res {
                                    Ok(()) => {
                                        let imported_settings = load_app_settings();
                                        *app_check.search_engine.borrow_mut() = imported_settings.search_engine.clone();                                        reload_customize_from_disk();
                                        app_check.reload_css_editor();
                                        *app_check.profiles.borrow_mut() = ProfileManager::new(rail_data_dir());
                                        app_check.rebuild_profiles_modal_list();
                                        let active_id = app_check.profiles.borrow().active_profile().id;
                                        app_check.select_palette_profile(&active_id);
                                        app_check.restore_session();
                                        app_check.refresh_sidebar();
                                        app_check.refresh_data_modal_info();
                                        app_check.data_modal_status_lbl.set_markup(&format!(
                                            "<span foreground='#4ade80'><b>{}</b> {}</span>",
                                            tr("Restore Successful:"),
                                            tr("All workspaces, tabs, accounts & cookies restored!")
                                        ));
                                    }
                                    Err(e) => {
                                        app_check.data_modal_status_lbl.set_markup(&format!(
                                            "<span foreground='#f87171'><b>{}</b> {}</span>",
                                            tr("Restore Failed:"),
                                            glib::markup_escape_text(&e)
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = std::fs::remove_file(&temp_tar);
                                if e == "INCORRECT_PASSWORD" {
                                    app_check.data_modal_status_lbl.set_markup(&format!(
                                        "<span foreground='#f87171'><b>{}</b> {}</span>",
                                        tr("Error:"),
                                        tr("Incorrect password! Data cannot be restored without the correct password.")
                                    ));
                                } else {
                                    app_check.data_modal_status_lbl.set_markup(&format!(
                                        "<span foreground='#f87171'><b>{}</b> {}</span>",
                                        tr("Restore Failed:"),
                                        glib::markup_escape_text(&e)
                                    ));
                                }
                            }
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            },
        );
    }

    pub(crate) fn execute_export_action(self: &Rc<Self>) {
        self.save_session_immediate();
        let mut s = load_app_settings();
        s.search_engine = self.search_engine.borrow().clone();
        save_app_settings(&s);
        let path_str = self.data_export_entry.text().to_string();
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            self.data_modal_status_lbl.set_markup(&format!("<span foreground='#f87171'><b>{}</b> {}</span>", tr("Error:"), tr("Please specify an export destination file.")));
            return;
        }

        match export_data(&path) {
            Ok(()) => {
                let size_str = std::fs::metadata(&path)
                    .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
                    .unwrap_or_else(|_| tr("saved").to_string());
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#4ade80'><b>{}</b> {} ({})</span>",
                    tr("Export Successful:"), glib::markup_escape_text(&path.to_string_lossy()), size_str
                ));
            }
            Err(e) => {
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#f87171'><b>{}</b> {}</span>",
                    tr("Export Failed:"), glib::markup_escape_text(&e)
                ));
            }
        }
    }

    pub(crate) fn execute_import_action(self: &Rc<Self>) {
        let path_str = self.data_import_entry.text().to_string();
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() || !path.exists() {
            self.data_modal_status_lbl.set_markup(&format!("<span foreground='#f87171'><b>{}</b> {}</span>", tr("Error:"), tr("Please select a valid .tar.gz backup file.")));
            return;
        }

        match import_data(&path) {
            Ok(()) => {
                // Reload settings from imported settings.json
                let imported_settings = load_app_settings();
                *self.search_engine.borrow_mut() = imported_settings.search_engine.clone();
                // Reload profile manager from imported profiles.txt
                reload_customize_from_disk();
                self.reload_css_editor();
                *self.profiles.borrow_mut() = ProfileManager::new(rail_data_dir());
                self.rebuild_profiles_modal_list();
                let active_id = self.profiles.borrow().active_profile().id;
                self.select_palette_profile(&active_id);

                // Reload session tabs
                self.restore_session();
                self.refresh_sidebar();
                self.refresh_data_modal_info();

                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#4ade80'><b>{}</b> {}</span>",
                    tr("Import Successful:"), tr("All workspaces, tabs, accounts & cookies restored!")
                ));
            }
            Err(e) => {
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#f87171'><b>{}</b> {}</span>",
                    tr("Import Failed:"), glib::markup_escape_text(&e)
                ));
            }
        }
    }
}
