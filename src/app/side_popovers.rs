//! Floating fast popovers for Profile, Downloads, and Settings with expand-to-full-page buttons.

use crate::*;

impl App {
    pub(crate) fn sync_head_profile(&self) {
        let p = self.profiles.borrow().active_profile();
        let fg = p.text_color();
        self.head_profile_dot.set_markup(&format!(
            "<span foreground='{}' font_weight='800'>{}</span>",
            fg,
            glib::markup_escape_text(&p.initial_letter())
        ));
        for i in 0..12 {
            self.head_profile_dot.remove_css_class(&format!("prof-bg-{}", i));
        }
        self.head_profile_dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
        self.head_profile_btn.set_tooltip_text(Some(&format!("{}: {}", tr("Profiles"), p.name)));
    }

    // =========================================================================
    // 1. Profile Float Popover
    // =========================================================================

    pub(crate) fn toggle_side_profile_popover(self: &Rc<Self>) {
        if self.side_profile_popover.is_visible() {
            self.side_profile_popover.popdown();
        } else {
            self.side_downloads_popover.popdown();
            self.side_settings_popover.popdown();
            self.rebuild_side_profile_popover();
            self.side_profile_popover.popup();
        }
    }

    pub(crate) fn rebuild_side_profile_popover(self: &Rc<Self>) {
        let pop_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        pop_box.add_css_class("side-popover-content");
        pop_box.set_size_request(310, -1);

        // Header: Title + Expand Button + Close Button
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        header.add_css_class("side-popover-header");

        let title_lbl = gtk::Label::new(Some(tr("Profiles")));
        title_lbl.add_css_class("side-popover-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);
        header.append(&title_lbl);

        // Expand to full Settings > Profiles
        let expand_btn = gtk::Button::from_icon_name("window-maximize-symbolic");
        expand_btn.add_css_class("side-popover-btn");
        expand_btn.set_tooltip_text(Some(tr("Open in Settings (Alt+P)")));
        let a_exp = self.clone();
        expand_btn.connect_clicked(move |_| {
            a_exp.side_profile_popover.popdown();
            a_exp.open_settings_tab(SettingsTab::Profiles);
        });
        header.append(&expand_btn);

        let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("side-popover-btn");
        close_btn.set_tooltip_text(Some(tr("Close (Esc)")));
        let a_close = self.clone();
        close_btn.connect_clicked(move |_| {
            a_close.side_profile_popover.popdown();
        });
        header.append(&close_btn);

        pop_box.append(&header);

        // Profiles list
        let (profiles, active_id) = {
            let mgr = self.profiles.borrow();
            (mgr.profiles().to_vec(), mgr.active_id().to_string())
        };

        let list_box = gtk::Box::new(gtk::Orientation::Vertical, 3);
        list_box.set_margin_start(4);
        list_box.set_margin_end(4);

        for p in profiles {
            let row_btn = gtk::Button::new();
            row_btn.add_css_class("profile-item-btn");
            if p.id == active_id {
                row_btn.add_css_class("selected");
            }

            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);

            let dot = gtk::Label::new(None);
            let fg = p.text_color();
            dot.set_markup(&format!(
                "<span foreground='{}' font_weight='800'>{}</span>",
                fg,
                glib::markup_escape_text(&p.initial_letter())
            ));
            dot.add_css_class("profile-item-dot");
            dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
            dot.set_valign(gtk::Align::Center);
            dot.set_halign(gtk::Align::Center);
            dot.set_width_request(24);
            dot.set_height_request(24);

            let name_col = gtk::Box::new(gtk::Orientation::Vertical, 1);
            name_col.set_hexpand(true);

            let name_lbl = gtk::Label::new(Some(&p.display_name()));
            name_lbl.add_css_class("profile-item-name");
            name_lbl.set_xalign(0.0);

            let persona_lbl = gtk::Label::new(Some(tr("Isolated storage")));
            persona_lbl.add_css_class("profile-item-persona");
            persona_lbl.set_xalign(0.0);

            name_col.append(&name_lbl);
            name_col.append(&persona_lbl);

            row_box.append(&dot);
            row_box.append(&name_col);

            if p.id == active_id {
                let check = gtk::Image::from_icon_name("object-select-symbolic");
                check.add_css_class("profile-item-check");
                row_box.append(&check);
            }

            row_btn.set_child(Some(&row_box));

            let app = self.clone();
            let pid = p.id.clone();
            row_btn.connect_clicked(move |_| {
                app.profiles.borrow_mut().set_active(&pid);
                app.switch_focused_tab_profile(&pid);
                app.select_palette_profile(&pid);
                app.sync_head_profile();
                app.rebuild_side_profile_popover();
                app.side_profile_popover.popdown();
            });

            list_box.append(&row_btn);
        }

        pop_box.append(&list_box);

        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep.set_margin_top(4);
        sep.set_margin_bottom(4);
        pop_box.append(&sep);

        // Add Profile button
        let add_btn = gtk::Button::new();
        add_btn.add_css_class("profile-item-btn");
        let add_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
        let add_lbl = gtk::Label::new(Some(tr("Create Profile")));
        add_lbl.add_css_class("profile-item-name");
        add_box.append(&add_icon);
        add_box.append(&add_lbl);
        add_btn.set_child(Some(&add_box));

        let app_add = self.clone();
        add_btn.connect_clicked(move |_| {
            app_add.side_profile_popover.popdown();
            app_add.open_settings_tab(SettingsTab::Profiles);
        });
        pop_box.append(&add_btn);

        self.side_profile_popover.set_child(Some(&pop_box));
    }

