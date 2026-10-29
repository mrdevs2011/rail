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
        let tw = Self::create_tab_widget(&view);

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
                rows[next_r].tabs.push(Tab {
                    container: tw.container.clone(),
                    view: view.clone(),
                    freeze: tw.freeze.clone(),
                    height_handle: tw.height_handle.clone(),
                    profile_id,
                    custom_width: Cell::new(None),
                    custom_height: Cell::new(None),
                    col_id: Cell::new(new_col_id()),
                    saved_uri: RefCell::new(String::new()),
                    suspended: Cell::new(false),
                    keep_alive: Cell::new(false),
                    last_active: Cell::new(std::time::Instant::now()),
                    dt_box: tw.dt_box.clone(),
                    dt_handle: tw.dt_handle.clone(),
                    tab_inner: tw.tab_inner.clone(),
                    overlay: tw.overlay.clone(),
                    dt_open: Cell::new(false),
                    dt_height: Cell::new(self.default_devtools_height.get()),
                    dt_width: Cell::new(self.default_devtools_width.get()),
                    dt_dock: Cell::new(self.devtools_dock_side.get()),
                });
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
                rows[cur_r].tabs.insert(col_idx, Tab {
                    container: tw.container.clone(),
                    view: view.clone(),
                    freeze: tw.freeze.clone(),
                    height_handle: tw.height_handle.clone(),
                    profile_id,
                    custom_width: Cell::new(None),
                    custom_height: Cell::new(None),
                    col_id: Cell::new(new_col_id()),
                    saved_uri: RefCell::new(String::new()),
                    suspended: Cell::new(false),
                    keep_alive: Cell::new(false),
                    last_active: Cell::new(std::time::Instant::now()),
                    dt_box: tw.dt_box.clone(),
                    dt_handle: tw.dt_handle.clone(),
                    tab_inner: tw.tab_inner.clone(),
                    overlay: tw.overlay.clone(),
                    dt_open: Cell::new(false),
                    dt_height: Cell::new(self.default_devtools_height.get()),
                    dt_width: Cell::new(self.default_devtools_width.get()),
                    dt_dock: Cell::new(self.devtools_dock_side.get()),
                });
                (cur_r, col_idx)
            }
        };

        self.setup_tab_signals(
            &tw.container,
            &view,
            &tw.resize_handle,
            &tw.height_handle,
            &tw.dt_handle,
            &tw.dt_box,
        );
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
            pic.set_paintable(Option::<&gdk::Paintable>::None);
        }
    }

    /// F11: oyna fullscreen bo'lganda faqat kontent qoladi (yon panel, tab oralig'i, yuklanish
    /// chizig'i, chegara va burchaklar yo'q); chiqqanda hammasi joyiga qaytadi.
    pub(crate) fn apply_fullscreen_mode(self: &Rc<Self>) {
        let fs = self.win.is_fullscreen();
        if !fs {
            // oyna to'liq ekrandan chiqdi (F11 / tashqaridan) — sahifadagi element ham chiqsin
            self.leave_solo_fullscreen(true);
        }
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

    /// Sahifa ichidagi element (video va h.k.) to'liq ekran bo'lganda: o'sha tab butun oynani
    /// egallaydi, split / yon tomondagi tablar ko'rinmaydi — haqiqiy brauzerlardagidek.
    pub(crate) fn enter_solo_fullscreen(self: &Rc<Self>, view: &WebView) {
        let strip_w: gtk::Widget = self.strip.clone().upcast();
        let mut w: gtk::Widget = view.clone().upcast();
        while let Some(p) = w.parent() {
            if p == strip_w {
                self.layout.set_solo(Some(w));
                return;
            }
            w = p;
        }
    }

    /// Element to'liq ekrandan chiqdi. `exit_page` = oyna tashqaridan yopilgan (masalan F11):
    /// bu holda sahifadagi elementni ham to'liq ekrandan chiqaramiz.
    pub(crate) fn leave_solo_fullscreen(&self, exit_page: bool) {
        let Some(solo) = self.layout.solo() else { return };
        self.layout.set_solo(None);
        self.strip.queue_allocate();
        if !exit_page {
            return;
        }
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                let c: gtk::Widget = tab.container.clone().upcast();
                if c == solo {
                    tab.view.evaluate_javascript(
                        "document.fullscreenElement&&document.exitFullscreen&&document.exitFullscreen()",
                        None,
                        None,
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                    return;
                }
            }
        }
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

    /// Snapshot the WebView into an *immutable* MemoryTexture so the frame cannot go black
    /// when the web process is later terminated (live WidgetPaintable does).
    pub(crate) fn capture_durable_frame(view: &WebView) -> Option<gdk::Texture> {
        if !view.is_mapped() || view.width() < 8 || view.height() < 8 {
            return None;
        }
        let wp = gtk::WidgetPaintable::new(Some(view));
        let frame = gdk::prelude::PaintableExt::current_image(&wp);
        // Internal pages (rail://start) are legitimately near-black: accept their frame,
        // otherwise such a tab would never get a picture.
        let allow_black = view.uri().is_some_and(|u| u.starts_with("rail://"));
        Self::durableize_paintable(&frame, allow_black)
    }

    /// Max width for suspended-tab freeze frames. Full-res frames cost tens of MB each;
    /// at 480px a frame is ~0.5–1 MB so every sleeping tab can keep its picture forever.
    const FREEZE_MAX_W: i32 = 480;

    /// Copy any Paintable into a standalone (optionally downscaled) MemoryTexture.
    pub(crate) fn durableize_paintable(p: &gdk::Paintable, allow_black: bool) -> Option<gdk::Texture> {
        let tex = p.downcast_ref::<gdk::Texture>()?;
        let w = tex.width();
        let h = tex.height();
        if w < 8 || h < 8 {
            return None;
        }
        let stride = w as usize * 4;
        let mut buf = vec![0u8; stride * h as usize];
        tex.download(&mut buf, stride);
        // Reject near-black frames (failed / blank WebKit paint) — keep prior freeze instead.
        if !allow_black && Self::frame_is_near_black(&buf) {
            return None;
        }
        // Downscale so all suspended tabs keep a frame without ballooning RAM.
        if w > Self::FREEZE_MAX_W {
            let nw = Self::FREEZE_MAX_W;
            let nh = ((h as i64 * nw as i64) / w as i64).max(1) as i32;
            let nstride = nw as usize * 4;
            let mut out = vec![0u8; nstride * nh as usize];
            for y in 0..nh as usize {
                let sy = y * h as usize / nh as usize;
                let src_row = sy * stride;
                let dst_row = y * nstride;
                for x in 0..nw as usize {
                    let sx = x * w as usize / nw as usize;
                    let si = src_row + sx * 4;
                    let di = dst_row + x * 4;
                    out[di..di + 4].copy_from_slice(&buf[si..si + 4]);
                }
            }
            let bytes = glib::Bytes::from_owned(out);
            return Some(
                gdk::MemoryTexture::new(nw, nh, gdk::MemoryFormat::R8g8b8a8, &bytes, nstride).upcast(),
            );
        }
        let bytes = glib::Bytes::from_owned(buf);
        Some(
            gdk::MemoryTexture::new(w, h, gdk::MemoryFormat::R8g8b8a8, &bytes, stride).upcast(),
        )
    }

    /// True if >98% of samples are very dark (blank / unpainted WebView).
    fn frame_is_near_black(rgba: &[u8]) -> bool {
        if rgba.len() < 16 {
            return true;
        }
        let step = ((rgba.len() / 4) / 400).max(1) * 4; // ~400 samples
        let mut dark = 0u32;
        let mut n = 0u32;
        let mut i = 0;
        while i + 3 < rgba.len() {
            let r = rgba[i] as u32;
            let g = rgba[i + 1] as u32;
            let b = rgba[i + 2] as u32;
            if r + g + b < 24 {
                dark += 1;
            }
            n += 1;
            i += step;
        }
        n > 0 && dark * 100 / n >= 98
    }

    /// Store a durable last-good frame on the tab (visibility hide / pre-suspend).
    pub(crate) fn store_tab_frame(tab: &Tab) {
        if tab.suspended.get() {
            return;
        }
        if let Some(tex) = Self::capture_durable_frame(&tab.view) {
            tab.freeze.set_paintable(Some(&tex));
        }
    }

    /// Suspend a live tab: save URL + last frame, kill the web process completely.
    /// No CSS placeholder — freeze overlay is shown only when a real picture exists.
    /// Without a frame the process still dies (URL is enough to wake); the card stays
    /// empty of the charcoal placeholder layer so grey blocks never appear.
    pub(crate) fn suspend_tab(tab: &Tab) {
        if tab.suspended.get() || tab.keep_alive.get() || Self::is_settings_tab(tab) {
            return;
        }
        if tab.view.is_loading() || tab.view.is_playing_audio() {
            return;
        }
        let live = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
        let uri = if !live.is_empty() { live } else { tab.saved_uri.borrow().clone() };
        if uri.is_empty() {
            return;
        }
        *tab.saved_uri.borrow_mut() = uri;
        // Prefer a fresh durable snapshot while still mapped; otherwise keep the frame
        // grabbed on visibility-hide. Never clear an existing good frame here.
        if tab.view.is_mapped() {
            if let Some(tex) = Self::capture_durable_frame(&tab.view) {
                tab.freeze.set_paintable(Some(&tex));
            }
        }
        // ONLY suspend and terminate web process if a durable snapshot actually exists.
        // Terminating without a snapshot turns the tab into a pitch black crashed rectangle!
        if tab.freeze.paintable().is_some() {
            tab.freeze.set_visible(true);
            tab.suspended.set(true);
            tab.view.terminate_web_process();
        } else {
            tab.freeze.set_visible(false);
        }
    }

    /// Davriy tekshiruv:
    /// - Overview rejimida tablarni o'ldirmaymiz (foydalanuvchi galereyaga qarayapti)
    /// - faqat fokus / haqiqiy viewport kesishuvi `last_active` ni yangilaydi
    /// - SUSPEND_AFTER_SECS dan beri ko'rinmagan → suspend
    /// - MAX_LIVE_TABS dan oshsa eng eski non-focused tablar majburiy suspend
    pub(crate) fn suspend_hidden_tabs(&self) {
        if self.ov.get() {
            return;
        }
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

        // Frames of suspended tabs are never dropped: no placeholder may ever show.
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
