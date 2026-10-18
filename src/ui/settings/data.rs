//! Settings > Data: cloud sync, archive export / import and cache.

use crate::*;

pub(crate) struct DataPage {
    pub(crate) btn_clear_cache: gtk::Button,
    pub(crate) btn_clear_hist: gtk::Button,
    pub(crate) btn_copy_sync_url: gtk::Button,
    pub(crate) btn_export_local: gtk::Button,
    pub(crate) btn_import_local: gtk::Button,
    pub(crate) btn_sync_now: gtk::Button,
    pub(crate) btn_sync_restore: gtk::Button,
    pub(crate) data_export_entry: gtk::Entry,
    pub(crate) data_import_entry: gtk::Entry,
    pub(crate) data_info_lbl: gtk::Label,
    pub(crate) data_modal_status_lbl: gtk::Label,
    pub(crate) sync_restore_entry: gtk::Entry,
    pub(crate) sync_url_result_entry: gtk::Entry,
}

pub(crate) fn build_data_page(settings_stack: &gtk::Stack) -> DataPage {
    // --- Page 3: Data ---
    let data_info_lbl = gtk::Label::new(None);
    data_info_lbl.add_css_class("data-info-card");
    data_info_lbl.set_xalign(0.0);
    data_info_lbl.set_margin_top(4);

    // Subtab 1: Export
    let sub_page_export = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_export.set_vexpand(true);

    // Section 1.1: Cloud Sync (Encrypted)
    let sync_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sync_card.add_css_class("data-section-box");

    let sync_title = gtk::Label::new(Some(tr("Cloud Sync (Encrypted)")));
    sync_title.add_css_class("data-section-title");
    sync_title.set_xalign(0.0);

    let sync_desc = gtk::Label::new(Some(tr("Upload all workspaces, tabs, profiles, logins and settings with mandatory password encryption. Generates a secure sync link.")));
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

    // Subtab 2: Import
    let sub_page_import = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_import.set_vexpand(true);

    // Section 2.1: Restore from Sync URL
    let restore_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    restore_card.add_css_class("data-section-box");

    let restore_title = gtk::Label::new(Some(tr("Restore from Sync URL")));
    restore_title.add_css_class("data-section-title");
    restore_title.set_xalign(0.0);

    let restore_desc = gtk::Label::new(Some(tr("Enter a sync URL or Sync ID. You must enter the decryption password to restore your data.")));
    restore_desc.add_css_class("data-section-desc");
    restore_desc.set_xalign(0.0);
    restore_desc.set_wrap(true);

    let restore_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sync_restore_entry = gtk::Entry::new();
    sync_restore_entry.add_css_class("profile-entry-box");
    sync_restore_entry.set_hexpand(true);
    sync_restore_entry.set_placeholder_text(Some(tr("Enter Sync URL (e.g. https://.../sync?id=...) or Sync ID...")));

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

    let local_import_desc = gtk::Label::new(Some(tr("Restore workspaces and data from an offline .tar.gz archive.")));
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

    // Subtab 3: Cache
    let sub_page_cache = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sub_page_cache.set_vexpand(true);

    let maint_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    maint_card.add_css_class("data-section-box");

    let maint_title = gtk::Label::new(Some(tr("Data Maintenance & Cache")));
    maint_title.add_css_class("data-section-title");
    maint_title.set_xalign(0.0);

    let maint_desc = gtk::Label::new(Some(tr("Clear local Omnibar suggestions history or purge temporary network disk caches.")));
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
    data_stack.add_named(&sub_page_export, Some("export"));
    data_stack.add_named(&sub_page_import, Some("import"));
    data_stack.add_named(&sub_page_cache, Some("cache"));

    let data_sub_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sub_export_btn = gtk::Button::with_label(tr("Export"));
    sub_export_btn.add_css_class("settings-option-item");
    sub_export_btn.add_css_class("active");
    let sub_import_btn = gtk::Button::with_label(tr("Import"));
    sub_import_btn.add_css_class("settings-option-item");
    let sub_cache_btn = gtk::Button::with_label(tr("Cache"));
    sub_cache_btn.add_css_class("settings-option-item");
    data_sub_bar.append(&sub_export_btn);
    data_sub_bar.append(&sub_import_btn);
    data_sub_bar.append(&sub_cache_btn);

    {
        let stack = data_stack.clone();
        let b_exp = sub_export_btn.clone();
        let b_imp = sub_import_btn.clone();
        let b_cch = sub_cache_btn.clone();
        sub_export_btn.connect_clicked(move |_| {
            stack.set_visible_child_name("export");
            b_exp.add_css_class("active");
            b_imp.remove_css_class("active");
            b_cch.remove_css_class("active");
        });
    }
    {
        let stack = data_stack.clone();
        let b_exp = sub_export_btn.clone();
        let b_imp = sub_import_btn.clone();
        let b_cch = sub_cache_btn.clone();
        sub_import_btn.connect_clicked(move |_| {
            stack.set_visible_child_name("import");
            b_imp.add_css_class("active");
            b_exp.remove_css_class("active");
            b_cch.remove_css_class("active");
        });
    }
    {
        let stack = data_stack.clone();
        let b_exp = sub_export_btn.clone();
        let b_imp = sub_import_btn.clone();
        let b_cch = sub_cache_btn.clone();
        sub_cache_btn.connect_clicked(move |_| {
            stack.set_visible_child_name("cache");
            b_cch.add_css_class("active");
            b_exp.remove_css_class("active");
            b_imp.remove_css_class("active");
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
    sc_bind_text(&data_page_subtitle, "End-to-end encrypted backup and instant cloud synchronization across devices  (Alt+I)", &["data"]);
    data_page_subtitle.add_css_class("settings-page-subtitle");
    data_page_subtitle.set_xalign(0.0);

    page_data.append(&data_page_title);
    page_data.append(&data_page_subtitle);
    page_data.append(&data_info_lbl);
    page_data.append(&data_sub_bar);
    page_data.append(&data_stack);
    page_data.append(&data_modal_status_lbl);

    let page_data_scroll = gtk::ScrolledWindow::new();
    page_data_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_data_scroll.set_child(Some(&page_data));
    settings_stack.add_named(&page_data_scroll, Some("data"));

    DataPage { btn_clear_cache, btn_clear_hist, btn_copy_sync_url, btn_export_local, btn_import_local, btn_sync_now, btn_sync_restore, data_export_entry, data_import_entry, data_info_lbl, data_modal_status_lbl, sync_restore_entry, sync_url_result_entry }
}
