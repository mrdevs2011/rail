//! Address palette: entry, profile picker and suggestions.

use crate::*;

pub(crate) struct PaletteWidgets {
    pub(crate) entry: gtk::Entry,
    pub(crate) palette: gtk::Box,
    pub(crate) palette_suggestions_box: gtk::Box,
    pub(crate) profile_btn: gtk::Button,
    pub(crate) profile_btn_lbl: gtk::Label,
    pub(crate) profile_popover: gtk::Popover,
}

pub(crate) fn build_palette(profile_mgr: &ProfileManager) -> PaletteWidgets {
    // ---- address palette ----
    let entry = gtk::Entry::new();
    entry.add_css_class("palette-entry");
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some(tr("Search Google or enter web address...")));

    let sicon = create_google_icon(20);

    let profile_btn = gtk::Button::new();
    profile_btn.add_css_class("profile-circle-btn");
    profile_btn.set_valign(gtk::Align::Center);
    profile_btn.set_halign(gtk::Align::End);
    let active_p = profile_mgr.active_profile();
    let profile_btn_lbl = gtk::Label::new(None);
    profile_btn_lbl.set_markup(&format!(
        "<span foreground='{}' font_weight='800'>{}</span>",
        active_p.text_color(),
        glib::markup_escape_text(&active_p.initial_letter())
    ));
    profile_btn_lbl.add_css_class("profile-circle-label");
    profile_btn.add_css_class(&format!("prof-bg-{}", active_p.color_idx));
    profile_btn.set_child(Some(&profile_btn_lbl));

    let profile_popover = gtk::Popover::new();
    profile_popover.add_css_class("profile-popover");
    profile_popover.set_parent(&profile_btn);
    profile_popover.set_has_arrow(true);

    let prow = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    prow.append(&sicon);
    prow.append(&entry);
    prow.append(&profile_btn);

    let palette_suggestions_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    palette_suggestions_box.add_css_class("palette-suggestions-box");

    let palette_suggestions_scroll = gtk::ScrolledWindow::new();
    palette_suggestions_scroll.add_css_class("palette-suggestions-scroll");
    palette_suggestions_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    palette_suggestions_scroll.set_overlay_scrolling(false);
    palette_suggestions_scroll.set_min_content_height(0);
    palette_suggestions_scroll.set_max_content_height(220);
    palette_suggestions_scroll.set_child(Some(&palette_suggestions_box));
    palette_suggestions_scroll.set_visible(false);
    palette_suggestions_scroll.set_propagate_natural_height(true);

    let phint = gtk::Label::new(None);
    sc_bind_text(&phint, "Enter  go      Esc  cancel      Alt+P  profiles", &["profiles"]);
    phint.add_css_class("palette-hint");
    phint.set_xalign(0.0);
    let palette = gtk::Box::new(gtk::Orientation::Vertical, 0);
    palette.add_css_class("palette");
    palette.set_halign(gtk::Align::Center);
    palette.set_valign(gtk::Align::Start);
    palette.set_margin_top(90);
    palette.set_margin_start(16);
    palette.set_margin_end(16);
    palette.set_size_request(580, -1);
    palette.append(&prow);
    palette.append(&palette_suggestions_scroll);
    palette.append(&phint);
    palette.set_visible(false);

    let pop_c = profile_popover.clone();
    profile_btn.connect_clicked(move |_| {
        pop_c.popup();
    });

    PaletteWidgets { entry, palette, palette_suggestions_box, profile_btn, profile_btn_lbl, profile_popover }
}
