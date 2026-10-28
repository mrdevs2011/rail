//! Settings, profiles and data panels.

use crate::*;

impl App {
    // ---- floating settings modal (Alt+Shift+S / Alt+P / Alt+I) --------

    pub(crate) fn settings_modal_open(&self) -> bool {
        self.settings_modal.is_visible()
    }

    pub(crate) fn open_settings_tab(self: &Rc<Self>, tab: SettingsTab) {
        if self.palette_open() {
            self.hide_palette();
        }
        self.current_settings_tab.set(tab);
        let tab_name = match tab {
            SettingsTab::General => "general",
            SettingsTab::Profiles => "profiles",
            SettingsTab::Extensions => "extensions",
            SettingsTab::Data => "data",
            SettingsTab::Privacy => "privacy",
            SettingsTab::Customize => "customize",
            SettingsTab::Update => "update",
            SettingsTab::About => "about",
        };
        self.settings_stack.set_visible_child_name(tab_name);

        for (btn_tab, btn) in self.settings_tab_buttons.borrow().iter() {
            if *btn_tab == tab {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }

        match tab {
            SettingsTab::General => {            }
            SettingsTab::Profiles => {
                self.rebuild_profiles_modal_list();
                self.new_profile_entry.set_text("");
                self.select_new_profile_color(0);
                self.update_create_profile_btn_state();
                self.new_profile_entry.grab_focus();
            }
            SettingsTab::Data => {
                self.refresh_data_modal_info();
                let timestamp = glib::DateTime::now_local()
                    .ok()
                    .and_then(|d| d.format("%Y%m%d_%H%M%S").ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "backup".to_string());
                let default_export = glib::home_dir().join(format!("rail_backup_{}.tar.gz", timestamp));
                self.data_export_entry.set_text(&default_export.to_string_lossy());
                self.data_modal_status_lbl.set_text("");
                self.data_export_entry.grab_focus();
            }
            SettingsTab::Customize => {
                self.reload_css_editor();
            }
            _ => {}
        }

        // Responsive: narrow windows get stacked sidebar (tablet/mobile)
        self.sync_settings_responsive();

        // Settings page: switch to existing settings tab or create a new one
        let settings_uri = settings_uri_for_tab(tab);
        self.exit_overview();

        let existing_settings = {
            let rows = self.rows.borrow();
            let mut found = None;
            'outer: for (r_idx, row) in rows.iter().enumerate() {
                for (c_idx, tab_item) in row.tabs.iter().enumerate() {
                    let u = tab_item.view.uri().map(|s| s.to_string()).unwrap_or_default();
                    let s = tab_item.saved_uri.borrow().clone();
                    if u.starts_with("rail://settings") || s.starts_with("rail://settings") {
                        found = Some((r_idx, c_idx));
                        break 'outer;
                    }
                }
            }
            found
        };

        if let Some((r, c)) = existing_settings {
            self.focus_coords(r, c);
            if let Some(v) = self.focused_view() {
                v.load_html(&settings_shell_page(), Some(settings_uri));
                self.set_view_saved_uri(&v, settings_uri);
            }
        } else {
            let prof_id = self.profiles.borrow().active_id().to_string();
            let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
            let view = Self::create_web_view(&session, &prof_id);
            view.load_html(&settings_shell_page(), Some(settings_uri));
            self.add_view(view.clone(), prof_id, false);
            self.set_view_saved_uri(&view, settings_uri);
        }
        self.pill_label.set_text(settings_uri);
        self.refresh_sidebar();
        self.sync_sidebar_active();
        self.reveal_focus();
        self.schedule_save_session();
        // Mark settings WebView keep_alive so suspend never freezes a black empty shell.
        if let Some(v) = self.focused_view() {
            let rows = self.rows.borrow();
            for row in rows.iter() {
                for tab in &row.tabs {
                    if tab.view == v {
                        tab.keep_alive.set(true);
                        break;
                    }
                }
            }
        }
        // Attach to the (new) focused tab overlay — not the whole strip
        self.sync_settings_chrome();
    }

    pub(crate) fn reload_css_editor(&self) {
        self.css_buffer.set_text(&load_html_css());
    }

