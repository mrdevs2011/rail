//! Settings > Extensions.
//!
//! Manage unpacked / local Chrome and WebExtension add-ons.

use crate::extensions::{
    self, register_unpacked_extension, register_unpacked_or_archive, remove_extension,
    toggle_extension,
};
use crate::*;
use std::cell::RefCell;
use std::rc::Rc;

fn render_extensions_list(list_box: &gtk::Box, refresh: Rc<dyn Fn()>) {
    // Clear existing children
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let all = extensions::get_all_extensions();
    if all.is_empty() {
        let empty_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
        empty_card.add_css_class("settings-card");
        let empty_title = gtk::Label::new(Some(tr("No local extensions loaded yet")));
        empty_title.add_css_class("settings-card-title");
        empty_title.set_xalign(0.0);

        let empty_desc = gtk::Label::new(Some(tr(
            "You can load any unpacked Chrome or Brave extension directory containing a manifest.json file directly.",
        )));
        empty_desc.add_css_class("settings-card-desc");
        empty_desc.set_xalign(0.0);
        empty_desc.set_wrap(true);

        empty_card.append(&empty_title);
        empty_card.append(&empty_desc);
        list_box.append(&empty_card);
    } else {
        for ext in all {
            let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
            card.add_css_class("settings-card");

            // Row 1: Icon + Name + Version + Spacer + Switch + Remove
            let row1 = gtk::Box::new(gtk::Orientation::Horizontal, 8);

            let icon = extensions::extension_icon_widget(&ext, 20);

            let name_lbl = gtk::Label::new(Some(&ext.name));
            name_lbl.add_css_class("settings-card-title");

            let ver_lbl = gtk::Label::new(Some(&format!("v{}", ext.version)));
            ver_lbl.add_css_class("settings-badge");

            let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spacer.set_hexpand(true);

            let sw = gtk::Switch::new();
            sw.set_active(ext.enabled);
            sw.set_valign(gtk::Align::Center);
            let ext_id_toggle = ext.id.clone();
            sw.connect_state_set(move |_, state| {
                toggle_extension(&ext_id_toggle, state);
                glib::Propagation::Proceed
            });

            let btn_del = gtk::Button::from_icon_name("user-trash-symbolic");
            btn_del.add_css_class("flat-icon");
            btn_del.set_tooltip_text(Some(tr("Remove")));
            let ext_id_del = ext.id.clone();
            let ext_name_del = ext.name.clone();
            let refresh_del = Rc::clone(&refresh);
            btn_del.connect_clicked(move |b| {
                let alert = gtk::AlertDialog::builder()
                    .modal(true)
                    .message(tr("Are you sure you want to remove this extension?"))
                    .detail(&format!("\"{}\"", ext_name_del))
                    .buttons([tr("Cancel"), tr("Remove")])
                    .cancel_button(0)
                    .default_button(1)
                    .build();
                let ext_id = ext_id_del.clone();
                let ref_fn = refresh_del.clone();
                let parent_win = b.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                alert.choose(parent_win.as_ref(), gio::Cancellable::NONE, move |res| {
                    if res == Ok(1) {
                        remove_extension(&ext_id);
                        ref_fn();
                    }
                });
            });

            row1.append(&icon);
            row1.append(&name_lbl);
            row1.append(&ver_lbl);
            row1.append(&spacer);
            row1.append(&sw);
            row1.append(&btn_del);
            card.append(&row1);

            // Row 2: Description
            if !ext.description.is_empty() {
                let desc_lbl = gtk::Label::new(Some(&ext.description));
                desc_lbl.add_css_class("settings-card-desc");
                desc_lbl.set_xalign(0.0);
                desc_lbl.set_wrap(true);
                card.append(&desc_lbl);
            }

            // Row 3: Path + scripts count
            let cs_count = ext.content_scripts.len();
            let info_text = format!(
                "Path: {}  |  {} content script(s)",
                ext.path.display(),
                cs_count
            );
            let info_lbl = gtk::Label::new(Some(&info_text));
            info_lbl.add_css_class("shortcut-kbd");
            info_lbl.set_xalign(0.0);
            card.append(&info_lbl);

            list_box.append(&card);
        }
    }
}

