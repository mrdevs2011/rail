//! Rail: a niri-style scrolling browser on the system WebKitGTK.
//!
//! Layout of the crate:
//! - `app/`        the `App` state and its behaviour, one concern per file
//! - `ui/`         window construction: `widgets/` (widget trees), `settings/`
//!                 (Settings pages), `wire/` (signal wiring), `build.rs` (glue)
//! - `model/`      tabs, rows, session format, minimap state, tuning constants
//! - `shortcuts/`  shortcut definitions, live labels and the shortcut editor
//! - `pages/`      rail://start and rail://downloads
//! - `nav/`        address-bar input to URI, internal pages
//! - `web/`        user style sheets and ad & tracker blocking
//! - `config.rs`   data directory and settings.json; `cli.rs` export / import

mod blocklist;
mod history;
mod i18n;
mod i18n_tables;
mod jank;
mod minijson;
mod profile;
mod qr;
mod spring;
mod strip;
mod sync;
mod update;
use i18n::{tr, trf};

use gtk::glib;
use gtk::{gdk, gio};
use gtk4 as gtk;
use history::{HistoryManager, HistorySuggestion, SuggestionKind};
use profile::ProfileManager;
use spring::{SLIDE_DAMPING_RATIO, SLIDE_STIFFNESS, Spring, coast_distance};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;
use strip::{SIDE_W, ShellLayout, StripLayout, col_width, gap, group_ranges, stack_height};
use webkit6::prelude::*;
use webkit6::{
    ContextMenu, ContextMenuAction, ContextMenuItem, HitTestResult, ScriptDialog, ScriptDialogType,
    WebView,
};

mod app;
mod cli;
mod config;
mod model;
mod nav;
mod pages;
mod pwa;
mod shortcuts;
mod ui;
mod web;

use app::*;
use cli::*;
use config::*;
use model::*;
use nav::*;
use pages::*;
use shortcuts::*;
use ui::*;
use web::*;

