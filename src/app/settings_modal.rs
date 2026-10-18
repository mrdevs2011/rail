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
            SettingsTab::General => {
                self.tp_switch.set_active(self.touchpad_mode.get());
            }
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
        let w = self.win.width();
        if w > 0 && w < 900 {
            self.settings_modal.add_css_class("responsive-narrow");
        } else {
            self.settings_modal.remove_css_class("responsive-narrow");
        }

        // Settings page: new tab unless focused tab is already any rail://settings*
        let settings_uri = settings_uri_for_tab(tab);
        self.exit_overview();
        let on_settings = self
            .focused_view()
            .and_then(|v| v.uri())
            .map(|u| u.starts_with("rail://settings"))
            .unwrap_or(false);
        if on_settings {
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
        self.schedule_save_session();
        // Attach to the (new) focused tab overlay — not the whole strip
        self.sync_settings_chrome();
    }

    pub(crate) fn reload_css_editor(&self) {
        self.css_buffer.set_text(&load_html_css());
    }

    pub(crate) fn toggle_settings_modal(self: &Rc<Self>) {
        if self.settings_modal_open() {
            self.hide_settings_modal();
        } else {
            self.open_settings_tab(self.current_settings_tab.get());
        }
    }

    pub(crate) fn hide_settings_modal(self: &Rc<Self>) {
        self.settings_modal.set_visible(false);
        // Avoid leaving a blank shell: if this tab is settings, go back to start page
        // so Alt+S toggle does not show an empty black area.
        if let Some(v) = self.focused_view() {
            let uri = v.uri().map(|s| s.to_string()).unwrap_or_default();
            if uri.starts_with("rail://settings") {
                v.load_html(&start_page(), Some("rail://start/"));
                self.set_view_saved_uri(&v, "rail://start/");
                self.pill_label.set_text("rail://start/");
            }
            v.grab_focus();
        }
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    pub(crate) fn profiles_modal_open(&self) -> bool {
        self.settings_modal.is_visible() && self.current_settings_tab.get() == SettingsTab::Profiles
    }

    pub(crate) fn toggle_profiles_modal(self: &Rc<Self>) {
        if self.settings_modal_open() && self.current_settings_tab.get() == SettingsTab::Profiles {
            self.hide_settings_modal();
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
}
