//! Settings > Privacy & Shield.

use crate::*;

pub(crate) fn build_privacy_page(settings_stack: &gtk::Stack) {
    // --- Page 4: Privacy & Shield ---
    let page_privacy = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_privacy.add_css_class("settings-content-page");

    let priv_title = gtk::Label::new(Some(tr("Privacy & Security Shield")));
    priv_title.add_css_class("settings-page-title");
    priv_title.set_xalign(0.0);
    let priv_subtitle = gtk::Label::new(Some(tr("Container isolation and network leak prevention")));
    priv_subtitle.add_css_class("settings-page-subtitle");
    priv_subtitle.set_xalign(0.0);
    page_privacy.append(&priv_title);
    page_privacy.append(&priv_subtitle);

    let iso_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    iso_card.add_css_class("settings-card");
    let iso_title = gtk::Label::new(Some(tr("Multi-Container Storage Isolation")));
    iso_title.add_css_class("settings-card-title");
    iso_title.set_xalign(0.0);
    let iso_desc = gtk::Label::new(Some(tr("Each profile has its own WebKit data directory with separate cookies, IndexedDB and localStorage, so websites cannot link profiles through stored data. IP address, TLS fingerprint, GPU and fonts are shared between profiles and can still be used to correlate them.")));
    iso_desc.add_css_class("settings-card-desc");
    iso_desc.set_xalign(0.0);
    iso_desc.set_wrap(true);
    iso_card.append(&iso_title);
    iso_card.append(&iso_desc);
    page_privacy.append(&iso_card);

    let leak_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    leak_card.add_css_class("settings-card");
    let leak_title = gtk::Label::new(Some(tr("Network Leak Protections")));
    leak_title.add_css_class("settings-card-title");
    leak_title.set_xalign(0.0);
    leak_card.append(&leak_title);

    let webrtc_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let webrtc_lbl = gtk::Label::new(Some(tr("WebRTC Peer Connections (Enabled for video calls & audio)")));
    webrtc_lbl.add_css_class("shortcut-desc");
    webrtc_lbl.set_hexpand(true);
    webrtc_lbl.set_xalign(0.0);
    let webrtc_switch = gtk::Switch::new();
    webrtc_switch.set_active(true);
    webrtc_switch.set_valign(gtk::Align::Center);
    webrtc_row.append(&webrtc_lbl);
    webrtc_row.append(&webrtc_switch);
    leak_card.append(&webrtc_row);

    let dns_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dns_lbl = gtk::Label::new(Some(tr("DNS Prefetching Shield (Blocks unrequested domain lookup leaks)")));
    dns_lbl.add_css_class("shortcut-desc");
    dns_lbl.set_hexpand(true);
    dns_lbl.set_xalign(0.0);
    let dns_badge = gtk::Label::new(Some(tr("Shielded")));
    dns_badge.add_css_class("shortcut-kbd");
    dns_row.append(&dns_lbl);
    dns_row.append(&dns_badge);
    leak_card.append(&dns_row);

    let ping_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let ping_lbl = gtk::Label::new(Some(tr("Hyperlink Auditing (Blocks <a ping> click beacons)")));
    ping_lbl.add_css_class("shortcut-desc");
    ping_lbl.set_hexpand(true);
    ping_lbl.set_xalign(0.0);
    let ping_badge = gtk::Label::new(Some(tr("Blocked")));
    ping_badge.add_css_class("shortcut-kbd");
    ping_row.append(&ping_lbl);
    ping_row.append(&ping_badge);
    leak_card.append(&ping_row);

    page_privacy.append(&leak_card);

    let page_priv_scroll = gtk::ScrolledWindow::new();
    page_priv_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_priv_scroll.set_child(Some(&page_privacy));
    settings_stack.add_named(&page_priv_scroll, Some("privacy"));
}
