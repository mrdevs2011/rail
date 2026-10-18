//! Settings > Updates: release check and changelog.

use crate::*;

pub(crate) fn build_updates_page(settings_stack: &gtk::Stack) {
    // --- Page: Updates ---
    let page_update = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_update.add_css_class("settings-content-page");

    let update_title = gtk::Label::new(Some(tr("Updates")));
    update_title.add_css_class("settings-page-title");
    update_title.set_xalign(0.0);
    let update_subtitle = gtk::Label::new(Some(tr("Check for new versions and install updates from GitHub releases")));
    update_subtitle.add_css_class("settings-page-subtitle");
    update_subtitle.set_xalign(0.0);
    update_subtitle.set_wrap(true);
    page_update.append(&update_title);
    page_update.append(&update_subtitle);

    let auto_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    auto_card.add_css_class("settings-card");
    let auto_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let auto_lbl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    auto_lbl_box.set_hexpand(true);
    let auto_title = gtk::Label::new(Some(tr("Automatic updates")));
    auto_title.add_css_class("settings-card-title");
    auto_title.set_xalign(0.0);
    let auto_desc = gtk::Label::new(Some(tr("When enabled, Rail checks for a new version every launch and installs it")));
    auto_desc.add_css_class("settings-card-desc");
    auto_desc.set_xalign(0.0);
    auto_desc.set_wrap(true);
    auto_lbl_box.append(&auto_title);
    auto_lbl_box.append(&auto_desc);
    let auto_switch = gtk::Switch::new();
    auto_switch.set_active(load_app_settings().auto_update);
    auto_switch.set_valign(gtk::Align::Center);
    auto_row.append(&auto_lbl_box);
    auto_row.append(&auto_switch);
    auto_card.append(&auto_row);
    page_update.append(&auto_card);

    {
        auto_switch.connect_state_set(move |_, state| {
            let mut s = load_app_settings();
            s.auto_update = state;
            save_app_settings(&s);
            glib::Propagation::Proceed
        });
    }

    let update_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    update_card.add_css_class("settings-card");

    let cur_ver_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let cur_lbl = gtk::Label::new(Some(tr("Current version")));
    cur_lbl.add_css_class("settings-card-title");
    cur_lbl.set_xalign(0.0);
    cur_lbl.set_hexpand(true);
    let cur_badge = gtk::Label::new(Some(&format!("v{}", update::current_version())));
    cur_badge.add_css_class("settings-badge");
    cur_ver_row.append(&cur_lbl);
    cur_ver_row.append(&cur_badge);
    update_card.append(&cur_ver_row);

    let status_lbl = gtk::Label::new(Some(tr("Checking for updates...")));
    status_lbl.add_css_class("settings-card-desc");
    status_lbl.set_xalign(0.0);
    status_lbl.set_wrap(true);
    update_card.append(&status_lbl);

    let versions_lbl = gtk::Label::new(Some(tr("Available versions (from GitHub releases)")));
    versions_lbl.add_css_class("settings-card-desc");
    versions_lbl.set_xalign(0.0);
    versions_lbl.set_margin_top(8);
    update_card.append(&versions_lbl);

    let dropdown = gtk::DropDown::from_strings(&["Loading..."]);
    dropdown.set_hexpand(true);
    update_card.append(&dropdown);

    let update_btn = gtk::Button::with_label(tr("Update now"));
    update_btn.add_css_class("settings-option-item");
    update_btn.set_halign(gtk::Align::Start);
    update_btn.set_margin_top(8);
    update_btn.set_sensitive(false);
    update_card.append(&update_btn);

    let err_lbl = gtk::Label::new(None);
    err_lbl.add_css_class("settings-card-desc");
    err_lbl.set_xalign(0.0);
    err_lbl.set_wrap(true);
    err_lbl.set_visible(false);
    update_card.append(&err_lbl);

    let update_info: Rc<std::cell::RefCell<Option<update::UpdateInfo>>> = Rc::new(std::cell::RefCell::new(None));

    {
        let update_info = update_info.clone();
        let err_lbl = err_lbl.clone();
        update_btn.connect_clicked(move |b| {
            let Some(inf) = update_info.borrow().clone() else { return; };
            b.set_sensitive(false);
            b.set_label(tr("Updating..."));
            err_lbl.set_visible(false);
            let b = b.clone();
            let err_lbl = err_lbl.clone();
            glib::spawn_future_local(async move {
                let url = inf.url.clone();
                let res = gio::spawn_blocking(move || update::download_and_install(&url)).await;
                b.set_sensitive(true);
                match res {
                    Ok(Ok(())) => {
                        b.set_label(tr("Restarting..."));
                        b.set_sensitive(false);
                        if let Some(w) = b.root().and_then(|r| r.downcast::<gtk::ApplicationWindow>().ok()) {
                            glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || update::restart(&w));
                        }
                    }
                    Ok(Err(e)) => {
                        err_lbl.set_text(&e);
                        err_lbl.set_visible(true);
                        b.set_label(tr("Retry"));
                    }
                    Err(_) => {
                        err_lbl.set_text(tr("Update failed unexpectedly."));
                        err_lbl.set_visible(true);
                        b.set_label(tr("Retry"));
                    }
                }
            });
        });
    }




    page_update.append(&update_card);

    let page_update_scroll = gtk::ScrolledWindow::new();
    page_update_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_update_scroll.set_child(Some(&page_update));
    settings_stack.add_named(&page_update_scroll, Some("update"));

    // Populate asynchronously
    {
        let status_lbl = status_lbl.clone();
        let dropdown = dropdown.clone();
        let update_btn = update_btn.clone();
        let err_lbl = err_lbl.clone();
        let update_info = update_info.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(update::fetch_releases).await;
            match res {
                Ok(Some((tags, info))) => {
                    let model = gtk::StringList::new(&[]);
                    for t in &tags {
                        model.append(t);
                    }
                    dropdown.set_model(Some(&model));
                    // Select latest or current
                    let current = update::current_version();
                    let mut selected = 0;
                    for (i, t) in tags.iter().enumerate() {
                        if t.trim_start_matches('v') == current || t == &format!("v{}", current) {
                            selected = i;
                            break;
                        }
                    }
                    dropdown.set_selected(selected as u32);

                    if let Some(inf) = info {
                        let behind = format!("{}{}", inf.newer_count, if inf.more { "+" } else { "" });
                        status_lbl.set_text(&trf(
                            if inf.newer_count == 1 { "You are on v{}, {} version behind. Latest: {}" } else { "You are on v{}, {} versions behind. Latest: {}" },
                            &[&current, &behind, &inf.latest_tag],
                        ));
                        update_btn.set_sensitive(true);
                        update_btn.set_label(tr("Update now"));
                        *update_info.borrow_mut() = Some(inf);
                    } else {
                        status_lbl.set_text(&trf("rail {} is up to date", &[&current]));
                        update_btn.set_sensitive(false);
                        update_btn.set_label(tr("Up to date"));
                    }
                }
                _ => {
                    status_lbl.set_text(tr("Could not check for updates. Check your connection."));
                    err_lbl.set_text(tr("Failed to fetch releases from GitHub."));
                    err_lbl.set_visible(true);
                }
            }
        });
    }

    {
        let dropdown = dropdown.clone();
        let update_btn = update_btn.clone();
        let update_info = update_info.clone();
        let err_lbl = err_lbl.clone();
        dropdown.connect_selected_notify(move |dd| {
            let Some(item) = dd.selected_item() else { return; };
            let Some(str_obj) = item.downcast_ref::<gtk::StringObject>() else { return; };
            let tag = str_obj.string().to_string();
            let current = update::current_version();
            let current_parsed = update::parse_ver(&current);
            let sel_parsed = update::parse_ver(&tag);
            err_lbl.set_visible(false);

            match (sel_parsed, current_parsed) {
                (Some(s), Some(c)) if s == c => {
                    update_btn.set_sensitive(false);
                    update_btn.set_label(tr("Up to date"));
                    *update_info.borrow_mut() = None;
                }
                (Some(s), Some(c)) if s < c => {
                    update_btn.set_sensitive(true);
                    update_btn.set_label(tr("Install this version"));
                    *update_info.borrow_mut() = Some(update::UpdateInfo {
                        latest_tag: tag.clone(),
                        newer_count: 0,
                        more: false,
                        url: update::asset_url_for_tag(&tag),
                    });
                }
                _ => {
                    update_btn.set_sensitive(true);
                    update_btn.set_label(tr("Update now"));
                    *update_info.borrow_mut() = Some(update::UpdateInfo {
                        latest_tag: tag.clone(),
                        newer_count: 1,
                        more: false,
                        url: update::asset_url_for_tag(&tag),
                    });
                }
            }
        });
    }
}
