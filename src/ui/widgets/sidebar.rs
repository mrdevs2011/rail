//! Sidebar: header, address pill, tab list and footer.

use crate::*;

pub(crate) struct SidebarWidgets {
    pub(crate) footer_downloads: gtk::Button,
    pub(crate) footer_settings: gtk::Button,
    pub(crate) head_close_side: gtk::Button,
    pub(crate) head_back: gtk::Button,
    pub(crate) head_reload: gtk::Button,
    pub(crate) head_forward: gtk::Button,
    pub(crate) head_more: gtk::Button,
    pub(crate) brand_logo: gtk::Revealer,
    pub(crate) list: gtk::ListBox,
    pub(crate) pill: gtk::Button,
    pub(crate) pill_label: gtk::Label,
    pub(crate) side: gtk::Box,
}

pub(crate) fn build_sidebar() -> SidebarWidgets {
    // Brand: square Rail logo (visible only in fixed sidebar mode)
    let brand_img = create_rail_icon(22);
    brand_img.add_css_class("side-brand-logo");
    brand_img.set_halign(gtk::Align::Start);
    brand_img.set_valign(gtk::Align::Center);
    let brand_logo = gtk::Revealer::new();
    brand_logo.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    brand_logo.set_transition_duration(220);
    brand_logo.set_reveal_child(true); // default fixed mode
    brand_logo.set_hexpand(true);
    brand_logo.set_halign(gtk::Align::Start);
    brand_logo.set_child(Some(&brand_img));

    // Header: profile · back · forward · reload · sidebar mode (matches design)
    let head_more = gtk::Button::new();
    head_more.set_child(Some(&create_user_icon(16)));
    head_more.add_css_class("flat-icon");
    head_more.add_css_class("side-head-btn");
    head_more.set_tooltip_text(Some(tr("Profiles")));

    let head_back = gtk::Button::from_icon_name("go-previous-symbolic");
    head_back.add_css_class("flat-icon");
    head_back.add_css_class("side-head-btn");
    head_back.set_tooltip_text(Some(tr("Back")));

    let head_forward = gtk::Button::from_icon_name("go-next-symbolic");
    head_forward.add_css_class("flat-icon");
    head_forward.add_css_class("side-head-btn");
    head_forward.set_tooltip_text(Some(tr("Forward")));

    let head_reload = gtk::Button::from_icon_name("view-refresh-symbolic");
    head_reload.add_css_class("flat-icon");
    head_reload.add_css_class("side-head-btn");
    head_reload.set_tooltip_text(Some(tr("Reload")));
    sc_bind_tooltip(&head_reload, "Reload", &["reload"]);

    // Sidebar mode toggle (fixed ↔ auto) — dual-pane glyph
    let head_close_side = gtk::Button::from_icon_name("view-dual-symbolic");
    head_close_side.add_css_class("flat-icon");
    head_close_side.add_css_class("side-head-btn");
    head_close_side.set_tooltip_text(Some(tr("Sidebar mode")));
    sc_bind_tooltip(&head_close_side, "Toggle Sidebar", &["sidebar"]);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    actions.add_css_class("side-head-actions");
    actions.append(&head_more);
    actions.append(&head_back);
    actions.append(&head_forward);
    actions.append(&head_reload);
    actions.append(&head_close_side);

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    head.add_css_class("side-head");
    head.set_margin_start(4);
    head.append(&brand_logo);
    head.append(&actions);

    let pill_label = gtk::Label::new(Some(tr("Search or enter address")));
    pill_label.set_xalign(0.0);
    pill_label.set_hexpand(true);
    pill_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    pill_label.set_width_chars(1);
    let pill_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let s_icon = create_google_icon(18);
    s_icon.add_css_class("pill-icon");
    pill_inner.append(&s_icon);
    pill_inner.append(&pill_label);
    let pill_chip = gtk::Label::new(None);
    sc_bind_chip(&pill_chip, "focus_url");
    pill_chip.add_css_class("pill-chip");
    pill_inner.append(&pill_chip);
    let pill = gtk::Button::new();
    pill.add_css_class("pill");
    pill.set_child(Some(&pill_inner));
    sc_bind_tooltip(&pill, "Go to address  (Ctrl+L)", &["focus_url"]);

    let list = gtk::ListBox::new();
    list.add_css_class("tablist");
    list.set_selection_mode(gtk::SelectionMode::None);
    list.set_overflow(gtk::Overflow::Hidden);
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_vexpand(true);
    list_scroll.set_overflow(gtk::Overflow::Hidden);
    list_scroll.set_child(Some(&list));

    let footer_dl_lbl = gtk::Label::new(Some(tr("Downloads")));
    footer_dl_lbl.set_xalign(0.0);
    footer_dl_lbl.set_hexpand(true);
    let footer_dl_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let footer_dl_icon = gtk::Image::from_icon_name("folder-download-symbolic");
    footer_dl_inner.append(&footer_dl_icon);
    footer_dl_inner.append(&footer_dl_lbl);
    let footer_downloads = gtk::Button::new();
    footer_downloads.add_css_class("side-footer-btn");
    footer_downloads.set_child(Some(&footer_dl_inner));
    footer_downloads.set_hexpand(true);
    sc_bind_tooltip(&footer_downloads, "Open Downloads (Ctrl+J)", &["downloads"]);

    let footer_settings = gtk::Button::new();
    footer_settings.set_child(Some(&create_gear_icon(16)));
    footer_settings.add_css_class("flat-icon");
    footer_settings.set_tooltip_text(Some(tr("Settings")));
    sc_bind_tooltip(&footer_settings, "Settings", &["settings"]);

    let side_footer = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    side_footer.add_css_class("side-footer");
    side_footer.append(&footer_downloads);
    side_footer.append(&footer_settings);

    let side = gtk::Box::new(gtk::Orientation::Vertical, 10);
    side.add_css_class("side-bar");
    side.set_size_request(SIDE_W, -1);
    side.set_overflow(gtk::Overflow::Hidden);
    side.set_margin_top(10);
    side.set_margin_bottom(10);
    side.set_margin_start(8);
    side.set_margin_end(2);
    side.append(&head);
    side.append(&pill);
    side.append(&list_scroll);
    side.append(&side_footer);

    SidebarWidgets {
        footer_downloads,
        footer_settings,
        head_close_side,
        head_back,
        head_reload,
        head_forward,
        head_more,
        brand_logo,
        list,
        pill,
        pill_label,
        side,
    }
}