pub(crate) fn build_extensions_page(settings_stack: &gtk::Stack, win: gtk::ApplicationWindow) {
    let page_ext = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_ext.add_css_class("settings-content-page");

    let title = gtk::Label::new(Some(tr("Extensions")));
    title.add_css_class("settings-page-title");
    title.set_xalign(0.0);

    let subtitle = gtk::Label::new(Some(tr(
        "Manage local unpacked Chrome and WebExtension add-ons",
    )));
    subtitle.add_css_class("settings-page-subtitle");
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);

    page_ext.append(&title);
    page_ext.append(&subtitle);

    // Top action card: Load unpacked extension / .zip / .crx
    let action_card = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    action_card.add_css_class("settings-card");

    let btn_load = gtk::Button::with_label(tr("Load unpacked folder"));
    btn_load.add_css_class("suggested-action");
    btn_load.add_css_class("ext-load-btn");

    let btn_zip = gtk::Button::with_label(tr("Install .zip / .crx"));

    let status_lbl = gtk::Label::new(None);
    status_lbl.add_css_class("shortcut-desc");
    status_lbl.set_hexpand(true);
    status_lbl.set_xalign(0.0);

    action_card.append(&btn_load);
    action_card.append(&btn_zip);
    action_card.append(&status_lbl);
    page_ext.append(&action_card);

    // Extensions list container
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    page_ext.append(&list_box);

    let list_box_rc = list_box.clone();
    let refresh_slot: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refresh_slot_c = Rc::clone(&refresh_slot);
    let refresh_fn: Rc<dyn Fn()> = Rc::new(move || {
        if let Some(r) = refresh_slot_c.borrow().as_ref() {
            render_extensions_list(&list_box_rc, Rc::clone(r));
        }
    });
    *refresh_slot.borrow_mut() = Some(Rc::clone(&refresh_fn));

    // Initial render
    refresh_fn();

    // Wire Load button
    let win_weak = win.downgrade();
    let status_c = status_lbl.clone();
    let refresh_c = Rc::clone(&refresh_fn);
    btn_load.connect_clicked(move |_| {
        let Some(w) = win_weak.upgrade() else { return };
        let dialog = gtk::FileDialog::builder()
            .title(tr("Select unpacked extension folder (with manifest.json)"))
            .build();
        let status = status_c.clone();
        let refresh = Rc::clone(&refresh_c);
        dialog.select_folder(Some(&w), gio::Cancellable::NONE, move |res| match res {
            Ok(file) => {
                if let Some(path) = file.path() {
                    match register_unpacked_extension(&path) {
                        Ok(ext) => {
                            status.set_text(&format!("{}: {}!", ext.name, tr("Saved")));
                            refresh();
                        }
                        Err(e) => {
                            status.set_text(&format!("Error: {e}"));
                        }
                    }
                }
            }
            Err(_) => {}
        });
    });

    // Wire Install .zip / .crx button
    let win_weak_zip = win.downgrade();
    let status_zip = status_lbl.clone();
    let refresh_zip = Rc::clone(&refresh_fn);
    btn_zip.connect_clicked(move |_| {
        let Some(w) = win_weak_zip.upgrade() else {
            return;
        };
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Extension Packages (*.zip, *.crx)"));
        filter.add_pattern("*.zip");
        filter.add_pattern("*.crx");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title(tr("Select .zip or .crx extension archive"))
            .filters(&filters)
            .build();
        let status = status_zip.clone();
        let refresh = Rc::clone(&refresh_zip);
        dialog.open(Some(&w), gio::Cancellable::NONE, move |res| match res {
            Ok(file) => {
                if let Some(path) = file.path() {
                    match register_unpacked_or_archive(&path) {
                        Ok(ext) => {
                            status.set_text(&format!("{}: {}!", ext.name, tr("Saved")));
                            refresh();
                        }
                        Err(e) => {
                            status.set_text(&format!("Error: {e}"));
                        }
                    }
                }
            }
            Err(_) => {}
        });
    });

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&page_ext));
    settings_stack.add_named(&scroll, Some("extensions"));
}
