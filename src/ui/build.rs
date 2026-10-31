//! Builds the main window and wires every widget to the App.

use crate::*;

pub(crate) fn build(gapp: &gtk::Application) {
    setup_adblock();
    let provider = gtk::CssProvider::new();
    install_styles(&provider);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );

    let profile_mgr = ProfileManager::new(rail_data_dir());
    let history_mgr = HistoryManager::new(rail_data_dir());
    let initial_prof_id = profile_mgr.active_profile().id;

    let SidebarWidgets {
        footer_downloads,
        footer_settings,
        head_close_side,
        head_back,
        head_forward,
        head_reload,
        brand_logo,
        list,
        pill,
        pill_label,
        pill_engine_icon,
        pill_pwa_btn,
        side,
    } = build_sidebar();

    let StripWidgets {
        layout,
        shell,
        shell_layout,
        strip,
        strip_host,
    } = build_strip(side.clone());

    let PaletteWidgets {
        entry,
        palette,
        palette_suggestions_box,
        palette_engine_icon,
        profile_btn,
        profile_btn_lbl,
        profile_popover,
    } = build_palette(&profile_mgr);

    // ---- settings page in the tab square (Alt+S) — rail://settings, not window fullscreen ----
    let settings_modal = gtk::Box::new(gtk::Orientation::Vertical, 0);
    settings_modal.add_css_class("settings-modal");
    settings_modal.set_halign(gtk::Align::Fill);
    settings_modal.set_valign(gtk::Align::Fill);
    settings_modal.set_hexpand(true);
    settings_modal.set_vexpand(true);
    settings_modal.set_can_target(true);
    settings_modal.set_focusable(true);
    settings_modal.set_visible(false);
    // Fills the strip (tab area) only; sidebar stays visible

    // Settings Header
    let settings_header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    settings_header.add_css_class("settings-header");

    let settings_icon = gtk::Image::from_icon_name("preferences-system-symbolic");
    settings_icon.add_css_class("settings-header-icon");

    let settings_title = gtk::Label::new(Some(tr("Settings")));
    settings_title.add_css_class("settings-title");

    let settings_badge = gtk::Label::new(Some(tr("rail.preferences")));
    settings_badge.add_css_class("settings-badge");

    // Ads-style chip: visible only when a newer release exists
    let settings_update_chip = gtk::Button::with_label(tr("New version"));
    settings_update_chip.add_css_class("settings-update-chip");
    settings_update_chip.set_visible(false);
    settings_update_chip.set_tooltip_text(Some(tr("Open Updates")));

    let header_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    header_spacer.set_hexpand(true);

    let settings_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    settings_close_btn.add_css_class("flat-icon");
    settings_close_btn.set_tooltip_text(Some(tr("Close tab (Ctrl+W)")));

    settings_header.append(&settings_icon);
    settings_header.append(&settings_title);
    settings_header.append(&settings_badge);
    settings_header.append(&settings_update_chip);
    settings_header.append(&header_spacer);
    settings_header.append(&settings_close_btn);
    settings_modal.append(&settings_header);

    // Settings Body (Left sidebar + Right stack)
    let settings_body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    settings_body.add_css_class("settings-body");
    settings_body.set_hexpand(true);
    settings_body.set_vexpand(true);

    let settings_sidebar = gtk::Box::new(gtk::Orientation::Vertical, 4);
    settings_sidebar.add_css_class("settings-sidebar");

    let settings_stack = gtk::Stack::new();
    settings_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    settings_stack.set_transition_duration(150);
    settings_stack.set_hexpand(true);
    settings_stack.set_vexpand(true);

    let sidebar_items = [
        (
            SettingsTab::General,
            "preferences-system-symbolic",
            tr("General"),
        ),
        (
            SettingsTab::Profiles,
            "system-users-symbolic",
            tr("Profiles"),
        ),
        (SettingsTab::Data, "drive-harddisk-symbolic", tr("Data")),
        (
            SettingsTab::Privacy,
            "security-high-symbolic",
            tr("Privacy & Shield"),
        ),
        (
            SettingsTab::Customize,
            "document-edit-symbolic",
            tr("Customize"),
        ),
        (
            SettingsTab::Update,
            "software-update-available-symbolic",
            tr("Updates"),
        ),
        (SettingsTab::About, "help-about-symbolic", tr("About Rail")),
    ];

    let mut tab_buttons_vec: Vec<(SettingsTab, gtk::Button)> = Vec::new();
    for (tab_type, icon_name, label_text) in sidebar_items {
        let btn = gtk::Button::new();
        btn.add_css_class("settings-tab-btn");
        let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let btn_icon = gtk::Image::from_icon_name(icon_name);
        let btn_lbl = gtk::Label::new(Some(label_text));
        btn_lbl.set_xalign(0.0);
        btn_lbl.set_hexpand(true);
        btn_box.append(&btn_icon);
        btn_box.append(&btn_lbl);
        btn.set_child(Some(&btn_box));
        if tab_type == SettingsTab::General {
            btn.add_css_class("active");
        }
        settings_sidebar.append(&btn);
        tab_buttons_vec.push((tab_type, btn));
    }

    let sidebar_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_spacer.set_vexpand(true);
    settings_sidebar.append(&sidebar_spacer);

    let settings_sidebar_scroll = gtk::ScrolledWindow::new();
    settings_sidebar_scroll.add_css_class("settings-sidebar-scroll");
    settings_sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    settings_sidebar_scroll.set_child(Some(&settings_sidebar));
    settings_sidebar_scroll.set_vexpand(true);

    // Settings is a tab — no "Esc to close" hint
    settings_body.append(&settings_sidebar_scroll);
    settings_body.append(&settings_stack);
    settings_modal.append(&settings_body);

    let GeneralPage {
        engine_buttons,
        initial_settings,
    } = build_general_page(&settings_stack);
    let CustomizePage { css_buffer } = build_customize_page(&settings_stack);
    let ProfilesPage {
        add_prof_btn,
        color_btns,
        new_profile_color_idx,
        new_profile_entry,
        new_profile_error_lbl,
        page_prof_scroll,
        profiles_list_box,
    } = build_profiles_page(&settings_stack);
    let DataPage {
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
    } = build_data_page(&settings_stack);
    build_privacy_page(&settings_stack);
    build_updates_page(&settings_stack);
    build_about_page(&settings_stack);

    let HintWidgets { load_bar, ov_hint } = build_hints();

    // A compact, transient zoom readout shared by website zoom and spatial overview.
    let zoom_indicator_label = gtk::Label::new(Some("100%"));
    zoom_indicator_label.add_css_class("zoom-indicator-label");
    let zoom_indicator = gtk::Revealer::new();
    zoom_indicator.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    zoom_indicator.set_transition_duration(140);
    zoom_indicator.add_css_class("zoom-indicator");
    zoom_indicator.set_child(Some(&zoom_indicator_label));
    zoom_indicator.set_halign(gtk::Align::End);
    zoom_indicator.set_valign(gtk::Align::Start);
    zoom_indicator.set_margin_top(12);
    zoom_indicator.set_margin_end(12);
    zoom_indicator.set_can_target(false);
    zoom_indicator.set_reveal_child(false);

    let MinimapWidgets {
        map_close_btn,
        map_container,
        map_drawing_area,
        map_header_row,
        map_location_btn,
    } = build_minimap();

    let DevtoolsWidgets {
        devtools_close_btn,
        devtools_content_box,
        devtools_detach_btn,
        devtools_panel,
        devtools_resize_handle,
    } = build_devtools();

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&shell));
    overlay.add_overlay(&load_bar);

    // On narrow Niri columns the dock becomes an overlay, not a 260px reservation.
    // This launcher keeps the actual page reachable when the sidebar is collapsed.
    let responsive_sidebar_toggle = gtk::Button::from_icon_name("sidebar-show-symbolic");
    responsive_sidebar_toggle.add_css_class("flat-icon");
    responsive_sidebar_toggle.add_css_class("responsive-sidebar-toggle");
    responsive_sidebar_toggle.set_tooltip_text(Some(tr("Show sidebar")));
    responsive_sidebar_toggle.set_halign(gtk::Align::Start);
    responsive_sidebar_toggle.set_valign(gtk::Align::Start);
    responsive_sidebar_toggle.set_margin_start(8);
    responsive_sidebar_toggle.set_margin_top(8);
    responsive_sidebar_toggle.set_size_request(34, 34);
    responsive_sidebar_toggle.set_visible(false);
    overlay.add_overlay(&responsive_sidebar_toggle);

    // Browser-style link preview, anchored at the bottom-left of the content window.
    let hovered_link_label = gtk::Label::new(None);
    hovered_link_label.add_css_class("hovered-link-status");
    hovered_link_label.set_xalign(0.0);
    hovered_link_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    hovered_link_label.set_max_width_chars(100);
    hovered_link_label.set_halign(gtk::Align::Start);
    hovered_link_label.set_valign(gtk::Align::End);
    hovered_link_label.set_margin_start(8);
    hovered_link_label.set_margin_bottom(8);
    hovered_link_label.set_visible(false);
    hovered_link_label.set_can_target(false);
    overlay.add_overlay(&hovered_link_label);
    // Settings overlays the strip only (tab square), not the whole window
    strip_host.add_overlay(&settings_modal);

    overlay.add_overlay(&palette);
    overlay.add_overlay(&ov_hint);
    overlay.add_overlay(&map_container);
    overlay.add_overlay(&devtools_panel);

    let win = gtk::ApplicationWindow::builder()
        .application(gapp)
        .default_width(1400)
        .default_height(850)
        .decorated(false)
        .child(&overlay)
        .build();
    update::ensure_icon();
    gtk::Window::set_default_icon_name("rail");
    overlay.add_overlay(&update::build_card(&win));
    // Added last so zoom feedback remains above page, DevTools and update surfaces.
    overlay.add_overlay(&zoom_indicator);

    let app = Rc::new(App {
        win: win.clone(),
        shell: shell.clone(),
        shell_layout,
        side,
        responsive_sidebar_toggle: responsive_sidebar_toggle.clone(),
        responsive_layout: Cell::new(false),
        strip,
        layout: layout.clone(),
        brand_logo: brand_logo.clone(),
        list: list.clone(),
        pill_label,
        pill_engine_icon,
        pill_pwa_btn: pill_pwa_btn.clone(),
        load_bar,
        hovered_link_label,
        palette,
        palette_engine_icon,
        entry: entry.clone(),
        palette_for_new_tab: Cell::new(false),
        welcome_palette: Cell::new(false),
        palette_suggestions_box: palette_suggestions_box.clone(),
        history: RefCell::new(history_mgr),
        current_suggestions: RefCell::new(Vec::new()),
        selected_suggestion_idx: Cell::new(None),
        is_autofilling: Cell::new(false),
        last_entry_len: Cell::new(0),
        delete_key: Cell::new(false),
        ov_hint,
        zoom_indicator: zoom_indicator.clone(),
        zoom_indicator_label: zoom_indicator_label.clone(),
        zoom_indicator_timer: RefCell::new(None),
        find_ui: RefCell::new(None),
        ov: Cell::new(false),
        ov_exiting: Cell::new(false),
        ov_user_zoom: Cell::new(1.0),
        ov_return: Cell::new((0, 0)),
        ov_saved_x: Cell::new(0.0),
        ov_saved_y: Cell::new(0.0),
        target_zoom: Cell::new(1.0),
        rows: RefCell::new(Vec::new()),
        focus_row: Cell::new(0),
        focus_col: Cell::new(0),
        row_last_col: RefCell::new(std::collections::HashMap::new()),
        target_x: Cell::new(0.0),
        target_y: Cell::new(0.0),
        animating: Cell::new(false),
        cam_spring_x: Cell::new(None),
        cam_spring_y: Cell::new(None),
        anim_frozen: RefCell::new(Vec::new()),
        side_open: Cell::new(true),
        sidebar_auto: Cell::new(load_app_settings().sidebar_mode == "auto"),
        sidebar_locked: Cell::new(false),
        side_gen: Cell::new(0),
        side_open_timer: RefCell::new(None),
        side_close_timer: RefCell::new(None),
        fresh: RefCell::new(None),
        profiles: RefCell::new(profile_mgr),
        selected_palette_profile: RefCell::new(initial_prof_id.clone()),
        profile_btn: profile_btn.clone(),
        profile_btn_lbl: profile_btn_lbl.clone(),
        profile_popover: profile_popover.clone(),
        profiles_modal_scroll: page_prof_scroll.clone(),
        profiles_list_box: profiles_list_box.clone(),
        new_profile_entry: new_profile_entry.clone(),
        new_profile_error_lbl: new_profile_error_lbl.clone(),
        add_prof_btn: add_prof_btn.clone(),
        new_profile_color_idx,
        color_buttons: RefCell::new(color_btns),
        last_saved_session: RefCell::new(String::new()),
        save_session_gen: Cell::new(0),
        data_modal_info_lbl: data_info_lbl.clone(),
        data_modal_status_lbl: data_modal_status_lbl.clone(),
        data_export_entry: data_export_entry.clone(),
        data_import_entry: data_import_entry.clone(),
        sync_url_result_entry: sync_url_result_entry.clone(),
        sync_restore_entry: sync_restore_entry.clone(),
        map_container: map_container.clone(),
        map_drawing_area: map_drawing_area.clone(),
        map_location_btn: map_location_btn.clone(),
        map_state: RefCell::new(MinimapState::default()),
        devtools_open: Cell::new(false),
        default_devtools_height: Cell::new(340),
        default_devtools_width: Cell::new(420),
        devtools_dock_side: Cell::new(devtools_dock_from_str(&initial_settings.devtools_dock)),
        devtools_panel: devtools_panel.clone(),
        devtools_window: RefCell::new(None),
        qr_modal: RefCell::new(None),
        settings_modal: settings_modal.clone(),
        settings_body: settings_body.clone(),
        settings_sidebar: settings_sidebar.clone(),
        settings_sidebar_scroll: settings_sidebar_scroll.clone(),
        settings_stack: settings_stack.clone(),
        settings_tab_buttons: RefCell::new(tab_buttons_vec.clone()),
        current_settings_tab: Cell::new(SettingsTab::General),
        css_buffer: css_buffer.clone(),
        search_engine: RefCell::new(initial_settings.search_engine.clone()),
        recently_closed: RefCell::new(Vec::new()),
    });

    install_download_notifications(&app);
    app.select_palette_profile(&initial_prof_id);

    wire_minimap(
        &app,
        map_close_btn,
        map_container,
        map_drawing_area,
        map_header_row,
        map_location_btn,
        overlay,
    );

    wire_settings(
        &app,
        SettingsWidgets {
            add_prof_btn,
            btn_clear_cache,
            btn_clear_hist,
            btn_clear_hist_tab,
            btn_copy_sync_url,
            btn_export_local,
            btn_import_local,
            btn_sync_now,
            btn_sync_restore,
            engine_buttons,
            hist_list_box,
            hist_search_entry,
            layout: layout.clone(),
            new_profile_entry,
            settings_close_btn,
            settings_update_chip,
            sub_history_btn,
            sync_restore_entry,
            sync_url_result_entry,
            tab_buttons_vec,
            win: win.clone(),
        },
    );

    let a = app.clone();
    layout.connect_resized(move || {
        a.sync_responsive_sidebar();
        a.reveal_focus();
        if a.palette_open() {
            a.update_palette_size();
        }
        if a.settings_modal_open() {
            a.sync_settings_responsive();
        }
    });

    wire_devtools(
        &app,
        devtools_close_btn,
        devtools_content_box,
        devtools_detach_btn,
        devtools_resize_handle,
        footer_downloads,
        footer_settings,
    );
    wire_sidebar_header(&app, head_back, head_forward, head_reload, head_close_side);

    let a_responsive_toggle = app.clone();
    responsive_sidebar_toggle.connect_clicked(move |_| {
        a_responsive_toggle.set_sidebar(true);
    });
    let a_responsive_map = app.clone();
    win.connect_map(move |_| a_responsive_map.sync_responsive_sidebar());

    let a_pwa = app.clone();
    app.pill_pwa_btn.connect_clicked(move |_| {
        a_pwa.prompt_install_current_pwa();
    });

    // Auto mode: start closed (edge-hover only)
    app.shell_layout.set_overlay(app.sidebar_auto.get());
    if app.sidebar_auto.get() {
        app.side_open.set(false);
        app.side.set_visible(false);
        app.shell_layout.set_side(0.0);
    }
    app.sync_brand_logo();

    wire_palette(&app, entry, pill);

    wire_gestures(&app, shell, win.clone());

    wire_keys(&app, win.clone());

    wire_close(&app, win.clone());

    let restored = app.restore_session();
    app.sync_responsive_sidebar();
    if let Some(url) = TARGET_URL.get() {
        let uri = to_uri(url, &app.search_engine.borrow());
        let can_reuse = {
            let rows = app.rows.borrow();
            rows.len() == 1 && rows[0].tabs.len() == 1 && rows[0].tabs[0].real_uri().is_empty()
        };
        if can_reuse {
            app.navigate(&uri);
        } else {
            app.new_tab_with_url(&uri);
        }
    } else if !restored {
        app.new_tab();
    }
    app.update_window_title();
    win.present();
    app.sync_welcome_palette();
    MAIN_APP.with(|m| *m.borrow_mut() = Some(app.clone()));
}