fn main() -> glib::ExitCode {
    // libsoup3 HTTP/2 tunnel & proxy fix:
    // HTTP/2 multiplexing is required for YouTube video/audio streaming and instant seeking.
    // Setting SOUP_FORCE_HTTP1 globally starved video chunk downloads by forcing serial HTTP/1.1 connections.
    // For local tunnels/proxies with broken HTTP/2 RST_STREAM handling, user can set RAIL_FORCE_HTTP1=1.
    if std::env::var_os("RAIL_FORCE_HTTP1").is_some_and(|v| v != "0") {
        unsafe {
            std::env::set_var("SOUP_FORCE_HTTP1", "1");
        }
    }

    // RAM: constrain WebKit early (before any NetworkSession / WebView)
    // DocumentViewer cache model uses far less memory than full WebBrowser.
    // GPU compositing va DMABUF renderer yoqilgan holda qoldiriladi: ilgari ikkalasi majburan
    // o'chirilgan edi, bu sahifalarni CPU'da chizishga olib kelib scroll'ni qotirardi/dagal qilardi.
    // Muammo bo'lsa (bo'sh ekran, miltillash) eski rejim: `RAIL_SOFTWARE_RENDER=1 rail`.
    // Disable broken gst-plugins-bad alpha combiners that crash on WebM / screen recordings
    match std::env::var("GST_PLUGIN_FEATURE_RANK") {
        Ok(v) => {
            if !v.contains("vp8alphadecodebin:0") {
                let merged = format!("{v},vp8alphadecodebin:0,vp9alphadecodebin:0");
                unsafe {
                    std::env::set_var("GST_PLUGIN_FEATURE_RANK", merged);
                }
            }
        }
        Err(_) => unsafe {
            std::env::set_var(
                "GST_PLUGIN_FEATURE_RANK",
                "vp8alphadecodebin:0,vp9alphadecodebin:0",
            );
        },
    }

    if std::env::var_os("RAIL_SOFTWARE_RENDER").is_some_and(|v| v != "0") {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    } else {
        // Smooth 60/120 FPS GPU hardware acceleration defaults:
        if std::env::var_os("WEBKIT_FORCE_COMPOSITING_MODE").is_none() {
            unsafe {
                std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
            }
        }
        if std::env::var_os("LIBVA_DRIVER_NAME").is_none()
            && std::path::Path::new("/usr/lib/dri/iHD_drv_video.so").exists()
        {
            unsafe {
                std::env::set_var("LIBVA_DRIVER_NAME", "iHD");
            }
        }
        if std::env::var_os("GSK_RENDERER").is_none() {
            unsafe {
                std::env::set_var("GSK_RENDERER", "ngl");
            }
        }
    }
    if std::env::var_os("RAIL_LOW_MEM").is_some_and(|v| v != "0") {
        // Extreme low-RAM mode only: small limit, aggressive reclaim
        let mut mps = webkit6::MemoryPressureSettings::new();
        mps.set_memory_limit(256);
        mps.set_conservative_threshold(0.25);
        mps.set_strict_threshold(0.40);
        mps.set_kill_threshold(0.0);
        mps.set_poll_interval(3.0);
        webkit6::NetworkSession::set_memory_pressure_settings(&mut mps);
        if let Some(ctx) = webkit6::WebContext::default() {
            ctx.set_cache_model(webkit6::CacheModel::DocumentViewer);
        }
    } else if let Some(ctx) = webkit6::WebContext::default() {
        // Default mode: full browser caching without artificial RAM throttling.
        // This ensures YouTube / HTML5 video buffers remain in RAM for instant seeking
        // without constant buffer eviction thrashing.
        ctx.set_cache_model(webkit6::CacheModel::WebBrowser);
    }

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a.starts_with("--appimage-")) {
        return glib::ExitCode::FAILURE;
    }

    // First pass: extract custom data directory if specified
    for (idx, arg) in args.iter().enumerate() {
        if arg == "-d" || arg == "--data-dir" {
            if let Some(next) = args.get(idx + 1) {
                let _ = CUSTOM_DATA_DIR.set(PathBuf::from(next));
                break;
            }
        } else if arg.starts_with("--data-dir=") {
            let val = &arg["--data-dir=".len()..];
            let _ = CUSTOM_DATA_DIR.set(PathBuf::from(val));
            break;
        }
    }

    let mut target_url: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--version" || arg == "-v" {
            println!("rail {}", env!("CARGO_PKG_VERSION"));
            return glib::ExitCode::SUCCESS;
        }
        if arg == "--update" {
            return update::cli_update();
        }
        if arg == "--help" || arg == "-h" {
            println!("{}", tr("Usage: rail [OPTIONS] [URL]"));
            println!();
            println!("{}", tr("Options:"));
            println!(
                "{}",
                tr(
                    "  -d, --data-dir <PATH>      Use custom storage directory for 100% portable environment"
                )
            );
            println!(
                "{}",
                tr(
                    "      --export <FILE.tar.gz> Export all tabs, profiles, logins & cookies into archive"
                )
            );
            println!(
                "{}",
                tr(
                    "      --import <FILE.tar.gz> Import all tabs, profiles, logins & cookies from archive"
                )
            );
            println!(
                "{}",
                tr(
                    "      --data-info            Show current storage directory and environment details"
                )
            );
            println!(
                "{}",
                tr("      --update               Download and install the newest release")
            );
            println!("{}", tr("  -v, --version              Show version"));
            println!("{}", tr("  -h, --help                 Show help message"));
            println!();
            println!("{}", tr("Environment Variables:"));
            println!(
                "{}",
                tr(
                    "  RAIL_DATA_DIR              Override default storage directory (~/.local/share/rail)"
                )
            );
            return glib::ExitCode::SUCCESS;
        }
        if arg == "--data-info" {
            print_data_info();
            return glib::ExitCode::SUCCESS;
        }
        if arg == "--export" {
            if i + 1 < args.len() {
                let out = PathBuf::from(&args[i + 1]);
                if let Err(e) = export_data(&out) {
                    eprintln!("{}", trf("Error exporting data: {}", &[&e]));
                    return glib::ExitCode::FAILURE;
                }
                return glib::ExitCode::SUCCESS;
            } else {
                eprintln!(
                    "{}",
                    tr(
                        "Error: --export requires an output filename (e.g. rail --export backup.tar.gz)"
                    )
                );
                return glib::ExitCode::FAILURE;
            }
        }
        if arg == "--import" {
            if i + 1 < args.len() {
                let inp = PathBuf::from(&args[i + 1]);
                if let Err(e) = import_data(&inp) {
                    eprintln!("{}", trf("Error importing data: {}", &[&e]));
                    return glib::ExitCode::FAILURE;
                }
                return glib::ExitCode::SUCCESS;
            } else {
                eprintln!(
                    "{}",
                    tr(
                        "Error: --import requires an archive filename (e.g. rail --import backup.tar.gz)"
                    )
                );
                return glib::ExitCode::FAILURE;
            }
        }
        if arg == "-d" || arg == "--data-dir" {
            i += 2;
            continue;
        } else if arg.starts_with("--data-dir=") {
            i += 1;
            continue;
        } else if arg == "--app" {
            if i + 1 < args.len() {
                let _ = PWA_APP_URL.set(args[i + 1].clone());
                i += 2;
                continue;
            }
        } else if arg.starts_with("--app=") {
            let _ = PWA_APP_URL.set(arg["--app=".len()..].to_string());
            i += 1;
            continue;
        } else if !arg.starts_with('-') && target_url.is_none() {
            target_url = Some(arg.clone());
        }
        i += 1;
    }

    if let Some(u) = target_url {
        let _ = TARGET_URL.set(u);
    }

    // Single instance: running `rail [URL]` again hands the request to the already-open Rail
    // (profiles live inside Rail, no need for several processes). A custom data dir
    // (-d / RAIL_DATA_DIR) gets its own instance id, so portable setups stay independent.
    let app_id = if CUSTOM_DATA_DIR.get().is_some() || std::env::var_os("RAIL_DATA_DIR").is_some() {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        rail_data_dir().to_string_lossy().hash(&mut h);
        format!("dev.rail.browser.d{:x}", h.finish())
    } else {
        "dev.rail.browser".to_string()
    };
    let app = gtk::Application::builder()
        .application_id(app_id.as_str())
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    // Disable GTK Inspector default shortcut (Ctrl+Shift+I) so it never intercepts Rail's WebKit DevTools
    if let Some(source) = gio::SettingsSchemaSource::default() {
        for schema in ["org.gtk.gtk4.Settings.Debug", "org.gtk.Settings.Debug"] {
            if source.lookup(schema, true).is_some() {
                let settings = gio::Settings::new(schema);
                let _ = settings.set_boolean("enable-inspector-keybinding", false);
            }
        }
    }

    app.connect_activate(|gapp| {
        if let Some(app_url) = PWA_APP_URL.get() {
            crate::pwa::open_pwa_window(gapp, app_url, None);
            return;
        }
        build(gapp);
    });
    app.connect_open(|gapp, files, _hint| {
        let existing = MAIN_APP.with(|m| m.borrow().clone());
        match existing {
            Some(a) => {
                for file in files {
                    let uri = file.uri().to_string();
                    a.new_tab_with_url(&uri);
                }
                a.win.set_visible(true);
                a.win.present();
                if let Some(v) = a.focused_view() {
                    v.grab_focus();
                }
            }
            None => {
                if let Some(first) = files.first() {
                    let _ = TARGET_URL.set(first.uri().to_string());
                }
                gapp.activate();
            }
        }
    });
    app.connect_command_line(|gapp, cmdline| {
        let args: Vec<String> = cmdline
            .arguments()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        if let Some(app_url) = parse_cli_app(&args) {
            crate::pwa::open_pwa_window(gapp, &app_url, None);
            return glib::ExitCode::SUCCESS;
        }
        let url = parse_cli_url(&args);
        let existing = MAIN_APP.with(|m| m.borrow().clone());
        match existing {
            // Already running: open the URL in a new tab (or reuse empty start tab) and raise the window.
            Some(a) => {
                if let Some(u) = url {
                    let uri = to_uri(&u, &a.search_engine.borrow());
                    let can_reuse = {
                        let rows = a.rows.borrow();
                        rows.len() == 1
                            && rows[0].tabs.len() == 1
                            && rows[0].tabs[0].real_uri().is_empty()
                    };
                    if can_reuse {
                        a.navigate(&uri);
                    } else {
                        a.new_tab_with_url(&uri);
                    }
                }
                if let Some(token) = cmdline
                    .getenv("XDG_ACTIVATION_TOKEN")
                    .or_else(|| cmdline.getenv("DESKTOP_STARTUP_ID"))
                {
                    a.win.set_startup_id(&token);
                }
                a.win.set_visible(true);
                a.win.present();
                if let Some(v) = a.focused_view() {
                    v.grab_focus();
                }
            }
            // First launch: populate TARGET_URL and activate.
            None => {
                if let Some(u) = url {
                    let _ = TARGET_URL.set(u);
                }
                gapp.activate();
            }
        }
        glib::ExitCode::SUCCESS
    });
    app.run_with_args(&args)
}

/// First positional argument (the URL / search text), skipping `-d <dir>` and `--data-dir=…`.
fn parse_cli_url(args: &[String]) -> Option<String> {
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a == "-d" || a == "--data-dir" {
            i += 2;
            continue;
        }
        if a == "--app" {
            i += 2;
            continue;
        }
        if a.starts_with("--app=") {
            i += 1;
            continue;
        }
        if !a.starts_with('-') {
            return Some(a.clone());
        }
        i += 1;
    }
    None
}

/// Standalone PWA app argument: `--app="https://..."` or `--app https://...`
fn parse_cli_app(args: &[String]) -> Option<String> {
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a == "--app" {
            if i + 1 < args.len() {
                return Some(args[i + 1].clone());
            }
        } else if a.starts_with("--app=") {
            return Some(a["--app=".len()..].to_string());
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests;
