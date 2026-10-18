//! Settings > Profiles: profile list and the create-profile form.

use crate::*;

pub(crate) struct ProfilesPage {
    pub(crate) add_prof_btn: gtk::Button,
    pub(crate) color_btns: Vec<gtk::Button>,
    pub(crate) new_profile_color_idx: Cell<usize>,
    pub(crate) new_profile_entry: gtk::Entry,
    pub(crate) new_profile_error_lbl: gtk::Label,
    pub(crate) page_prof_scroll: gtk::ScrolledWindow,
    pub(crate) profiles_list_box: gtk::ListBox,
}

pub(crate) fn build_profiles_page(settings_stack: &gtk::Stack) -> ProfilesPage {
    // --- Page 2: Profiles ---
    let profiles_list_box = gtk::ListBox::new();
    profiles_list_box.add_css_class("settings-profiles-list");
    profiles_list_box.set_selection_mode(gtk::SelectionMode::None);

    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(8);
    sep.set_margin_bottom(8);

    let new_lbl = gtk::Label::new(Some(tr("CREATE NEW PROFILE")));
    new_lbl.add_css_class("profile-section-label");
    new_lbl.set_xalign(0.0);

    let new_profile_entry = gtk::Entry::new();
    new_profile_entry.add_css_class("profile-entry-box");
    new_profile_entry.set_placeholder_text(Some(tr("Enter unique profile name (e.g. Work, School)...")));
    new_profile_entry.set_hexpand(true);

    let new_profile_error_lbl = gtk::Label::new(None);
    new_profile_error_lbl.add_css_class("profile-error-label");
    new_profile_error_lbl.set_xalign(0.0);
    new_profile_error_lbl.set_visible(false);

    let add_prof_btn = gtk::Button::with_label(tr("Create Profile"));
    add_prof_btn.add_css_class("btn-create-profile");
    add_prof_btn.set_sensitive(false);
    add_prof_btn.set_valign(gtk::Align::Center);

    let color_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    color_box.set_valign(gtk::Align::Center);
    let mut color_btns = Vec::new();
    let new_profile_color_idx = Cell::new(0usize);

    for i in 0..8 {
        let cb = gtk::Button::new();
        cb.add_css_class("color-choice");
        cb.add_css_class(&format!("prof-bg-{}", i));
        cb.set_valign(gtk::Align::Center);
        cb.set_halign(gtk::Align::Center);
        if i == 0 {
            cb.add_css_class("selected");
        }
        color_box.append(&cb);
        color_btns.push(cb);
    }

    let create_bottom_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    create_bottom_row.set_margin_top(4);
    create_bottom_row.append(&color_box);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    create_bottom_row.append(&spacer);
    create_bottom_row.append(&add_prof_btn);

    let page_profiles = gtk::Box::new(gtk::Orientation::Vertical, 6);
    page_profiles.add_css_class("settings-content-page");

    let prof_page_title = gtk::Label::new(Some(tr("Profiles & Workspaces")));
    prof_page_title.add_css_class("settings-page-title");
    prof_page_title.set_xalign(0.0);
    let prof_page_subtitle = gtk::Label::new(None);
    sc_bind_text(&prof_page_subtitle, "Isolated workspaces, sessions & cookie storage  (Alt+P)", &["profiles"]);
    prof_page_subtitle.add_css_class("settings-page-subtitle");
    prof_page_subtitle.set_xalign(0.0);

    page_profiles.append(&prof_page_title);
    page_profiles.append(&prof_page_subtitle);
    page_profiles.append(&profiles_list_box);
    page_profiles.append(&sep);
    page_profiles.append(&new_lbl);
    page_profiles.append(&new_profile_entry);
    page_profiles.append(&new_profile_error_lbl);
    page_profiles.append(&create_bottom_row);

    let page_prof_scroll = gtk::ScrolledWindow::new();
    page_prof_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_prof_scroll.set_vexpand(true);
    page_prof_scroll.set_hexpand(true);
    page_prof_scroll.set_child(Some(&page_profiles));
    settings_stack.add_named(&page_prof_scroll, Some("profiles"));

    ProfilesPage { add_prof_btn, color_btns, new_profile_color_idx, new_profile_entry, new_profile_error_lbl, page_prof_scroll, profiles_list_box }
}