    // =========================================================================
    // 2. Downloads Float Popover
    // =========================================================================

    pub(crate) fn toggle_side_downloads_popover(self: &Rc<Self>) {
        if self.side_downloads_popover.is_visible() {
            self.side_downloads_popover.popdown();
        } else {
            self.side_profile_popover.popdown();
            self.side_settings_popover.popdown();
            self.rebuild_side_downloads_popover();
            self.side_downloads_popover.popup();

            // Auto-refresh active downloads progress while visible
            let app = self.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(600), move || {
                if !app.side_downloads_popover.is_visible() {
                    return glib::ControlFlow::Break;
                }
                let has_active = crate::DOWNLOADS.with(|d| {
                    d.borrow()
                        .iter()
                        .any(|item| item.status == "downloading")
                });
                if has_active {
                    app.rebuild_side_downloads_popover();
                }
                glib::ControlFlow::Continue
            });
        }
    }

    pub(crate) fn open_downloads_full_page(self: &Rc<Self>) {
        self.side_downloads_popover.popdown();
        self.open_downloads();
    }

    pub(crate) fn rebuild_side_downloads_popover(self: &Rc<Self>) {
        crate::load_downloads_from_disk();

        let pop_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        pop_box.add_css_class("side-popover-content");
        pop_box.set_size_request(330, -1);

        // Header: Title + Clear All + Expand + Close
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        header.add_css_class("side-popover-header");

        let title_lbl = gtk::Label::new(Some(tr("Downloads")));
        title_lbl.add_css_class("side-popover-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);
        header.append(&title_lbl);

        let dl_count = crate::DOWNLOADS.with(|d| d.borrow().len());

        if dl_count > 0 {
            let clear_btn = gtk::Button::with_label(tr("Clear All"));
            clear_btn.add_css_class("side-popover-clear-btn");
            let a_clr = self.clone();
            clear_btn.connect_clicked(move |_| {
                let a_inner = a_clr.clone();
                a_clr.show_confirm_dialog(
                    tr("Clear Download History"),
                    tr("Are you sure you want to clear all download history?"),
                    tr("Clear All"),
                    true,
                    move || {
                        crate::clear_all_downloads();
                        a_inner.rebuild_side_downloads_popover();
                    },
                );
            });
            header.append(&clear_btn);
        }

        // Expand to full Downloads tab
        let expand_btn = gtk::Button::from_icon_name("window-maximize-symbolic");
        expand_btn.add_css_class("side-popover-btn");
        expand_btn.set_tooltip_text(Some(tr("Open full Downloads page")));
        let a_exp = self.clone();
        expand_btn.connect_clicked(move |_| {
            a_exp.open_downloads_full_page();
        });
        header.append(&expand_btn);

        let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("side-popover-btn");
        close_btn.set_tooltip_text(Some(tr("Close (Esc)")));
        let a_close = self.clone();
        close_btn.connect_clicked(move |_| {
            a_close.side_downloads_popover.popdown();
        });
        header.append(&close_btn);

        pop_box.append(&header);

        // Downloads list in scrolled window
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_max_content_height(340);
        scroll.set_propagate_natural_height(true);

        let list_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        list_box.set_margin_start(4);
        list_box.set_margin_end(4);

        let items: Vec<crate::DownloadEntry> = crate::DOWNLOADS.with(|d| d.borrow().clone());

        if items.is_empty() {
            let empty_lbl = gtk::Label::new(Some(tr("No downloads yet")));
            empty_lbl.add_css_class("side-popover-empty");
            empty_lbl.set_margin_top(24);
            empty_lbl.set_margin_bottom(24);
            list_box.append(&empty_lbl);
        } else {
            for e in items.iter().rev().take(15) {
                let row_card = gtk::Box::new(gtk::Orientation::Vertical, 4);
                row_card.add_css_class("side-dl-item");

                // Top: Filename and remove '✕'
                let top_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);

                let fname_btn = gtk::Button::new();
                fname_btn.add_css_class("side-dl-name-btn");
                fname_btn.set_hexpand(true);
                let fname_lbl = gtk::Label::new(Some(&e.filename));
                fname_lbl.add_css_class("side-dl-name");
                fname_lbl.set_xalign(0.0);
                fname_lbl.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
                fname_btn.set_child(Some(&fname_lbl));

                let path_c = e.path.clone();
                fname_btn.connect_clicked(move |_| {
                    let uri = format!("file://{}", path_c);
                    let _ = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>);
                });
                top_row.append(&fname_btn);

                let rem_btn = gtk::Button::new();
                rem_btn.add_css_class("side-dl-rem-btn");
                let x_lbl = gtk::Label::new(Some("✕"));
                rem_btn.set_child(Some(&x_lbl));
                rem_btn.set_tooltip_text(Some(tr("Remove from history")));

                let eid = e.id;
                let a_rem = self.clone();
                rem_btn.connect_clicked(move |_| {
                    crate::remove_download(eid);
                    a_rem.rebuild_side_downloads_popover();
                });
                top_row.append(&rem_btn);
                row_card.append(&top_row);

                // Progress bar if downloading
                if e.status == "downloading" {
                    let prog = gtk::ProgressBar::new();
                    prog.add_css_class("dl-toast-prog");
                    prog.set_fraction(e.progress);
                    row_card.append(&prog);
                }

                // Meta row: size & status
                let meta_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                let pct = (e.progress * 100.0).round() as i32;
                let size_str = if e.total > 0 {
                    format!("{} / {} · {}%", crate::format_bytes(e.received), crate::format_bytes(e.total), pct)
                } else if e.received > 0 {
                    crate::format_bytes(e.received)
                } else {
                    e.status.clone()
                };
                let meta_lbl = gtk::Label::new(Some(&size_str));
                meta_lbl.add_css_class("side-dl-meta");
                meta_lbl.set_xalign(0.0);
                meta_row.append(&meta_lbl);

                row_card.append(&meta_row);
                list_box.append(&row_card);
            }
        }

        scroll.set_child(Some(&list_box));
        pop_box.append(&scroll);

        self.side_downloads_popover.set_child(Some(&pop_box));
    }

    // =========================================================================
    // 3. Settings Float Popover
    // =========================================================================

    pub(crate) fn toggle_side_settings_popover(self: &Rc<Self>) {
        if self.side_settings_popover.is_visible() {
            self.side_settings_popover.popdown();
        } else {
            self.side_profile_popover.popdown();
            self.side_downloads_popover.popdown();
            self.rebuild_side_settings_popover();
            self.side_settings_popover.popup();
        }
    }

    pub(crate) fn show_settings_modal_full(self: &Rc<Self>) {
        self.side_settings_popover.popdown();
        self.open_settings_tab(SettingsTab::General);
    }

    pub(crate) fn rebuild_side_settings_popover(self: &Rc<Self>) {
        let pop_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        pop_box.add_css_class("side-popover-content");
        pop_box.set_size_request(320, -1);

        // Header: Title + Expand + Close
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        header.add_css_class("side-popover-header");

        let title_lbl = gtk::Label::new(Some(tr("Quick Settings")));
        title_lbl.add_css_class("side-popover-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);
        header.append(&title_lbl);

        // Expand to full Settings Modal
        let expand_btn = gtk::Button::from_icon_name("window-maximize-symbolic");
        expand_btn.add_css_class("side-popover-btn");
        expand_btn.set_tooltip_text(Some(tr("Expand to Full Settings")));
        let a_exp = self.clone();
        expand_btn.connect_clicked(move |_| {
            a_exp.show_settings_modal_full();
        });
        header.append(&expand_btn);

        let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("side-popover-btn");
        close_btn.set_tooltip_text(Some(tr("Close (Esc)")));
        let a_close = self.clone();
        close_btn.connect_clicked(move |_| {
            a_close.side_settings_popover.popdown();
        });
        header.append(&close_btn);

        pop_box.append(&header);

        // Quick Settings rows
        let list_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        list_box.set_margin_start(4);
        list_box.set_margin_end(4);

        // General
        let row_gen = self.create_popover_action_row("emblem-system-symbolic", tr("General"), "Search Engine, Home");
        let a_g = self.clone();
        row_gen.connect_clicked(move |_| {
            a_g.side_settings_popover.popdown();
            a_g.open_settings_tab(SettingsTab::General);
        });
        list_box.append(&row_gen);

        // Appearance / Customize
        let row_app = self.create_popover_action_row("applications-graphics-symbolic", tr("Customize"), "Themes, Layout, CSS");
        let a_ap = self.clone();
        row_app.connect_clicked(move |_| {
            a_ap.side_settings_popover.popdown();
            a_ap.open_settings_tab(SettingsTab::Customize);
        });
        list_box.append(&row_app);

        // Privacy
        let row_priv = self.create_popover_action_row("security-high-symbolic", tr("Privacy"), "Shields, AdBlock, Cookies");
        let a_pr = self.clone();
        row_priv.connect_clicked(move |_| {
            a_pr.side_settings_popover.popdown();
            a_pr.open_settings_tab(SettingsTab::Privacy);
        });
        list_box.append(&row_priv);

        // Profiles / Accounts
        let row_prof = self.create_popover_action_row("avatar-default-symbolic", tr("Profiles"), "Multiple Identities");
        let a_pf = self.clone();
        row_prof.connect_clicked(move |_| {
            a_pf.side_settings_popover.popdown();
            a_pf.open_settings_tab(SettingsTab::Profiles);
        });
        list_box.append(&row_prof);

        // Shortcuts
        let row_sc = self.create_popover_action_row("input-keyboard-symbolic", tr("Shortcuts"), "Key Bindings & Navigation");
        let a_sc = self.clone();
        row_sc.connect_clicked(move |_| {
            a_sc.side_settings_popover.popdown();
            a_sc.open_settings_tab(SettingsTab::Customize);
        });
        list_box.append(&row_sc);

        // Data & History
        let row_data = self.create_popover_action_row("drive-harddisk-symbolic", tr("Data"), "History, Cache, Sync");
        let a_dt = self.clone();
        row_data.connect_clicked(move |_| {
            a_dt.side_settings_popover.popdown();
            a_dt.open_settings_tab(SettingsTab::Data);
        });
        list_box.append(&row_data);

        pop_box.append(&list_box);

        self.side_settings_popover.set_child(Some(&pop_box));
    }

    fn create_popover_action_row(&self, icon_name: &str, title: &str, subtitle: &str) -> gtk::Button {
        let btn = gtk::Button::new();
        btn.add_css_class("side-settings-row-btn");

        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);

        let icon = gtk::Image::from_icon_name(icon_name);
        icon.add_css_class("side-settings-row-icon");
        row_box.append(&icon);

        let col = gtk::Box::new(gtk::Orientation::Vertical, 1);
        col.set_hexpand(true);

        let title_lbl = gtk::Label::new(Some(title));
        title_lbl.add_css_class("side-settings-row-title");
        title_lbl.set_xalign(0.0);

        let sub_lbl = gtk::Label::new(Some(subtitle));
        sub_lbl.add_css_class("side-settings-row-sub");
        sub_lbl.set_xalign(0.0);

        col.append(&title_lbl);
        col.append(&sub_lbl);
        row_box.append(&col);

        let arrow = gtk::Image::from_icon_name("go-next-symbolic");
        arrow.add_css_class("side-settings-row-arrow");
        row_box.append(&arrow);

        btn.set_child(Some(&row_box));
        btn
    }
}
