//! Address palette, suggestions and navigation.

use crate::*;

impl App {
    pub(crate) fn show_palette(self: &Rc<Self>) {
        if self.focused_is_welcome() {
            if self.palette_open() {
                self.hide_palette();
            }
            self.focus_welcome_input(true);
            return;
        }
        self.palette_for_new_tab.set(false);
        let shown = self.focused_edit_uri();
        let shown = if shown == tr("Search or enter address") || shown.is_empty() {
            String::new()
        } else {
            shown
        };
        self.entry.set_text(&shown);

        let prof_id = self.profiles.borrow().active_id().to_string();
        self.select_palette_profile(&prof_id);
        self.rebuild_profile_popover();

        self.set_suggestions_visible(false);
        self.clear_suggestions_ui();
        *self.current_suggestions.borrow_mut() = Vec::new();
        self.selected_suggestion_idx.set(None);
        self.last_entry_len.set(self.entry.text().chars().count());

        self.update_palette_size();
        self.palette.set_visible(true);
        let on_welcome = self.focused_is_welcome();
        self.welcome_palette.set(on_welcome);
        if on_welcome {
            // Keep the omnibar glued to the welcome page every frame (tab strip scroll,
            // sidebar toggle, window resize) so it never floats away from the brand.
            thread_local! { static WELCOME_TICK: Cell<bool> = Cell::new(false); }
            if !WELCOME_TICK.with(|t| t.replace(true)) {
                let app = self.clone();
                self.palette.add_tick_callback(move |_, _| {
                    if app.palette_open() && app.welcome_palette.get() {
                        app.update_palette_size();
                        glib::ControlFlow::Continue
                    } else {
                        WELCOME_TICK.with(|t| t.set(false));
                        glib::ControlFlow::Break
                    }
                });
            }
            // Widgets may not be laid out yet (new tab / startup): re-pin once they are.
            for ms in [60u64, 250, 700] {
                let app = self.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
                    if app.palette_open() && app.welcome_palette.get() {
                        app.update_palette_size();
                    }
                });
            }
        }
        self.entry.grab_focus();
        if shown.is_empty() {
            // Welcome / empty: caret at start, nothing selected
            self.entry.set_position(0);
        } else {
            self.entry.select_region(0, -1);
        }
    }

    pub(crate) fn hide_palette(&self) {
        self.welcome_palette.set(false);
        self.set_suggestions_visible(false);
        self.clear_suggestions_ui();
        *self.current_suggestions.borrow_mut() = Vec::new();
        self.selected_suggestion_idx.set(None);
        self.profile_popover.popdown();
        self.palette.set_visible(false);
        if !self.profiles_modal_open() {
            if let Some(v) = self.focused_view() {
                v.grab_focus();
            }
        }
    }

    /// Show/hide the suggestions list together with its ScrolledWindow (so an empty
    /// omnibar has no tall blank area).
    pub(crate) fn set_suggestions_visible(&self, vis: bool) {
        self.palette_suggestions_box.set_visible(vis);
        let mut w = self.palette_suggestions_box.parent();
        while let Some(p) = w {
            if p.is::<gtk::ScrolledWindow>() {
                p.set_visible(vis);
                break;
            }
            w = p.parent();
        }
    }

    pub(crate) fn clear_suggestions_ui(&self) {
        while let Some(child) = self.palette_suggestions_box.first_child() {
            self.palette_suggestions_box.remove(&child);
        }
    }

    pub(crate) fn render_suggestions_ui(self: &Rc<Self>, suggestions: &[HistorySuggestion]) {
        self.clear_suggestions_ui();
        if suggestions.is_empty() {
            self.set_suggestions_visible(false);
            return;
        }

        let sel_idx = self.selected_suggestion_idx.get();

        for (idx, sug) in suggestions.iter().enumerate() {
            let item_btn = gtk::Button::new();
            item_btn.add_css_class("palette-sug-item");
            if Some(idx) == sel_idx {
                item_btn.add_css_class("selected");
            }

            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row_box.set_valign(gtk::Align::Center);

            let icon_name = match sug.kind {
                SuggestionKind::History => "document-open-recent-symbolic",
                SuggestionKind::Popular => "starred-symbolic",
                SuggestionKind::Search => "system-search-symbolic",
            };
            let icon = gtk::Image::from_icon_name(icon_name);
            icon.add_css_class("palette-sug-icon");
            row_box.append(&icon);

            let title_lbl = gtk::Label::new(Some(&sug.display_title));
            title_lbl.add_css_class("palette-sug-title");
            title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title_lbl.set_xalign(0.0);
            row_box.append(&title_lbl);

            if sug.kind != SuggestionKind::Search {
                let url_lbl = gtk::Label::new(Some(&sug.display_url));
                url_lbl.add_css_class("palette-sug-url");
                url_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                url_lbl.set_xalign(0.0);
                url_lbl.set_hexpand(true);
                row_box.append(&url_lbl);
            } else {
                title_lbl.set_hexpand(true);
            }

            let badge_text = match sug.kind {
                SuggestionKind::History => tr("History"),
                SuggestionKind::Popular => tr("Top"),
                SuggestionKind::Search => tr("Search"),
            };
            let badge = gtk::Label::new(Some(badge_text));
            badge.add_css_class("palette-sug-badge");
            badge.set_halign(gtk::Align::End);
            row_box.append(&badge);

            item_btn.set_child(Some(&row_box));

            let app = self.clone();
            let target = sug.target_url.clone();
            item_btn.connect_clicked(move |_| {
                app.navigate(&target);
            });

            self.palette_suggestions_box.append(&item_btn);
        }

        self.set_suggestions_visible(true);
    }

    pub(crate) fn handle_entry_changed(self: &Rc<Self>) {
        if self.is_autofilling.get() {
            return;
        }

        let text = self.entry.text().to_string();
        let cur_len = text.chars().count();
        let prev_len = self.last_entry_len.get();
        self.last_entry_len.set(cur_len);

        let is_typing_forward = cur_len > prev_len && !self.delete_key.get();
        self.selected_suggestion_idx.set(None);

        if text.trim().is_empty() {
            self.set_suggestions_visible(false);
            self.clear_suggestions_ui();
            *self.current_suggestions.borrow_mut() = Vec::new();
            return;
        }

        let suggestions = self.history.borrow().suggest(&text);
        if suggestions.is_empty() {
            self.set_suggestions_visible(false);
            self.clear_suggestions_ui();
            *self.current_suggestions.borrow_mut() = Vec::new();
            return;
        }

        // 1. Inline autofill if typing forward (not on backspace/deletion)
        if is_typing_forward {
            if let Some(first) = suggestions.first() {
                if first.kind != SuggestionKind::Search {
                    let clean_q = text.trim().to_lowercase();
                    let autofill = &first.autofill_text;
                    let autofill_lower = autofill.to_lowercase();
                    if autofill_lower.starts_with(&clean_q) && autofill.chars().count() > cur_len {
                        self.is_autofilling.set(true);
                        self.entry.set_text(autofill);
                        self.entry.select_region(cur_len as i32, -1);
                        self.is_autofilling.set(false);
                        // Foydalanuvchi yozgan uzunlikni eslaymiz (to'ldirilgan uzunlikni emas),
                        // aks holda keyingi harf "o'chirish" deb hisoblanib to'ldirish bir harf oralatib ishlardi.
                        self.last_entry_len.set(cur_len);
                    }
                }
            }
        }

        // 2. Render suggestions UI
        self.render_suggestions_ui(&suggestions);
        *self.current_suggestions.borrow_mut() = suggestions;
    }

    pub(crate) fn move_suggestion_selection(self: &Rc<Self>, delta: isize) -> bool {
        let sugs = self.current_suggestions.borrow();
        if sugs.is_empty() {
            return false;
        }
        let total = sugs.len() as isize;
        let cur = self.selected_suggestion_idx.get();

        let new_idx = match cur {
            None => {
                if delta > 0 {
                    0
                } else {
                    (total - 1) as usize
                }
            }
            Some(i) => {
                let next = i as isize + delta;
                if next < 0 {
                    (total - 1) as usize
                } else if next >= total {
                    0
                } else {
                    next as usize
                }
            }
        };
        drop(sugs);

        self.selected_suggestion_idx.set(Some(new_idx));

        let sugs = self.current_suggestions.borrow();
        if let Some(s) = sugs.get(new_idx) {
            self.is_autofilling.set(true);
            let display_text = if s.kind == SuggestionKind::Search {
                s.autofill_text.clone()
            } else {
                s.display_url.clone()
            };
            self.entry.set_text(&display_text);
            self.entry.set_position(-1);
            self.last_entry_len.set(display_text.chars().count());
            self.is_autofilling.set(false);
        }
        let sugs_clone = sugs.clone();
        drop(sugs);

        self.render_suggestions_ui(&sugs_clone);
        true
    }

    pub(crate) fn accept_autofill_or_next_suggestion(self: &Rc<Self>) -> bool {
        if let Some((start, end)) = self.entry.selection_bounds() {
            if start != end {
                self.entry.set_position(-1);
                self.entry.select_region(-1, -1);
                return true;
            }
        }
        self.move_suggestion_selection(1)
    }

    pub(crate) fn cancel_palette(self: &Rc<Self>) {
        // Always dismiss the omnibar/input first (Esc on new tab / Ctrl+L).
        self.palette_for_new_tab.set(false);
        self.hide_palette();

        // If this was an unused fresh start tab (created by new_tab, never navigated)
        // and other tabs exist, discard it — same as Chrome closing an empty new tab flow.
        // If it is the only tab, keep the start page and just close the input.
        let view = self.focused_view();
        let fresh = self.fresh.borrow().clone();
        if let (Some(v), Some(f)) = (view, fresh) {
            if v == f {
                let total_tabs: usize = self.rows.borrow().iter().map(|r| r.tabs.len()).sum();
                if total_tabs > 1 {
                    self.close_view(&v);
                } else {
                    *self.fresh.borrow_mut() = None;
                }
            }
        }
    }

    pub(crate) fn switch_focused_tab_profile(self: &Rc<Self>, target_profile_id: &str) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        let tab_info = rows.get(cur_r).and_then(|r| r.tabs.get(cur_c)).map(|t| (t.profile_id.clone(), t.view.uri()));
        drop(rows);

        let Some((current_pid, current_uri)) = tab_info else { return };
        if current_pid == target_profile_id {
            return;
        }

        let session = self.profiles.borrow_mut().get_or_create_session(target_profile_id);
        let new_view = Self::create_web_view(&session, target_profile_id);
        new_view.set_can_target(!self.ov.get());
        self.setup_view_signals(&new_view);

        let mut rows = self.rows.borrow_mut();
        if let Some(row) = rows.get_mut(cur_r) {
            if let Some(tab) = row.tabs.get_mut(cur_c) {
                tab.container.remove(&tab.view);
                tab.container.prepend(&new_view);
                new_view.add_css_class("tab");
                new_view.set_overflow(gtk::Overflow::Hidden);
                new_view.set_vexpand(true);
                new_view.set_hexpand(true);
                tab.view = new_view.clone();
                tab.profile_id = target_profile_id.to_string();
            }
        }
        drop(rows);

        if let Some(uri) = current_uri {
            if uri.starts_with("about:") || uri.is_empty() {
                new_view.load_html(&start_page(), Some("rail://start/"));
            } else {
                if is_internal_uri(&uri) {
                    load_internal_page(&new_view, &uri);
                } else {
                    new_view.load_uri(&uri);
                }
            }
        } else {
            new_view.load_html(&start_page(), Some("rail://start/"));
        }

        self.refresh_sidebar();
        self.schedule_save_session();
        self.focus_coords(cur_r, cur_c);
    }

    pub(crate) fn navigate(self: &Rc<Self>, text: &str) {
        let is_new_tab = self.palette_for_new_tab.get();
        self.palette_for_new_tab.set(false);

        if !text.trim().is_empty() {
            let target_profile_id = self.selected_palette_profile.borrow().clone();
            let uri = to_uri(text, &self.search_engine.borrow());
            self.history.borrow_mut().record_visit(&uri, "");

            // Internal pages always open in a new tab (target=_blank) with real rail:// URL
            if is_internal_uri(&uri) {
                let canon = normalize_internal_uri(&uri);
                self.profiles.borrow_mut().set_active(&target_profile_id);
                let session = self.profiles.borrow_mut().get_or_create_session(&target_profile_id);
                let view = Self::create_web_view(&session, &target_profile_id);
                load_internal_page(&view, &canon);
                self.add_view(view.clone(), target_profile_id, false);
                self.set_view_saved_uri(&view, &canon);
                self.pill_label.set_text(&canon);
                self.hide_palette();
                if canon.starts_with("rail://settings") {
                    let tab = settings_tab_from_uri(&canon).unwrap_or(SettingsTab::General);
                    // Show settings UI without creating yet another tab
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
                    self.settings_modal.set_visible(true);
                } else {
                    self.hide_settings_modal();
                }
                self.refresh_sidebar();
                self.schedule_save_session();
                let _ = is_new_tab; // always new-tab for internal
                return;
            }

            if is_new_tab {
                self.profiles.borrow_mut().set_active(&target_profile_id);
                self.new_tab_with_url_and_profile(&uri, &target_profile_id);
            } else {
                let cur_r = self.focus_row.get();
                let cur_c = self.focus_col.get();

                let needs_view_recreation = {
                    let rows = self.rows.borrow();
                    rows.get(cur_r)
                        .and_then(|r| r.tabs.get(cur_c))
                        .map(|t| t.profile_id != target_profile_id)
                        .unwrap_or(false)
                };

                if needs_view_recreation {
                    self.profiles.borrow_mut().set_active(&target_profile_id);
                    self.switch_focused_tab_profile(&target_profile_id);
                }

                if let Some(v) = self.focused_view() {
                    self.pill_label.set_text(&host_of(&uri).unwrap_or(uri.clone()));
                    if is_internal_uri(&uri) {
                        load_internal_page(&v, &uri);
                    } else {
                        v.load_uri(&uri);
                    }
                    *self.fresh.borrow_mut() = None;
                    self.refresh_sidebar();
                    self.schedule_save_session();
                }
            }
        }
        self.hide_palette();
    }
}
