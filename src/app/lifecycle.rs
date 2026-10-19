//! Adding tabs, remembered URIs, freezing and suspending tabs.

use crate::*;

impl App {
    pub(crate) fn set_tab_stack_height(&self, r: usize, c: usize, height: f64) {
        let rows = self.rows.borrow();
        if let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) {
            tab.custom_height.set(Some(height.max(80.0)));
        }
        drop(rows);
        self.layout.set_tab_height(r, c, Some(height.max(80.0)));
        self.strip.queue_allocate();
    }

    pub(crate) fn add_view(self: &Rc<Self>, view: WebView, profile_id: String, new_row: bool) {
        view.set_can_target(!self.ov.get());
        let (container, resize_handle, height_handle, freeze) = Self::create_tab_widget(&view);

        let (target_r, target_c) = {
            let mut rows = self.rows.borrow_mut();
            if rows.is_empty() {
                rows.push(Row { tabs: Vec::new() });
            }
            if new_row {
                let cur_r = self.focus_row.get();
                let next_r = cur_r + 1;
                while rows.len() <= next_r {
                    rows.push(Row { tabs: Vec::new() });
                }
                rows[next_r].tabs.push(Tab { container: container.clone(), view: view.clone(), freeze: freeze.clone(), height_handle: height_handle.clone(), profile_id, custom_width: Cell::new(None), custom_height: Cell::new(None), col_id: Cell::new(new_col_id()), saved_uri: RefCell::new(String::new()), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) });
                let c_idx = rows[next_r].tabs.len() - 1;
                (next_r, c_idx)
            } else {
                let cur_r = self.focus_row.get().min(rows.len() - 1);
                let cur_c = self.focus_col.get();
                let col_idx = if rows[cur_r].tabs.is_empty() {
                    0
                } else {
                    // a new tab opens in a column of its own, right after the focused column
                    rows[cur_r].col_span(cur_c.min(rows[cur_r].tabs.len() - 1)).1
                };
                rows[cur_r].tabs.insert(col_idx, Tab { container: container.clone(), view: view.clone(), freeze: freeze.clone(), height_handle: height_handle.clone(), profile_id, custom_width: Cell::new(None), custom_height: Cell::new(None), col_id: Cell::new(new_col_id()), saved_uri: RefCell::new(String::new()), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) });
                (cur_r, col_idx)
            }
        };

        self.setup_tab_signals(&container, &view, &resize_handle, &height_handle);
        self.sync_strip();
        self.focus_coords(target_r, target_c);
    }

    pub(crate) fn saved_uri_of(&self, view: &WebView) -> String {
        let Ok(rows) = self.rows.try_borrow() else { return String::new() };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    return tab.saved_uri.borrow().clone();
                }
            }
        }
        String::new()
    }

    /// Remember live URI (http(s) or rail://) — web process kill clears view.uri()
    pub(crate) fn remember_uri(&self, view: &WebView) {
        let Some(u) = view.uri() else { return };
        let u = u.to_string();
        if !(u.starts_with("http://")
            || u.starts_with("https://")
            || u.starts_with("rail://"))
        {
            return;
        }
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    *tab.saved_uri.borrow_mut() = u;
                    return;
                }
            }
        }
    }

    pub(crate) fn set_view_saved_uri(&self, view: &WebView, uri: &str) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    *tab.saved_uri.borrow_mut() = uri.to_string();
                    return;
                }
            }
        }
    }

    /// Best-known URI for the focused tab (live or saved), for omnibar / pill.
    pub(crate) fn focused_display_uri(&self) -> String {
        let Some(v) = self.focused_view() else {
            return tr("Search or enter address").into();
        };
        if let Some(u) = v.uri() {
            let us = u.as_str();
            if !us.is_empty() && !us.starts_with("about:") {
                return display_uri_for_pill(us);
            }
        }
        let saved = self.saved_uri_of(&v);
        if !saved.is_empty() {
            return display_uri_for_pill(&saved);
        }
        tr("Search or enter address").into()
    }

    /// Full URL of the focused tab for editing in the address entry (Ctrl+L).
    pub(crate) fn focused_edit_uri(&self) -> String {
        let Some(v) = self.focused_view() else {
            return tr("Search or enter address").into();
        };
        if let Some(u) = v.uri() {
            let us = u.as_str();
            if !us.is_empty() && !us.starts_with("about:") {
                return edit_uri_for_entry(us);
            }
        }
        let saved = self.saved_uri_of(&v);
        if !saved.is_empty() {
            return edit_uri_for_entry(&saved);
        }
        tr("Search or enter address").into()
    }

    pub(crate) fn mark_keep_alive(&self, view: &WebView) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    tab.keep_alive.set(true);
                }
            }
        }
    }

    /// Animatsiya (slide / overview zoom) boshida ko'rinib turgan jonli tablarning oxirgi kadrini
    /// freeze'ga qo'yadi: animatsiya davomida ekranda tayyor tekstura siljiydi, WebView esa qayta
    /// chizilmaydi. Video/audio o'ynayotgan va yuklanayotgan tablar tegilmaydi (jonli qoladi).
    /// `RAIL_NO_ANIM_FREEZE=1` bilan o'chiriladi (GPU'da solishtirish uchun).
    pub(crate) fn freeze_for_anim(&self) {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if !*ENABLED.get_or_init(|| std::env::var_os("RAIL_NO_ANIM_FREEZE").is_none()) {
            return;
        }
        if !self.anim_frozen.borrow().is_empty() {
            return;
        }
        let Ok(rows) = self.rows.try_borrow() else { return };
        let mut frozen = Vec::new();
        for row in rows.iter() {
            for tab in &row.tabs {
                // uxlagan / uyg'onayotgan tabning freeze'i o'ziniki — tegmaymiz
                if tab.suspended.get() || tab.freeze.is_visible() || !tab.view.is_mapped() {
                    continue;
                }
                if tab.view.is_loading() || tab.view.is_playing_audio() || !self.tab_is_visible(tab) {
                    continue;
                }
                let wp = gtk::WidgetPaintable::new(Some(&tab.view));
                let frame = gdk::prelude::PaintableExt::current_image(&wp);
                tab.freeze.set_paintable(Some(&frame));
                tab.freeze.set_visible(true);
                frozen.push((tab.freeze.clone(), tab.view.clone()));
            }
        }
        *self.anim_frozen.borrow_mut() = frozen;
    }

    /// Animatsiya tugadi: `freeze_for_anim` qo'ygan kadrlarni olib tashlaydi.
    pub(crate) fn unfreeze_after_anim(self: &Rc<Self>) {
        if self.anim_frozen.borrow().is_empty() {
            return;
        }
        let Ok(rows) = self.rows.try_borrow() else {
            // rows band — keyinroq qayta urinamiz, tab qotib qolmasin
            let app = self.clone();
            glib::idle_add_local_once(move || app.unfreeze_after_anim());
            return;
        };
        let frozen = std::mem::take(&mut *self.anim_frozen.borrow_mut());
        for (pic, view) in frozen {
            let Some(tab) = rows.iter().flat_map(|r| r.tabs.iter()).find(|t| t.view == view) else { continue };
            // suspend ulgurgan bo'lsa, freeze endi suspend'ning o'zi
            if tab.suspended.get() {
                continue;
            }
            pic.set_visible(false);
            // yashirilgan (unmapped) tabning kadri suspend uchun kerak — faqat ko'rinayotganini tozalaymiz
            if tab.view.is_mapped() {
                pic.set_paintable(Option::<&gdk::Paintable>::None);
            }
        }
    }

    /// F11: oyna fullscreen bo'lganda faqat kontent qoladi (yon panel, tab oralig'i, yuklanish
    /// chizig'i, chegara va burchaklar yo'q); chiqqanda hammasi joyiga qaytadi.
    pub(crate) fn apply_fullscreen_mode(self: &Rc<Self>) {
        let fs = self.win.is_fullscreen();
        strip::set_content_only(fs);
        // ishlab turgan sidebar animatsiyasini bekor qilamiz
        self.side_gen.set(self.side_gen.get().wrapping_add(1));
        if fs {
            self.win.add_css_class("fs-content");
            self.shell_layout.set_side(0.0);
            self.side.set_visible(false);
            self.strip.set_margin_top(0);
            self.strip.set_margin_bottom(0);
            self.strip.set_margin_start(0);
            self.strip.set_margin_end(0);
            self.load_bar.set_visible(false);
        } else {
            self.win.remove_css_class("fs-content");
            let open = self.side_open.get();
            self.shell_layout.set_side(if open { SIDE_W as f64 } else { 0.0 });
            self.side.set_visible(open);
            self.strip.set_margin_top(8);
            self.strip.set_margin_bottom(8);
            self.strip.set_margin_start(6);
            self.strip.set_margin_end(8);
            self.update_load_bar();
        }
        self.shell.queue_allocate();
        self.strip.queue_allocate();
        self.reveal_focus();
    }

    pub(crate) fn clear_tab_freeze(&self, view: &WebView) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view && !tab.suspended.get() {
                    tab.freeze.set_visible(false);
                    tab.freeze.set_paintable(Option::<&gdk::Paintable>::None);
                    return;
                }
            }
        }
    }

    /// Tab hozir foydalanuvchiga ko'rinyaptimi (strip viewport bilan kesishadimi)?
    /// Split da fokussiz, lekin yonma-yon ko'rinib turgan tab ham "ko'rinadi" hisoblanadi.
    pub(crate) fn tab_is_visible(&self, tab: &Tab) -> bool {
        if self.ov.get() {
            return true; // overview: hamma tab ekranda
        }
        let Some(b) = tab.container.compute_bounds(&self.strip) else { return false };
        let (vw, vh) = (self.strip.width() as f64, self.strip.height() as f64);
        let (x0, y0) = (b.x() as f64, b.y() as f64);
        let ow = (x0 + b.width() as f64).min(vw) - x0.max(0.0);
        let oh = (y0 + b.height() as f64).min(vh) - y0.max(0.0);
        ow > 40.0 && oh > 40.0
    }

    /// Is this tab just showing the built-in start page (nothing worth keeping alive)?
    pub(crate) fn is_blank_start_tab(tab: &Tab) -> bool {
        let live = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
        let uri = if live.is_empty() { tab.saved_uri.borrow().clone() } else { live };
        uri.starts_with("rail://start")
    }

    /// Terminate every tab web process (window close / quit). Network process dies with the app.
    pub(crate) fn shutdown_all_web_processes(&self) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                tab.view.terminate_web_process();
            }
        }
    }

    /// Settings UI is a GTK overlay on a black WebView shell. Suspending it freezes only
    /// the black shell — the overlay is gone → solid black rectangle until reload.
    pub(crate) fn is_settings_tab(tab: &Tab) -> bool {
        let live = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
        let saved = tab.saved_uri.borrow().clone();
        live.starts_with("rail://settings") || saved.starts_with("rail://settings")
    }

    /// Suspend a live http(s) tab: freeze last frame, kill web process, free RAM.
    pub(crate) fn suspend_tab(tab: &Tab) {
        if tab.suspended.get() || tab.keep_alive.get() || Self::is_settings_tab(tab) {
            return;
        }
        if tab.view.is_loading() || tab.view.is_playing_audio() {
            return;
        }
        let live = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
        let uri = if !live.is_empty() { live } else { tab.saved_uri.borrow().clone() };
        // Suspend both real pages and internal pages (settings/start) to free RAM
        if uri.is_empty() {
            return;
        }
        *tab.saved_uri.borrow_mut() = uri;
        // Only snapshot while mapped. An unmapped WebView paints blank — that became the
        // "black tab" bug. Prefer the frame grabbed on hide (connect_visibility); if we still
        // have none, leave freeze empty (focus will reload and show content).
        if tab.view.is_mapped() {
            let wp = gtk::WidgetPaintable::new(Some(&tab.view));
            let frame = gdk::prelude::PaintableExt::current_image(&wp);
            tab.freeze.set_paintable(Some(&frame));
        }
        tab.freeze.set_visible(tab.freeze.paintable().is_some());
        tab.suspended.set(true);
        tab.view.terminate_web_process();
    }

    /// Davriy tekshiruv:
    /// - faqat fokus / haqiqiy viewport kesishuvi `last_active` ni yangilaydi
    ///   (overview endi BARCHA tablarni "tirik" ushlab turmaydi)
    /// - SUSPEND_AFTER_SECS dan beri ko'rinmagan → suspend
    /// - MAX_LIVE_TABS dan oshsa eng eski non-focused tablar majburiy suspend
    pub(crate) fn suspend_hidden_tabs(&self) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        let fr = self.focus_row.get();
        let fc = self.focus_col.get();
        let in_overview = self.ov.get();
        let (suspend_after, max_live) = suspend_policy();

        // 1) Touch last_active only for focused tab, or actually on-screen when NOT in overview.
        //    Overview scales everything into view — treating all as visible prevented ANY suspend.
        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                let is_focused = ri == fr && ci == fc;
                if is_focused {
                    tab.last_active.set(std::time::Instant::now());
                } else if !in_overview && self.tab_is_visible(tab) {
                    tab.last_active.set(std::time::Instant::now());
                }
            }
        }

        // 2) Time-based suspend for off-screen tabs
        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                let is_focused = ri == fr && ci == fc;
                if is_focused || tab.suspended.get() || tab.keep_alive.get() {
                    continue;
                }
                // In normal mode keep truly visible split tabs alive until max-live kicks in
                if !in_overview && self.tab_is_visible(tab) {
                    continue;
                }
                let after = if Self::is_blank_start_tab(tab) { suspend_after.min(BLANK_TAB_SUSPEND_SECS) } else { suspend_after };
                if tab.last_active.get().elapsed() < Duration::from_secs(after) {
                    continue;
                }
                Self::suspend_tab(tab);
            }
        }

        // 3) Hard cap: too many live web processes → suspend oldest non-focused first
        let mut live: Vec<(std::time::Instant, usize, usize)> = Vec::new();
        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                if tab.suspended.get() || tab.keep_alive.get() {
                    continue;
                }
                let uri = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
                let saved = tab.saved_uri.borrow().clone();
                let is_real = uri.starts_with("http") || uri.starts_with("rail://") || saved.starts_with("http") || saved.starts_with("rail://");
                if !is_real {
                    continue;
                }
                live.push((tab.last_active.get(), ri, ci));
            }
        }
        if live.len() > max_live {
            live.sort_by_key(|(t, _, _)| *t); // oldest first
            let excess = live.len() - max_live;
            for (_, ri, ci) in live.into_iter().take(excess) {
                if ri == fr && ci == fc {
                    continue; // never suspend focused
                }
                if let Some(tab) = rows.get(ri).and_then(|r| r.tabs.get(ci)) {
                    // Visible split tabs must stay live — suspending them left a black freeze on screen.
                    if !in_overview && self.tab_is_visible(tab) {
                        continue;
                    }
                    Self::suspend_tab(tab);
                }
            }
        }
    }

    /// (first tab of the column, first tab of the next column) for tab (r, c).
    pub(crate) fn col_head_and_next(&self, r: usize, c: usize) -> (usize, usize) {
        match self.rows.borrow().get(r) {
            Some(row) if c < row.tabs.len() => row.col_span(c),
            _ => (c, c + 1),
        }
    }

    pub(crate) fn find_view_coords(&self, view: &WebView) -> Option<(usize, usize)> {
        let rows = self.rows.borrow();
        for (r_idx, row) in rows.iter().enumerate() {
            for (c_idx, tab) in row.tabs.iter().enumerate() {
                if &tab.view == view {
                    return Some((r_idx, c_idx));
                }
            }
        }
        None
    }
}