    pub(crate) fn toggle_settings_modal(self: &Rc<Self>) {
        // Already on settings → close that tab (do NOT replace with start/new-tab page).
        let on_settings = self
            .focused_view()
            .and_then(|v| v.uri())
            .map(|u| u.starts_with("rail://settings"))
            .unwrap_or(false)
            || self
                .focused_view()
                .map(|v| self.saved_uri_of(&v).starts_with("rail://settings"))
                .unwrap_or(false);
        if self.settings_modal_open() || on_settings {
            if let Some(v) = self.focused_view() {
                let is_settings = v
                    .uri()
                    .map(|u| u.starts_with("rail://settings"))
                    .unwrap_or(false)
                    || self.saved_uri_of(&v).starts_with("rail://settings");
                if is_settings {
                    self.close_view(&v);
                    return;
                }
            }
            self.hide_settings_modal();
        } else {
            self.open_settings_tab(self.current_settings_tab.get());
        }
    }

    pub(crate) fn hide_settings_modal(self: &Rc<Self>) {
        self.settings_modal.set_visible(false);
        // Do not navigate the tab to start page — that felt like "new tab" after Alt+S.
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    pub(crate) fn profiles_modal_open(&self) -> bool {
        self.settings_modal.is_visible() && self.current_settings_tab.get() == SettingsTab::Profiles
    }

    pub(crate) fn toggle_profiles_modal(self: &Rc<Self>) {
        let is_on_profiles = self
            .focused_view()
            .map(|v| {
                let u = v.uri().map(|s| s.to_string()).unwrap_or_default();
                let s = self.saved_uri_of(&v);
                (u.starts_with("rail://settings") || s.starts_with("rail://settings"))
                    && self.current_settings_tab.get() == SettingsTab::Profiles
            })
            .unwrap_or(false);

        if is_on_profiles && self.settings_modal_open() {
            if let Some(v) = self.focused_view() {
                self.close_view(&v);
            } else {
                self.hide_settings_modal();
            }
        } else {
            self.open_settings_tab(SettingsTab::Profiles);
        }
    }

    pub(crate) fn toggle_data_modal(self: &Rc<Self>) {
        if self.settings_modal_open() && self.current_settings_tab.get() == SettingsTab::Data {
            self.hide_settings_modal();
        } else {
            self.open_settings_tab(SettingsTab::Data);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn clear_history_action(self: &Rc<Self>) {
        self.history.borrow_mut().clear();
        self.data_modal_status_lbl.set_markup(&format!(
            "<span foreground='#4ade80'><b>{}</b> {}</span>",
            tr("History Cleared:"), tr("All Omnibar history and suggestions have been purged.")
        ));
    }

    #[allow(dead_code)]
    pub(crate) fn clear_cache_action(self: &Rc<Self>) {
        let dir = rail_data_dir();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("profile_")).unwrap_or(false) {
                    let cache_dir = p.join("cache");
                    let _ = std::fs::remove_dir_all(&cache_dir);
                }
            }
        }
        self.data_modal_status_lbl.set_markup(&format!(
            "<span foreground='#4ade80'><b>{}</b> {}</span>",
            tr("Cache Cleared:"), tr("Profile network disk caches purged successfully.")
        ));
    }

    pub(crate) fn refresh_data_modal_info(&self) {
        let dir = rail_data_dir();
        let rows = self.rows.borrow();
        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let ws_count = rows.len();
        drop(rows);
        let prof_count = self.profiles.borrow().profiles().len();

        let info_text = trf(
            "Directory: {}\nStatus:    {} open tab(s) in {} workspace(s) | {} profiles (cookies/logins active)",
            &[&dir.display().to_string(), &total_tabs.to_string(), &ws_count.to_string(), &prof_count.to_string()],
        );
        self.data_modal_info_lbl.set_text(&info_text);
    }

    pub(crate) fn sync_settings_responsive(&self) {
        let w = self.settings_modal.width();
        let w = if w > 50 { w } else { self.win.width() };
        // Desktop (>= 820px): tabs on the left
        // Tablet / Mobile (< 820px): tabs on top
        let is_narrow = w > 0 && w < 820;
        if is_narrow {
            self.settings_modal.add_css_class("responsive-narrow");
            self.settings_body.set_orientation(gtk::Orientation::Vertical);
            self.settings_sidebar.set_orientation(gtk::Orientation::Horizontal);
            self.settings_sidebar_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
            self.settings_sidebar_scroll.set_hexpand(true);
            self.settings_sidebar_scroll.set_vexpand(false);
        } else {
            self.settings_modal.remove_css_class("responsive-narrow");
            self.settings_body.set_orientation(gtk::Orientation::Horizontal);
            self.settings_sidebar.set_orientation(gtk::Orientation::Vertical);
            self.settings_sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
            self.settings_sidebar_scroll.set_hexpand(false);
            self.settings_sidebar_scroll.set_vexpand(true);
        }
    }
}
