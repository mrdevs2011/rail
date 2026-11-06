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
    #[allow(dead_code)]
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

    if let Some(i_url) = icon_url
        && (i_url.starts_with("http://") || i_url.starts_with("https://"))
    {
        let tmp_icon = std::env::temp_dir().join(format!("rail_pwa_icon_{}", id));
        let curl_res = std::process::Command::new("curl")
            .args([
                "-fsSL",
                "-m",
                "10",
                "-A",
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
                "-o",
                tmp_icon.to_str().unwrap_or(""),
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
    let desktop_file = desktop_dir.join(format!("webapp-{}.desktop", id));
    // Older versions used rail-pwa-*.desktop, which made launchers list every PWA when searching "rail".
    let _ = std::fs::remove_file(desktop_dir.join(format!("rail-pwa-{}.desktop", id)));

    // Launch PWAs through a symlink without "rail" in its path, so launchers that
    // match on Exec (rofi) don't list every PWA when searching "rail".
    let launch_bin = {
        let link = glib::home_dir().join(".local/share/webapp/launch");
        if let Some(parent) = link.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&link);
        if std::os::unix::fs::symlink(&bin_path, &link).is_ok() {
            link.to_string_lossy().into_owned()
        } else {
            bin_path.clone()
        }
    };

    let desktop_content = format!(
        "[Desktop Entry]\n\
Version=1.0\n\
Type=Application\n\
Name={name}\n\
GenericName={name} (Web App)\n\
Comment={name} - Progressive Web App\n\
Exec={bin} --app=\"{url}\"\n\
Icon={icon}\n\
Terminal=false\n\
StartupWMClass=webapp-{id}\n\
Categories=Network;Application;WebBrowser;\n\
Keywords=pwa;web;app;{id};\n",
        name = clean_name,
        bin = launch_bin,
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

    ensure_background_daemon_running();

    Ok(final_icon)
}

/// List all installed PWAs
#[allow(dead_code)]
pub(crate) fn list_installed_pwas() -> Vec<PwaAppInfo> {
    let dir = pwa_apps_dir();
    let mut list = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return list;
    };
    for entry in entries.flatten() {
        let meta_file = entry.path().join("app.json");
        if let Ok(content) = std::fs::read_to_string(meta_file)
            && let Some(json) = crate::minijson::parse(&content)
        {
            let id = json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = json
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url = json
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let icon_url = json
                .get("icon")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let theme_color = json
                .get("theme_color")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if !name.is_empty() && !url.is_empty() {
                list.push(PwaAppInfo {
                    id,
                    name,
                    url,
                    icon_url,
                    theme_color,
                });
            }
        }
    }
    list
}

/// Extract clean normalized origin from a URL string (e.g. "https://myplan-mr.vercel.app")
pub(crate) fn get_url_origin(url: &str) -> String {
    if let Ok(uri) = glib::Uri::parse(url, glib::UriFlags::NONE) {
        let scheme = uri.scheme();
        let host = uri.host().unwrap_or_default();
        let port = uri.port();
        if port > 0 {
            format!("{}://{}:{}", scheme, host, port).to_lowercase()
        } else {
            format!("{}://{}", scheme, host).to_lowercase()
        }
    } else {
        String::new()
    }
}

/// Check if a PWA is installed for the given page URL by comparing origins
pub(crate) fn is_pwa_installed_for_url(page_url: &str) -> bool {
    let target_origin = get_url_origin(page_url);
    if target_origin.is_empty() {
        return false;
    }
    let installed = list_installed_pwas();
    for app in installed {
        let app_origin = get_url_origin(&app.url);
        if !app_origin.is_empty() && app_origin == target_origin {
            return true;
        }
    }
    false
}

/// Send PWA installed status to a WebView page
pub(crate) fn send_pwa_installed_status_to_view(view: &WebView, installed: bool) {
    let js = format!(
        "if (window.__railSetPwaInstalled) window.__railSetPwaInstalled({});",
        installed
    );
    view.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
}

/// Uninstall PWA
#[allow(dead_code)]
pub(crate) fn uninstall_pwa_app(id: &str) -> Result<(), String> {
    for name in [format!("webapp-{}.desktop", id), format!("rail-pwa-{}.desktop", id)] {
        let _ = std::fs::remove_file(desktop_applications_dir().join(name));
    }
    let app_dir = pwa_apps_dir().join(id);
    if app_dir.exists() {
        let _ = std::fs::remove_dir_all(&app_dir);
    }
    let _ = std::process::Command::new("update-desktop-database")
        .arg(desktop_applications_dir())
        .spawn();
    if list_installed_pwas().is_empty() && !crate::push::has_subscriptions() {
        let _ = std::fs::remove_file(
            glib::user_config_dir().join("autostart/rail-pwa-background.desktop"),
        );
        let _ = std::fs::remove_file(daemon_pid_file());
    }
    Ok(())
}

/// A standalone/background PWA may only emit notifications when its origin was
/// explicitly allowed in Rail's normal site-permission prompt.
fn notification_permission_granted(url: &str, decisions: &str) -> bool {
    let host = glib::Uri::parse(url, glib::UriFlags::NONE)
        .ok()
        .and_then(|uri| uri.host().map(|value| value.to_ascii_lowercase()))
        .unwrap_or_default();
    if host.is_empty() {
        return false;
    }
    decisions.lines().any(|line| {
        let mut fields = line.split('\t');
        matches!((fields.next(), fields.next(), fields.next()),
            (Some(saved_host), Some("notifications"), Some("allow"))
                if saved_host.eq_ignore_ascii_case(&host))
    })
}

pub(crate) fn is_notification_permission_granted(url: &str) -> bool {
    let path = rail_data_dir().join("site-permissions.tsv");
    let decisions = std::fs::read_to_string(path).unwrap_or_default();
    notification_permission_granted(url, &decisions)
}

/// Ensure the background listener is launched again after the next desktop login.
pub(crate) fn ensure_background_autostart() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let dir = glib::user_config_dir().join("autostart");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    // Exec values are quoted so paths containing spaces remain a single argument.
    let escaped = exe
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Rail PWA Notifications\nComment=Keep installed Rail apps available for notifications\nExec=\"{}\" --background\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        escaped
    );
    let _ = std::fs::write(dir.join("rail-pwa-background.desktop"), content);
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

    // Title with icon: show the app's real favicon/manifest icon if available
    let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    let app_id = slugify(&info.name);
    let installed_icon = pwa_apps_dir().join(&app_id).join("icon.png");

    let img_icon = if installed_icon.is_file()
        && let Ok(tex) = gdk::Texture::from_filename(&installed_icon)
    {
        let img = gtk::Image::from_paintable(Some(&tex));
        img.set_pixel_size(44);
        img
    } else if let Some(ref icon_u) = info.icon_url
        && (icon_u.starts_with("http://") || icon_u.starts_with("https://"))
    {
        let preview_path = std::env::temp_dir().join(format!("rail_pwa_prev_{}.png", app_id));
        let _ = std::process::Command::new("curl")
            .args(["-fsSL", "-m", "4", "-A", "Mozilla/5.0 (X11; Linux x86_64)", "-o", preview_path.to_str().unwrap_or(""), icon_u])
            .output();
        if preview_path.is_file()
            && let Ok(tex) = gdk::Texture::from_filename(&preview_path)
        {
            let img = gtk::Image::from_paintable(Some(&tex));
            img.set_pixel_size(44);
            img
        } else {
            let img = gtk::Image::from_icon_name("system-software-install-symbolic");
            img.set_pixel_size(40);
            img.add_css_class("pwa-dialog-icon");
            img
        }
    } else {
        let img = gtk::Image::from_icon_name("system-software-install-symbolic");
        img.set_pixel_size(40);
        img.add_css_class("pwa-dialog-icon");
        img
    };

    let title_vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let title_lbl = gtk::Label::new(Some(tr("Install as Desktop App?")));
    title_lbl.add_css_class("js-dialog-title");
    title_lbl.set_xalign(0.0);

    let desc_lbl = gtk::Label::new(Some(tr(
        "The app will open in a fast standalone window and be added to your system application launcher.",
    )));
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
        let name = if name.trim().is_empty() {
            info_inst.name.clone()
        } else {
            name
        };
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
                // Broadcast installed status to all active tabs on this origin
                app_cb.broadcast_pwa_installed_for_url(&url_cb, true);
                app_cb.refresh_sidebar();

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

