//! Page-level context menu (right-click on empty page area): Back / Forward / Reload,
//! QR code, translate toggle, view source, inspect. Drawn as a GtkPopover so every row can
//! carry an SVG icon and a shortcut hint (WebKit's native menu has no icon API).
//! Links, images, media, editable text and selections keep the native WebKit menu.

use super::translate::translate_page;
use crate::*;

/// Context-menu icon from `icons/svg/{name}.svg` (16px).
fn menu_icon(name: &str) -> gtk::Image {
    load_icon(name, 16)
}

fn menu_row(icon: &str, label: &str, shortcut: &str, enabled: bool) -> gtk::Button {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&menu_icon(icon));
    let l = gtk::Label::new(Some(label));
    l.set_xalign(0.0);
    l.set_hexpand(true);
    content.append(&l);
    if !shortcut.is_empty() {
        let s = gtk::Label::new(Some(shortcut));
        s.add_css_class("ctxm-sc");
        content.append(&s);
    }
    let b = gtk::Button::new();
    b.set_child(Some(&content));
    b.add_css_class("ctxm-row");
    b.set_has_frame(false);
    b.set_focus_on_click(false);
    b.set_sensitive(enabled);
    b
}

/// "O‘zbek" for "uz", "Ingliz" for "en" (the parenthesized native name is dropped).
fn target_language_name(code: &str) -> String {
    SUPPORTED_LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, n)| n.split(" (").next().unwrap_or(n).to_string())
        .unwrap_or_else(|| code.to_string())
}

fn is_web_uri(uri: &str) -> bool {
    uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("file://")
}

impl App {
    /// True when the right-click landed on plain page area (no link/image/media/text target).
    pub(crate) fn is_page_level_hit(hit: &HitTestResult) -> bool {
        !(hit.context_is_link()
            || hit.context_is_image()
            || hit.context_is_media()
            || hit.context_is_editable()
            || hit.context_is_selection())
    }

    /// Pointer position in `view` coordinates (the context-menu signal carries no position).
    fn pointer_in_view(&self, view: &WebView) -> Option<(f64, f64)> {
        let seat = gdk::Display::default()?.default_seat()?;
        let dev = seat.pointer()?;
        let (surface, sx, sy) = dev.surface_at_position();
        surface?;
        let (ox, oy) = self.win.surface_transform();
        let origin = view.compute_point(&self.win, &gtk::graphene::Point::new(0.0, 0.0))?;
        Some((sx - ox - origin.x() as f64, sy - oy - origin.y() as f64))
    }

