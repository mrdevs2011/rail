//! Data directory and persisted app settings (settings.json).

use crate::*;

pub(crate) static CUSTOM_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
pub(crate) static TARGET_URL: OnceLock<String> = OnceLock::new();

pub(crate) fn rail_data_dir() -> PathBuf {
    if let Some(dir) = CUSTOM_DATA_DIR.get() {
        return dir.clone();
    }
    if let Ok(dir) = std::env::var("RAIL_DATA_DIR") {
        let p = PathBuf::from(dir.trim());
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    glib::user_data_dir().join("rail")
}

pub(crate) fn settings_json_path() -> PathBuf {
    rail_data_dir().join("settings.json")
}

#[derive(Clone, Debug)]
pub(crate) struct AppSettings {
    pub(crate) search_engine: String,
    pub(crate) touchpad_mode: bool,
    pub(crate) smooth_scrolling: bool,
    pub(crate) webrtc_enabled: bool,
    /// Launchda yangi versiyani tekshirib o'rnatish
    pub(crate) auto_update: bool,
    /// DevTools oxirgi dock holati: "bottom" | "left" | "right" | "undocked"
    pub(crate) devtools_dock: String,
    /// Page translation: source language code, "auto" = detect from the page.
    pub(crate) translate_source_lang: String,
    /// Page translation: target language code (default "uz").
    pub(crate) translate_target_lang: String,
    /// Sidebar behavior: "fixed" (always open when shown) or "auto" (edge-hover).
    pub(crate) sidebar_mode: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            search_engine: "google".to_string(),
            touchpad_mode: false,
            smooth_scrolling: true,
            webrtc_enabled: true,
            auto_update: false,
            devtools_dock: "bottom".to_string(),
            translate_source_lang: "auto".to_string(),
            translate_target_lang: "uz".to_string(),
            sidebar_mode: "fixed".to_string(),
        }
    }
}

pub(crate) fn devtools_dock_to_str(side: DevToolsDockSide) -> &'static str {
    match side {
        DevToolsDockSide::Bottom => "bottom",
        DevToolsDockSide::Left => "left",
        DevToolsDockSide::Right => "right",
        DevToolsDockSide::Undocked => "undocked",
    }
}

pub(crate) fn devtools_dock_from_str(s: &str) -> DevToolsDockSide {
    match s {
        "left" => DevToolsDockSide::Left,
        "right" => DevToolsDockSide::Right,
        // "undocked" ataylab tiklanmaydi: niri'da alohida oyna butun ekranga ochiladi.
        _ => DevToolsDockSide::Bottom,
    }
}

/// WebRTC on/off from settings.json (default on), read once per launch.
pub(crate) fn webrtc_allowed() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| load_app_settings().webrtc_enabled)
}

pub(crate) fn load_app_settings() -> AppSettings {
    let mut s = AppSettings::default();
    let Ok(text) = std::fs::read_to_string(settings_json_path()) else {
        return s;
    };
    if let Some(root) = minijson::parse(&text) {
        if let Some(se) = root.get("search_engine").and_then(|v| v.as_str()) {
            s.search_engine = se.to_string();
        }
        if let Some(tp) = root.get("touchpad_navigation").and_then(|v| v.as_bool()) {
            s.touchpad_mode = tp;
        }
        if let Some(ss) = root.get("smooth_scrolling").and_then(|v| v.as_bool()) {
            s.smooth_scrolling = ss;
        }
        if let Some(wr) = root.get("webrtc_enabled").and_then(|v| v.as_bool()) {
            s.webrtc_enabled = wr;
        }
        if let Some(au) = root.get("auto_update").and_then(|v| v.as_bool()) {
            s.auto_update = au;
        }
        if let Some(dd) = root.get("devtools_dock").and_then(|v| v.as_str()) {
            s.devtools_dock = dd.to_string();
        }
        if let Some(sl) = root.get("translate_source_lang").and_then(|v| v.as_str()) {
            s.translate_source_lang = clean_lang_code(sl, "auto");
        }
        if let Some(tl) = root.get("translate_target_lang").and_then(|v| v.as_str()) {
            s.translate_target_lang = clean_lang_code(tl, "uz");
        }
        if let Some(sm) = root.get("sidebar_mode").and_then(|v| v.as_str()) {
            if sm == "auto" || sm == "fixed" {
                s.sidebar_mode = sm.to_string();
            }
        }
    }
    s
}

pub(crate) fn save_app_settings(s: &AppSettings) -> bool {
    let mut out = String::from("{
");
    out.push_str(&format!("  \"search_engine\": \"{}\",
", minijson::escape(&s.search_engine)));
    out.push_str(&format!("  \"touchpad_navigation\": {},
", s.touchpad_mode));
    out.push_str(&format!("  \"smooth_scrolling\": {},
", s.smooth_scrolling));
    out.push_str(&format!("  \"webrtc_enabled\": {},
", s.webrtc_enabled));
    out.push_str(&format!("  \"auto_update\": {},
", s.auto_update));
    out.push_str(&format!("  \"devtools_dock\": \"{}\",\n", minijson::escape(&s.devtools_dock)));
    out.push_str(&format!("  \"translate_source_lang\": \"{}\",\n", minijson::escape(&s.translate_source_lang)));
    out.push_str(&format!("  \"translate_target_lang\": \"{}\",\n", minijson::escape(&s.translate_target_lang)));
    out.push_str(&format!("  \"sidebar_mode\": \"{}\"\n", minijson::escape(&s.sidebar_mode)));
    out.push_str("}
");
    let path = settings_json_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out).is_ok()
}
