//! Progressive Web App (PWA) installation and standalone desktop app mode.
//!
//! Features:
//! - Full PWA metadata detection (Web App Manifest, apple-touch-icon, theme-color)
//! - Generates standard Linux desktop entry (.desktop in ~/.local/share/applications)
//! - Saves app icon to ~/.local/share/rail/apps/<id>/icon.png and system hicolor theme
//! - Registers with Linux desktop database (Rofi, Wofi, GNOME, KDE, Niri find it immediately)
//! - Standalone PWA window mode (`rail --app="<url>"`) with minimal native chrome,
//!   instant hardware acceleration, shared cookies/session, smooth video playback and shortcuts.

use crate::profile::ProfileManager;
use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub(crate) struct PwaAppInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon_url: Option<String>,
    pub theme_color: Option<String>,
}

thread_local! {
    /// Cached PWA info for each tab's active page
    pub(crate) static TAB_PWA_MAP: RefCell<HashMap<usize, PwaAppInfo>> = RefCell::new(HashMap::new());
}

/// Directory for storing installed PWA metadata and icons: ~/.local/share/rail/apps
pub(crate) fn pwa_apps_dir() -> PathBuf {
    rail_data_dir().join("apps")
}

/// System user applications directory: ~/.local/share/applications
pub(crate) fn desktop_applications_dir() -> PathBuf {
    glib::user_data_dir().join("applications")
}

/// Generate a filesystem-friendly and desktop-entry-friendly ID from a name or URL
pub(crate) fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_dash = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "pwa-app".to_string()
    } else {
        trimmed
    }
}

/// Generate a sharp SVG fallback icon if no remote icon is available or download fails
fn generate_svg_icon(name: &str, theme_color: Option<&str>) -> String {
    let initial = name.chars().next().unwrap_or('A').to_uppercase();
    let bg = theme_color
        .filter(|c| c.starts_with('#') && (c.len() == 7 || c.len() == 4))
        .unwrap_or("#2b5c8f");
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 256 256\" width=\"256\" height=\"256\"><rect width=\"256\" height=\"256\" rx=\"54\" fill=\"{}\"/><text x=\"128\" y=\"165\" font-family=\"sans-serif\" font-size=\"118\" font-weight=\"bold\" fill=\"#ffffff\" text-anchor=\"middle\">{}</text></svg>",
        bg, initial
    )
}

/// Install PWA: saves metadata, fetches/creates icon, writes ~/.local/share/applications/rail-pwa-{id}.desktop
pub(crate) fn install_pwa_app(
    name: &str,
    url: &str,
    icon_url: Option<&str>,
    theme_color: Option<&str>,
) -> Result<PathBuf, String> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err("App name cannot be empty".into());
    }
    let clean_url = url.trim();
    if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
        return Err("App URL must start with http:// or https://".into());
    }

    let id = slugify(clean_name);
    let app_dir = pwa_apps_dir().join(&id);
    let _ = std::fs::create_dir_all(&app_dir);

    // Save app icon
    let icon_path = app_dir.join("icon.png");
    let mut icon_downloaded = false;

    if let Some(i_url) = icon_url {
        if i_url.starts_with("http://") || i_url.starts_with("https://") {
            let tmp_icon = std::env::temp_dir().join(format!("rail_pwa_icon_{}", id));
            let curl_res = std::process::Command::new("curl")
                .args([
                    "-fsSL",
                    "-m", "10",
                    "-A", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
                    "-o", tmp_icon.to_str().unwrap_or(""),
                    i_url,
                ])
                .status();

            if curl_res.map(|s| s.success()).unwrap_or(false) && tmp_icon.is_file() {
                // If the downloaded icon is already an image (PNG/ICO/JPG/SVG), convert or copy
                if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&tmp_icon) {
                    let _ = pixbuf.savev(&icon_path, "png", &[]);
                    icon_downloaded = icon_path.is_file();
                } else {
                    let _ = std::fs::copy(&tmp_icon, &icon_path);
                    icon_downloaded = icon_path.is_file();
                }
                let _ = std::fs::remove_file(&tmp_icon);
            }
        }
    }

    if !icon_downloaded {
        // Fallback: write crisp SVG icon
        let svg_path = app_dir.join("icon.svg");
        let svg_data = generate_svg_icon(clean_name, theme_color);
        let _ = std::fs::write(&svg_path, svg_data);
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&svg_path) {
            let _ = pixbuf.savev(&icon_path, "png", &[]);
        }
    }

    let final_icon = if icon_path.exists() {
        icon_path.clone()
    } else {
        app_dir.join("icon.svg")
    };

    // System icon installation for desktop environments (hicolor/256x256)
    let sys_icons_dir = glib::user_data_dir()
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps");
    let _ = std::fs::create_dir_all(&sys_icons_dir);
    let sys_icon_path = sys_icons_dir.join(format!("rail-pwa-{}.png", id));
    if final_icon.exists() {
        let _ = std::fs::copy(&final_icon, &sys_icon_path);
    }

    // Write app metadata manifest.json
    let meta_json = format!(
        "{{\n  \"id\": \"{}\",\n  \"name\": \"{}\",\n  \"url\": \"{}\",\n  \"icon\": \"{}\",\n  \"theme_color\": \"{}\"\n}}\n",
        id,
        clean_name.replace('"', "\\\""),
        clean_url.replace('"', "\\\""),
        final_icon.to_string_lossy().replace('"', "\\\""),
        theme_color.unwrap_or("").replace('"', "\\\"")
    );
    let _ = std::fs::write(app_dir.join("app.json"), meta_json);

    // Determine binary path for Exec
    let bin_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "rail".to_string());

    // Write Linux .desktop file in ~/.local/share/applications/
    let desktop_dir = desktop_applications_dir();
    let _ = std::fs::create_dir_all(&desktop_dir);
    let desktop_file = desktop_dir.join(format!("rail-pwa-{}.desktop", id));

    let desktop_content = format!(
        "[Desktop Entry]\n\
Version=1.0\n\
Type=Application\n\
Name={name}\n\
GenericName={name} (Web App)\n\
Comment={name} - Progressive Web App via Rail Browser\n\
Exec={bin} --app=\"{url}\"\n\
Icon={icon}\n\
Terminal=false\n\
StartupWMClass=rail-pwa-{id}\n\
Categories=Network;Application;WebBrowser;\n\
Keywords=pwa;web;app;{id};\n",
        name = clean_name,
        bin = bin_path,
        url = clean_url,
        icon = final_icon.display(),
        id = id,
    );

    std::fs::write(&desktop_file, desktop_content)
        .map_err(|e| format!("Failed to write desktop launcher: {e}"))?;

    // Refresh Linux desktop database so Wofi, Rofi, GNOME, KDE, Niri see it immediately
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&desktop_dir)
        .spawn();

    Ok(final_icon)
}

