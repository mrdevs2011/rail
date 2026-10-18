//! Settings > General: search engine, language and browsing controls.

use crate::*;

pub(crate) struct GeneralPage {
    pub(crate) engine_buttons: Vec<(String, gtk::Button)>,
    pub(crate) initial_settings: AppSettings,
    pub(crate) tp_switch: gtk::Switch,
}

pub(crate) fn build_general_page(settings_stack: &gtk::Stack) -> GeneralPage {
    // --- Page 1: General ---
    let page_general = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_general.add_css_class("settings-content-page");

    let gen_title = gtk::Label::new(Some(tr("General Preferences")));
    gen_title.add_css_class("settings-page-title");
    gen_title.set_xalign(0.0);
    let gen_subtitle = gtk::Label::new(Some(tr("Configure default search engine, new tab behavior, and browsing controls")));
    gen_subtitle.add_css_class("settings-page-subtitle");
    gen_subtitle.set_xalign(0.0);
    page_general.append(&gen_title);
    page_general.append(&gen_subtitle);

    // Search Engine Card
    let engine_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    engine_card.add_css_class("settings-card");
    let engine_title = gtk::Label::new(Some(tr("Default Search Engine")));
    engine_title.add_css_class("settings-card-title");
    engine_title.set_xalign(0.0);
    let engine_desc = gtk::Label::new(None);
    sc_bind_text(&engine_desc, "Applied when searching via the Omnibar (Ctrl+T / Ctrl+L)", &["new_tab", "focus_url"]);
    engine_desc.add_css_class("settings-card-desc");
    engine_desc.set_xalign(0.0);
    engine_card.append(&engine_title);
    engine_card.append(&engine_desc);

    let initial_settings = load_app_settings();
    let initial_engine = initial_settings.search_engine.clone();

    let engines_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let engine_choices = [
        ("google", "Google"),
        ("duckduckgo", "DuckDuckGo"),
        ("brave", "Brave"),
        ("bing", "Bing"),
        ("ecosia", "Ecosia"),
    ];
    let mut engine_buttons: Vec<(String, gtk::Button)> = Vec::new();
    for (id, label) in engine_choices {
        let btn = gtk::Button::with_label(label);
        btn.add_css_class("settings-option-item");
        if id == initial_engine {
            btn.add_css_class("active");
        }
        engines_row.append(&btn);
        engine_buttons.push((id.to_string(), btn));
    }
    engine_card.append(&engines_row);
    page_general.append(&engine_card);

    // Language Card
    let lang_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    lang_card.add_css_class("settings-card");
    let lang_title = gtk::Label::new(Some(tr("Interface Language")));
    lang_title.add_css_class("settings-card-title");
    lang_title.set_xalign(0.0);
    let lang_desc = gtk::Label::new(Some(tr("Choose the language of Rail menus and settings. Applies after restart.")));
    lang_desc.add_css_class("settings-card-desc");
    lang_desc.set_xalign(0.0);
    lang_desc.set_wrap(true);
    lang_card.append(&lang_title);
    lang_card.append(&lang_desc);
    let lang_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let lang_status = gtk::Label::new(None);
    lang_status.add_css_class("settings-card-desc");
    lang_status.set_xalign(0.0);
    lang_status.set_visible(false);
    let lang_restart = gtk::Button::with_label(tr("Restart now"));
    lang_restart.add_css_class("settings-option-item");
    lang_restart.set_halign(gtk::Align::Start);
    lang_restart.set_visible(false);
    lang_restart.connect_clicked(|b| {
        if let Some(w) = b.root().and_then(|r| r.downcast::<gtk::ApplicationWindow>().ok()) {
            update::restart(&w);
        }
    });
    let mut lang_buttons: Vec<(i18n::Lang, gtk::Button)> = Vec::new();
    for (lang, name) in [(i18n::Lang::Uz, "O'zbekcha"), (i18n::Lang::En, "English"), (i18n::Lang::Ru, "Русский")] {
        let btn = gtk::Button::with_label(name);
        btn.add_css_class("settings-option-item");
        if lang == i18n::current() {
            btn.add_css_class("active");
        }
        lang_row.append(&btn);
        lang_buttons.push((lang, btn));
    }
    for (lang, btn) in lang_buttons.iter() {
        let lang = *lang;
        let all = lang_buttons.clone();
        let status = lang_status.clone();
        let restart_btn = lang_restart.clone();
        btn.connect_clicked(move |_| {
            if i18n::set_language(lang) {
                for (l, b) in all.iter() {
                    if *l == lang {
                        b.add_css_class("active");
                    } else {
                        b.remove_css_class("active");
                    }
                }
                let changed = lang != i18n::current();
                status.set_text(tr("Language saved. Restart Rail to apply."));
                status.set_visible(changed);
                restart_btn.set_visible(changed);
            }
        });
    }
    lang_card.append(&lang_row);
    lang_card.append(&lang_status);
    lang_card.append(&lang_restart);
    page_general.append(&lang_card);

    // Browsing Controls Card
    let browse_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browse_card.add_css_class("settings-card");
    let browse_title = gtk::Label::new(Some(tr("Browsing Ergonomics & Gestures")));
    browse_title.add_css_class("settings-card-title");
    browse_title.set_xalign(0.0);
    browse_card.append(&browse_title);

    let tp_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let tp_lbl = gtk::Label::new(Some(tr("Touchpad Canvas Navigation (Two-finger swipe navigates 2D canvas)")));
    tp_lbl.add_css_class("shortcut-desc");
    tp_lbl.set_hexpand(true);
    tp_lbl.set_xalign(0.0);
    let tp_switch = gtk::Switch::new();
    tp_switch.set_active(false);
    tp_switch.set_valign(gtk::Align::Center);
    tp_row.append(&tp_lbl);
    tp_row.append(&tp_switch);
    browse_card.append(&tp_row);

    let scroll_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let scroll_lbl = gtk::Label::new(Some(tr("WebKit Smooth Kinetic Scrolling")));
    scroll_lbl.add_css_class("shortcut-desc");
    scroll_lbl.set_hexpand(true);
    scroll_lbl.set_xalign(0.0);
    let scroll_switch = gtk::Switch::new();
    scroll_switch.set_active(true);
    scroll_switch.set_valign(gtk::Align::Center);
    scroll_row.append(&scroll_lbl);
    scroll_row.append(&scroll_switch);
    browse_card.append(&scroll_row);

    let hw_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let hw_lbl = gtk::Label::new(Some(tr("Hardware Acceleration Policy")));
    hw_lbl.add_css_class("shortcut-desc");
    hw_lbl.set_hexpand(true);
    hw_lbl.set_xalign(0.0);
    let hw_badge = gtk::Label::new(Some(tr("Always (GSK GPU)")));
    hw_badge.add_css_class("shortcut-kbd");
    hw_row.append(&hw_lbl);
    hw_row.append(&hw_badge);
    browse_card.append(&hw_row);

    page_general.append(&browse_card);

    let page_gen_scroll = gtk::ScrolledWindow::new();
    page_gen_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_gen_scroll.set_child(Some(&page_general));
    settings_stack.add_named(&page_gen_scroll, Some("general"));

    GeneralPage { engine_buttons, initial_settings, tp_switch }
}