    pub(crate) fn show_page_menu(self: &Rc<Self>, view: &WebView) {
        let uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
        let pop = gtk::Popover::new();
        pop.add_css_class("ctxm");
        pop.set_has_arrow(false);
        pop.set_halign(gtk::Align::Start);
        let col = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Header: Back / Forward / Reload in one horizontal line (Zen-style)
        let hdr = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        hdr.add_css_class("ctxm-hdr");
        hdr.set_halign(gtk::Align::Fill);

        fn hdr_btn(icon: &str, enabled: bool) -> gtk::Button {
            let b = gtk::Button::new();
            b.set_child(Some(&menu_icon(icon)));
            b.add_css_class("ctxm-hdr-btn");
            b.set_has_frame(false);
            b.set_focus_on_click(false);
            b.set_sensitive(enabled);
            b.set_hexpand(true);
            b
        }

        let back = hdr_btn("back", view.can_go_back());
        let (v, p) = (view.clone(), pop.downgrade());
        back.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            v.go_back();
        });
        hdr.append(&back);

        let fwd = hdr_btn("forward", view.can_go_forward());
        let (v, p) = (view.clone(), pop.downgrade());
        fwd.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            v.go_forward();
        });
        hdr.append(&fwd);

        let reload = hdr_btn("reload", true);
        let (v, p) = (view.clone(), pop.downgrade());
        reload.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            v.reload();
        });
        hdr.append(&reload);

        col.append(&hdr);
        // QR code
        let qr = menu_row("qr", tr("Create QR Code for page"), "", is_web_uri(&uri));
        let (app, p, u) = (self.clone(), pop.downgrade(), uri.clone());
        qr.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            app.show_qr_window(&u);
        });
        col.append(&qr);

        // Translate <-> show original: exactly one of the two, depending on the page state.
        if let Some(original) = untranslate_url(&uri) {
            let row = menu_row("translate-menu", tr("Show Original Page"), "", true);
            let (v, p) = (view.clone(), pop.downgrade());
            row.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.popdown();
                }
                v.load_uri(&original);
            });
            col.append(&row);
        } else if translate_url(&uri, "auto", "uz").is_some() {
            let name = target_language_name(&load_app_settings().translate_target_lang);
            let label = tr("Translate to {}").replace("{}", &name);
            let row = menu_row("translate-menu", &label, "", true);
            let (v, p) = (view.clone(), pop.downgrade());
            row.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.popdown();
                }
                translate_page(&v);
            });
            col.append(&row);
        }
        // Screenshot (Zen-style region + full page)
        let shot = menu_row("screenshot", tr("Take Screenshot"), "", is_web_uri(&uri));
        let (app, p, v) = (self.clone(), pop.downgrade(), view.clone());
        shot.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            app.start_screenshot_mode(&v);
        });
        col.append(&shot);
        // Developer
        let src = menu_row("source", tr("View Page Source"), "Ctrl+U", is_web_uri(&uri));
        let (app, p, u) = (self.clone(), pop.downgrade(), uri.clone());
        src.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            app.new_tab_with_url(&format!("view-source:{u}"));
        });
        col.append(&src);

        let insp_sc = {
            let s = sc_display("devtools");
            if s.is_empty() { "F12".to_string() } else { s }
        };
        let inspect = menu_row("inspect", tr("Inspect"), &insp_sc, true);
        let (app, p) = (self.clone(), pop.downgrade());
        inspect.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
            app.show_devtools();
        });
        col.append(&inspect);

        pop.set_child(Some(&col));
        pop.set_parent(view);
        let (x, y) = self
            .pointer_in_view(view)
            .unwrap_or((view.width() as f64 / 2.0, view.height() as f64 / 2.0));
        pop.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        let w = pop.downgrade();
        pop.connect_closed(move |_| {
            let w = w.clone();
            glib::idle_add_local_once(move || {
                if let Some(p) = w.upgrade() {
                    p.unparent();
                }
            });
        });
        pop.popup();
    }

    pub(crate) fn qr_modal_open(&self) -> bool {
        self.qr_modal.borrow().is_some()
    }

    pub(crate) fn hide_qr_modal(self: &Rc<Self>) {
        if let Some(backdrop) = self.qr_modal.borrow_mut().take() {
            if let Some(parent) = backdrop.parent() {
                if let Ok(overlay) = parent.downcast::<gtk::Overlay>() {
                    overlay.remove_overlay(&backdrop);
                }
            }
        }
    }

    /// Brutalist sharp modal card with a scannable QR code of the page address.
    /// Rendered as an in-window overlay to avoid OS window-manager corner rounding.
    pub(crate) fn show_qr_window(self: &Rc<Self>, url: &str) {
        self.hide_qr_modal();

        let root_overlay = match self
            .win
            .child()
            .and_then(|c| c.downcast::<gtk::Overlay>().ok())
        {
            Some(ov) => ov,
            None => return,
        };

        // Full-window dim backdrop
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("qr-backdrop");
        backdrop.set_hexpand(true);
        backdrop.set_vexpand(true);
        backdrop.set_halign(gtk::Align::Fill);
        backdrop.set_valign(gtk::Align::Fill);
        backdrop.set_can_target(true);

        // Center card with 100% sharp corners
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("qr-card");
        card.set_halign(gtk::Align::Center);
        card.set_valign(gtk::Align::Center);
        card.set_hexpand(true);
        card.set_vexpand(true);
        card.set_can_target(true);

        // ── Top bar: title + close ───────────────────────────────────────────
        let topbar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        topbar.add_css_class("qr-topbar");

        let title_lbl = gtk::Label::new(Some(tr("Send to phone")));
        title_lbl.add_css_class("qr-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);
        topbar.append(&title_lbl);

        let close_btn = gtk::Button::new();
        close_btn.add_css_class("qr-close");
        close_btn.set_has_frame(false);
        close_btn.set_focus_on_click(false);
        let x_label = gtk::Label::new(Some("✕"));
        close_btn.set_child(Some(&x_label));
        let app_close = Rc::downgrade(self);
        close_btn.connect_clicked(move |_| {
            if let Some(app) = app_close.upgrade() {
                app.hide_qr_modal();
            }
        });
        topbar.append(&close_btn);
        card.append(&topbar);

        // ── QR area ─────────────────────────────────────────────────────────
        match crate::qr::encode(url.as_bytes()) {
            Some(code) => {
                let n = code.size as i32;
                let scale = (280 / (n + 8)).max(4);
                let px = (n + 8) * scale;

                let qr_frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
                qr_frame.add_css_class("qr-frame");
                qr_frame.set_halign(gtk::Align::Center);

                let area = gtk::DrawingArea::new();
                area.set_content_width(px);
                area.set_content_height(px);
                area.set_halign(gtk::Align::Center);
                area.set_draw_func(move |_, cr, w, h| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    let _ = cr.paint();
                    cr.set_source_rgb(0.0, 0.0, 0.0);
                    let ox = ((w - px) / 2 + 4 * scale) as f64;
                    let oy = ((h - px) / 2 + 4 * scale) as f64;
                    for y in 0..code.size {
                        for x in 0..code.size {
                            if code.get(x, y) {
                                cr.rectangle(
                                    ox + (x as i32 * scale) as f64,
                                    oy + (y as i32 * scale) as f64,
                                    scale as f64,
                                    scale as f64,
                                );
                            }
                        }
                    }
                    let _ = cr.fill();
                });
                qr_frame.append(&area);
                card.append(&qr_frame);

                // Hint
                let hint = gtk::Label::new(Some(tr("Scan to open this page on your phone")));
                hint.add_css_class("qr-hint");
                hint.set_halign(gtk::Align::Center);
                card.append(&hint);
            }
            None => {
                let err = gtk::Label::new(Some(tr("This address is too long for a QR code")));
                err.add_css_class("qr-hint");
                card.append(&err);
            }
        }

        // ── URL row: truncated label + copy button ───────────────────────────
        let url_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        url_row.add_css_class("qr-url-row");

        let addr = gtk::Label::new(Some(url));
        addr.add_css_class("qr-url");
        addr.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        addr.set_max_width_chars(36);
        addr.set_selectable(false);
        addr.set_cursor_from_name(Some("pointer"));
        addr.set_hexpand(true);
        addr.set_xalign(0.0);
        url_row.append(&addr);

        let copy_btn = gtk::Button::with_label(tr("Copy"));
        copy_btn.add_css_class("qr-copy-btn");
        copy_btn.set_has_frame(false);
        copy_btn.set_focus_on_click(false);
        let url_owned = url.to_string();

        let do_copy = {
            let app_c = Rc::downgrade(self);
            let btn_c = copy_btn.downgrade();
            let url_c = url_owned.clone();
            move || {
                if let Some(clip) = gdk::Display::default().map(|d| d.clipboard()) {
                    clip.set_text(&url_c);
                }
                if let Some(btn) = btn_c.upgrade() {
                    btn.set_label(tr("Copied!"));
                    let b = btn.downgrade();
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(1500),
                        move || {
                            if let Some(btn) = b.upgrade() {
                                btn.set_label(tr("Copy"));
                            }
                        },
                    );
                }
                if let Some(app) = app_c.upgrade() {
                    app.show_copied_toast(tr("Copied!"), &url_c);
                }
            }
        };

        {
            let dc = do_copy.clone();
            copy_btn.connect_clicked(move |_| dc());
        }
        {
            let dc = do_copy.clone();
            let click_addr = gtk::GestureClick::new();
            click_addr.connect_released(move |_, _, _, _| dc());
            url_row.add_controller(click_addr);
        }
        url_row.append(&copy_btn);
        card.append(&url_row);

        backdrop.append(&card);

        // Click on backdrop outside the card to close
        let click = gtk::GestureClick::new();
        let app_click = Rc::downgrade(self);
        let card_weak = card.downgrade();
        let bd_weak = backdrop.downgrade();
        click.connect_released(move |_, _, x, y| {
            if let (Some(c), Some(bd)) = (card_weak.upgrade(), bd_weak.upgrade()) {
                if let Some(bounds) = c.compute_bounds(&bd) {
                    let bx = bounds.x() as f64;
                    let by = bounds.y() as f64;
                    let bw = bounds.width() as f64;
                    let bh = bounds.height() as f64;
                    if x < bx || x > bx + bw || y < by || y > by + bh {
                        if let Some(app) = app_click.upgrade() {
                            app.hide_qr_modal();
                        }
                    }
                }
            }
        });
        backdrop.add_controller(click);

        // Escape key to close
        let key = gtk::EventControllerKey::new();
        let app_key = Rc::downgrade(self);
        key.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                if let Some(app) = app_key.upgrade() {
                    app.hide_qr_modal();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        backdrop.add_controller(key);

        root_overlay.add_overlay(&backdrop);
        *self.qr_modal.borrow_mut() = Some(backdrop);
    }
}
