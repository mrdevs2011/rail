//! Settings > About Rail.

use crate::*;

pub(crate) fn build_about_page(settings_stack: &gtk::Stack) {
    // --- Page 6: About Rail ---
    let page_about = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_about.add_css_class("settings-content-page");

    let about_title = gtk::Label::new(Some(tr("About Rail Browser")));
    about_title.add_css_class("settings-page-title");
    about_title.set_xalign(0.0);
    let about_subtitle = gtk::Label::new(Some(tr(
        "High-performance native browser engineered in Rust",
    )));
    about_subtitle.add_css_class("settings-page-subtitle");
    about_subtitle.set_xalign(0.0);
    about_subtitle.set_wrap(true);
    page_about.append(&about_title);
    page_about.append(&about_subtitle);

    let about_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    about_card.add_css_class("settings-card");

    let brand_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let brand_logo = gtk::Label::new(None);
    brand_logo.set_markup("<span weight='bold' font_family='monospace' size='xx-large' foreground='#f8fafc'>rail</span><span weight='bold' size='xx-large' foreground='#38bdf8'>.</span>");
    let version_badge = gtk::Label::new(Some(&format!("v{}", update::current_version())));
    version_badge.add_css_class("settings-badge");
    version_badge.set_valign(gtk::Align::Center);
    brand_row.append(&brand_logo);
    brand_row.append(&version_badge);
    about_card.append(&brand_row);

    let specs = [
        (tr("Core Engine"), "WebKitGTK 4.1 / WebKit 6.0"),
        (tr("GUI Framework"), "GTK 4.0 + GSK GPU Compositor"),
        (tr("Language"), tr("100% Pure Native Rust")),
        (
            tr("Architecture"),
            tr("Zero-Electron / Sandboxed Storage Profiles"),
        ),
        (tr("Data Directory"), "~/.local/share/rail/"),
        (tr("License"), "MIT License"),
    ];
    for (k_txt, v_txt) in specs {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("shortcut-row");
        let k = gtk::Label::new(Some(k_txt));
        k.add_css_class("shortcut-desc");
        k.set_xalign(0.0);
        k.set_hexpand(true);
        k.set_wrap(true);
        let v = gtk::Label::new(Some(v_txt));
        v.add_css_class("shortcut-kbd");
        v.set_wrap(true);
        row.append(&k);
        row.append(&v);
        about_card.append(&row);
    }
    page_about.append(&about_card);

    let page_about_scroll = gtk::ScrolledWindow::new();
    page_about_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    page_about_scroll.set_child(Some(&page_about));
    settings_stack.add_named(&page_about_scroll, Some("about"));
}
