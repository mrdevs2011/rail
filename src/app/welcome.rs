//! Welcome page search and palette profile picker.

use crate::*;

impl App {
    // ---- address palette & profiles --------------------------------------

    pub(crate) fn select_palette_profile(&self, id: &str) {
        *self.selected_palette_profile.borrow_mut() = id.to_string();
        if let Some(p) = self.profiles.borrow().get_profile(id) {
            let fg = p.text_color();
            self.profile_btn_lbl.set_markup(&format!(
                "<span foreground='{}' font_weight='800'>{}</span>",
                fg,
                glib::markup_escape_text(&p.initial_letter())
            ));
            for i in 0..8 {
                self.profile_btn.remove_css_class(&format!("prof-bg-{}", i));
            }
            self.profile_btn
                .add_css_class(&format!("prof-bg-{}", p.color_idx));
            self.profile_btn.set_tooltip_text(Some(&trf(
                "Profile: {} (isolated storage)",
                &[&p.display_name()],
            )));
        }
    }

    pub(crate) fn rebuild_profile_popover(self: &Rc<Self>) {
        let box_menu = gtk::Box::new(gtk::Orientation::Vertical, 4);
        box_menu.set_margin_top(4);
        box_menu.set_margin_bottom(4);
        box_menu.set_margin_start(4);
        box_menu.set_margin_end(4);

        let title = gtk::Label::new(Some(tr("Select Profile")));
        title.add_css_class("profile-popover-title");
        title.set_xalign(0.0);
        title.set_margin_start(8);
        title.set_margin_bottom(4);
        box_menu.append(&title);

        let (profiles, selected_id) = {
            let mgr = self.profiles.borrow();
            (
                mgr.profiles().to_vec(),
                self.selected_palette_profile.borrow().clone(),
            )
        };

        for p in profiles {
            let row_btn = gtk::Button::new();
            row_btn.add_css_class("profile-item-btn");
            if p.id == selected_id {
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

            if p.id == selected_id {
                let check = gtk::Image::from_icon_name("object-select-symbolic");
                check.add_css_class("profile-item-check");
                row_box.append(&check);
            }

            row_btn.set_child(Some(&row_box));

            let app = self.clone();
            let pid = p.id.clone();
            row_btn.connect_clicked(move |_| {
                let is_new = app.palette_for_new_tab.get();
                app.select_palette_profile(&pid);
                if !is_new {
                    app.profiles.borrow_mut().set_active(&pid);
                    app.switch_focused_tab_profile(&pid);
                }
                app.rebuild_profile_popover();
                app.profile_popover.popdown();
                app.entry.grab_focus();
            });

            box_menu.append(&row_btn);
        }

        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep.set_margin_top(6);
        sep.set_margin_bottom(6);
        box_menu.append(&sep);

        let manage_btn = gtk::Button::new();
        manage_btn.add_css_class("profile-item-btn");
        let manage_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let manage_icon = gtk::Image::from_icon_name("preferences-system-symbolic");
        let manage_lbl = gtk::Label::new(None);
        sc_bind_text(
            &manage_lbl,
            "Manage Profiles in Settings (Alt+P)",
            &["profiles"],
        );
        manage_lbl.add_css_class("profile-item-name");
        manage_lbl.set_xalign(0.0);
        manage_box.append(&manage_icon);
        manage_box.append(&manage_lbl);
        manage_btn.set_child(Some(&manage_box));

        let app_c = self.clone();
        manage_btn.connect_clicked(move |_| {
            app_c.profile_popover.popdown();
            app_c.open_settings_tab(SettingsTab::Profiles);
        });
        box_menu.append(&manage_btn);

        self.profile_popover.set_child(Some(&box_menu));
    }

    pub(crate) fn update_palette_size(&self) {
        let win_w = self.win.width().max(1);
        let mut target_w = 580.min(win_w.saturating_sub(32));
        // Welcome page: pin the omnibar directly under the "rail." brand, aligned with it.
        // (Start page = centered column, max 580px, 20px side padding; brand block ~72px tall.)
        if self.focused_is_welcome() {
            if let (Some(v), Some(parent)) = (self.focused_view(), self.palette.parent()) {
                if let Some(b) = v.compute_bounds(&parent) {
                    if b.width() > 120.0 && b.height() > 120.0 {
                        let cw = ((b.width() as i32) - 40).clamp(200, 580);
                        let cx = b.x() + b.width() / 2.0;
                        self.palette.set_size_request(cw, -1);
                        self.palette.set_halign(gtk::Align::Start);
                        self.palette
                            .set_margin_start(((cx - cw as f32 / 2.0).round() as i32).max(0));
                        let top = b.y() + b.height() / 2.0 + 36.0 + 28.0;
                        self.palette.set_margin_top(top.round() as i32);
                        return;
                    }
                }
            }
        }
        // Ctrl+L on a normal page: classic centered palette near the top
        target_w = target_w.max(200);
        self.palette.set_size_request(target_w, -1);
        self.palette.set_halign(gtk::Align::Center);
        self.palette.set_margin_start(16);
        self.palette.set_margin_top(90);
    }

    /// Focus the search <input> that lives inside the welcome page itself.
    pub(crate) fn focus_welcome_input(&self, select: bool) {
        let Some(v) = self.focused_view() else { return };
        v.grab_focus();
        let js = if select {
            "(function(){var e=document.getElementById('q');if(e){e.focus();e.select();}})();"
        } else {
            "(function(){var e=document.getElementById('q');if(e&&document.activeElement!==e){e.focus();}})();"
        };
        v.evaluate_javascript(js, None, None, None::<&gio::Cancellable>, |_| {});
    }

    pub(crate) fn start_eval(view: &WebView, js: &str) {
        view.evaluate_javascript(js, None, None, None::<&gio::Cancellable>, |_| {});
    }

    /// Everything the in-page omnibar needs: engine icon, placeholder, hints, profiles.
    pub(crate) fn start_state_json(&self, view: &WebView) -> String {
        use minijson::escape as e;
        let mgr = self.profiles.borrow();
        let tab_prof = self
            .rows
            .borrow()
            .iter()
            .flat_map(|r| r.tabs.iter())
            .find(|t| &t.view == view)
            .map(|t| t.profile_id.clone())
            .unwrap_or_else(|| mgr.active_id().to_string());
        let profs: Vec<String> = mgr
            .profiles()
            .iter()
            .map(|p| {
                format!(
                    "{{\"id\":\"{}\",\"name\":\"{}\",\"persona\":\"{}\",\"letter\":\"{}\",\"bg\":\"{}\",\"fg\":\"{}\"}}",
                    e(&p.id),
                    e(&p.display_name()),
                    e(tr("Isolated storage")),
                    e(&p.initial_letter()),
                    p.hex_color(),
                    p.text_color()
                )
            })
            .collect();
        let engine = self.search_engine.borrow().clone();
        let display_name = search_engine_display_name(&engine);
        let svg_str = std::str::from_utf8(search_engine_svg_bytes(&engine)).unwrap_or("");
        format!(
            "{{\"placeholder\":\"{}\",\"engineSvg\":\"{}\",\"hint\":\"{}\",\"selTitle\":\"{}\",\"manage\":\"{}\",\"badges\":[\"{}\",\"{}\",\"{}\"],\"selected\":\"{}\",\"profiles\":[{}]}}",
            e(&format!("{} {}...", tr("Search with"), display_name)),
            e(svg_str),
            e(&sc_apply(
                tr("Enter  go      Esc  cancel      Alt+P  profiles"),
                &["profiles"]
            )),
            e(tr("Select Profile")),
            e(&sc_apply(
                tr("Manage Profiles in Settings (Alt+P)"),
                &["profiles"]
            )),
            e(tr("History")),
            e(tr("Top")),
            e(tr("Search")),
            e(&tab_prof),
            profs.join(",")
        )
    }

    pub(crate) fn sync_search_engine_ui(self: &Rc<Self>) {
        let engine = self.search_engine.borrow().clone();
        let display_name = search_engine_display_name(&engine);
        let placeholder = tr("Search with {} or enter URL").replace("{}", &display_name);

        self.entry.set_placeholder_text(Some(&placeholder));

        let pal_icon = load_search_engine_icon(&engine, 20);
        self.palette_engine_icon
            .set_paintable(pal_icon.paintable().as_ref());

        let pill_icon = load_search_engine_icon(&engine, 18);
        self.pill_engine_icon
            .set_paintable(pill_icon.paintable().as_ref());

        if let Ok(rows) = self.rows.try_borrow() {
            for row in rows.iter() {
                for tab in &row.tabs {
                    let u = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
                    let s = tab.saved_uri.borrow().clone();
                    if u.starts_with("rail://start") || s.starts_with("rail://start") {
                        self.push_start_state(&tab.view);
                    }
                }
            }
        }
    }

    pub(crate) fn push_start_state(&self, view: &WebView) {
        let js = format!(
            "window.railState&&window.railState({})",
            self.start_state_json(view)
        );
        Self::start_eval(view, &js);
    }

    /// Alt+P on the welcome page: toggle the in-page profile menu.
    pub(crate) fn toggle_welcome_profiles(&self) {
        if let Some(v) = self.focused_view() {
            Self::start_eval(&v, "window.railToggleProfiles&&window.railToggleProfiles()");
        }
    }

    /// Messages from the welcome page's own omnibar (see start_page.html):
    /// `init`, `q\t<text>` (suggestions), `go\t<profile>\t<text>`, `manage`.
    pub(crate) fn on_start_search(self: &Rc<Self>, view: &WebView, msg: &str) {
        // Only the real start page may talk to us (the handler exists in every page).
        let mut u = view.uri().map(|s| s.to_string()).unwrap_or_default();
        if u.is_empty() || u.starts_with("about:") {
            u = self.saved_uri_of(view);
        }
        if !u.starts_with("rail://start") {
            return;
        }
        let (kind, rest) = msg.split_once('\t').unwrap_or((msg, ""));
        match kind {
            "init" => self.push_start_state(view),
            "q" => {
                use minijson::escape as e;
                let sugs = self.history.borrow().suggest(rest);
                let items: Vec<String> = sugs
                    .iter()
                    .map(|s| {
                        let k = match s.kind {
                            SuggestionKind::History => 0,
                            SuggestionKind::Popular => 1,
                            SuggestionKind::Search => 2,
                        };
                        format!(
                            "{{\"title\":\"{}\",\"url\":\"{}\",\"target\":\"{}\",\"autofill\":\"{}\",\"kind\":{}}}",
                            e(&s.display_title),
                            e(&s.display_url),
                            e(&s.target_url),
                            e(&s.autofill_text),
                            k
                        )
                    })
                    .collect();
                let js = format!(
                    "window.railSugg&&window.railSugg(\"{}\",[{}])",
                    e(rest),
                    items.join(",")
                );
                Self::start_eval(view, &js);
            }
            "go" => {
                let (pid, text) = rest.split_once('\t').unwrap_or(("", ""));
                if text.trim().is_empty() || self.focused_view().as_ref() != Some(view) {
                    return;
                }
                let tab_prof = self
                    .rows
                    .borrow()
                    .iter()
                    .flat_map(|r| r.tabs.iter())
                    .find(|t| &t.view == view)
                    .map(|t| t.profile_id.clone());
                let chosen = if !pid.is_empty() && self.profiles.borrow().get_profile(pid).is_some()
                {
                    Some(pid.to_string())
                } else {
                    tab_prof
                };
                if let Some(p) = chosen {
                    self.select_palette_profile(&p);
                }
                self.palette_for_new_tab.set(false);
                self.navigate(text);
            }
            "manage" => self.open_settings_tab(SettingsTab::Profiles),
            _ => {}
        }
    }

    /// Focused tab is the welcome / start page (rail://start).
    pub(crate) fn focused_is_welcome(&self) -> bool {
        let Some(v) = self.focused_view() else {
            return false;
        };
        let mut u = v.uri().map(|s| s.to_string()).unwrap_or_default();
        if u.is_empty() || u.starts_with("about:") {
            u = self.saved_uri_of(&v);
        }
        u.starts_with("rail://start")
    }

    /// Welcome tab focused => omnibar is always shown, fixed under the brand.
    /// Focus moved to another page => unpin it again.
    pub(crate) fn sync_welcome_palette(self: &Rc<Self>) {
        if self.ov.get() || self.settings_modal_open() {
            return;
        }
        if self.focused_is_welcome() {
            // The search box is part of the page now: no floating GTK palette here.
            if self.palette_open() {
                self.hide_palette();
            }
            self.focus_welcome_input(false);
            if let Some(v) = self.focused_view() {
                self.push_start_state(&v);
            }
        } else if self.welcome_palette.get() && self.palette_open() {
            self.hide_palette();
        }
    }

    pub(crate) fn open_new_tab_palette(self: &Rc<Self>) {
        // Ctrl+T = new tab with welcome page + empty omnibar (not rail://start in the input)
        self.new_tab();
    }
}
