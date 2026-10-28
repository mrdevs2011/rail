//! Settings > General: search engine, language and browsing controls.

use crate::*;

pub(crate) struct GeneralPage {
    pub(crate) engine_buttons: Vec<(String, gtk::Button)>,
    pub(crate) initial_settings: AppSettings,
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
    gen_subtitle.set_wrap(true);
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
    engine_desc.set_wrap(true);
    engine_card.append(&engine_title);
    engine_card.append(&engine_desc);

    let initial_settings = load_app_settings();
    let initial_engine = canonical_search_engine(&initial_settings.search_engine);

    let engines_flow = gtk::FlowBox::new();
    engines_flow.set_selection_mode(gtk::SelectionMode::None);
    engines_flow.set_max_children_per_line(5);
    engines_flow.set_min_children_per_line(1);
    engines_flow.set_row_spacing(8);
    engines_flow.set_column_spacing(8);
    engines_flow.set_homogeneous(true);
    engines_flow.add_css_class("settings-engines-flow");

    let mut engine_buttons: Vec<(String, gtk::Button)> = Vec::new();
    for &(id, label, _) in SEARCH_ENGINE_LIST {
        let btn = gtk::Button::new();
        btn.add_css_class("settings-engine-btn");
        let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        btn_box.set_halign(gtk::Align::Center);
        btn_box.set_valign(gtk::Align::Center);
        let icon = load_search_engine_icon(id, 18);
        icon.add_css_class("settings-engine-icon");
        let lbl = gtk::Label::new(Some(label));
        lbl.add_css_class("settings-engine-label");
        lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        btn_box.append(&icon);
        btn_box.append(&lbl);
        btn.set_child(Some(&btn_box));
        btn.set_tooltip_text(Some(&format!("{label} ({id})")));
        if id == initial_engine {
            btn.add_css_class("active");
        }
        engines_flow.append(&btn);
        engine_buttons.push((id.to_string(), btn));
    }
    engine_card.append(&engines_flow);
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

    // Page Translation Card
    let trl_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    trl_card.add_css_class("settings-card");
    let trl_title = gtk::Label::new(Some(tr("Page Translation")));
    trl_title.add_css_class("settings-card-title");
    trl_title.set_xalign(0.0);
    let trl_desc = gtk::Label::new(Some(tr("Right-click a page and choose Translate Page. The page address is sent to Google Translate.")));
    trl_desc.add_css_class("settings-card-desc");
    trl_desc.set_xalign(0.0);
    trl_desc.set_wrap(true);
    trl_card.append(&trl_title);
    trl_card.append(&trl_desc);

    let mut src_codes: Vec<&'static str> = Vec::new();
    let mut src_names: Vec<&'static str> = Vec::new();
    let mut tgt_codes: Vec<&'static str> = Vec::new();
    let mut tgt_names: Vec<&'static str> = Vec::new();
    for (code, name) in SUPPORTED_LANGUAGES {
        let name: &'static str = if *code == "auto" { tr("Auto-detect") } else { *name };
        src_codes.push(*code);
        src_names.push(name);
        if *code != "auto" {
            tgt_codes.push(*code);
            tgt_names.push(name);
        }
    }
    for (label, names, codes, current, is_source) in [
        (tr("Source Language"), src_names, src_codes, initial_settings.translate_source_lang.clone(), true),
        (tr("Target Language"), tgt_names, tgt_codes, initial_settings.translate_target_lang.clone(), false),
    ] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let lbl = gtk::Label::new(Some(label));
        lbl.add_css_class("shortcut-desc");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        let rows: Rc<Vec<(&'static str, &'static str)>> = Rc::new(codes.iter().copied().zip(names.iter().copied()).collect());
        let dd = gtk::DropDown::from_strings(&names);
        dd.set_factory(Some(&language_row_factory(rows.clone())));
        dd.set_list_factory(Some(&language_row_factory(rows)));
        let idx = codes.iter().position(|c| *c == current).unwrap_or(0);
        dd.set_selected(idx as u32);
        dd.connect_selected_notify(move |d| {
            let Some(code) = codes.get(d.selected() as usize) else { return };
            let mut s = load_app_settings();
            if is_source {
                s.translate_source_lang = code.to_string();
            } else {
                s.translate_target_lang = code.to_string();
            }
            save_app_settings(&s);
        });
        row.append(&lbl);
        row.append(&dd);
        trl_card.append(&row);
    }
    page_general.append(&trl_card);

    // Browsing Controls Card
    let browse_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browse_card.add_css_class("settings-card");
    let browse_title = gtk::Label::new(Some(tr("Browsing Ergonomics & Gestures")));
    browse_title.add_css_class("settings-card-title");
    browse_title.set_xalign(0.0);
    browse_card.append(&browse_title);

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
    page_gen_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    page_gen_scroll.set_child(Some(&page_general));
    settings_stack.add_named(&page_gen_scroll, Some("general"));

    GeneralPage { engine_buttons, initial_settings }
}

/// Dropdown row: country flag + language name (flag decoded lazily, only for rows that are shown).
fn language_row_factory(rows: Rc<Vec<(&'static str, &'static str)>>) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, obj| {
        let Some(item) = obj.downcast_ref::<gtk::ListItem>() else { return };
        let b = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        // gtk::Image + pixel_size keeps the same small size in the button and in the popup list
        let pic = gtk::Image::new();
        pic.add_css_class("flag-icon");
        pic.set_pixel_size(24);
        pic.set_size_request(24, 24);
        pic.set_valign(gtk::Align::Center);
        let lbl = gtk::Label::new(None);
        lbl.set_xalign(0.0);
        b.append(&pic);
        b.append(&lbl);
        item.set_child(Some(&b));
    });
    factory.connect_bind(move |_, obj| {
        let Some(item) = obj.downcast_ref::<gtk::ListItem>() else { return };
        let Some((code, name)) = rows.get(item.position() as usize) else { return };
        let Some(b) = item.child().and_downcast::<gtk::Box>() else { return };
        let Some(pic) = b.first_child().and_downcast::<gtk::Image>() else { return };
        let Some(lbl) = pic.next_sibling().and_downcast::<gtk::Label>() else { return };
        pic.set_paintable(lang_flag_texture(*code).as_ref());
        lbl.set_text(name);
    });
    factory
}
