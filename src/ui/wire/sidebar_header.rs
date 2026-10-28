//! Sidebar header buttons and fixed/auto mode.

use crate::extensions;
use crate::*;

fn rebuild_extensions_popover(app: &Rc<App>, popover: &gtk::Popover) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);

    // Header
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("ext-popover-header");

    let hdr_icon = create_extension_icon(16);
    let hdr_title = gtk::Label::new(Some(tr("Extensions")));
    hdr_title.add_css_class("ext-popover-title");
    hdr_title.set_xalign(0.0);
    hdr_title.set_hexpand(true);

    header.append(&hdr_icon);
    header.append(&hdr_title);
    root.append(&header);

    // Extensions list
    let exts = extensions::get_all_extensions();
    if exts.is_empty() {
        let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        empty_box.set_margin_top(6);
        empty_box.set_margin_bottom(6);
        empty_box.set_margin_start(4);
        empty_box.set_margin_end(4);

        let empty_lbl = gtk::Label::new(Some(tr("No local extensions loaded yet")));
        empty_lbl.add_css_class("shortcut-desc");
        empty_lbl.set_wrap(true);
        empty_lbl.set_xalign(0.0);
        empty_box.append(&empty_lbl);
        root.append(&empty_box);
    } else {
        let list_box = gtk::Box::new(gtk::Orientation::Vertical, 3);
        for ext in exts {
            let item_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            item_row.add_css_class("ext-popover-item");

            let icon = gtk::Image::from_icon_name("application-x-addon-symbolic");
            icon.set_pixel_size(16);
            icon.set_valign(gtk::Align::Center);

            let text_box = gtk::Box::new(gtk::Orientation::Vertical, 1);
            text_box.set_hexpand(true);
            text_box.set_valign(gtk::Align::Center);

            let name_lbl = gtk::Label::new(Some(&ext.name));
            name_lbl.add_css_class("ext-popover-name");
            name_lbl.set_xalign(0.0);
            name_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);

            let ver_lbl = gtk::Label::new(Some(&format!("v{}", ext.version)));
            ver_lbl.add_css_class("ext-popover-ver");
            ver_lbl.set_xalign(0.0);

            text_box.append(&name_lbl);
            text_box.append(&ver_lbl);

            let sw = gtk::Switch::new();
            sw.set_active(ext.enabled);
            sw.set_valign(gtk::Align::Center);
            sw.set_halign(gtk::Align::End);
            let ext_id = ext.id.clone();
            sw.connect_state_set(move |_, state| {
                extensions::toggle_extension(&ext_id, state);
                glib::Propagation::Proceed
            });

            item_row.append(&icon);
            item_row.append(&text_box);
            item_row.append(&sw);

            list_box.append(&item_row);
        }
        root.append(&list_box);
    }

    // Separator
    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    root.append(&sep);

    // Footer Button 1: Load unpacked extension
    let btn_load = gtk::Button::new();
    btn_load.add_css_class("ext-popover-footer-btn");
    let load_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let load_icon = gtk::Image::from_icon_name("list-add-symbolic");
    load_icon.set_pixel_size(14);
    let load_lbl = gtk::Label::new(Some(tr("Load unpacked extension")));
    load_lbl.set_xalign(0.0);
    load_box.append(&load_icon);
    load_box.append(&load_lbl);
    btn_load.set_child(Some(&load_box));

    let app_load = app.clone();
    let popover_load = popover.clone();
    btn_load.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title(tr("Select unpacked extension folder (with manifest.json)"))
            .build();
        let app_cb = app_load.clone();
        let popover_cb = popover_load.clone();
        dialog.select_folder(Some(&app_load.win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    if let Ok(_) = extensions::register_unpacked_extension(&path) {
                        rebuild_extensions_popover(&app_cb, &popover_cb);
                    }
                }
            }
        });
    });
    root.append(&btn_load);

    // Footer Button 2: Manage extensions
    let btn_manage = gtk::Button::new();
    btn_manage.add_css_class("ext-popover-footer-btn");
    let manage_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let manage_icon = gtk::Image::from_icon_name("emblem-system-symbolic");
    manage_icon.set_pixel_size(14);
    let manage_lbl = gtk::Label::new(Some(tr("Manage extensions")));
    manage_lbl.set_xalign(0.0);
    manage_box.append(&manage_icon);
    manage_box.append(&manage_lbl);
    btn_manage.set_child(Some(&manage_box));

    let app_manage = app.clone();
    let popover_manage = popover.clone();
    btn_manage.connect_clicked(move |_| {
        popover_manage.popdown();
        app_manage.open_settings_tab(SettingsTab::Extensions);
    });
    root.append(&btn_manage);

    popover.set_child(Some(&root));
}

pub(crate) fn wire_sidebar_header(
    app: &Rc<App>,
    head_back: gtk::Button,
    head_forward: gtk::Button,
    head_reload: gtk::Button,
    head_ext: gtk::Button,
    head_mode: gtk::Button,
) {
    let a = app.clone();
    head_back.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            if v.can_go_back() {
                v.go_back();
            }
        }
    });

    let a = app.clone();
    head_forward.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            if v.can_go_forward() {
                v.go_forward();
            }
        }
    });

    let a = app.clone();
    head_reload.connect_clicked(move |_| {
        if let Some(v) = a.focused_view() {
            reload_or_start(&v);
        }
    });

    // Extensions popover anchored to head_ext
    let ext_popover = gtk::Popover::new();
    ext_popover.add_css_class("extensions-popover");
    ext_popover.set_parent(&head_ext);
    ext_popover.set_has_arrow(true);
    ext_popover.set_position(gtk::PositionType::Bottom);

    let app_click = app.clone();
    let pop_click = ext_popover.clone();
    head_ext.connect_clicked(move |_| {
        if pop_click.is_visible() {
            pop_click.popdown();
        } else {
            rebuild_extensions_popover(&app_click, &pop_click);
            pop_click.popup();
        }
    });

    // Cycle: fixed open ↔ auto (edge hover). Persisted to settings.json.
    let a = app.clone();
    head_mode.connect_clicked(move |_| {
        a.cycle_sidebar_mode();
    });

    // Edge-hover for auto mode: motion on the window captured before WebKit consumes it.
    let motion = gtk::EventControllerMotion::new();
    motion.set_propagation_phase(gtk::PropagationPhase::Capture);
    let last_x = std::rc::Rc::new(std::cell::Cell::new(0.0f64));

    let a_enter = app.clone();
    let last_x_enter = last_x.clone();
    motion.connect_enter(move |_, x, _y| {
        last_x_enter.set(x);
        a_enter.on_shell_pointer_x(x);
    });

    let a_motion = app.clone();
    let last_x_motion = last_x.clone();
    motion.connect_motion(move |_, x, _y| {
        last_x_motion.set(x);
        a_motion.on_shell_pointer_x(x);
    });

    let a_leave = app.clone();
    let last_x_leave = last_x.clone();
    motion.connect_leave(move |_| {
        a_leave.on_shell_pointer_leave(last_x_leave.get());
    });
    app.win.add_controller(motion);

    let a_active = app.clone();
    app.win.connect_is_active_notify(move |w| {
        if !w.is_active() && a_active.sidebar_auto.get() && a_active.side_open.get() {
            a_active.set_sidebar(false);
        }
    });
}
