//! Closing tabs and moving focus between tabs, columns and rows.

use crate::*;

impl App {
    pub(crate) fn close_view(self: &Rc<Self>, view: &WebView) {
        let Some((r_idx, c_idx)) = self.find_view_coords(view) else { return };
        // Closing the settings tab (or any tab) dismisses the settings chrome
        if self.settings_modal_open() {
            let closing_settings = view
                .uri()
                .map(|u| u.starts_with("rail://settings"))
                .unwrap_or(false)
                || self.saved_uri_of(view).starts_with("rail://settings");
            if closing_settings {
                self.settings_modal.set_visible(false);
            }
        }
        if self.fresh.borrow().as_ref() == Some(view) {
            *self.fresh.borrow_mut() = None;
        }

        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let is_closing_focused = r_idx == cur_r && c_idx == cur_c;

        let mut rows = self.rows.borrow_mut();
        if r_idx >= rows.len() || c_idx >= rows[r_idx].tabs.len() {
            return;
        }
        let tab = rows[r_idx].detach(c_idx);
        self.strip.remove(&tab.container);

        let row_removed = if rows[r_idx].tabs.is_empty() && rows.len() > 1 {
            rows.remove(r_idx);
            true
        } else {
            false
        };

        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let total_rows = rows.len();
        drop(rows);

        // Free the web process right away instead of waiting for the WebView to be finalized.
        tab.view.terminate_web_process();

        self.sync_strip();

        if total_tabs == 0 {
            self.save_session_immediate();
            self.win.close();
            return;
        }

        if is_closing_focused {
            let next_r = r_idx.min(total_rows.saturating_sub(1));
            let rows = self.rows.borrow();
            if rows.is_empty() || rows[next_r].tabs.is_empty() {
                if let Some((first_r, _)) = rows.iter().enumerate().find(|(_, r)| !r.tabs.is_empty()) {
                    let next_c = c_idx.min(rows[first_r].tabs.len().saturating_sub(1));
                    drop(rows);
                    self.focus_coords(first_r, next_c);
                } else {
                    drop(rows);
                    self.new_tab();
                }
            } else {
                let next_c = c_idx.min(rows[next_r].tabs.len().saturating_sub(1));
                drop(rows);
                self.focus_coords(next_r, next_c);
            }
        } else {
            let mut new_r = cur_r;
            let mut new_c = cur_c;

            if row_removed && r_idx < cur_r {
                new_r = cur_r.saturating_sub(1);
            }
            if r_idx == cur_r && c_idx < cur_c {
                new_c = cur_c.saturating_sub(1);
            }

            self.focus_row.set(new_r);
            self.focus_col.set(new_c);

            self.refresh_sidebar();
            self.update_window_title();
            self.schedule_save_session();
            if self.map_open() {
                self.trigger_map_animation();
            }
        }
    }

