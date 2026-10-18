//! Minimap / roadmap window.

use crate::*;

pub(crate) struct MinimapWidgets {
    pub(crate) map_close_btn: gtk::Button,
    pub(crate) map_container: gtk::Box,
    pub(crate) map_drawing_area: gtk::DrawingArea,
    pub(crate) map_header_row: gtk::Box,
    pub(crate) map_location_btn: gtk::Button,
}

pub(crate) fn build_minimap() -> MinimapWidgets {
    // ---- Minimap / Roadmap (Alt+M) ----
    let map_header_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    map_header_row.add_css_class("minimap-header");
    map_header_row.set_cursor_from_name(Some("grab"));

    let map_icon = gtk::Image::from_icon_name("view-grid-symbolic");
    map_icon.add_css_class("minimap-header-icon");

    let map_title = gtk::Label::new(None);
    map_title.set_markup("<span>roadmap</span><span foreground='#38bdf8'>.</span>");
    map_title.add_css_class("minimap-header-title");
    map_title.set_hexpand(true);
    map_title.set_xalign(0.0);

    let map_badge = gtk::Label::new(None);
    sc_bind_chip(&map_badge, "map");
    map_badge.add_css_class("minimap-badge");

    let map_hint = gtk::Label::new(Some(tr("Drag header")));
    map_hint.add_css_class("minimap-header-hint");
    map_hint.set_xalign(0.0);

    let map_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    map_close_btn.add_css_class("flat-icon");
    sc_bind_tooltip(&map_close_btn, "Close Map (Alt+M / Esc)", &["map"]);
    map_close_btn.set_cursor_from_name(Some("pointer"));

    map_header_row.append(&map_icon);
    map_header_row.append(&map_title);
    map_header_row.append(&map_badge);
    map_header_row.append(&map_hint);
    map_header_row.append(&map_close_btn);

    let map_drawing_area = gtk::DrawingArea::new();
    map_drawing_area.add_css_class("minimap-canvas");
    map_drawing_area.set_content_width(380);
    map_drawing_area.set_content_height(260);

    let map_location_btn = gtk::Button::new();
    let loc_box = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let loc_icon = gtk::Image::from_icon_name("find-location-symbolic");
    let loc_lbl = gtk::Label::new(Some(tr("recenter")));
    loc_box.append(&loc_icon);
    loc_box.append(&loc_lbl);
    map_location_btn.set_child(Some(&loc_box));
    map_location_btn.add_css_class("minimap-location-btn");
    map_location_btn.set_halign(gtk::Align::Center);
    map_location_btn.set_valign(gtk::Align::End);
    map_location_btn.set_margin_bottom(12);
    map_location_btn.set_visible(false);

    let map_overlay = gtk::Overlay::new();
    map_overlay.set_child(Some(&map_drawing_area));
    map_overlay.add_overlay(&map_location_btn);

    let map_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    map_container.add_css_class("minimap-window");
    map_container.set_halign(gtk::Align::End);
    map_container.set_valign(gtk::Align::End);
    map_container.set_margin_end(24);
    map_container.set_margin_bottom(24);
    map_container.set_visible(false);

    map_container.append(&map_header_row);
    map_container.append(&map_overlay);

    MinimapWidgets { map_close_btn, map_container, map_drawing_area, map_header_row, map_location_btn }
}
