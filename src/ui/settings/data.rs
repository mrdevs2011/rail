//! Settings > Data: history, cloud sync, archive export / import and cache.

use crate::*;
use std::rc::Rc;

pub(crate) struct DataPage {
    pub(crate) btn_clear_cache: gtk::Button,
    pub(crate) btn_clear_hist: gtk::Button,
    pub(crate) btn_clear_hist_tab: gtk::Button,
    pub(crate) btn_copy_sync_url: gtk::Button,
    pub(crate) btn_export_local: gtk::Button,
    pub(crate) btn_import_local: gtk::Button,
    pub(crate) btn_sync_now: gtk::Button,
    pub(crate) btn_sync_restore: gtk::Button,
    pub(crate) data_export_entry: gtk::Entry,
    pub(crate) data_import_entry: gtk::Entry,
    pub(crate) data_info_lbl: gtk::Label,
    pub(crate) data_modal_status_lbl: gtk::Label,
    pub(crate) hist_list_box: gtk::Box,
    pub(crate) hist_search_entry: gtk::Entry,
    pub(crate) sub_history_btn: gtk::Button,
    pub(crate) sync_restore_entry: gtk::Entry,
    pub(crate) sync_url_result_entry: gtk::Entry,
}

pub(crate) fn build_data_page(settings_stack: &gtk::Stack) -> DataPage {
    // --- Page 3: Data ---
    let data_info_lbl = gtk::Label::new(None);
    data_info_lbl.add_css_class("data-info-card");
    data_info_lbl.set_xalign(0.0);
    data_info_lbl.set_margin_top(4);

    // ==========================================
    // Subtab 0: History (Tarix)
    // ==========================================
    let sub_page_history = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_history.set_vexpand(true);

    let hist_top_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let hist_search_entry = gtk::Entry::new();
    hist_search_entry.add_css_class("profile-entry-box");
    hist_search_entry.set_placeholder_text(Some(tr("Search history...")));
    hist_search_entry.set_hexpand(true);

    let btn_clear_hist_tab = gtk::Button::with_label(tr("Clear History"));
    btn_clear_hist_tab.add_css_class("btn-data-action");

    hist_top_row.append(&hist_search_entry);
    hist_top_row.append(&btn_clear_hist_tab);
    sub_page_history.append(&hist_top_row);

    let hist_list_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    hist_list_box.set_vexpand(true);

    let hist_scroll = gtk::ScrolledWindow::new();
    hist_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    hist_scroll.set_vexpand(true);
    hist_scroll.set_min_content_height(420);
    hist_scroll.set_child(Some(&hist_list_box));
    sub_page_history.append(&hist_scroll);

    // ==========================================
    // Subtab 1: Export
    // ==========================================
    let sub_page_export = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_export.set_vexpand(true);

    // Section 1.1: Cloud Sync (Encrypted)
    let sync_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sync_card.add_css_class("data-section-box");

    let sync_title = gtk::Label::new(Some(tr("Cloud Sync (Encrypted)")));
    sync_title.add_css_class("data-section-title");
    sync_title.set_xalign(0.0);

    let sync_desc = gtk::Label::new(Some(tr(
        "Upload all workspaces, tabs, profiles, logins and settings with mandatory password encryption. Generates a secure sync link.",
    )));
    sync_desc.add_css_class("data-section-desc");
    sync_desc.set_xalign(0.0);
    sync_desc.set_wrap(true);

    let btn_sync_now = gtk::Button::with_label(tr("Sync Your Data"));
    btn_sync_now.add_css_class("btn-data-primary");
    btn_sync_now.set_halign(gtk::Align::Start);

    let sync_result_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sync_url_result_entry = gtk::Entry::new();
    sync_url_result_entry.add_css_class("profile-entry-box");
    sync_url_result_entry.set_hexpand(true);
    sync_url_result_entry.set_placeholder_text(Some(tr("Generated Sync URL will appear here...")));
    sync_url_result_entry.set_editable(false);

    let btn_copy_sync_url = gtk::Button::with_label(tr("Copy Link"));
    btn_copy_sync_url.add_css_class("btn-data-action");

    sync_result_box.append(&sync_url_result_entry);
    sync_result_box.append(&btn_copy_sync_url);

    sync_card.append(&sync_title);
    sync_card.append(&sync_desc);
    sync_card.append(&btn_sync_now);
    sync_card.append(&sync_result_box);

    // Section 1.2: Local Archive Export
    let local_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    local_card.add_css_class("data-section-box");

    let local_title = gtk::Label::new(Some(tr("Local Archive (.tar.gz)")));
    local_title.add_css_class("data-section-title");
    local_title.set_xalign(0.0);

    let local_desc = gtk::Label::new(Some(tr("Offline backup file on your local disk.")));
    local_desc.add_css_class("data-section-desc");
    local_desc.set_xalign(0.0);

    let local_btns_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let btn_export_local = gtk::Button::with_label(tr("Export to File..."));
    btn_export_local.add_css_class("btn-data-action");

    local_btns_row.append(&btn_export_local);

    local_card.append(&local_title);
    local_card.append(&local_desc);
    local_card.append(&local_btns_row);

    sub_page_export.append(&sync_card);
    sub_page_export.append(&local_card);

    // ==========================================
    // Subtab 2: Import
    // ==========================================
    let sub_page_import = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_import.set_vexpand(true);

    // Section 2.1: Restore from Sync URL
    let restore_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    restore_card.add_css_class("data-section-box");

    let restore_title = gtk::Label::new(Some(tr("Restore from Sync URL")));
    restore_title.add_css_class("data-section-title");
    restore_title.set_xalign(0.0);

    let restore_desc = gtk::Label::new(Some(tr(
        "Enter a sync URL or Sync ID. You must enter the decryption password to restore your data.",
    )));
    restore_desc.add_css_class("data-section-desc");
    restore_desc.set_xalign(0.0);
    restore_desc.set_wrap(true);

    let restore_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sync_restore_entry = gtk::Entry::new();
    sync_restore_entry.add_css_class("profile-entry-box");
    sync_restore_entry.set_hexpand(true);
    sync_restore_entry.set_placeholder_text(Some(tr(
        "Enter Sync URL (e.g. https://.../sync?id=...) or Sync ID...",
    )));

    let btn_sync_restore = gtk::Button::with_label(tr("Restore Data"));
    btn_sync_restore.add_css_class("btn-data-primary");

    restore_row.append(&sync_restore_entry);
    restore_row.append(&btn_sync_restore);

    restore_card.append(&restore_title);
    restore_card.append(&restore_desc);
    restore_card.append(&restore_row);

    // Section 2.2: Local Archive Import
    let local_import_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    local_import_card.add_css_class("data-section-box");

    let local_import_title = gtk::Label::new(Some(tr("Local Archive Import")));
    local_import_title.add_css_class("data-section-title");
    local_import_title.set_xalign(0.0);

    let local_import_desc = gtk::Label::new(Some(tr(
        "Restore workspaces and data from an offline .tar.gz archive.",
    )));
    local_import_desc.add_css_class("data-section-desc");
    local_import_desc.set_xalign(0.0);

    let local_import_btns_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let btn_import_local = gtk::Button::with_label(tr("Import from File..."));
    btn_import_local.add_css_class("btn-data-action");

    local_import_btns_row.append(&btn_import_local);

    local_import_card.append(&local_import_title);
    local_import_card.append(&local_import_desc);
    local_import_card.append(&local_import_btns_row);

    sub_page_import.append(&restore_card);
    sub_page_import.append(&local_import_card);

    // ==========================================
    // Subtab 3: Cache
    // ==========================================
    let sub_page_cache = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_cache.set_vexpand(true);

    let maint_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    maint_card.add_css_class("data-section-box");

    let maint_title = gtk::Label::new(Some(tr("Data Maintenance & Cache")));
    maint_title.add_css_class("data-section-title");
    maint_title.set_xalign(0.0);

    let maint_desc = gtk::Label::new(Some(tr(
        "Clear local Omnibar suggestions history or purge temporary network disk caches.",
    )));
    maint_desc.add_css_class("data-section-desc");
    maint_desc.set_xalign(0.0);

    let maint_btns_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let btn_clear_hist = gtk::Button::with_label(tr("Clear Omnibar History"));
    btn_clear_hist.add_css_class("btn-data-action");
    let btn_clear_cache = gtk::Button::with_label(tr("Clear Disk Cache"));
    btn_clear_cache.add_css_class("btn-data-action");

    maint_btns_row.append(&btn_clear_hist);
    maint_btns_row.append(&btn_clear_cache);

    maint_card.append(&maint_title);
    maint_card.append(&maint_desc);
    maint_card.append(&maint_btns_row);

    sub_page_cache.append(&maint_card);

    // Data Stack & Subtabs bar
    let data_stack = gtk::Stack::new();
    data_stack.set_vexpand(true);
    data_stack.add_named(&sub_page_history, Some("history"));
    data_stack.add_named(&sub_page_export, Some("export"));
    data_stack.add_named(&sub_page_import, Some("import"));
    data_stack.add_named(&sub_page_cache, Some("cache"));

    let data_sub_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sub_history_btn = gtk::Button::with_label(tr("History"));
    sub_history_btn.add_css_class("settings-option-item");
    sub_history_btn.add_css_class("active");
    let sub_export_btn = gtk::Button::with_label(tr("Export"));
    sub_export_btn.add_css_class("settings-option-item");
    let sub_import_btn = gtk::Button::with_label(tr("Import"));
    sub_import_btn.add_css_class("settings-option-item");
    let sub_cache_btn = gtk::Button::with_label(tr("Cache"));
    sub_cache_btn.add_css_class("settings-option-item");
    data_sub_bar.append(&sub_history_btn);
    data_sub_bar.append(&sub_export_btn);
    data_sub_bar.append(&sub_import_btn);
    data_sub_bar.append(&sub_cache_btn);

    let all_sub_btns = vec![
        ("history", sub_history_btn.clone()),
        ("export", sub_export_btn.clone()),
        ("import", sub_import_btn.clone()),
        ("cache", sub_cache_btn.clone()),
    ];

    for (name, btn) in all_sub_btns.iter() {
        let stack = data_stack.clone();
        let target_name = name.to_string();
        let btns = all_sub_btns.clone();
        btn.connect_clicked(move |_| {
            stack.set_visible_child_name(&target_name);
            for (n, b) in btns.iter() {
                if n == &target_name {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
        });
    }

    // Keep hidden entries for legacy CLI or local paths if needed
    let data_export_entry = gtk::Entry::new();
    let data_import_entry = gtk::Entry::new();

    let data_modal_status_lbl = gtk::Label::new(None);
    data_modal_status_lbl.add_css_class("data-status-lbl");
    data_modal_status_lbl.set_xalign(0.0);
    data_modal_status_lbl.set_wrap(true);

    let page_data = gtk::Box::new(gtk::Orientation::Vertical, 8);
    page_data.add_css_class("settings-content-page");

    let data_page_title = gtk::Label::new(Some(tr("Data")));
    data_page_title.add_css_class("settings-page-title");
    data_page_title.set_xalign(0.0);
    let data_page_subtitle = gtk::Label::new(None);
    sc_bind_text(
        &data_page_subtitle,
        "End-to-end encrypted backup, browsing history and instant cloud synchronization  (Alt+I)",
        &["data"],
    );
    data_page_subtitle.add_css_class("settings-page-subtitle");
    data_page_subtitle.set_xalign(0.0);

    page_data.append(&data_page_title);
    page_data.append(&data_page_subtitle);
    page_data.append(&data_info_lbl);
    page_data.append(&data_sub_bar);
    page_data.append(&data_stack);
    page_data.append(&data_modal_status_lbl);

    let page_data_scroll = gtk::ScrolledWindow::new();
    page_data_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    page_data_scroll.set_child(Some(&page_data));
    settings_stack.add_named(&page_data_scroll, Some("data"));

    DataPage {
        btn_clear_cache,
        btn_clear_hist,
        btn_clear_hist_tab,
        btn_copy_sync_url,
        btn_export_local,
        btn_import_local,
        btn_sync_now,
        btn_sync_restore,
        data_export_entry,
        data_import_entry,
        data_info_lbl,
        data_modal_status_lbl,
        hist_list_box,
        hist_search_entry,
        sub_history_btn,
        sync_restore_entry,
        sync_url_result_entry,
    }
}

pub(crate) fn render_history_list(
    box_widget: &gtk::Box,
    items: &[crate::history::HistoryEntry],
    filter: &str,
    app: &Rc<App>,
    hist_search_entry: &gtk::Entry,
) {
    while let Some(c) = box_widget.first_child() {
        box_widget.remove(&c);
    }

    let filter_lower = filter.trim().to_lowercase();
    let mut rendered_count = 0usize;
    let mut last_date = String::new();

    for entry in items {
        if !filter_lower.is_empty() {
            let u_match = entry.url.to_lowercase().contains(&filter_lower);
            let t_match = entry.title.to_lowercase().contains(&filter_lower);
            if !u_match && !t_match {
                continue;
            }
        }

        let date_str = if let Ok(dt) = glib::DateTime::from_unix_local(entry.last_visit) {
            let now = glib::DateTime::now_local().unwrap_or_else(|_| dt.clone());
            if dt.year() == now.year() && dt.day_of_year() == now.day_of_year() {
                tr("Today").to_string()
            } else if dt.year() == now.year() && dt.day_of_year() + 1 == now.day_of_year() {
                tr("Yesterday").to_string()
            } else {
                dt.format("%A, %d %B %Y")
                    .unwrap_or_else(|_| dt.format("%Y-%m-%d").unwrap())
                    .to_string()
            }
        } else {
            tr("Earlier").to_string()
        };

        if date_str != last_date {
            let hdr = gtk::Label::new(Some(&date_str));
            hdr.add_css_class("data-section-title");
            hdr.set_xalign(0.0);
            hdr.set_margin_top(8);
            hdr.set_margin_bottom(2);
            box_widget.append(&hdr);
            last_date = date_str;
        }

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("hist-row");
        row.set_valign(gtk::Align::Center);

        // Time
        let time_str = glib::DateTime::from_unix_local(entry.last_visit)
            .and_then(|d| d.format("%d.%m.%Y  %H:%M"))
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "--.--.----  --:--".into());
        let time_lbl = gtk::Label::new(Some(&time_str));
        time_lbl.add_css_class("shortcut-kbd");
        time_lbl.set_valign(gtk::Align::Center);
        row.append(&time_lbl);

        // Icon
        let icon = gtk::Image::from_icon_name("document-open-recent-symbolic");
        icon.add_css_class("palette-sug-icon");
        icon.set_valign(gtk::Align::Center);
        row.append(&icon);

        // Info: Title & URL
        let info_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        info_box.set_hexpand(true);
        info_box.set_valign(gtk::Align::Center);

        let title_text = if entry.title.trim().is_empty() {
            crate::history::extract_clean_domain(&entry.url)
        } else {
            entry.title.clone()
        };
        let title_lbl = gtk::Label::new(Some(&title_text));
        title_lbl.add_css_class("settings-card-title");
        title_lbl.set_xalign(0.0);
        title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let url_lbl = gtk::Label::new(Some(&entry.url));
        url_lbl.add_css_class("settings-card-desc");
        url_lbl.add_css_class("history-url-link");
        url_lbl.set_xalign(0.0);
        url_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        url_lbl.set_cursor_from_name(Some("pointer"));
        url_lbl.set_tooltip_text(Some(tr("Open in a new tab")));

        let open_link = gtk::GestureClick::new();
        let a_open = app.clone();
        let u_open = entry.url.clone();
        open_link.connect_released(move |_, _, _, _| {
            a_open.new_tab_with_url(&u_open);
            a_open.hide_settings_modal();
        });
        url_lbl.add_controller(open_link);

        for l in [&title_lbl, &url_lbl] {
            l.set_hexpand(true);
            l.set_width_chars(1);
            l.set_max_width_chars(1);
        }
        info_box.append(&title_lbl);
        info_box.append(&url_lbl);
        row.append(&info_box);

        // Delete button
        let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
        del_btn.add_css_class("flat-icon");
        del_btn.set_tooltip_text(Some(tr("Remove")));
        let a_del = app.clone();
        let u_del = entry.url.clone();
        let b_box = box_widget.clone();
        let s_ent = hist_search_entry.clone();
        del_btn.connect_clicked(move |_| {
            a_del.history.borrow_mut().remove_entry(&u_del);
            let items = a_del.history.borrow().get_all_sorted();
            render_history_list(&b_box, &items, &s_ent.text(), &a_del, &s_ent);
        });
        row.append(&del_btn);

        box_widget.append(&row);
        rendered_count += 1;
    }

    if rendered_count == 0 {
        let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        empty_box.set_valign(gtk::Align::Center);
        empty_box.set_halign(gtk::Align::Center);
        empty_box.set_margin_top(40);
        let empty_icon = gtk::Image::from_icon_name("document-open-recent-symbolic");
        empty_icon.set_pixel_size(48);
        empty_icon.add_css_class("settings-card-desc");
        let empty_lbl = gtk::Label::new(Some(tr("No browsing history yet")));
        empty_lbl.add_css_class("settings-card-title");
        empty_box.append(&empty_icon);
        empty_box.append(&empty_lbl);
        box_widget.append(&empty_box);
    }
}