    pub(crate) fn focus_coords(self: &Rc<Self>, r_idx: usize, c_idx: usize) {
        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }
        let r_clamped = r_idx.min(rows.len() - 1);
        if rows[r_clamped].tabs.is_empty() {
            return;
        }
        let c_clamped = c_idx.min(rows[r_clamped].tabs.len() - 1);
        self.focus_row.set(r_clamped);
        self.focus_col.set(c_clamped);
        self.row_last_col.borrow_mut().insert(r_clamped, c_clamped);

        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                if ri == r_clamped && ci == c_clamped {
                    tab.view.add_css_class("focused");
                    tab.last_active.set(std::time::Instant::now());
                    // Uyg'otish: sahifani qayta yukla; freeze load tugaguncha turadi
                    if tab.suspended.get() {
                        // Wake: keep freeze visible until load finishes (clear_tab_freeze on is_loading false).
                        tab.suspended.set(false);
                        let uri = tab.saved_uri.borrow().clone();
                        if !uri.is_empty() {
                            // Cover dead WebView only when we have a real last-frame picture.
                            tab.freeze.set_visible(tab.freeze.paintable().is_some());
                            if is_internal_uri(&uri) {
                                load_internal_page(&tab.view, &uri);
                            } else {
                                tab.view.load_uri(&uri);
                            }
                        } else {
                            tab.freeze.set_visible(false);
                            tab.freeze.set_paintable(Option::<&gdk::Paintable>::None);
                        }
                    }
                } else {
                    if tab.view.has_css_class("focused") {
                        tab.last_active.set(std::time::Instant::now());
                    }
                    tab.view.remove_css_class("focused");
                    // Uxlatish fokusga emas, ko'rinishga bog'liq: suspend_hidden_tabs davriy tekshiradi
                }
            }
        }

        if let Some(tab) = rows[r_clamped].tabs.get(c_clamped) {
            if !tab.view.is_loading() {
                self.load_bar.set_visible(false);
            }
        }
        drop(rows);

        // Overview navigation (h/j/k/l, arrows) must stay snappy: only move the camera.
        // Sidebar rebuild, session save, tab wake-up etc. run once on exit_overview().
        if self.ov.get() {
            self.reveal_focus();
            // Keep the left bar's active tab and the minimap in step with the overview cursor.
            self.sync_sidebar_active();
            if self.map_open() {
                self.trigger_map_animation();
            }
            return;
        }

        self.update_load_bar();
        self.reveal_focus();
        self.refresh_sidebar();
        self.update_window_title();
        self.schedule_save_session();

        if self.map_open() {
            self.trigger_map_animation();
        }

        if self.devtools_open.get() {
            self.show_devtools();
        }

        // Settings chrome rides *inside* the focused tab overlay so tab-switch
        // and overview treat it like any other page (no floating strip-level panel).
        self.sync_settings_chrome();
        self.sync_welcome_palette();

        if !self.palette_open() && !self.ov.get() && !self.settings_modal_open() {
            if let Some(v) = self.focused_view() {
                v.grab_focus();
            }
        }
    }

    /// Keep settings UI on the settings tab's Overlay (like downloads lives in its
    /// WebView) so overview / tab-switch show real content, not an empty shell.
    pub(crate) fn sync_settings_chrome(self: &Rc<Self>) {
        // Prefer the focused tab if it is settings; otherwise any open settings tab.
        let mut target: Option<(webkit6::WebView, SettingsTab)> = None;
        if let Some(v) = self.focused_view() {
            let uri = v.uri().map(|s| s.to_string()).unwrap_or_default();
            let saved = self.saved_uri_of(&v);
            let check = if uri.starts_with("rail://settings") {
                uri.clone()
            } else {
                saved
            };
            if let Some(section) = settings_tab_from_uri(&check) {
                target = Some((v, section));
            }
        }
        if target.is_none() {
            let rows = self.rows.borrow();
            'outer: for row in rows.iter() {
                for tab in &row.tabs {
                    let uri = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
                    let saved = tab.saved_uri.borrow().clone();
                    let check = if uri.starts_with("rail://settings") {
                        uri
                    } else {
                        saved
                    };
                    if let Some(section) = settings_tab_from_uri(&check) {
                        target = Some((tab.view.clone(), section));
                        break 'outer;
                    }
                }
            }
        }

        let Some((v, section)) = target else {
            self.settings_modal.set_visible(false);
            return;
        };

        if self.current_settings_tab.get() != section {
            self.current_settings_tab.set(section);
            let tab_name = match section {
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
                if *btn_tab == section {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
        }

        // Stay parented on the settings tab overlay even when another tab is focused,
        // so overview shows settings content the same way it shows downloads HTML.
        if let Some(parent) = v.parent() {
            if let Ok(ov) = parent.downcast::<gtk::Overlay>() {
                let already = self
                    .settings_modal
                    .parent()
                    .map(|p| p == *ov.upcast_ref::<gtk::Widget>())
                    .unwrap_or(false);
                if !already {
                    if let Some(old) = self.settings_modal.parent() {
                        if let Ok(old_ov) = old.downcast::<gtk::Overlay>() {
                            old_ov.remove_overlay(&self.settings_modal);
                        }
                    }
                    self.settings_modal.set_halign(gtk::Align::Fill);
                    self.settings_modal.set_valign(gtk::Align::Fill);
                    self.settings_modal.set_hexpand(true);
                    self.settings_modal.set_vexpand(true);
                    ov.add_overlay(&self.settings_modal);
                }
            }
        }

        self.settings_modal.set_visible(true);
        // If a previous suspend left a black freeze over settings, clear it and keep process alive.
        {
            let rows = self.rows.borrow();
            for row in rows.iter() {
                for tab in &row.tabs {
                    if tab.view == v {
                        tab.keep_alive.set(true);
                        if tab.suspended.get() {
                            tab.suspended.set(false);
                            tab.freeze.set_visible(false);
                            tab.freeze.set_paintable(None::<&gdk::Paintable>);
                            let uri = tab.saved_uri.borrow().clone();
                            if uri.starts_with("rail://settings") {
                                tab.view.load_html(&settings_shell_page(), Some(&uri));
                            }
                        } else {
                            tab.freeze.set_visible(false);
                        }
                        break;
                    }
                }
            }
        }
        // Clicks only when this tab is focused and not in overview (overview picks the card)
        let focused_is_settings = self
            .focused_view()
            .map(|fv| fv == v)
            .unwrap_or(false);
        self.settings_modal
            .set_can_target(focused_is_settings && !self.ov.get());
        // GTK Stack/TextView can go blank after tab switches — force a resize when active.
        if focused_is_settings {
            self.settings_modal.queue_resize();
            self.settings_stack.queue_resize();
            self.sync_settings_responsive();
        }
    }

    /// Focus the neighbouring column, landing on the window at the same height when possible.
    pub(crate) fn focus_column_left(self: &Rc<Self>) {
        let (r, c) = (self.focus_row.get(), self.focus_col.get());
        let target = {
            let rows = self.rows.borrow();
            rows.get(r).and_then(|row| {
                if c >= row.tabs.len() {
                    return None;
                }
                let (s, _) = row.col_span(c);
                if s == 0 {
                    return None;
                }
                let (ls, le) = row.col_span(s - 1);
                Some(ls + (c - s).min(le - ls - 1))
            })
        };
        if let Some(t) = target {
            self.focus_coords(r, t);
        }
    }

    pub(crate) fn focus_column_right(self: &Rc<Self>) {
        let (r, c) = (self.focus_row.get(), self.focus_col.get());
        let target = {
            let rows = self.rows.borrow();
            rows.get(r).and_then(|row| {
                if c >= row.tabs.len() {
                    return None;
                }
                let (s, e) = row.col_span(c);
                if e >= row.tabs.len() {
                    return None;
                }
                let (rs, re) = row.col_span(e);
                Some(rs + (c - s).min(re - rs - 1))
            })
        };
        if let Some(t) = target {
            self.focus_coords(r, t);
        }
    }

    /// Prefer the last tab the user focused in `target_r`; else the same column ordinal as `from_c`.
    pub(crate) fn col_for_workspace(&self, from_r: usize, from_c: usize, target_r: usize) -> usize {
        let rows = self.rows.borrow();
        if target_r >= rows.len() || rows[target_r].tabs.is_empty() {
            return 0;
        }
        let n = rows[target_r].tabs.len();
        // 1) Last focused tab in that workspace (if still valid)
        if let Some(&c) = self.row_last_col.borrow().get(&target_r) {
            if c < n {
                return c;
            }
        }
        // 2) Same column ordinal + stack depth as the source tab
        if from_r < rows.len() && from_c < rows[from_r].tabs.len() {
            let from_cols = rows[from_r].columns();
            let to_cols = rows[target_r].columns();
            if let Some(ci) = from_cols.iter().position(|(s, len)| from_c >= *s && from_c < s + len) {
                let (fs, _) = from_cols[ci];
                let depth = from_c - fs;
                if let Some(&(ts, tlen)) = to_cols.get(ci).or_else(|| to_cols.last()) {
                    return ts + depth.min(tlen.saturating_sub(1));
                }
            }
        }
        from_c.min(n.saturating_sub(1))
    }

    /// Window above inside a stacked column; at the top of the column: one workspace up,
    /// restoring the last focused tab in that workspace.
    pub(crate) fn focus_row_up(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let above = {
            let rows = self.rows.borrow();
            rows.get(cur_r)
                .filter(|row| cur_c < row.tabs.len())
                .and_then(|row| (cur_c > row.col_span(cur_c).0).then(|| cur_c - 1))
        };
        if let Some(c) = above {
            self.focus_coords(cur_r, c);
            return;
        }
        if cur_r > 0 {
            let dest_c = self.col_for_workspace(cur_r, cur_c, cur_r - 1);
            self.focus_coords(cur_r - 1, dest_c);
        }
    }

    /// Window below inside a stacked column; at the bottom: one workspace down,
    /// restoring the last focused tab in that workspace.
    pub(crate) fn focus_row_down(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let below = {
            let rows = self.rows.borrow();
            rows.get(cur_r)
                .filter(|row| cur_c < row.tabs.len())
                .and_then(|row| (cur_c + 1 < row.col_span(cur_c).1).then(|| cur_c + 1))
        };
        if let Some(c) = below {
            self.focus_coords(cur_r, c);
            return;
        }
        let n_rows = self.rows.borrow().len();
        if cur_r + 1 < n_rows {
            let dest_c = self.col_for_workspace(cur_r, cur_c, cur_r + 1);
            self.focus_coords(cur_r + 1, dest_c);
        }
    }

    pub(crate) fn focus_row(self: &Rc<Self>, target_r: usize) {
        let n = self.rows.borrow().len();
        if target_r < n {
            let cur_r = self.focus_row.get();
            let cur_c = self.focus_col.get();
            let dest_c = self.col_for_workspace(cur_r, cur_c, target_r);
            self.focus_coords(target_r, dest_c);
        } else if target_r == n {
            self.split_vertical();
        }
    }

    pub(crate) fn focus_tab_index(self: &Rc<Self>, target_c: usize) {
        if self.overview_open() {
            self.exit_overview();
        }
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r < rows.len() && !rows[cur_r].tabs.is_empty() {
            let num_tabs = rows[cur_r].tabs.len();
            if target_c < num_tabs {
                drop(rows);
                self.focus_coords(cur_r, target_c);
            }
        }
    }

    pub(crate) fn focus_last_tab(self: &Rc<Self>) {
        if self.overview_open() {
            self.exit_overview();
        }
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r < rows.len() && !rows[cur_r].tabs.is_empty() {
            let last_c = rows[cur_r].tabs.len() - 1;
            drop(rows);
            self.focus_coords(cur_r, last_c);
        }
    }

    /// Ctrl+Tab / Ctrl+Shift+Tab: cycle tabs inside the current workspace (row), wrapping around.
    pub(crate) fn cycle_tab(self: &Rc<Self>, dir: isize) {
        if self.overview_open() {
            self.exit_overview();
        }
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let n = {
            let rows = self.rows.borrow();
            if cur_r >= rows.len() { return; }
            rows[cur_r].tabs.len()
        };
        if n < 2 { return; }
        let next = (cur_c as isize + dir).rem_euclid(n as isize) as usize;
        self.focus_coords(cur_r, next);
    }
}