/// List all installed PWAs
pub(crate) fn list_installed_pwas() -> Vec<PwaAppInfo> {
    let dir = pwa_apps_dir();
    let mut list = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return list;
    };
    for entry in entries.flatten() {
        let meta_file = entry.path().join("app.json");
        if let Ok(content) = std::fs::read_to_string(meta_file) {
            if let Some(json) = crate::minijson::parse(&content) {
                let id = json.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let url = json.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let icon_url = json.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
                let theme_color = json.get("theme_color").and_then(|v| v.as_str()).map(|s| s.to_string());
                if !name.is_empty() && !url.is_empty() {
                    list.push(PwaAppInfo { id, name, url, icon_url, theme_color });
                }
            }
        }
    }
    list
}

/// Uninstall PWA
pub(crate) fn uninstall_pwa_app(id: &str) -> Result<(), String> {
    let desktop_file = desktop_applications_dir().join(format!("rail-pwa-{}.desktop", id));
    if desktop_file.exists() {
        let _ = std::fs::remove_file(&desktop_file);
    }
    let app_dir = pwa_apps_dir().join(id);
    if app_dir.exists() {
        let _ = std::fs::remove_dir_all(&app_dir);
    }
    let _ = std::process::Command::new("update-desktop-database")
        .arg(desktop_applications_dir())
        .spawn();
    Ok(())
}

