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
            if let Some(p) = p.upgrade() { p.popdown(); }
            v.go_back();
        });
        hdr.append(&back);

        let fwd = hdr_btn("forward", view.can_go_forward());
        let (v, p) = (view.clone(), pop.downgrade());
        fwd.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() { p.popdown(); }
            v.go_forward();
        });
        hdr.append(&fwd);

        let reload = hdr_btn("reload", true);
        let (v, p) = (view.clone(), pop.downgrade());
        reload.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() { p.popdown(); }
            v.reload();
        });
        hdr.append(&reload);

        col.append(&hdr);
        // QR code
        let qr = menu_row("qr", tr("Create QR Code for page"), "", is_web_uri(&uri));
        let (app, p, u) = (self.clone(), pop.downgrade(), uri.clone());
        qr.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() { p.popdown(); }
            app.show_qr_window(&u);
        });
        col.append(&qr);

        // Translate <-> show original: exactly one of the two, depending on the page state.
        if let Some(original) = untranslate_url(&uri) {
            let row = menu_row("translate-menu", tr("Show Original Page"), "", true);
            let (v, p) = (view.clone(), pop.downgrade());
            row.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() { p.popdown(); }
                v.load_uri(&original);
            });
            col.append(&row);
        } else if translate_url(&uri, "auto", "uz").is_some() {
            let name = target_language_name(&load_app_settings().translate_target_lang);
            let label = tr("Translate to {}").replace("{}", &name);
            let row = menu_row("translate-menu", &label, "", true);
            let (v, p) = (view.clone(), pop.downgrade());
            row.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() { p.popdown(); }
                translate_page(&v);
            });
            col.append(&row);
        }
        // Screenshot (Zen-style region + full page)
        let shot = menu_row("screenshot", tr("Take Screenshot"), "", is_web_uri(&uri));
        let (app, p, v) = (self.clone(), pop.downgrade(), view.clone());
        shot.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() { p.popdown(); }
            app.start_screenshot_mode(&v);
        });
        col.append(&shot);
        // Developer
        let src = menu_row("source", tr("View Page Source"), "Ctrl+U", is_web_uri(&uri));
        let (app, p, u) = (self.clone(), pop.downgrade(), uri.clone());
        src.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() { p.popdown(); }
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
            if let Some(p) = p.upgrade() { p.popdown(); }
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

    /// Small modal window with a scannable QR code of the page address.
    pub(crate) fn show_qr_window(self: &Rc<Self>, url: &str) {
        let win = gtk::Window::new();
        win.set_title(Some(tr("Create QR Code for page")));
        win.set_transient_for(Some(&self.win));
        win.set_modal(true);
        win.set_resizable(false);

        let col = gtk::Box::new(gtk::Orientation::Vertical, 12);
        col.set_margin_top(18);
        col.set_margin_bottom(18);
        col.set_margin_start(18);
        col.set_margin_end(18);

        match crate::qr::encode(url.as_bytes()) {
            Some(code) => {
                let n = code.size as i32;
                let scale = (320 / (n + 8)).max(4);
                let px = (n + 8) * scale;
                let area = gtk::DrawingArea::new();
                area.set_content_width(px);
                area.set_content_height(px);
                area.set_halign(gtk::Align::Center);
                area.set_draw_func(move |_, cr, w, h| {
                    // White quiet zone + black modules: scanners need real contrast.
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    let _ = cr.paint();
                    cr.set_source_rgb(0.0, 0.0, 0.0);
                    let ox = ((w - px) / 2 + 4 * scale) as f64;
                    let oy = ((h - px) / 2 + 4 * scale) as f64;
                    for y in 0..code.size {
                        for x in 0..code.size {
                            if code.get(x, y) {
                                cr.rectangle(ox + (x as i32 * scale) as f64, oy + (y as i32 * scale) as f64, scale as f64, scale as f64);
                            }
                        }
                    }
                    let _ = cr.fill();
                });
                col.append(&area);
                let hint = gtk::Label::new(Some(tr("Scan to open this page on your phone")));
                hint.add_css_class("dim-label");
                col.append(&hint);
            }
            None => {
                col.append(&gtk::Label::new(Some(tr("This address is too long for a QR code"))));
            }
        }

        let addr = gtk::Label::new(Some(url));
        addr.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        addr.set_max_width_chars(44);
        addr.set_selectable(true);
        col.append(&addr);

        win.set_child(Some(&col));
        let key = gtk::EventControllerKey::new();
        let w = win.downgrade();
        key.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                if let Some(w) = w.upgrade() { w.close(); }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        win.add_controller(key);
        win.present();
    }
}
