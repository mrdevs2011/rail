//! "Translate Page" context-menu entry (Google Translate proxy, see nav/translate.rs).

use crate::*;

fn open_translated(view: &WebView, source: &str, target: &str) {
    let current = view.uri().map(|s| s.to_string()).unwrap_or_default();
    if let Some(url) = translate_url(&current, source, target) {
        view.load_uri(&url);
    }
}

pub(super) fn translate_page(view: &WebView) {
    let s = load_app_settings();
    if s.translate_source_lang != "auto" {
        open_translated(view, &s.translate_source_lang, &s.translate_target_lang);
        return;
    }
    // Source "auto": ask the page itself (<html lang>, then og:locale); "auto" if neither is set.
    let v = view.clone();
    view.evaluate_javascript(DETECT_LANG_JS, None, None, None::<&gio::Cancellable>, move |res| {
        let raw = res.ok().map(|j| j.to_string()).unwrap_or_default();
        open_translated(&v, &normalize_detected_lang(&raw), &s.translate_target_lang);
    });
}

impl App {
    /// Adds "Translate Page" (or "Show Original Page" on a translated page) to the context menu.
    pub(crate) fn append_translate_item(self: &Rc<Self>, view: &WebView, menu: &ContextMenu) {
        let uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
        if let Some(original) = untranslate_url(&uri) {
            // Already translated: only offer the way back (Translate Page comes back on the original).
            let v = view.clone();
            let act = gio::SimpleAction::new("ctx-translate-original", None);
            act.connect_activate(move |_, _| v.load_uri(&original));
            self.win.add_action(&act);
            menu.append(&ContextMenuItem::from_gaction(&act, tr("Show Original Page"), None));
            menu.append(&ContextMenuItem::new_separator());
            return;
        }
        if translate_url(&uri, "auto", "uz").is_none() {
            return; // rail://, localhost, IP, custom port ...
        }
        let v = view.clone();
        let act = gio::SimpleAction::new("ctx-translate-page", None);
        act.connect_activate(move |_, _| translate_page(&v));
        self.win.add_action(&act);
        menu.append(&ContextMenuItem::from_gaction(&act, tr("Translate Page"), None));
        menu.append(&ContextMenuItem::new_separator());
    }
}

/// Sets the "icon" attribute on every menu item whose label is in `labels`. Recurses into sections.
fn add_menu_icons(model: &gio::MenuModel, icon: &glib::Variant, labels: &[&str]) -> bool {
    let mut changed = false;
    for i in 0..model.n_items() {
        if let Some(sec) = model.item_link(i, "section") {
            changed |= add_menu_icons(&sec, icon, labels);
            continue;
        }
        let label = model
            .item_attribute_value(i, "label", None)
            .and_then(|v| v.str().map(|s| s.to_string()))
            .unwrap_or_default();
        if !labels.iter().any(|l| *l == label) {
            continue;
        }
        if model.item_attribute_value(i, "icon", None).is_some() {
            continue;
        }
        if let Some(menu) = model.downcast_ref::<gio::Menu>() {
            let item = gio::MenuItem::from_model(model, i);
            item.set_attribute_value("icon", Some(icon));
            menu.remove(i);
            menu.insert_item(i, &item);
            changed = true;
        }
    }
    changed
}

impl App {
    /// WebKit builds the context-menu popover (a GtkPopoverMenu child of the view) after the
    /// `context-menu` signal returns, so look for it a few times and add the translate icon.
    pub(crate) fn decorate_context_menu_icons(view: &WebView) {
        let Some(icon) = translate_icon_variant() else { return };
        let v = view.downgrade();
        let tries = std::rc::Rc::new(std::cell::Cell::new(0u8));
        glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
            let Some(v) = v.upgrade() else { return glib::ControlFlow::Break };
            tries.set(tries.get() + 1);
            let labels = [tr("Translate Page"), tr("Show Original Page")];
            let mut any = false;
            let mut child = v.first_child();
            while let Some(w) = child {
                if let Some(pm) = w.downcast_ref::<gtk::PopoverMenu>() {
                    if let Some(m) = pm.menu_model() {
                        any |= add_menu_icons(&m, &icon, &labels);
                    }
                }
                child = w.next_sibling();
            }
            if any || tries.get() >= 20 { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
        });
    }
}
