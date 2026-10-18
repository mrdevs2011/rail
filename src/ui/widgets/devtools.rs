//! DevTools panel: toolbar, resize handle and content box.

use crate::*;

pub(crate) struct DevtoolsWidgets {
    pub(crate) devtools_close_btn: gtk::Button,
    pub(crate) devtools_content_box: gtk::Box,
    pub(crate) devtools_detach_btn: gtk::Button,
    pub(crate) devtools_panel: gtk::Box,
    pub(crate) devtools_resize_handle: gtk::Box,
    pub(crate) devtools_url_lbl: gtk::Label,
}

pub(crate) fn build_devtools() -> DevtoolsWidgets {
    // ---- Native Rail Developer Console (F12 / Alt+Shift+I) ----
    let devtools_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    devtools_header.add_css_class("devtools-header");

    let devtools_icon = gtk::Image::from_icon_name("utilities-terminal-symbolic");
    devtools_icon.add_css_class("devtools-brand-icon");

    let devtools_title = gtk::Label::new(None);
    devtools_title.set_markup("<span font_family='ui-monospace' font_weight='800'>rail</span><span foreground='#38bdf8'>.</span><span font_family='ui-monospace' foreground='#a1a1a1'>devtools</span>");
    devtools_title.add_css_class("devtools-brand-label");

    let devtools_badge = gtk::Label::new(None);
    sc_bind_chip(&devtools_badge, "devtools");
    devtools_badge.add_css_class("devtools-badge");

    let devtools_url_lbl = gtk::Label::new(None);
    devtools_url_lbl.add_css_class("devtools-url-lbl");
    devtools_url_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
    devtools_url_lbl.set_max_width_chars(60);

    let dt_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dt_spacer.set_hexpand(true);

    let devtools_detach_btn = gtk::Button::from_icon_name("view-restore-symbolic");
    devtools_detach_btn.add_css_class("flat-icon");
    devtools_detach_btn.set_tooltip_text(Some(tr("Detach to Separate Window / Dock")));

    let devtools_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    devtools_close_btn.add_css_class("flat-icon");
    sc_bind_tooltip(&devtools_close_btn, "Close DevTools (Esc / F12)", &["devtools"]);

    devtools_header.append(&devtools_icon);
    devtools_header.append(&devtools_title);
    devtools_header.append(&devtools_badge);
    devtools_header.append(&devtools_url_lbl);
    devtools_header.append(&dt_spacer);
    devtools_header.append(&devtools_detach_btn);
    devtools_header.append(&devtools_close_btn);

    let devtools_resize_handle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    devtools_resize_handle.add_css_class("devtools-resize-handle");

    let devtools_content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    devtools_content_box.add_css_class("devtools-content-area");
    devtools_content_box.set_vexpand(true);
    devtools_content_box.set_hexpand(true);

    let devtools_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    devtools_panel.add_css_class("devtools-panel");
    devtools_panel.add_css_class("dock-bottom");
    devtools_panel.set_halign(gtk::Align::Fill);
    devtools_panel.set_valign(gtk::Align::End);
    devtools_panel.set_height_request(400);
    devtools_panel.set_visible(false);

    devtools_panel.append(&devtools_resize_handle);
    devtools_panel.append(&devtools_content_box);

    DevtoolsWidgets { devtools_close_btn, devtools_content_box, devtools_detach_btn, devtools_panel, devtools_resize_handle, devtools_url_lbl }
}
