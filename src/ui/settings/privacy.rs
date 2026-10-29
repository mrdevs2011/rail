//! Settings > Privacy & Shield.

use crate::*;

pub(crate) fn build_privacy_page(settings_stack: &gtk::Stack) {
    let page_privacy = gtk::Box::new(gtk::Orientation::Vertical, 14);
    page_privacy.add_css_class("settings-content-page");

    let priv_title = gtk::Label::new(Some(tr("Privacy & Security Shield")));
    priv_title.add_css_class("settings-page-title");
    priv_title.set_xalign(0.0);
    let priv_subtitle =
        gtk::Label::new(Some(tr("Container isolation and network leak prevention")));
    priv_subtitle.add_css_class("settings-page-subtitle");
    priv_subtitle.set_xalign(0.0);
    priv_subtitle.set_wrap(true);
    page_privacy.append(&priv_title);
    page_privacy.append(&priv_subtitle);

    // ==========================================
    // 1. AdBlocker & Tracking Shield
    // ==========================================
    let adb_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    adb_card.add_css_class("settings-card");

    let adb_title = gtk::Label::new(Some(tr("AdBlock & Tracker Shield")));
    adb_title.add_css_class("settings-card-title");
    adb_title.set_xalign(0.0);
    adb_card.append(&adb_title);

    let adb_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let adb_lbl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    adb_lbl_box.set_hexpand(true);

    let adb_lbl = gtk::Label::new(Some(tr("Block advertisements and tracking scripts")));
    adb_lbl.add_css_class("shortcut-desc");
    adb_lbl.set_xalign(0.0);
    let adb_sub = gtk::Label::new(Some(tr(
        "Filters intrusive ads, crypto miners, and behavioral telemetry.",
    )));
    adb_sub.add_css_class("settings-card-desc");
    adb_sub.set_xalign(0.0);
    adb_sub.set_wrap(true);
    adb_lbl_box.append(&adb_lbl);
    adb_lbl_box.append(&adb_sub);

    let adb_switch = gtk::Switch::new();
    adb_switch.set_active(crate::web::adblock_enabled());
    adb_switch.set_valign(gtk::Align::Center);
    adb_switch.connect_state_set(|_, state| {
        let off_file = rail_data_dir().join("adblock-off");
        if state {
            let _ = std::fs::remove_file(&off_file);
        } else {
            let _ = std::fs::write(&off_file, "1");
        }
        crate::web::setup_adblock();
        glib::Propagation::Proceed
    });

    adb_row.append(&adb_lbl_box);
    adb_row.append(&adb_switch);
    adb_card.append(&adb_row);
    page_privacy.append(&adb_card);

    // ==========================================
    // 2. Developer Security & Testing (Dasturchilar xavfsizligi)
    // ==========================================
    let dev_card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    dev_card.add_css_class("settings-card");

    let dev_title = gtk::Label::new(Some(tr("Developer Security & Testing")));
    dev_title.add_css_class("settings-card-title");
    dev_title.set_xalign(0.0);
    let dev_desc = gtk::Label::new(Some(tr(
        "Advanced security overrides for web development, API testing, and local debugging.",
    )));
    dev_desc.add_css_class("settings-card-desc");
    dev_desc.set_xalign(0.0);
    dev_desc.set_wrap(true);
    dev_card.append(&dev_title);
    dev_card.append(&dev_desc);

    // Dev 1: CORS Bypass for local dev
    let cors_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let cors_lbl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    cors_lbl_box.set_hexpand(true);
    let cors_lbl = gtk::Label::new(Some(tr(
        "Bypass CORS for Local Development (localhost & file://)",
    )));
    cors_lbl.add_css_class("shortcut-desc");
    cors_lbl.set_xalign(0.0);
    let cors_hint = gtk::Label::new(Some(tr(
        "Allows cross-origin fetch/XHR requests between local dev ports without header restrictions.",
    )));
    cors_hint.add_css_class("settings-card-desc");
    cors_hint.set_xalign(0.0);
    cors_hint.set_wrap(true);
    cors_lbl_box.append(&cors_lbl);
    cors_lbl_box.append(&cors_hint);

    let cors_switch = gtk::Switch::new();
    cors_switch.set_active(rail_data_dir().join("dev-cors-bypass").exists());
    cors_switch.set_valign(gtk::Align::Center);
    cors_switch.connect_state_set(|_, state| {
        let f = rail_data_dir().join("dev-cors-bypass");
        if state {
            let _ = std::fs::write(&f, "1");
        } else {
            let _ = std::fs::remove_file(&f);
        }
        glib::Propagation::Proceed
    });
    cors_row.append(&cors_lbl_box);
    cors_row.append(&cors_switch);
    dev_card.append(&cors_row);

    // Dev 2: Disable Cache for live reload
    let cache_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let cache_lbl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    cache_lbl_box.set_hexpand(true);
    let cache_lbl = gtk::Label::new(Some(tr("Disable Cache during Development")));
    cache_lbl.add_css_class("shortcut-desc");
    cache_lbl.set_xalign(0.0);
    let cache_hint = gtk::Label::new(Some(tr(
        "Always fetches fresh HTML, CSS, and JS from the server on reload instead of using memory cache.",
    )));
    cache_hint.add_css_class("settings-card-desc");
    cache_hint.set_xalign(0.0);
    cache_hint.set_wrap(true);
    cache_lbl_box.append(&cache_lbl);
    cache_lbl_box.append(&cache_hint);

    let cache_switch = gtk::Switch::new();
    cache_switch.set_active(rail_data_dir().join("dev-disable-cache").exists());
    cache_switch.set_valign(gtk::Align::Center);
    cache_switch.connect_state_set(|_, state| {
        let f = rail_data_dir().join("dev-disable-cache");
        if state {
            let _ = std::fs::write(&f, "1");
        } else {
            let _ = std::fs::remove_file(&f);
        }
        glib::Propagation::Proceed
    });
    cache_row.append(&cache_lbl_box);
    cache_row.append(&cache_switch);
    dev_card.append(&cache_row);

    // Dev 3: Mixed content (HTTP on HTTPS)
    let mixed_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let mixed_lbl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    mixed_lbl_box.set_hexpand(true);
    let mixed_lbl = gtk::Label::new(Some(tr("Allow Mixed Insecure Content (HTTP on HTTPS)")));
    mixed_lbl.add_css_class("shortcut-desc");
    mixed_lbl.set_xalign(0.0);
    let mixed_hint = gtk::Label::new(Some(tr(
        "Allows loading local HTTP assets (e.g. localhost:8000) inside HTTPS frontend web apps.",
    )));
    mixed_hint.add_css_class("settings-card-desc");
    mixed_hint.set_xalign(0.0);
    mixed_hint.set_wrap(true);
    mixed_lbl_box.append(&mixed_lbl);
    mixed_lbl_box.append(&mixed_hint);

    let mixed_switch = gtk::Switch::new();
    mixed_switch.set_active(rail_data_dir().join("dev-mixed-content").exists());
    mixed_switch.set_valign(gtk::Align::Center);
    mixed_switch.connect_state_set(|_, state| {
        let f = rail_data_dir().join("dev-mixed-content");
        if state {
            let _ = std::fs::write(&f, "1");
        } else {
            let _ = std::fs::remove_file(&f);
        }
        glib::Propagation::Proceed
    });
    mixed_row.append(&mixed_lbl_box);
    mixed_row.append(&mixed_switch);
    dev_card.append(&mixed_row);

    // Dev 4: DevTools badge
    let dt_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dt_lbl = gtk::Label::new(Some(tr(
        "Developer Tools & Web Inspector (F12 / Ctrl+Shift+I)",
    )));
    dt_lbl.add_css_class("shortcut-desc");
    dt_lbl.set_hexpand(true);
    dt_lbl.set_xalign(0.0);
    dt_lbl.set_wrap(true);
    let dt_badge = gtk::Label::new(Some(tr("Always Enabled")));
    dt_badge.add_css_class("shortcut-kbd");
    dt_row.append(&dt_lbl);
    dt_row.append(&dt_badge);
    dev_card.append(&dt_row);

    page_privacy.append(&dev_card);

    // ==========================================
    // 3. Network Leak Protections
    // ==========================================
    let leak_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    leak_card.add_css_class("settings-card");
    let leak_title = gtk::Label::new(Some(tr("Network Leak Protections")));
    leak_title.add_css_class("settings-card-title");
    leak_title.set_xalign(0.0);
    leak_card.append(&leak_title);

    let webrtc_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let webrtc_lbl = gtk::Label::new(Some(tr(
        "WebRTC Peer Connections (Enabled for video calls & audio)",
    )));
    webrtc_lbl.add_css_class("shortcut-desc");
    webrtc_lbl.set_hexpand(true);
    webrtc_lbl.set_xalign(0.0);
    webrtc_lbl.set_wrap(true);
    let webrtc_switch = gtk::Switch::new();
    webrtc_switch.set_active(load_app_settings().webrtc_enabled);
    webrtc_switch.set_valign(gtk::Align::Center);
    webrtc_switch.connect_state_set(|_, state| {
        let mut s = load_app_settings();
        s.webrtc_enabled = state;
        save_app_settings(&s);
        glib::Propagation::Proceed
    });
    webrtc_row.append(&webrtc_lbl);
    webrtc_row.append(&webrtc_switch);
    leak_card.append(&webrtc_row);

    let dns_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dns_lbl = gtk::Label::new(Some(tr(
        "DNS Prefetching Shield (Blocks unrequested domain lookup leaks)",
    )));
    dns_lbl.add_css_class("shortcut-desc");
    dns_lbl.set_hexpand(true);
    dns_lbl.set_xalign(0.0);
    dns_lbl.set_wrap(true);
    let dns_badge = gtk::Label::new(Some(tr("Shielded")));
    dns_badge.add_css_class("shortcut-kbd");
    dns_row.append(&dns_lbl);
    dns_row.append(&dns_badge);
    leak_card.append(&dns_row);

    let ping_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let ping_lbl = gtk::Label::new(Some(tr(
        "Hyperlink Auditing (Blocks <a ping> click beacons)",
    )));
    ping_lbl.add_css_class("shortcut-desc");
    ping_lbl.set_hexpand(true);
    ping_lbl.set_xalign(0.0);
    ping_lbl.set_wrap(true);
    let ping_badge = gtk::Label::new(Some(tr("Blocked")));
    ping_badge.add_css_class("shortcut-kbd");
    ping_row.append(&ping_lbl);
    ping_row.append(&ping_badge);
    leak_card.append(&ping_row);

    page_privacy.append(&leak_card);

    // ==========================================
    // 4. Multi-Container Storage Isolation
    // ==========================================
    let iso_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    iso_card.add_css_class("settings-card");
    let iso_title = gtk::Label::new(Some(tr("Multi-Container Storage Isolation")));
    iso_title.add_css_class("settings-card-title");
    iso_title.set_xalign(0.0);
    let iso_desc = gtk::Label::new(Some(tr(
        "Each profile has its own WebKit data directory with separate cookies, IndexedDB and localStorage, so websites cannot link profiles through stored data. IP address, TLS fingerprint, GPU and fonts are shared between profiles and can still be used to correlate them.",
    )));
    iso_desc.add_css_class("settings-card-desc");
    iso_desc.set_xalign(0.0);
    iso_desc.set_wrap(true);
    iso_card.append(&iso_title);
    iso_card.append(&iso_desc);
    page_privacy.append(&iso_card);

    let page_priv_scroll = gtk::ScrolledWindow::new();
    page_priv_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    page_priv_scroll.set_child(Some(&page_privacy));
    settings_stack.add_named(&page_priv_scroll, Some("privacy"));
}