/// Open native PWA install confirmation dialog
pub(crate) fn prompt_install_pwa_dialog(app: &Rc<App>, info: &PwaAppInfo) {
    let win = gtk::Window::builder()
        .transient_for(&app.win)
        .modal(true)
        .title(tr("Install App"))
        .default_width(440)
        .default_height(260)
        .destroy_with_parent(true)
        .build();
    win.add_css_class("js-dialog-window");

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 14);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);

    // Title with icon
    let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    let img_icon = gtk::Image::from_icon_name("system-software-install-symbolic");
    img_icon.set_pixel_size(36);
    img_icon.add_css_class("pwa-dialog-icon");

    let title_vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let title_lbl = gtk::Label::new(Some(tr("Install as Desktop App?")));
    title_lbl.add_css_class("js-dialog-title");
    title_lbl.set_xalign(0.0);

    let desc_lbl = gtk::Label::new(Some(tr("The app will open in a fast standalone window and be added to your system application launcher.")));
    desc_lbl.add_css_class("js-dialog-message");
    desc_lbl.set_xalign(0.0);
    desc_lbl.set_wrap(true);

    title_vbox.append(&title_lbl);
    title_vbox.append(&desc_lbl);
    header_box.append(&img_icon);
    header_box.append(&title_vbox);
    vbox.append(&header_box);

    // App name entry
    let name_entry = gtk::Entry::new();
    name_entry.add_css_class("js-dialog-entry");
    name_entry.set_text(&info.name);
    name_entry.set_placeholder_text(Some(tr("App name")));
    vbox.append(&name_entry);

    // App URL display
    let url_lbl = gtk::Label::new(Some(&info.url));
    url_lbl.add_css_class("settings-badge");
    url_lbl.set_xalign(0.0);
    url_lbl.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    vbox.append(&url_lbl);

    // Action buttons
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    btn_box.set_halign(gtk::Align::End);
    btn_box.set_margin_top(8);

    let cancel_btn = gtk::Button::with_label(tr("Cancel"));
    cancel_btn.add_css_class("js-dialog-btn-cancel");
    let win_cancel = win.clone();
    cancel_btn.connect_clicked(move |_| win_cancel.close());

    let install_btn = gtk::Button::with_label(tr("Install"));
    install_btn.add_css_class("js-dialog-btn-ok");

    let app_inst = app.clone();
    let info_inst = info.clone();
    let name_ent = name_entry.clone();
    let win_inst = win.clone();

    install_btn.connect_clicked(move |_| {
        let name = name_ent.text().to_string();
        let name = if name.trim().is_empty() { info_inst.name.clone() } else { name };
        win_inst.close();

        let app_cb = app_inst.clone();
        let url_cb = info_inst.url.clone();
        let icon_cb = info_inst.icon_url.clone();
        let theme_cb = info_inst.theme_color.clone();

        match install_pwa_app(&name, &url_cb, icon_cb.as_deref(), theme_cb.as_deref()) {
            Ok(_) => {
                app_cb.show_copied_toast(
                    &name,
                    tr("App installed successfully! Opening standalone window..."),
                );
                if let Some(gapp) = app_cb.win.application() {
                    open_pwa_window(&gapp, &url_cb, Some(&name));
                }
            }
            Err(e) => {
                app_cb.show_copied_toast(tr("Install error"), &e);
            }
        }
    });

    btn_box.append(&cancel_btn);
    btn_box.append(&install_btn);
    vbox.append(&btn_box);

    win.set_child(Some(&vbox));
    name_entry.grab_focus();
    win.present();
}