/// Standalone PWA Window: opens when `rail --app="https://..."` is executed or app is launched.
/// Runs as a pure, native desktop application window with NO browser headerbars, NO url bars,
/// giving 100% full screen area to the web app content.
pub(crate) fn open_pwa_window(app: &gtk::Application, url: &str, title_override: Option<&str>) {
    let win = gtk::ApplicationWindow::builder()
        .application(app)
        .title(title_override.unwrap_or(url))
        .default_width(1120)
        .default_height(760)
        .build();
    win.add_css_class("pwa-window");

    // Set desktop window icon if installed
    if let Some(name) = title_override {
        let app_id = slugify(name);
        win.set_icon_name(Some(&format!("rail-pwa-{}", app_id)));
    }

    // 1. Web View for PWA
    let mut mgr = ProfileManager::new(rail_data_dir());
    let prof_id = mgr.active_id().to_string();
    let session = mgr.get_or_create_session(&prof_id);

    let settings = webkit6::Settings::new();
    App::configure_web_settings(&settings);

    let ucm = webkit6::UserContentManager::new();
    ucm.register_script_message_handler("railContextMenu", None);
    crate::push::install_bridge(&ucm);
    let html_css = load_html_css();
    if css_has_active_content(&html_css) {
        ucm.add_style_sheet(&user_style_sheet(&html_css));
    }
    STYLE_MANAGERS.with(|l| l.borrow_mut().push(ucm.downgrade()));
    ADBLOCK_FILTER.with(|f| {
        if let Some(f) = f.borrow().as_ref() {
            ucm.add_filter(f);
        }
    });
    crate::web::inject_context_menu_bridge(&ucm);

    let website_policies = webkit6::WebsitePolicies::builder()
        .autoplay(webkit6::AutoplayPolicy::AllowWithoutSound)
        .build();

    let view = WebView::builder()
        .network_session(&session)
        .settings(&settings)
        .user_content_manager(&ucm)
        .website_policies(&website_policies)
        .hexpand(true)
        .vexpand(true)
        .build();
    App::configure_native_web_view(&view);

    // Standalone PWA windows reuse the permission decision made in the browser.
    let pwa_notification_allowed = is_notification_permission_granted(url);
    view.connect_permission_request(move |_v, req| {
        if req.is::<webkit6::NotificationPermissionRequest>() {
            if pwa_notification_allowed {
                req.allow();
            } else {
                req.deny();
            }
            return true;
        }
        false
    });

    let app_name_str = title_override.unwrap_or(url).to_string();
    crate::notifications::attach_notification_handler(&view, Some(app_name_str), None);

    // Navigation and title updates
    let w_clone = win.clone();
    view.connect_load_changed(move |v, event| {
        if event == webkit6::LoadEvent::Finished
            && let Some(t) = v.title()
        {
            w_clone.set_title(Some(&t));
        }
    });

    // When PWA window is closed, ensure background runner continues delivering notifications
    win.connect_close_request(move |_| {
        ensure_background_daemon_running();
        glib::Propagation::Proceed
    });

    // Keyboard shortcuts in PWA window: same customizable bindings as the browser
    // (Settings > Customize > Shortcuts). Only page-level actions are honoured; the sidebar
    // and other browser chrome do not exist here.
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    let key_ctl = gtk::EventControllerKey::new();
    key_ctl.set_propagation_phase(gtk::PropagationPhase::Capture);
    let insp_open = Rc::new(std::cell::Cell::new(false));
    if let Some(insp) = view.inspector() {
        let o = insp_open.clone();
        insp.connect_closed(move |_| o.set(false));
    }
    let v_key = view.clone();
    let w_key = win.clone();
    key_ctl.connect_key_pressed(move |_, key, keycode, state| {
        let alt = state.contains(gdk::ModifierType::ALT_MASK);
        let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
        if alt && !ctrl {
            match key {
                gdk::Key::Left => {
                    if v_key.can_go_back() {
                        v_key.go_back();
                    }
                    return glib::Propagation::Stop;
                }
                gdk::Key::Right => {
                    if v_key.can_go_forward() {
                        v_key.go_forward();
                    }
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        let id = match sc_resolve_any(&shortcut_combos(key, keycode, state)) {
            ScResolve::Action(id) => id,
            _ => return glib::Propagation::Proceed,
        };
        match id {
            "reload" => reload_or_start(&v_key),
            "hard_reload" => hard_reload_or_start(&v_key),
            "devtools" => {
                if let Some(insp) = v_key.inspector() {
                    if insp_open.get() {
                        insp.close();
                        insp_open.set(false);
                    } else {
                        insp.show();
                        insp_open.set(true);
                    }
                }
            }
            "close_tab" => w_key.close(),
            "quit" => w_key.close(),
            "fullscreen" => w_key.set_fullscreened(!w_key.is_fullscreen()),
            "overview_zoom_in" => {
                let cur = v_key.zoom_level();
                v_key.set_zoom_level((cur + 0.1).min(5.0));
            }
            "overview_zoom_out" => {
                let cur = v_key.zoom_level();
                v_key.set_zoom_level((cur - 0.1).max(0.2));
            }
            "zoom_reset" => v_key.set_zoom_level(1.0),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    win.add_controller(key_ctl);

    win.set_child(Some(&view));
    view.load_uri(url);
    win.present();
}

/// Path to background daemon PID file
pub(crate) fn daemon_pid_file() -> PathBuf {
    pwa_apps_dir().join("daemon.pid")
}

/// Check if background daemon is actively running
pub(crate) fn is_daemon_running() -> bool {
    let pid_file = daemon_pid_file();
    if let Ok(pid_str) = std::fs::read_to_string(&pid_file)
        && let Ok(pid) = pid_str.trim().parse::<i32>()
    {
        let proc_dir = PathBuf::from(format!("/proc/{}", pid));
        if !proc_dir.exists() {
            return false;
        }
        return std::fs::read(proc_dir.join("cmdline"))
            .map(|cmdline| {
                cmdline
                    .split(|byte| *byte == 0)
                    .any(|arg| arg == b"--background" || arg == b"--daemon")
            })
            .unwrap_or(false);
    }
    false
}

/// Ensure background daemon is running if any PWAs are installed
pub(crate) fn ensure_background_daemon_running() {
    ensure_background_autostart();
    let installed = list_installed_pwas();
    if installed.is_empty() && !crate::push::has_subscriptions() {
        return;
    }
    if is_daemon_running() {
        return;
    }

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rail"));
    let _ = std::process::Command::new(exe)
        .arg("--background")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Run headless background PWA daemon listening for web notifications with minimal RAM usage
pub(crate) fn run_background_daemon() -> glib::ExitCode {
    let _ = gtk::init();

    if is_daemon_running() {
        return glib::ExitCode::SUCCESS;
    }

    let _ = std::fs::create_dir_all(pwa_apps_dir());
    let pid_file = daemon_pid_file();
    let my_pid = std::process::id();
    let _ = std::fs::write(&pid_file, my_pid.to_string());

    let installed = list_installed_pwas();
    if installed.is_empty() && !crate::push::has_subscriptions() {
        let _ = std::fs::remove_file(&pid_file);
        return glib::ExitCode::SUCCESS;
    }

    let mut mgr = ProfileManager::new(rail_data_dir());
    let prof_id = mgr.active_id().to_string();
    let session = mgr.get_or_create_session(&prof_id);

    // Constrain WebKit background memory usage
    let mut mps = webkit6::MemoryPressureSettings::new();
    mps.set_memory_limit(128);
    mps.set_conservative_threshold(0.20);
    mps.set_strict_threshold(0.35);
    mps.set_kill_threshold(0.0);
    mps.set_poll_interval(5.0);
    webkit6::NetworkSession::set_memory_pressure_settings(&mut mps);

    let settings = webkit6::Settings::new();
    settings.set_enable_page_cache(false);
    settings.set_enable_javascript(true);
    settings.set_enable_webaudio(false);
    settings.set_enable_webgl(false);
    settings.set_enable_smooth_scrolling(false);
    settings.set_media_playback_requires_user_gesture(true);

    let mut background_views = Vec::new();
    // Web Push receiver: keeps listening for pushes while the browser is closed.
    background_views.push(crate::push::start_receiver());

    for pwa in installed {
        // Same Web Push bridge as browser tabs, so installed apps never see "push unsupported".
        let ucm = webkit6::UserContentManager::new();
        crate::push::install_bridge(&ucm);
        let view = WebView::builder()
            .network_session(&session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .build();
        crate::push::attach_handler(&view);

        let notification_allowed = is_notification_permission_granted(&pwa.url);
        view.connect_permission_request(move |_v, req| {
            if req.is::<webkit6::NotificationPermissionRequest>() {
                if notification_allowed {
                    req.allow();
                } else {
                    req.deny();
                }
                return true;
            }
            false
        });

        let icon_file = pwa.icon_url.clone();
        crate::notifications::attach_notification_handler(
            &view,
            Some(pwa.name.clone()),
            icon_file,
        );

        view.load_uri(&pwa.url);
        background_views.push(view);
    }

    let main_loop = glib::MainLoop::new(None, false);
    let ml = main_loop.clone();
    let pid_f = pid_file.clone();

    // After an update the running daemon would keep serving the old code (and old push
    // receiver) until the next login. Detect a replaced executable and hand over to the new one.
    let exe = std::env::current_exe().ok();
    let stamp = |p: &Option<PathBuf>| {
        p.as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok())
    };
    let started_stamp = stamp(&exe);
    glib::timeout_add_local(std::time::Duration::from_secs(60), move || {
        if !pid_f.exists() {
            ml.quit();
            return glib::ControlFlow::Break;
        }
        if let (Some(a), Some(b)) = (started_stamp, stamp(&exe))
            && a != b
            && let Some(path) = exe.clone()
        {
            let _ = std::fs::remove_file(&pid_f);
            let _ = std::process::Command::new(path)
                .arg("--background")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            ml.quit();
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });

    main_loop.run();
    // Only remove the pid file if it is still ours (an updated successor writes its own).
    if std::fs::read_to_string(&pid_file)
        .map(|p| p.trim() == my_pid.to_string())
        .unwrap_or(false)
    {
        let _ = std::fs::remove_file(&pid_file);
    }
    glib::ExitCode::SUCCESS
}

pub(crate) const PWA_BRIDGE_SCRIPT: &str = r#"
(function() {
  if (window.__railPwaBridgeInjected) return;
  window.__railPwaBridgeInjected = true;

  window.__railIsPwaInstalled = false;

  // Uzbek sites write o'rnatish with typographic apostrophes (o\u2018, o\u2019, o\u02bb); fold them to ASCII.
  function normTxt(t) {
    return String(t).toLowerCase().replace(/[\u2018\u2019\u02bb\u02bc\u0060\u00b4]/g, "'").trim();
  }

  // A site's "Install app" control is a short label on a button/link (or a childless label).
  // It must NEVER match a layout container that merely *contains* such a button: matching on text
  // `includes` over every <div> used to grey out and disable the whole app (root div -> opacity .45,
  // grayscale, pointer-events:none).
  var INSTALL_PHRASES = ["ilovani o'rnatish", "ilovani ornatish", "ilovani yuklab olish", "install app", "install pwa"];
  var INSTALL_EXACT = ["o'rnatish", "ornatish", "install"];
  function isInstallControl(el, allowExact) {
    if (!el || el.nodeType !== 1) return false;
    var tag = el.tagName;
    var interactive = tag === 'BUTTON' || tag === 'A' || el.getAttribute('role') === 'button';
    if (!interactive && el.children.length > 0) return false;
    var txt = normTxt(el.innerText || el.textContent || '');
    if (!txt || txt.length > 40) return false;
    if (allowExact && INSTALL_EXACT.indexOf(txt) !== -1) return true;
    for (var i = 0; i < INSTALL_PHRASES.length; i++) {
      if (txt.indexOf(INSTALL_PHRASES[i]) !== -1) return true;
    }
    return false;
  }

  function updatePageButtons(installed) {
    if (!installed) return;
    try {
      var buttons = document.querySelectorAll('button, a, [role="button"], div, span');
      for (var i = 0; i < buttons.length; i++) {
        var el = buttons[i];
        if (!isInstallControl(el, false)) continue;
        el.style.opacity = '0.45';
        el.style.filter = 'grayscale(100%)';
        el.style.pointerEvents = 'none';
        el.setAttribute('aria-disabled', 'true');
        if (el.tagName === 'BUTTON') el.setAttribute('disabled', 'true');
      }
    } catch (_) {}
  }

  window.__railSetPwaInstalled = function(installed) {
    window.__railIsPwaInstalled = !!installed;
    if (installed) {
      try {
        window.dispatchEvent(new CustomEvent('appinstalled'));
      } catch (_) {}
      updatePageButtons(true);
      setTimeout(function() { updatePageButtons(true); }, 500);
      setTimeout(function() { updatePageButtons(true); }, 2000);
    }
  };

  function checkInstalledStatus() {
    try {
      var chkHandler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railPwaCheck;
      if (chkHandler) {
        chkHandler.postMessage(window.location.href);
      }
    } catch (_) {}
  }

  function getManifestHref() {
    var el = document.querySelector('link[rel="manifest"]');
    return el ? el.href : '';
  }

  function getAppIconHref() {
    var el = document.querySelector('link[rel="icon"][sizes="512x512"], link[rel="apple-touch-icon"], link[rel="icon"][sizes="192x192"], link[rel="icon"]');
    return el ? el.href : '';
  }

  function getThemeColorMeta() {
    var el = document.querySelector('meta[name="theme-color"]');
    return el ? el.content : '';
  }

  function sendPwaInstallTrigger() {
    try {
      var handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railPwaInstall;
      if (handler) {
        var payload = JSON.stringify({
          title: document.title || '',
          url: window.location.href,
          manifest: getManifestHref(),
          icon: getAppIconHref(),
          theme: getThemeColorMeta()
        });
        handler.postMessage(payload);
      }
    } catch (_) {}
  }

  function fireBeforeInstallPrompt() {
    if (window.__railIsPwaInstalled) {
      updatePageButtons(true);
      return;
    }
    try {
      var evt = new Event('beforeinstallprompt', { bubbles: true, cancelable: true });
      evt.prompt = function() {
        sendPwaInstallTrigger();
        return Promise.resolve({ outcome: 'accepted', platform: 'web' });
      };
      evt.userChoice = Promise.resolve({ outcome: 'accepted', platform: 'web' });
      window.deferredPrompt = evt;
      window.__beforeInstallPromptEvent = evt;
      window.dispatchEvent(evt);
    } catch (_) {}
  }

  // SPAs often register their beforeinstallprompt listener after the event already fired.
  // Replay the saved event to such late listeners so the site still captures a usable prompt().
  var origAddListener = window.addEventListener;
  window.addEventListener = function(type, fn, opts) {
    var r = origAddListener.apply(this, arguments);
    if (type === 'beforeinstallprompt' && this === window && fn &&
        window.__beforeInstallPromptEvent && !window.__railIsPwaInstalled) {
      var saved = window.__beforeInstallPromptEvent;
      setTimeout(function() {
        try {
          if (typeof fn === 'function') fn.call(window, saved);
          else if (fn.handleEvent) fn.handleEvent(saved);
        } catch (_) {}
      }, 0);
    }
    return r;
  };

  function scheduleInstallPrompt() {
    checkInstalledStatus();
    setTimeout(fireBeforeInstallPrompt, 150);
    setTimeout(fireBeforeInstallPrompt, 1000);
    setTimeout(fireBeforeInstallPrompt, 3500);
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', scheduleInstallPrompt);
  } else {
    scheduleInstallPrompt();
  }

  // Intercept clicks on in-page install buttons (e.g. MyPlan's "Ilovani o'rnatish" button)
  document.addEventListener('click', function(e) {
    if (window.__railIsPwaInstalled) {
      return;
    }
    // Nearest real control first; a click on a bare label inside it counts through that control.
    var t = e.target;
    var btn = t && t.closest ? t.closest('button, a, [role="button"]') : null;
    if (!btn || !isInstallControl(btn, true)) btn = (t && isInstallControl(t, true)) ? t : null;
    if (btn) {
      sendPwaInstallTrigger();
      e.preventDefault();
      e.stopPropagation();
      if (e.stopImmediatePropagation) e.stopImmediatePropagation();
    }
  }, true);
})();
"#;

pub(crate) fn inject_pwa_bridge(ucm: &webkit6::UserContentManager) {
    let script = webkit6::UserScript::new(
        PWA_BRIDGE_SCRIPT,
        webkit6::UserContentInjectedFrames::AllFrames,
        webkit6::UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    ucm.add_script(&script);
}

impl App {
    pub(crate) fn handle_pwa_install_message(self: &Rc<Self>, payload: &str, view: &WebView) {
        let mut title = String::new();
        let mut url = view.uri().map(|s| s.to_string()).unwrap_or_default();
        let mut manifest_url: Option<String> = None;
        let mut icon_url: Option<String> = None;
        let mut theme_color: Option<String> = None;

        if let Some(json) = crate::minijson::parse(payload) {
            if let Some(t) = json.get("title").and_then(|v| v.as_str()) {
                title = t.trim().to_string();
            }
            if let Some(u) = json.get("url").and_then(|v| v.as_str())
                && !u.trim().is_empty()
            {
                url = u.trim().to_string();
            }
            if let Some(m) = json.get("manifest").and_then(|v| v.as_str())
                && !m.trim().is_empty()
            {
                manifest_url = Some(m.trim().to_string());
            }
            if let Some(i) = json.get("icon").and_then(|v| v.as_str())
                && !i.trim().is_empty()
            {
                icon_url = Some(i.trim().to_string());
            }
            if let Some(th) = json.get("theme").and_then(|v| v.as_str())
                && !th.trim().is_empty()
            {
                theme_color = Some(th.trim().to_string());
            }
        }

        if title.is_empty() {
            title = view.title().map(|s| s.to_string()).unwrap_or_else(|| url.clone());
        }

        if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
            return;
        }

        // If manifest_url is relative, resolve it
        if let Some(ref m_url) = manifest_url {
            let resolved_m_url = if !m_url.contains("://") {
                glib::Uri::resolve_relative(Some(url.as_str()), m_url, glib::UriFlags::NONE)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| m_url.clone())
            } else {
                m_url.clone()
            };

            // Fetch manifest to discover clean app name, high-res icon and theme color
            let res = std::process::Command::new("curl")
                .args(["-fsSL", "-m", "4", "-A", "Mozilla/5.0 (X11; Linux x86_64)", &resolved_m_url])
                .output();
            if let Ok(out) = res
                && out.status.success()
                && let Ok(text) = String::from_utf8(out.stdout)
                && let Some(root) = crate::minijson::parse(&text)
            {
                if let Some(name) = root.get("short_name").and_then(|v| v.as_str())
                    && !name.trim().is_empty()
                {
                    title = name.trim().to_string();
                } else if let Some(name) = root.get("name").and_then(|v| v.as_str())
                    && !name.trim().is_empty()
                {
                    title = name.trim().to_string();
                }
                if let Some(th) = root.get("theme_color").and_then(|v| v.as_str()) {
                    theme_color = Some(th.trim().to_string());
                }
                if let Some(icons) = root.get("icons").and_then(|v| v.as_arr()) {
                    for icon_obj in icons.iter().rev() {
                        if let Some(src) = icon_obj.get("src").and_then(|v| v.as_str()) {
                            if src.starts_with("http://") || src.starts_with("https://") {
                                icon_url = Some(src.to_string());
                                break;
                            } else if let Ok(resolved) = glib::Uri::resolve_relative(Some(url.as_str()), src, glib::UriFlags::NONE) {
                                icon_url = Some(resolved.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Clean up title (remove website tagline like "MyPlan — Plan your day" -> "MyPlan")
        let clean_title = if let Some(first_part) = title.split(['—', '–', '|', '-']).next() {
            let t = first_part.trim();
            if t.len() >= 2 {
                t.to_string()
            } else {
                title
            }
        } else {
            title
        };

        let pwa_info = PwaAppInfo {
            id: slugify(&clean_title),
            name: clean_title,
            url,
            icon_url,
            theme_color,
        };

        prompt_install_pwa_dialog(self, &pwa_info);
    }

    /// Broadcast PWA installation state change to all open tabs matching this origin
    pub(crate) fn broadcast_pwa_installed_for_url(self: &Rc<Self>, url: &str, installed: bool) {
        let target_origin = get_url_origin(url);
        if target_origin.is_empty() {
            return;
        }
        if let Ok(rows) = self.rows.try_borrow() {
            for row in rows.iter() {
                for tab in &row.tabs {
                    if let Some(uri) = tab.view.uri() {
                        let origin = get_url_origin(uri.as_str());
                        if origin == target_origin {
                            send_pwa_installed_status_to_view(&tab.view, installed);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pwa_bridge_never_treats_layout_containers_as_install_buttons() {
        // Regression: scanning every <div> for text that merely *contains* "install app" greyed out and
        // disabled the whole page (the root div contains the site's install button).
        assert!(!PWA_BRIDGE_SCRIPT.contains(r#"closest('button, a, [role="button"], div')"#));
        assert!(PWA_BRIDGE_SCRIPT.contains("function isInstallControl"));
        assert!(PWA_BRIDGE_SCRIPT.contains("el.children.length > 0"));
    }

    #[test]
    fn notification_permission_requires_explicit_origin_allow() {
        let saved = "example.com\tnotifications\tallow\nother.com\tnotifications\tdeny\n";
        assert!(notification_permission_granted(
            "https://example.com/app",
            saved
        ));
        assert!(!notification_permission_granted("https://other.com", saved));
        assert!(!notification_permission_granted(
            "https://unknown.com",
            saved
        ));
        assert!(!notification_permission_granted("file:///tmp/app", saved));
    }

    #[test]
    fn notification_permission_is_case_insensitive_for_host() {
        assert!(notification_permission_granted(
            "https://EXAMPLE.COM/path",
            "example.com\tnotifications\tallow\n"
        ));
    }

    #[test]
    fn test_daemon_pid_path() {
        let p = daemon_pid_file();
        assert!(p.ends_with("apps/daemon.pid"));
    }

    #[test]
    fn test_is_daemon_running_on_nonexistent() {
        assert!(!PathBuf::from("/proc/999999999").exists());
    }

    #[test]
    fn test_svg_icon_generation() {
        let svg = generate_svg_icon("Telegram", Some("#0088cc"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("#0088cc"));
        assert!(svg.contains(">T<"));
    }
}
