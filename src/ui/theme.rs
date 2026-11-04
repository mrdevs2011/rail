//! Theme management: Dark, Light, and System synchronization with Linux desktop and WebKit.

use crate::*;
use std::cell::RefCell;

pub(crate) const LIGHT_THEME_CSS: &str = include_str!("style_light.css");

thread_local! {
    pub(crate) static THEME_PROVIDER: gtk::CssProvider = gtk::CssProvider::new();
    static SYSTEM_THEME_INITIALIZED: RefCell<bool> = const { RefCell::new(false) };
}

/// Detects whether the Linux system is currently configured for dark mode.
pub(crate) fn detect_system_is_dark() -> bool {
    // 1. Try querying org.gnome.desktop.interface color-scheme via gio::Settings
    if let Some(source) = gio::SettingsSchemaSource::default()
        && source.lookup("org.gnome.desktop.interface", true).is_some()
    {
        let s = gio::Settings::new("org.gnome.desktop.interface");
        let scheme = s.string("color-scheme").to_string();
        if scheme.contains("dark") {
            return true;
        }
        if scheme.contains("light") || scheme.contains("default") {
            let theme = s.string("gtk-theme").to_string();
            if theme.to_lowercase().contains("dark") {
                return true;
            }
            return false;
        }
    }

    // 2. Fallback to GtkSettings default theme name or dark preference
    if let Some(s) = gtk::Settings::default() {
        if let Some(theme) = s.gtk_theme_name()
            && theme.to_lowercase().contains("dark")
        {
            return true;
        }
        if s.is_gtk_application_prefer_dark_theme() {
            return true;
        }
    }

    // Default to true on Linux developer setups if ambiguous
    true
}

/// Applies the theme preference ("dark", "light", or "system") to Rail and WebKit.
pub(crate) fn apply_theme_preference(pref: &str) {
    let effective_dark = match pref {
        "light" => false,
        "dark" => true,
        _ => detect_system_is_dark(),
    };

    if let Some(s) = gtk::Settings::default() {
        s.set_gtk_application_prefer_dark_theme(effective_dark);
        if s.find_property("gtk-interface-color-scheme").is_some() {
            s.set_property(
                "gtk-interface-color-scheme",
                if effective_dark { 2i32 } else { 3i32 },
            );
        }
    }

    THEME_PROVIDER.with(|p| {
        if effective_dark {
            p.load_from_string("");
        } else {
            p.load_from_string(LIGHT_THEME_CSS);
        }
    });

    MAIN_APP.with(|m| {
        if let Some(app) = m.borrow().as_ref() {
            if effective_dark {
                app.win.remove_css_class("light-theme");
                app.win.add_css_class("dark-theme");
            } else {
                app.win.remove_css_class("dark-theme");
                app.win.add_css_class("light-theme");
            }
        }
    });
}

/// Sets up a reactive listener for Linux system dark/light theme changes.
pub(crate) fn setup_system_theme_listener() {
    let already_init = SYSTEM_THEME_INITIALIZED.with(|i| {
        if *i.borrow() {
            true
        } else {
            *i.borrow_mut() = true;
            false
        }
    });
    if already_init {
        return;
    }

    if let Some(source) = gio::SettingsSchemaSource::default()
        && source.lookup("org.gnome.desktop.interface", true).is_some()
    {
        let s = gio::Settings::new("org.gnome.desktop.interface");
        s.connect_changed(Some("color-scheme"), move |_, _| {
            if load_app_settings().theme == "system" {
                apply_theme_preference("system");
            }
        });
        s.connect_changed(Some("gtk-theme"), move |_, _| {
            if load_app_settings().theme == "system" {
                apply_theme_preference("system");
            }
        });
    }
}