/// Standalone PWA Window: opens when `rail --app="https://..."` is executed or app is launched
pub(crate) fn open_pwa_window(
    app: &gtk::Application,
    url: &str,
    title_override: Option<&str>,
) {
    let win = gtk::ApplicationWindow::builder()
        .application(app)
        .title(title_override.unwrap_or(url))
        .default_width(1120)
        .default_height(760)
        .build();
    win.add_css_class("pwa-window");

    let root_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // 1. Sleek HeaderBar
    let header = gtk::HeaderBar::new();
    header.add_css_class("pwa-headerbar");

    let nav_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);

    let btn_back = gtk::Button::from_icon_name("go-previous-symbolic");
    btn_back.add_css_class("flat-icon");
    btn_back.set_tooltip_text(Some(tr("Back")));
    btn_back.set_sensitive(false);

    let btn_forward = gtk::Button::from_icon_name("go-next-symbolic");
    btn_forward.add_css_class("flat-icon");
    btn_forward.set_tooltip_text(Some(tr("Forward")));
    btn_forward.set_sensitive(false);

    let btn_reload = gtk::Button::from_icon_name("view-refresh-symbolic");
    btn_reload.add_css_class("flat-icon");
    btn_reload.set_tooltip_text(Some(tr("Reload")));

    nav_box.append(&btn_back);
    nav_box.append(&btn_forward);
    nav_box.append(&btn_reload);
    header.pack_start(&nav_box);

    // Center domain title pill
    let center_pill = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    center_pill.add_css_class("pwa-title-pill");
    let lock_icon = gtk::Image::from_icon_name("channel-secure-symbolic");
    lock_icon.set_pixel_size(12);
    let title_lbl = gtk::Label::new(Some(title_override.unwrap_or(url)));
    title_lbl.add_css_class("pwa-title-text");
    center_pill.append(&lock_icon);
    center_pill.append(&title_lbl);
    header.set_title_widget(Some(&center_pill));

    // Right-side app menu
    let menu_btn = gtk::MenuButton::new();
    menu_btn.set_icon_name("open-menu-symbolic");
    menu_btn.add_css_class("flat-icon");

    let popover = gtk::Popover::new();
    let pop_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    pop_box.set_margin_start(10);
    pop_box.set_margin_end(10);
    pop_box.set_margin_top(8);
    pop_box.set_margin_bottom(8);

    let pop_c1 = popover.clone();
    let url_copy = url.to_string();
    let btn_copy_url = gtk::Button::with_label(tr("Copy Page Address"));
    btn_copy_url.add_css_class("ext-popover-footer-btn");
    btn_copy_url.connect_clicked(move |_| {
        pop_c1.popdown();
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(&url_copy);
        }
    });
    pop_box.append(&btn_copy_url);

    let pop_c2 = popover.clone();
    let win_close = win.clone();
    let btn_close = gtk::Button::with_label(tr("Close"));
    btn_close.add_css_class("ext-popover-footer-btn");
    btn_close.connect_clicked(move |_| {
        pop_c2.popdown();
        win_close.close();
    });
    pop_box.append(&btn_close);

    popover.set_child(Some(&pop_box));
    menu_btn.set_popover(Some(&popover));
    header.pack_end(&menu_btn);

    root_box.append(&header);

    // 2. Web View for PWA
    let mut mgr = ProfileManager::new(rail_data_dir());
    let prof_id = mgr.active_id().to_string();
    let session = mgr.get_or_create_session(&prof_id);

    let settings = webkit6::Settings::new();
    settings.set_enable_javascript(true);
    settings.set_enable_webgl(true);
    settings.set_property("enable-2d-canvas-acceleration", true);
    settings.set_enable_smooth_scrolling(false);
    settings.set_enable_mediasource(true);
    settings.set_enable_media(true);
    settings.set_enable_media_capabilities(true);
    settings.set_media_playback_allows_inline(true);
    settings.set_media_playback_requires_user_gesture(false);
    settings.set_enable_webaudio(true);
    settings.set_enable_media_stream(true);
    settings.set_enable_html5_local_storage(true);
    settings.set_enable_html5_database(true);
    settings.set_enable_site_specific_quirks(true);
    settings.set_javascript_can_access_clipboard(true);
    settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
    settings.set_auto_load_images(true);
    settings.set_load_icons_ignoring_image_load_setting(true);
    settings.set_enable_page_cache(true);

    let ucm = webkit6::UserContentManager::new();
    crate::web::inject_web_optimizations(&ucm);

    let website_policies = webkit6::WebsitePolicies::builder()
        .autoplay(webkit6::AutoplayPolicy::Allow)
        .build();

    let view = WebView::builder()
        .network_session(&session)
        .settings(&settings)
        .user_content_manager(&ucm)
        .website_policies(&website_policies)
        .hexpand(true)
        .vexpand(true)
        .build();

    let v_back = view.clone();
    btn_back.connect_clicked(move |_| v_back.go_back());

    let v_fwd = view.clone();
    btn_forward.connect_clicked(move |_| v_fwd.go_forward());

    let v_reload = view.clone();
    btn_reload.connect_clicked(move |_| v_reload.reload());

    // Navigation updates
    let b_back = btn_back.clone();
    let b_fwd = btn_forward.clone();
    let t_lbl = title_lbl.clone();
    let w_clone = win.clone();
    view.connect_load_changed(move |v, event| {
        if event == webkit6::LoadEvent::Finished {
            b_back.set_sensitive(v.can_go_back());
            b_fwd.set_sensitive(v.can_go_forward());
            if let Some(t) = v.title() {
                w_clone.set_title(Some(&t));
                t_lbl.set_text(&t);
            }
        }
    });

    // Keyboard shortcuts in PWA window
    let key_ctl = gtk::EventControllerKey::new();
    let v_key = view.clone();
    let w_key = win.clone();
    key_ctl.connect_key_pressed(move |_, key, _, state| {
        let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
        let alt = state.contains(gdk::ModifierType::ALT_MASK);

        match key {
            gdk::Key::F5 => {
                v_key.reload();
                glib::Propagation::Stop
            }
            gdk::Key::r if ctrl => {
                v_key.reload();
                glib::Propagation::Stop
            }
            gdk::Key::w if ctrl => {
                w_key.close();
                glib::Propagation::Stop
            }
            gdk::Key::Left if alt => {
                if v_key.can_go_back() {
                    v_key.go_back();
                }
                glib::Propagation::Stop
            }
            gdk::Key::Right if alt => {
                if v_key.can_go_forward() {
                    v_key.go_forward();
                }
                glib::Propagation::Stop
            }
            gdk::Key::equal | gdk::Key::plus if ctrl => {
                let cur = v_key.zoom_level();
                v_key.set_zoom_level((cur + 0.1).min(3.0));
                glib::Propagation::Stop
            }
            gdk::Key::minus if ctrl => {
                let cur = v_key.zoom_level();
                v_key.set_zoom_level((cur - 0.1).max(0.3));
                glib::Propagation::Stop
            }
            gdk::Key::_0 if ctrl => {
                v_key.set_zoom_level(1.0);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    win.add_controller(key_ctl);

    view.load_uri(url);
    root_box.append(&view);
    win.set_child(Some(&root_box));
    win.present();
}
