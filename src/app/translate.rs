//! "Translate Page" context-menu entry (Google Translate proxy, see nav/translate.rs).

use crate::*;

fn open_translated(view: &WebView, source: &str, target: &str) {
    let current = view.uri().map(|s| s.to_string()).unwrap_or_default();
    if let Some(url) = translate_url(&current, source, target) {
        view.load_uri(&url);
    }
}

fn translate_page(view: &WebView) {
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
            // Already translated (e.g. opened from Google's "Translate this page"): offer to
            // translate the ORIGINAL again with the language pair from Settings.
            let v = view.clone();
            let orig = original.clone();
            let again = gio::SimpleAction::new("ctx-translate-page", None);
            again.connect_activate(move |_, _| {
                let s = load_app_settings();
                if let Some(url) = translate_url(&orig, &s.translate_source_lang, &s.translate_target_lang) {
                    v.load_uri(&url);
                }
            });
            self.win.add_action(&again);
            menu.append(&ContextMenuItem::from_gaction(&again, tr("Translate Page"), None));
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
