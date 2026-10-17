mod history;
mod minijson;
mod profile;
mod strip;
mod i18n;
mod i18n_tables;
mod update;
mod sync;
use i18n::{tr, trf};

use gtk::glib;
use gtk::{gdk, gio};
use gtk4 as gtk;
use history::{HistoryManager, HistorySuggestion, SuggestionKind};
use profile::{ProfileManager, ProfilePersona};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;
use strip::{col_width, gap, ShellLayout, StripLayout, SIDE_W};
use webkit6::prelude::*;
use webkit6::{
    ContextMenu, ContextMenuAction, ContextMenuItem, HitTestResult,
    ScriptDialog, ScriptDialogType, WebView,
};

static CUSTOM_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static TARGET_URL: OnceLock<String> = OnceLock::new();

pub fn rail_data_dir() -> PathBuf {
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

pub fn settings_json_path() -> PathBuf {
    rail_data_dir().join("settings.json")
}

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub search_engine: String,
    pub touchpad_mode: bool,
    pub smooth_scrolling: bool,
    pub webrtc_enabled: bool,
    /// Launchda yangi versiyani tekshirib o'rnatish
    pub auto_update: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            search_engine: "google".to_string(),
            touchpad_mode: false,
            smooth_scrolling: true,
            webrtc_enabled: true,
            auto_update: false,
        }
    }
}

pub fn load_app_settings() -> AppSettings {
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
    }
    s
}

pub fn save_app_settings(s: &AppSettings) -> bool {
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
    out.push_str(&format!("  \"auto_update\": {}
", s.auto_update));
    out.push_str("}
");
    let path = settings_json_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out).is_ok()
}

const DEFAULT_HTML_CSS: &str = include_str!("html_default.css");

thread_local! {
    static STYLE_MANAGERS: RefCell<Vec<glib::WeakRef<webkit6::UserContentManager>>> =
        RefCell::new(Vec::new());
}

/// Editable Chrome-like default stylesheet, stored next to the rest of Rail's data.
pub fn html_css_path() -> PathBuf {
    customize_dir().join("html.css")
}

/// Everything Customize writes lives here, so a Data & Backup export/import carries it along.
pub fn customize_dir() -> PathBuf {
    rail_data_dir().join(".railassets").join("customize")
}

/// Reads html.css; creates it from the built-in default the first time.
pub fn load_html_css() -> String {
    let path = html_css_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            // First run after the move: carry over the old html.css from the data root.
            let legacy = rail_data_dir().join("html.css");
            let initial =
                std::fs::read_to_string(&legacy).unwrap_or_else(|_| DEFAULT_HTML_CSS.to_string());
            let _ = std::fs::write(&path, &initial);
            if legacy.exists() {
                let _ = std::fs::remove_file(&legacy);
            }
            initial
        }
    }
}

fn user_style_sheet(css: &str) -> webkit6::UserStyleSheet {
    webkit6::UserStyleSheet::new(
        css,
        webkit6::UserContentInjectedFrames::AllFrames,
        webkit6::UserStyleLevel::User,
        &[],
        &[],
    )
}

/// Pushes the css to every live web view; returns how many views were updated.
pub fn apply_html_css(css: &str) -> usize {
    let mut applied = 0;
    STYLE_MANAGERS.with(|list| {
        let mut list = list.borrow_mut();
        list.retain(|w| w.upgrade().is_some());
        for w in list.iter() {
            if let Some(ucm) = w.upgrade() {
                ucm.remove_all_style_sheets();
                if !css.trim().is_empty() {
                    ucm.add_style_sheet(&user_style_sheet(css));
                }
                applied += 1;
            }
        }
    });
    applied
}

fn save_css_editor(buffer: &gtk::TextBuffer, btn: &gtk::Button) {
    let (start, end) = buffer.bounds();
    let text = buffer.text(&start, &end, false).to_string();
    let path = html_css_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::write(&path, &text) {
        Ok(_) => {
            apply_html_css(&text);
            btn.set_label(tr("Saved"));
        }
        Err(_) => btn.set_label(tr("Save failed")),
    }
    let b = btn.clone();
    glib::timeout_add_local_once(Duration::from_millis(1500), move || b.set_label(tr("Save")));
}

const START_PAGE: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<title>Rail Browser</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
:root {
  --bg: #050505;
  --panel: #0b0b0b;
  --line: #242424;
  --line-hi: #3a3a3a;
  --fg: #ededed;
  --mut: #8a8a8a;
  --dim: #525252;
  --acc: #38bdf8;
}
body {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg);
  color: var(--mut);
  font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', ui-monospace, monospace;
  font-size: 13px;
  padding: 32px 20px;
  overflow-y: auto;
  user-select: none;
}
.container {
  width: 100%;
  max-width: 680px;
  display: flex;
  flex-direction: column;
  gap: 26px;
  animation: fadeIn 160ms ease-out;
}
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
.brand-title {
  font-size: 46px;
  font-weight: 700;
  letter-spacing: -0.05em;
  color: var(--fg);
  line-height: 1;
}
.brand-title .dot { color: var(--acc); }
.brand-subtitle {
  margin-top: 10px;
  font-size: 12px;
  color: var(--dim);
}
.brand-subtitle::before { content: '// '; color: var(--line-hi); }
.search-wrapper { position: relative; width: 100%; }
.prompt {
  position: absolute;
  left: 14px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--fg);
  font-weight: 700;
  pointer-events: none;
}
.search-bar {
  width: 100%;
  background: var(--panel);
  border: 1px solid var(--line-hi);
  border-radius: 3px;
  padding: 13px 92px 13px 34px;
  font: inherit;
  font-size: 14px;
  color: var(--fg);
  caret-color: var(--acc);
  outline: none;
  transition: border-color 120ms ease;
}
.search-bar:focus { border-color: var(--fg); }
.search-bar::placeholder { color: var(--dim); }
.search-hint {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 10.5px;
  color: var(--mut);
  border: 1px solid var(--line-hi);
  border-bottom-width: 2px;
  border-radius: 3px;
  padding: 1px 7px;
  pointer-events: none;
}
</style>
</head>
<body>
<div class="container">
  <div class="brand-area">
    <div class="brand-title">rail<span class="dot">.</span></div>
    <div class="brand-subtitle">keyboard-first browser for developers</div>
  </div>

  <div class="search-wrapper">
    <span class="prompt">&gt;</span>
    <input
      type="text"
      class="search-bar"
      placeholder="search or enter url"
      autocomplete="off"
      spellcheck="false"
      onkeydown="if(event.key==='Enter'){var q=this.value.trim();if(q){if(q.indexOf('://')!==-1||(q.indexOf('.')!==-1&&q.indexOf(' ')===-1)){location.href=q.indexOf('://')===-1?'https://'+q:q;}else{location.href='https://www.google.com/search?q='+encodeURIComponent(q);}}}"
    />
    <span class="search-hint">Alt+G</span>
  </div>
</div>
</body>
</html>"##;

/// Boshlang'ich sahifa: til, sarlavha, tavsif va qidiruv placeholder joriy tilga o'giriladi.
fn start_page() -> String {
    START_PAGE
        .replacen("<html lang=\"en\">", &format!("<html lang=\"{}\">", i18n::current().code()), 1)
        .replacen("keyboard-first browser for developers", tr("keyboard-first browser for developers"), 1)
        .replacen("placeholder=\"search or enter url\"", &format!("placeholder=\"{}\"", tr("search or enter url")), 1)
}

const GOOGLE_G_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
  <path fill="#4285F4" d="M23.745 12.27c0-.7-.06-1.4-.19-2.07H12v4.51h6.6c-.29 1.52-1.14 2.82-2.4 3.68v3.05h3.88c2.27-2.09 3.66-5.17 3.66-9.17z"/>
  <path fill="#34A853" d="M12 24c3.24 0 5.95-1.08 7.93-2.91l-3.88-3.05c-1.08.72-2.45 1.16-4.05 1.16-3.12 0-5.77-2.1-6.72-4.93H1.25v3.15C3.27 21.36 7.33 24 12 24z"/>
  <path fill="#FBBC05" d="M5.28 14.27c-.25-.72-.38-1.49-.38-2.27s.13-1.55.38-2.27V6.58H1.25C.45 8.17 0 9.99 0 12s.45 3.83 1.25 5.42l4.03-3.15z"/>
  <path fill="#EA4335" d="M12 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C17.95 1.19 15.24 0 12 0 7.33 0 3.27 2.64 1.25 6.58l4.03 3.15c.95-2.83 3.6-4.98 6.72-4.98z"/>
</svg>"##;

fn create_google_icon(size: i32) -> gtk::Image {
    let bytes = glib::Bytes::from_static(GOOGLE_G_SVG.as_bytes());
    match gdk::Texture::from_bytes(&bytes) {
        Ok(t) => {
            let img = gtk::Image::from_paintable(Some(&t));
            img.set_pixel_size(size);
            img
        }
        Err(_) => {
            let img = gtk::Image::from_icon_name("system-search-symbolic");
            img.set_pixel_size(size);
            img
        }
    }
}

/// Rail-style settings gear (SVG, not emoji / theme icon)
const GEAR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF">
  <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.07.62-.07.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .43-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
</svg>"##;

fn create_gear_icon(size: i32) -> gtk::Image {
    let bytes = glib::Bytes::from_static(GEAR_SVG.as_bytes());
    match gdk::Texture::from_bytes(&bytes) {
        Ok(t) => {
            let img = gtk::Image::from_paintable(Some(&t));
            img.set_pixel_size(size);
            img
        }
        Err(_) => {
            let img = gtk::Image::from_icon_name("preferences-system-symbolic");
            img.set_pixel_size(size);
            img
        }
    }
}

const CSS: &str = "
window {
    background: #080808;
    border-radius: 0;
    box-shadow: none;
    color: #f4f4f4;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
label {
    color: #a1a1a1;
}

/* ---- dev-style ultra-thin minimalist scrollbars ---- */
scrollbar,
scrollbar.horizontal,
scrollbar.vertical,
scrollbar.overlay-indicator {
    background: transparent;
    background-color: transparent;
    border: none;
    box-shadow: none;
    padding: 0;
    margin: 0;
    min-width: 0;
    min-height: 0;
    -GtkScrollbar-has-backward-stepper: false;
    -GtkScrollbar-has-forward-stepper: false;
}
scrollbar.horizontal,
scrollbar.horizontal.overlay-indicator {
    min-height: 3px;
    height: 3px;
}
scrollbar.vertical,
scrollbar.vertical.overlay-indicator {
    min-width: 3px;
    width: 3px;
}
scrollbar trough,
scrollbar.horizontal trough,
scrollbar.vertical trough,
scrollbar.overlay-indicator trough,
scrollbar range trough {
    background: transparent;
    background-color: transparent;
    border: none;
    box-shadow: none;
    margin: 0;
    padding: 0;
    min-width: 0;
    min-height: 0;
}
scrollbar slider,
scrollbar.horizontal slider,
scrollbar.vertical slider,
scrollbar.overlay-indicator slider,
scrollbar.overlay-indicator:not(.dragging):not(.hovering) slider,
scrollbar range trough slider {
    min-width: 3px;
    min-height: 24px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.25);
    background-color: rgba(255, 255, 255, 0.25);
    border: none;
    box-shadow: none;
    margin: 0;
    padding: 0;
    transition: background-color 120ms ease, min-width 120ms ease, min-height 120ms ease;
}
scrollbar.horizontal slider,
scrollbar.horizontal.overlay-indicator slider,
scrollbar.overlay-indicator:not(.dragging):not(.hovering).horizontal slider {
    min-height: 3px;
    height: 3px;
    min-width: 24px;
    margin: 0;
    border: none;
}
scrollbar.vertical slider,
scrollbar.vertical.overlay-indicator slider,
scrollbar.overlay-indicator:not(.dragging):not(.hovering).vertical slider {
    min-width: 3px;
    width: 3px;
    min-height: 24px;
    margin: 0;
    border: none;
}
scrollbar:hover slider,
scrollbar.dragging slider,
scrollbar.horizontal:hover slider,
scrollbar.horizontal.dragging slider {
    background: rgba(255, 255, 255, 0.55);
    background-color: rgba(255, 255, 255, 0.55);
    min-height: 5px;
    height: 5px;
    border: none;
}
scrollbar.vertical:hover slider,
scrollbar.vertical.dragging slider {
    background: rgba(255, 255, 255, 0.55);
    background-color: rgba(255, 255, 255, 0.55);
    min-width: 5px;
    width: 5px;
    border: none;
}
scrollbar button {
    min-width: 0;
    min-height: 0;
    padding: 0;
    margin: 0;
    border: none;
    background: transparent;
    background-color: transparent;
    -gtk-icon-source: none;
}

/* ---- columns & tabs (Spatial 2D Strip) ---- */
.rail-col {
    background: transparent;
}
.tab-freeze {
    background: #050505;
}
.tab {
    border-radius: 5px;
    border: 1px solid #181818;
    background: #0c0c0c;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    transition: border-color 150ms ease, box-shadow 150ms ease;
}
.tab.focused {
    border-color: #ededed;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.05), 0 12px 36px rgba(0, 0, 0, 0.7);
}

/* ---- interactive tab resize splitter handle ---- */
.tab-resize-handle {
    min-width: 6px;
    background: transparent;
    transition: background-color 150ms ease, box-shadow 150ms ease;
}
.tab-resize-handle:hover,
.tab-resize-handle.active {
    background: #ededed;
    box-shadow: 0 0 10px rgba(255, 255, 255, 0.05);
}

/* ---- overview (the live strip, zoomed out in 2D) ---- */
.zoomed .tab {
    opacity: 0.88;
    border-radius: 6px;
    border: 1px solid #282828;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.7);
}
.zoomed .tab.focused {
    opacity: 1;
    border-color: #ededed;
    outline: 2px solid #ededed;
    outline-offset: 1px;
    box-shadow: 0 0 24px rgba(255, 255, 255, 0.05);
}
.ov-hint {
    background: #0d0d0d;
    color: #d4d4d4;
    border: 1px solid #2c2c2c;
    border-radius: 3px;
    padding: 7px 16px;
    font-size: 11.5px;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.8);
}

/* ---- sidebar (Modern Obsidian Acrylic) ---- */
.side-bar {
    background: #0a0a0a;
    border-right: 1px solid #121212;
    padding-right: 2px;
}
.brand {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.02em;
    color: #fafafa;
}
.brand-dot {
    color: #ededed;
}
button.btn-add-tab {
    background: #111111;
    border: 1px solid #1a1a1a;
    border-radius: 3px;
    min-width: 28px;
    min-height: 28px;
    color: #a1a1a1;
    padding: 0;
    transition: all 40ms ease;
}
button.btn-add-tab:hover {
    background: #161616;
    border-color: #ededed;
    color: #ededed;
}
button.flat-icon {
    background: transparent;
    border: none;
    box-shadow: none;
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border-radius: 2px;
    color: #727272;
    transition: all 100ms ease;
}
button.flat-icon:hover {
    background: rgba(255, 255, 255, 0.06);
    color: #f4f4f4;
}
button.pill {
    background: #111111;
    border: 1px solid #1a1a1a;
    border-radius: 4px;
    padding: 7px 11px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
    color: #a1a1a1;
    transition: all 120ms ease;
}
button.pill:hover {
    background: #141414;
    border-color: #333333;
    color: #f4f4f4;
}
button.pill label {
    color: #a1a1a1;
    font-size: 12px;
    font-weight: 500;
}
button.pill:hover label {
    color: #f4f4f4;
}
.pill-icon {
    color: #727272;
}
button.pill:hover .pill-icon {
    color: #ededed;
}
.pill-chip {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 9.5px;
    font-weight: 600;
    color: #727272;
    background: #090909;
    border: 1px solid #1b1b1b;
    border-radius: 2px;
    padding: 1px 5px;
}
button.pill:hover .pill-chip {
    color: #ededed;
    border-color: rgba(255, 255, 255, 0.18);
}

/* ---- tab list & workspaces ---- */
.tablist, .tablist row {
    background: transparent;
}
.tablist row {
    padding: 0;
    margin: 2px 0;
    border-radius: 3px;
    border: 1px solid transparent;
    transition: all 100ms ease;
}
.tablist row:hover {
    background: #111111;
    border-color: #181818;
}
.tablist row:selected {
    background: transparent;
    border-color: transparent;
}
.workspace-header {
    padding: 6px 8px;
    margin: 8px 2px 3px 2px;
    border-radius: 3px;
    background: #0c0c0c;
    border: 1px solid #141414;
    transition: all 120ms ease;
}
.workspace-header:hover {
    background: #111111;
    border-color: #2a2a2a;
}
.workspace-header.active-workspace {
    background: #131313;
    border-color: #343434;
    border-left: 3px solid #ededed;
}
.workspace-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: #a1a1a1;
    text-transform: uppercase;
}
.workspace-header.active-workspace .workspace-title {
    color: #ededed;
}
.workspace-badge {
    font-size: 8.5px;
    font-weight: 700;
    color: #ededed;
    background: rgba(255, 255, 255, 0.07);
    border-radius: 1px;
    padding: 1px 4px;
    margin-right: 2px;
}
.workspace-tab-count {
    font-size: 10px;
    color: #535353;
    margin-right: 4px;
    opacity: 0;
    transition: opacity 120ms ease;
}
/* Tab soni va + tugmasi faqat sarlavha ustiga hover qilinganda ko'rinadi (kenglik o'zgarmaydi) */
.workspace-header:hover .workspace-tab-count {
    opacity: 1;
}
.btn-ws-add {
    min-width: 22px;
    min-height: 22px;
    padding: 0;
    border-radius: 2px;
    background: transparent;
    border: none;
    color: #727272;
    opacity: 0;
    transition: all 100ms ease;
}
.workspace-header:hover .btn-ws-add,
.btn-ws-add:focus {
    opacity: 1;
}
.btn-ws-add:hover {
    background: rgba(255, 255, 255, 0.07);
    color: #ededed;
}

.tabrow {
    padding: 6px 6px 6px 10px;
    margin: 2px 2px;
    border-radius: 3px;
    border: 1px solid transparent;
    transition: all 120ms ease;
}
.tabrow:hover {
    background: #0f0f0f;
    border-color: #181818;
}
.tabrow.active-tab {
    background: #141414;
    border-color: #2d2d2d;
    border-left: 3px solid #ededed;
}
.tabrow.active-tab .tab-title {
    color: #fafafa;
    font-weight: 600;
}
.tabrow.active-tab .tab-index {
    color: #ededed;
    background: rgba(255, 255, 255, 0.07);
    border-color: rgba(255, 255, 255, 0.18);
}
.tabrow.dragging {
    opacity: 0.88;
    background: #191919;
    border: 1px solid #ededed;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.65), 0 0 14px rgba(255, 255, 255, 0.05);
}
.tabrow.drop-target-above {
    border-top: 3px solid #ededed;
    background: rgba(255, 255, 255, 0.08);
    box-shadow: 0 -3px 10px rgba(255, 255, 255, 0.05);
}
.tabrow.drop-target-below {
    border-bottom: 3px solid #ededed;
    background: rgba(255, 255, 255, 0.08);
    box-shadow: 0 3px 10px rgba(255, 255, 255, 0.05);
}
.tab-index {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 10px;
    font-weight: 600;
    color: #535353;
    background: #090909;
    border: 1px solid #161616;
    border-radius: 2px;
    padding: 1px 5px;
    min-width: 14px;
}
.tab-title {
    font-size: 12.5px;
    font-weight: 500;
    color: #a1a1a1;
}
.tabrow image {
    color: #727272;
}
.tab-close {
    opacity: 0;
    min-width: 20px;
    min-height: 20px;
    padding: 0;
    border-radius: 2px;
    background: transparent;
    border: none;
    box-shadow: none;
    color: #727272;
    transition: all 120ms ease;
}
.tab-close:hover {
    background: rgba(244, 63, 94, 0.18);
    color: #fb7185;
    opacity: 1;
}
.tabrow:hover .tab-close, .tabrow.active-tab .tab-close {
    opacity: 0.7;
}
.btn-new-workspace {
    margin: 10px 2px 6px 2px;
    padding: 7px 8px;
    background: #0c0c0c;
    border: 1px dashed #1d1d1d;
    border-radius: 3px;
    color: #727272;
    font-size: 11px;
    font-weight: 600;
    transition: all 120ms ease;
}
.btn-new-workspace:hover {
    background: #131313;
    border-color: #ededed;
    color: #ededed;
}
.side-hint {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 9.5px;
    color: #535353;
    letter-spacing: -0.01em;
}

.fav, .tabrow label.fav, row:selected .tabrow label.fav {
    border-radius: 2px;
    font-size: 10px;
    font-weight: 800;
    color: #000000;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.fav-0 { background: #ededed; } .fav-1 { background: #c084fc; } .fav-2 { background: #4ade80; }
.fav-3 { background: #facc15; } .fav-4 { background: #f87171; } .fav-5 { background: #22d3ee; }
.fav-6 { background: #fb923c; } .fav-7 { background: #2dd4bf; }

/* ---- load progress bar ---- */
progressbar.load-bar { min-height: 2px; margin: 0; }
progressbar.load-bar trough { min-height: 2px; background: transparent; border: none; }
progressbar.load-bar progress {
    min-height: 2px;
    background: #ededed;
    border-radius: 0px;
    box-shadow: 0 0 10px rgba(255, 255, 255, 0.05);
}

/* ---- address & command palette (Spotlight / Raycast style) ---- */
.palette {
    background: #0c0c0c;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 14px 18px 12px;
    box-shadow: 0 24px 72px rgba(0, 0, 0, 0.85), 0 0 0 1px rgba(255, 255, 255, 0.05);
}
.palette image {
    color: #727272;
}
entry.palette-entry, entry.palette-entry:focus-within {
    background: none;
    border: none;
    box-shadow: none;
    outline: none;
    padding: 0;
    min-height: 0;
    font-size: 15px;
    font-weight: 500;
    color: #fafafa;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.palette-hint {
    color: #727272;
    font-size: 10.5px;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    margin-top: 6px;
}

/* ---- palette smart suggestions ---- */
.palette-suggestions-box {
    margin-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    padding-top: 6px;
    margin-bottom: 4px;
}
.palette-sug-item {
    padding: 7px 11px;
    border-radius: 3px;
    background: transparent;
    transition: background 100ms ease;
    border: none;
    box-shadow: none;
}
.palette-sug-item:hover, .palette-sug-item.selected {
    background: rgba(255, 255, 255, 0.07);
}
.palette-sug-icon {
    color: #727272;
    margin-right: 8px;
}
.palette-sug-item:hover .palette-sug-icon, .palette-sug-item.selected .palette-sug-icon {
    color: #ededed;
}
.palette-sug-title {
    color: #f4f4f4;
    font-size: 13px;
    font-weight: 500;
}
.palette-sug-url {
    color: #727272;
    font-size: 11.5px;
    margin-left: 8px;
}
.palette-sug-badge {
    color: #ededed;
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 2px;
    padding: 1px 5px;
    margin-left: 8px;
}

/* ---- profile colors (True Dark Multi-Persona) ---- */
.prof-bg-0 { background-color: #ededed; color: #050811 !important; }
.prof-bg-1 { background-color: #c084fc; color: #050811 !important; }
.prof-bg-2 { background-color: #4ade80; color: #050811 !important; }
.prof-bg-3 { background-color: #facc15; color: #050811 !important; }
.prof-bg-4 { background-color: #f87171; color: #050811 !important; }
.prof-bg-5 { background-color: #22d3ee; color: #050811 !important; }
.prof-bg-6 { background-color: #fb923c; color: #050811 !important; }
.prof-bg-7 { background-color: #2dd4bf; color: #050811 !important; }

/* ---- profile circle button in URL palette ---- */
button.profile-circle-btn {
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border-radius: 9999px;
    border: 2px solid rgba(255, 255, 255, 0.25);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    background-image: none;
    opacity: 1;
    transition: transform 120ms ease, border-color 120ms ease;
}
button.profile-circle-btn:hover {
    transform: scale(1.08);
    border-color: #ededed;
}
button.profile-circle-btn:backdrop,
button.profile-circle-btn:active,
button.profile-circle-btn:focus,
button.profile-circle-btn:disabled {
    opacity: 1;
    background-image: none;
}
button.profile-circle-btn label,
.profile-circle-label {
    opacity: 1;
    font-weight: 800;
    font-size: 11px;
    letter-spacing: -0.04em;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}

/* ---- profile popover dropdown ---- */
popover.profile-popover,
popover.profile-popover.background {
    background: none;
    background-color: transparent;
    border: none;
    box-shadow: none;
    padding: 0;
}
popover.profile-popover > contents {
    background: #0c0c0c;
    background-color: #0c0c0c;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 5px;
    padding: 6px 8px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.95);
    color: #ffffff;
}
popover.profile-popover > arrow {
    background: #0c0c0c;
    background-color: #0c0c0c;
    border-color: rgba(255, 255, 255, 0.14);
}
.profile-popover-title,
popover.profile-popover .profile-popover-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #a1a1a1;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    padding: 4px 8px 2px;
}
button.profile-item-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    border-radius: 3px;
    padding: 6px 9px;
    color: #ffffff;
    transition: background 100ms;
}
button.profile-item-btn:hover {
    background: #131313;
    color: #ffffff;
}
button.profile-item-btn.selected {
    background: #181818;
    border-left: 2px solid #ededed;
    color: #ffffff;
}
button.profile-item-btn {
    color: #ffffff;
}
.profile-item-dot,
button.profile-item-btn .profile-item-dot {
    min-width: 24px;
    min-height: 24px;
    border-radius: 9999px;
    font-size: 10px;
    letter-spacing: -0.03em;
    font-weight: 800;
    padding: 0 2px;
    opacity: 1;
}
.profile-item-name,
button.profile-item-btn .profile-item-name,
button.profile-item-btn label.profile-item-name {
    font-size: 12.5px;
    font-weight: 600;
    color: #f4f4f4;
}
.profile-item-persona,
button.profile-item-btn .profile-item-persona,
button.profile-item-btn label.profile-item-persona {
    font-size: 9.5px;
    color: #a1a1a1;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
button.profile-item-btn image {
    color: #a1a1a1;
}
button.profile-item-btn:hover image {
    color: #ededed;
}
.profile-item-check {
    font-size: 12px;
    font-weight: 700;
    color: #ededed;
}

/* ---- profiles manager modal (Alt+P) ---- */
.profiles-modal {
    background: #0c0c0c;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 24px 28px;
    box-shadow: 0 32px 84px rgba(0, 0, 0, 0.95), 0 0 0 1px rgba(255, 255, 255, 0.06);
    min-width: 440px;
    margin-left: 16px;
    margin-right: 16px;
}
.profiles-modal-title {
    font-size: 17px;
    font-weight: 700;
    color: #fafafa;
    letter-spacing: -0.02em;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.profiles-modal-subtitle {
    font-size: 11.5px;
    color: #727272;
    margin-top: 2px;
}
/* ---- profiles list in settings modal ---- */
.settings-profiles-list, .settings-profiles-list row {
    background: transparent;
    padding: 0;
    margin: 0;
    border: none;
    box-shadow: none;
}
.settings-profiles-list row:hover,
.settings-profiles-list row:selected {
    background: transparent;
    border-color: transparent;
}
.profile-card {
    background: #111111;
    border: 1px solid #191919;
    border-radius: 4px;
    padding: 10px 14px;
    margin-bottom: 6px;
    transition: transform 180ms cubic-bezier(0.2, 0, 0, 1), background-color 120ms ease, border-color 120ms ease;
}
.profile-card:hover {
    border-color: #333333;
    background: #121212;
}
.profile-card.active-profile {
    border-color: #ededed;
    background: #131313;
    box-shadow: 0 0 20px rgba(255, 255, 255, 0.05);
}
.profile-card.shift-up {
    transform: translateY(-60px);
}
.profile-card.shift-down {
    transform: translateY(60px);
}
.profile-card.dragging {
    opacity: 0.98;
    border-color: #ededed;
    background: #181818;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.8), 0 0 24px rgba(255, 255, 255, 0.05);
    transition: none;
}
.profile-drag-handle {
    font-size: 15px;
    font-weight: bold;
    color: #535353;
    padding: 0 4px 0 2px;
    transition: color 150ms ease, transform 150ms ease;
}
.profile-drag-handle:hover {
    color: #ededed;
    transform: scale(1.15);
}
.profile-avatar-circle {
    min-width: 30px;
    min-height: 30px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 800;
    margin: 0;
    padding: 0;
    opacity: 1;
}
.profile-card-name {
    font-size: 13.5px;
    font-weight: 600;
    color: #f4f4f4;
}
.profile-card-sub {
    font-size: 10.5px;
    color: #727272;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.profile-badge-active {
    font-size: 9.5px;
    font-weight: 700;
    border-radius: 9999px;
    padding: 2px 8px;
    background: rgba(255, 255, 255, 0.07);
    color: #ededed;
    border: 1px solid rgba(255, 255, 255, 0.21);
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
button.btn-make-active {
    background: #121212;
    color: #a1a1a1;
    border: 1px solid #2c2c2c;
    border-radius: 2px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    transition: all 120ms ease;
}
button.btn-make-active:hover {
    background: #ededed;
    color: #0c0c0c;
    border-color: #ededed;
}
button.btn-del-profile {
    background: transparent;
    border: none;
    border-radius: 2px;
    padding: 4px;
    color: #727272;
    opacity: 0;
    transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
}
button.btn-del-profile image {
    color: inherit;
}
.profile-card:hover button.btn-del-profile,
.settings-profiles-list row:hover button.btn-del-profile {
    opacity: 0.75;
}
button.btn-del-profile:hover {
    background: rgba(244, 63, 94, 0.18);
    color: #fb7185;
    opacity: 1;
}
.profile-section-label {
    font-size: 10.5px;
    font-weight: 700;
    color: #727272;
    letter-spacing: 0.05em;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    margin-top: 6px;
}
.profile-entry-box {
    background: #0e0e0e;
    border: 1px solid #1c1c1c;
    border-radius: 3px;
    padding: 8px 12px;
    color: #f4f4f4;
    font-size: 13px;
    transition: all 120ms ease;
}
.profile-entry-box:focus {
    border-color: #ededed;
    box-shadow: 0 0 0 1px #ededed;
}
.profile-entry-box.error {
    border-color: #f87171;
    box-shadow: 0 0 0 1px #f87171;
}
.profile-error-label {
    font-size: 11px;
    font-weight: 500;
    color: #f87171;
    margin-top: 2px;
}
button.btn-create-profile {
    background: #111111;
    border: 1px solid #1d1d1d;
    border-radius: 3px;
    padding: 7px 18px;
    transition: all 30ms ease;
}
button.btn-create-profile label {
    color: #727272;
    font-weight: 600;
    font-size: 12px;
}
button.btn-create-profile:disabled {
    background: #0e0e0e;
    border-color: #181818;
}
button.btn-create-profile.active-ready {
    background: #ededed;
    border-color: #ededed;
}
button.btn-create-profile.active-ready label {
    color: #0c0c0c;
    font-weight: 700;
}
button.btn-create-profile.active-ready:hover {
    background: #d4d4d4;
    border-color: #d4d4d4;
}

.profile-badge-mini {
    font-size: 8.5px;
    font-weight: 800;
    letter-spacing: -0.03em;
    min-width: 18px;
    min-height: 16px;
    border-radius: 9999px;
    margin-left: 2px;
    margin-right: 4px;
    padding: 0 3px;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    opacity: 1;
}
button.color-choice {
    min-width: 22px;
    min-height: 22px;
    padding: 0;
    border-radius: 9999px;
    border: 2px solid transparent;
    transition: transform 100ms ease, border-color 100ms ease;
}
button.color-choice:hover {
    transform: scale(1.15);
}
button.color-choice.selected {
    border-color: #ffffff;
    box-shadow: 0 0 0 2px #0c0c0c;
    transform: scale(1.15);
}

/* ---- JS script dialogs ---- */
.js-dialog-window {
    background: #0c0c0c;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 5px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.9);
}
.js-dialog-title {
    color: #ededed;
    font-size: 13px;
    font-weight: 700;
}
.js-dialog-message {
    color: #e7e7e7;
    font-size: 13px;
    line-height: 1.45;
}
.js-dialog-entry {
    background: #0f0f0f;
    color: #ffffff;
    border: 1px solid #2c2c2c;
    border-radius: 3px;
    padding: 7px 11px;
    font-size: 13px;
}
.js-dialog-entry:focus {
    border-color: #ededed;
    outline: none;
}
.js-dialog-btn-ok {
    background: #2b2b2b;
    color: #ffffff;
    border-radius: 3px;
    padding: 6px 18px;
    font-size: 12px;
    font-weight: 600;
    border: none;
}
.js-dialog-btn-ok:hover {
    background: #3d3d3d;
}
.js-dialog-btn-cancel {
    background: #181818;
    color: #a1a1a1;
    border-radius: 3px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border: none;
}
.js-dialog-btn-cancel:hover {
    background: #2e2e2e;
    color: #ffffff;
}

/* ---- data backup & migration modal (Alt+I) ---- */
.data-modal {
    background: #0c0c0c;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 24px 28px;
    box-shadow: 0 32px 84px rgba(0, 0, 0, 0.95), 0 0 0 1px rgba(255, 255, 255, 0.06);
    min-width: 520px;
    margin-left: 16px;
    margin-right: 16px;
}
.data-modal-title {
    font-size: 17px;
    font-weight: 700;
    color: #fafafa;
    letter-spacing: -0.02em;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.data-modal-subtitle {
    font-size: 11.5px;
    color: #727272;
    margin-top: 2px;
}
.data-section-box {
    background: #0f0f0f;
    border: 1px solid #191919;
    border-radius: 4px;
    padding: 12px 16px;
    margin-top: 8px;
}
.data-section-title {
    font-size: 12px;
    font-weight: 700;
    color: #ededed;
    letter-spacing: 0.02em;
}
.data-section-desc {
    font-size: 11px;
    color: #727272;
    margin-top: 2px;
    margin-bottom: 8px;
}
.data-info-card {
    background: #111111;
    border: 1px solid #1c1c1c;
    border-radius: 3px;
    padding: 8px 12px;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    color: #a1a1a1;
}
.btn-data-action {
    background: #141414;
    border: 1px solid #333333;
    border-radius: 3px;
    color: #f4f4f4;
    font-size: 12px;
    font-weight: 600;
    padding: 5px 12px;
    transition: all 120ms ease;
}
.btn-data-action:hover {
    background: #1d1d1d;
    border-color: #ededed;
    color: #ededed;
}
.btn-data-primary {
    background: #2b2b2b;
    border: 1px solid #ededed;
    border-radius: 3px;
    color: #ffffff;
    font-size: 12px;
    font-weight: 700;
    padding: 5px 14px;
    transition: all 120ms ease;
}
.btn-data-primary:hover {
    background: #3d3d3d;
    border-color: #d4d4d4;
}
.data-status-lbl {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11.5px;
    margin-top: 6px;
}

/* ---- html.css editor (Settings > Customize) ---- */
.css-editor {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 12px;
}
.css-editor text {
    background-color: #070707;
    color: #d4d4d4;
}
.css-editor-frame {
    border: 1px solid #191919;
    border-radius: 3px;
}

/* ---- floating settings modal (Alt+Shift+S) ---- */
.settings-modal {
    background: #0b0b0b;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    box-shadow: 0 32px 84px rgba(0, 0, 0, 0.95), 0 0 0 1px rgba(255, 255, 255, 0.06);
    min-width: 820px;
    min-height: 540px;
    margin-left: 20px;
    margin-right: 20px;
}
.settings-header {
    background: #0e0e0e;
    padding: 12px 18px;
    border-bottom: 1px solid #191919;
}
.settings-header-icon {
    color: #ededed;
    margin-right: 8px;
}
.settings-title {
    font-size: 16px;
    font-weight: 700;
    color: #fafafa;
    letter-spacing: -0.02em;
}
.settings-update-chip {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.02em;
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid #2f6bff;
    background: rgba(47, 107, 255, 0.18);
    color: #93b4ff;
    margin-left: 6px;
}
.settings-update-chip:hover {
    background: #2f6bff;
    color: #fff;
    border-color: #2f6bff;
}
.settings-badge {
    background: rgba(255, 255, 255, 0.07);
    color: #ededed;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 2px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    margin-left: 8px;
}
.settings-body {
    background: #0b0b0b;
}
.settings-sidebar {
    background: #090909;
    border-right: 1px solid #151515;
    padding: 12px 8px;
    min-width: 195px;
}
.settings-tab-btn {
    background: transparent;
    border: none;
    border-radius: 3px;
    color: #a1a1a1;
    padding: 9px 12px;
    margin-bottom: 4px;
    font-size: 12.5px;
    font-weight: 500;
    transition: all 120ms ease;
}
.settings-tab-btn:hover {
    background: #111111;
    color: #f4f4f4;
}
.settings-tab-btn.active {
    background: #171717;
    color: #ededed;
    font-weight: 600;
    box-shadow: inset 3px 0 0 0 #ededed;
}
.settings-content-page {
    padding: 18px 24px;
}
.settings-page-title {
    font-size: 16px;
    font-weight: 700;
    color: #fafafa;
    letter-spacing: -0.01em;
}
.settings-page-subtitle {
    font-size: 11.5px;
    color: #727272;
    margin-top: 2px;
    margin-bottom: 12px;
}
.settings-card {
    background: #111111;
    border: 1px solid #191919;
    border-radius: 4px;
    padding: 12px 16px;
    margin-bottom: 10px;
}
.settings-card-title {
    font-size: 12.5px;
    font-weight: 600;
    color: #e7e7e7;
}
.settings-card-desc {
    font-size: 11px;
    color: #727272;
    margin-top: 2px;
    margin-bottom: 6px;
}
.settings-option-item {
    background: #0c0c0c;
    border: 1px solid #191919;
    border-radius: 3px;
    padding: 6px 12px;
    margin-right: 6px;
    color: #d4d4d4;
    font-size: 12px;
    font-weight: 500;
    transition: all 120ms ease;
}
.settings-option-item:hover {
    background: #131313;
    border-color: #383838;
    color: #fafafa;
}
.settings-option-item.active {
    background: #1b1b1b;
    border-color: #ededed;
    color: #ededed;
    font-weight: 600;
}
.shortcut-row {
    padding: 6px 10px;
    border-bottom: 1px solid #131313;
}
.shortcut-row:last-child {
    border-bottom: none;
}
.shortcut-desc {
    font-size: 12px;
    color: #d4d4d4;
}
.shortcut-kbd {
    background: #161616;
    border: 1px solid #333333;
    border-radius: 2px;
    color: #ededed;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 7px;
}

/* ---- floating minimap / roadmap (Alt+M) ---- */
.minimap-window {
    background: #0b0b0b;
    border: 1px solid #1b1b1b;
    border-radius: 5px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.9), 0 0 0 1px rgba(255, 255, 255, 0.05);
}
.minimap-header {
    background: #0e0e0e;
    border-bottom: 1px solid #161616;
    padding: 7px 12px;
}
.minimap-header:active {
}
.minimap-header-icon {
    color: #ededed;
    margin-right: 4px;
}
.minimap-header-title {
    font-size: 11px;
    font-weight: 700;
    color: #fafafa;
    letter-spacing: 0.03em;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.minimap-header-title b {
    color: #ededed;
}
.minimap-badge {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 9px;
    font-weight: 700;
    color: #ededed;
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 2px;
    padding: 1px 5px;
    margin: 0 4px;
}
.minimap-header-hint {
    font-size: 9.5px;
    color: #727272;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
}
.minimap-canvas {
    border-radius: 0 0 4px 4px;
}
.minimap-canvas:active {
}
.minimap-location-btn {
    background: #101010;
    border: 1px solid #ededed;
    border-radius: 8px;
    padding: 3px 10px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.85);
    transition: all 120ms ease;
}
.minimap-location-btn image {
    color: #ededed;
    margin-right: 4px;
}
.minimap-location-btn label {
    font-size: 10px;
    font-weight: 700;
    color: #ededed;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    letter-spacing: 0.02em;
}
.minimap-location-btn:hover {
    background: #2b2b2b;
    border-color: #d4d4d4;
}
.minimap-location-btn:hover image,
.minimap-location-btn:hover label {
    color: #ffffff;
}

/* ---- Rail DevTools Panel (F12 / Alt+Shift+I / Alt+Shift+D) ---- */
.devtools-panel {
    background: #0b0b0b;
    box-shadow: 0 0 48px rgba(0, 0, 0, 0.85);
}
.devtools-panel.dock-bottom {
    border-top: 1px solid #181818;
    box-shadow: 0 -16px 48px rgba(0, 0, 0, 0.85);
}
.devtools-panel.dock-left {
    border-right: 1px solid #181818;
    box-shadow: 16px 0 48px rgba(0, 0, 0, 0.85);
}
.devtools-panel.dock-right {
    border-left: 1px solid #181818;
    box-shadow: -16px 0 48px rgba(0, 0, 0, 0.85);
}
.devtools-resize-handle {
    background: transparent;
    transition: background 0.15s ease;
}
.devtools-panel.dock-bottom .devtools-resize-handle {
    min-height: 4px;
}
.devtools-panel.dock-bottom .devtools-resize-handle:hover {
    background: rgba(255, 255, 255, 0.27);
}
.devtools-panel.dock-left .devtools-resize-handle,
.devtools-panel.dock-right .devtools-resize-handle {
    min-width: 4px;
}
.devtools-panel.dock-left .devtools-resize-handle:hover,
.devtools-panel.dock-right .devtools-resize-handle:hover {
    background: rgba(255, 255, 255, 0.27);
}
.devtools-window-box {
    background: #0b0b0b;
}
.devtools-header {
    background: #0e0e0e;
    border-bottom: 1px solid #161616;
    padding: 6px 14px;
}
.devtools-brand-icon {
    color: #ededed;
    margin-right: 6px;
}
.devtools-brand-label {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    font-weight: 700;
    color: #fafafa;
    margin-right: 14px;
}
.devtools-badge {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 9px;
    font-weight: 700;
    color: #ededed;
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 2px;
    padding: 1px 5px;
    margin-right: 12px;
}
.devtools-tab-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    padding: 3px 10px;
    color: #a1a1a1;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    font-weight: 600;
    transition: all 120ms ease;
}
.devtools-tab-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #fafafa;
}
.devtools-tab-btn.active {
    background: rgba(255, 255, 255, 0.08);
    color: #ededed;
    border-color: rgba(255, 255, 255, 0.18);
}
.devtools-log-area {
    background: #0b0b0b;
}
.devtools-log-box {
    padding: 6px 12px;
}
.devtools-log-row {
    padding: 3px 10px;
    border-radius: 2px;
    margin-bottom: 2px;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11.5px;
}
.devtools-log-row.level-in {
    background: rgba(255, 255, 255, 0.03);
    color: #a1a1a1;
    border-left: 2px solid #727272;
}
.devtools-log-row.level-out {
    background: rgba(74, 222, 128, 0.06);
    color: #4ade80;
    border-left: 2px solid #4ade80;
}
.devtools-log-row.level-err {
    background: rgba(248, 113, 113, 0.08);
    color: #f87171;
    border-left: 2px solid #f87171;
}
.devtools-log-row.level-info {
    background: rgba(255, 255, 255, 0.05);
    color: #ededed;
    border-left: 2px solid #ededed;
}
.devtools-input-bar {
    background: #0d0d0d;
    border-top: 1px solid #181818;
    padding: 5px 12px;
}
.devtools-prompt-char {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 13px;
    font-weight: 800;
    color: #ededed;
    margin-right: 8px;
}
.devtools-entry {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #fafafa;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 12px;
}
.devtools-text-view {
    background: #0b0b0b;
    color: #d4d4d4;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11.5px;
    padding: 12px;
}
.devtools-card {
    background: #111111;
    border: 1px solid #191919;
    border-radius: 3px;
    padding: 12px 16px;
    margin: 8px 12px;
}
.devtools-card-title {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    font-weight: 700;
    color: #ededed;
    margin-bottom: 6px;
}
.devtools-card-row {
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'DejaVu Sans Mono', monospace;
    font-size: 11px;
    color: #a1a1a1;
    padding: 2px 0;
}
.devtools-card-row b {
    color: #fafafa;
}
/* ---- dev style: neutral black/white with a thin blue accent ---- */
selection {
    background: rgba(255, 255, 255, 0.24);
    color: #ffffff;
}
.tab.focused {
    border-color: #ededed;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.10);
}
.zoomed .tab.focused {
    border-color: #ededed;
    outline: 1px solid #ededed;
    box-shadow: none;
}
.tab-resize-handle:hover,
.tab-resize-handle.active {
    background: #ededed;
    box-shadow: none;
}

/* ---- update card ---- */
.update-card {
    background: #111111;
    border: 1px solid #2e2e2e;
    border-radius: 4px;
    padding: 12px 14px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.8);
}
.update-kicker {
    font-size: 10px;
    letter-spacing: 1.5px;
    color: #8a8a8a;
    font-weight: 700;
}
.update-ver {
    font-size: 20px;
    font-weight: 700;
    color: #fafafa;
}
.update-sub {
    font-size: 11.5px;
    color: #a3a3a3;
}
.update-err {
    font-size: 11px;
    color: #f87171;
}
button.update-get {
    background: #ffffff;
    background-image: none;
    color: #000000;
    border: 1px solid #ffffff;
    border-radius: 3px;
    padding: 5px 20px;
    font-weight: 700;
    box-shadow: none;
}
button.update-get label {
    color: #000000;
}
button.update-get:hover {
    background: #f0f0f0;
}
button.update-get:hover label {
    color: #000000;
}
button.update-get:disabled {
    opacity: 0.6;
}
button.update-close {
    background: transparent;
    background-image: none;
    border: none;
    box-shadow: none;
    color: #8a8a8a;
    opacity: 0.35;
    min-height: 0;
    min-width: 0;
    padding: 0 6px;
}
button.update-close:hover {
    opacity: 0.9;
    background: rgba(255, 255, 255, 0.08);
}
";

/// How far the strip zooms out in overview (niri uses a similar factor).
const OV_ZOOM: f64 = 0.4;

/// Fon tab shuncha soniyadan keyin RAM tejash uchun to'xtatiladi (suspend)
const SUSPEND_AFTER_SECS: u64 = 600; // 10 daqiqa fon tab uxlaydi

struct Tab {
    container: gtk::Box,
    view: WebView,
    /// Oxirgi kadr (freeze) — uxlaganda skrinshot o'rnida turadi
    freeze: gtk::Picture,
    profile_id: String,
    custom_width: Cell<Option<f64>>,
    /// Oxirgi ma'lum http(s) manzil. WebKit web process to'xtatilganda uri() ni tozalab yuboradi,
    /// shuning uchun uyg'otish va sessiyani saqlash shu nusxadan foydalanadi.
    saved_uri: RefCell<String>,
    suspended: Cell<bool>,
    /// Popup (window.open) bilan bog'langan tablar bitta web process'ni bo'lishadi: ularni to'xtatib bo'lmaydi
    keep_alive: Cell<bool>,
    last_active: Cell<std::time::Instant>,
}

impl Tab {
    /// Haqiqiy manzil: jonli uri bo'sh bo'lsa (to'xtatilgan tab) saqlangan nusxa
    fn real_uri(&self) -> String {
        let live = self.view.uri().map(|s| s.to_string()).unwrap_or_default();
        if live.is_empty() { self.saved_uri.borrow().clone() } else { live }
    }
}

struct Row {
    tabs: Vec<Tab>,
}

struct MinimapState {
    cam_x: f64,
    cam_y: f64,
    zoom: f64,
    target_zoom: f64,
    offset_x: f64,
    offset_y: f64,
    target_offset_x: f64,
    target_offset_y: f64,
    user_panned: bool,
    drag_start_offset_x: f64,
    drag_start_offset_y: f64,
    vel_x: f64,
    vel_y: f64,
    cursor_x: f64,
    cursor_y: f64,
    cursor_w: f64,
    cursor_h: f64,
    cursor_initialized: bool,
    animating: bool,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            target_zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            target_offset_x: 0.0,
            target_offset_y: 0.0,
            user_panned: false,
            drag_start_offset_x: 0.0,
            drag_start_offset_y: 0.0,
            vel_x: 0.0,
            vel_y: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_w: MAP_TAB_W,
            cursor_h: MAP_TAB_H,
            cursor_initialized: false,
            animating: false,
        }
    }
}

const MAP_TAB_W: f64 = 28.0;
const MAP_TAB_W_FULL: f64 = 46.0;
const MAP_TAB_H: f64 = 18.0;
const MAP_GAP_X: f64 = 6.0;
const MAP_GAP_Y: f64 = 6.0;

struct App {
    win: gtk::ApplicationWindow,
    shell: gtk::Box,
    shell_layout: ShellLayout,
    side: gtk::Box,
    strip: gtk::Box,
    layout: StripLayout,
    list: gtk::ListBox,
    pill_label: gtk::Label,
    load_bar: gtk::ProgressBar,
    palette: gtk::Box,
    entry: gtk::Entry,
    palette_for_new_tab: Cell<bool>,
    palette_suggestions_box: gtk::Box,
    history: RefCell<HistoryManager>,
    current_suggestions: RefCell<Vec<HistorySuggestion>>,
    selected_suggestion_idx: Cell<Option<usize>>,
    is_autofilling: Cell<bool>,
    last_entry_len: Cell<usize>,
    /// Oxirgi bosilgan tugma o'chirish (Backspace/Delete/Ctrl+X...) bo'lganmi: shunda inline to'ldirish ishlamaydi
    delete_key: Cell<bool>,
    ov_hint: gtk::Label,
    /// Overview is active: the strip is zoomed out and tabs ignore the pointer.
    ov: Cell<bool>,
    /// Tab coordinates (row, col) that were focused when overview opened; Esc returns here.
    ov_return: Cell<(usize, usize)>,
    /// Scroll position before overview, restored when zooming back in.
    ov_saved_x: Cell<f64>,
    ov_saved_y: Cell<f64>,
    target_zoom: Cell<f64>,
    rows: RefCell<Vec<Row>>,
    focus_row: Cell<usize>,
    focus_col: Cell<usize>,
    target_x: Cell<f64>,
    target_y: Cell<f64>,
    animating: Cell<bool>,
    /// Touchpad slide: at most one strip allocate per frame (avoids jank)
    pan_alloc_pending: Cell<bool>,
    side_open: Cell<bool>,
    side_gen: Cell<u32>,
    /// A just-created tab that has never navigated; Esc discards it.
    fresh: RefCell<Option<WebView>>,
    // Profiles
    profiles: RefCell<ProfileManager>,
    selected_palette_profile: RefCell<String>,
    profile_btn: gtk::Button,
    profile_btn_lbl: gtk::Label,
    profile_popover: gtk::Popover,
    profiles_modal_scroll: gtk::ScrolledWindow,
    profiles_list_box: gtk::ListBox,
    new_profile_entry: gtk::Entry,
    new_profile_error_lbl: gtk::Label,
    add_prof_btn: gtk::Button,
    new_profile_color_idx: Cell<usize>,
    color_buttons: RefCell<Vec<gtk::Button>>,
    touchpad_mode: Cell<bool>,
    tp_switch: gtk::Switch,
    touchpad_toast: gtk::Label,
    last_saved_session: RefCell<String>,
    save_session_gen: Cell<u64>,
    data_modal_info_lbl: gtk::Label,
    data_modal_status_lbl: gtk::Label,
    data_export_entry: gtk::Entry,
    data_import_entry: gtk::Entry,
    sync_url_result_entry: gtk::Entry,
    sync_restore_entry: gtk::Entry,
    // Minimap / Roadmap (Alt+M)
    map_container: gtk::Box,
    map_drawing_area: gtk::DrawingArea,
    map_location_btn: gtk::Button,
    map_state: RefCell<MinimapState>,
    // DevTools Panel (F12 / Alt+Shift+I / Alt+Shift+D)
    devtools_open: Cell<bool>,
    devtools_dock_side: Cell<DevToolsDockSide>,
    devtools_panel: gtk::Box,
    devtools_content_box: gtk::Box,
    devtools_resize_handle: gtk::Box,
    devtools_url_lbl: gtk::Label,
    devtools_window: RefCell<Option<gtk::Window>>,
    // Floating Settings Panel (Alt+Shift+S / Alt+P / Alt+I)
    settings_modal: gtk::Box,
    settings_stack: gtk::Stack,
    settings_tab_buttons: RefCell<Vec<(SettingsTab, gtk::Button)>>,
    current_settings_tab: Cell<SettingsTab>,
    css_buffer: gtk::TextBuffer,
    search_engine: RefCell<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DevToolsDockSide {
    Bottom,
    Left,
    Right,
    Undocked,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Profiles,
    Data,
    Privacy,
    Customize,
    Update,
    About,
}

// ---------------------------------------------------------------------
// Customizable keyboard shortcuts (Settings > Customize > Shortcuts)
// ---------------------------------------------------------------------

struct ShortcutDef {
    id: &'static str,
    group: &'static str,
    desc: &'static str,
    defaults: &'static [&'static str],
}

macro_rules! sc {
    ($id:expr, $group:expr, $desc:expr, [$($d:expr),*]) => {
        ShortcutDef { id: $id, group: $group, desc: $desc, defaults: &[$($d),*] }
    };
}

const SC_LOCKED: &str = "settings";

const G_TABS: &str = "Tabs & Workspaces";
const G_PANELS: &str = "Panels, Settings & Tools";

const SHORTCUT_DEFS: &[ShortcutDef] = &[
    sc!("new_tab", G_TABS, "Open Omnibar / New tab", ["Alt+T"]),
    sc!("focus_url", G_TABS, "Focus URL / Address bar", ["Alt+G"]),
    sc!("close_tab", G_TABS, "Close focused tab", ["Alt+W"]),
    sc!("reload", G_TABS, "Reload active tab", ["Alt+R"]),
    sc!("full_tab", G_TABS, "Toggle Full Tab View", ["Alt+F"]),
    sc!("split_v", G_TABS, "Split column vertically", ["Alt+N"]),
    sc!("new_workspace", G_TABS, "New Workspace", ["Alt+Shift+V"]),
    sc!("focus_left", G_TABS, "Focus column Left", ["Alt+H"]),
    sc!("focus_right", G_TABS, "Focus column Right", ["Alt+L"]),
    sc!("focus_down", G_TABS, "Focus row Down", ["Alt+J"]),
    sc!("focus_up", G_TABS, "Focus row Up", ["Alt+K"]),
    sc!("move_left", G_TABS, "Move tab column Left", ["Alt+Shift+H"]),
    sc!("move_right", G_TABS, "Move tab column Right", ["Alt+Shift+L"]),
    sc!("move_down", G_TABS, "Move tab row Down", ["Alt+Shift+J"]),
    sc!("move_up", G_TABS, "Move tab row Up", ["Alt+Shift+K"]),
    sc!("tab_1", G_TABS, "Switch to tab 1", ["Ctrl+1"]),
    sc!("tab_2", G_TABS, "Switch to tab 2", ["Ctrl+2"]),
    sc!("tab_3", G_TABS, "Switch to tab 3", ["Ctrl+3"]),
    sc!("tab_4", G_TABS, "Switch to tab 4", ["Ctrl+4"]),
    sc!("tab_5", G_TABS, "Switch to tab 5", ["Ctrl+5"]),
    sc!("tab_6", G_TABS, "Switch to tab 6", ["Ctrl+6"]),
    sc!("tab_7", G_TABS, "Switch to tab 7", ["Ctrl+7"]),
    sc!("tab_8", G_TABS, "Switch to tab 8", ["Ctrl+8"]),
    sc!("tab_9", G_TABS, "Switch to tab 9 (or last tab)", ["Ctrl+9"]),
    sc!("ws_1", G_TABS, "Switch to workspace 1", ["Alt+1"]),
    sc!("ws_2", G_TABS, "Switch to workspace 2", ["Alt+2"]),
    sc!("ws_3", G_TABS, "Switch to workspace 3", ["Alt+3"]),
    sc!("ws_4", G_TABS, "Switch to workspace 4", ["Alt+4"]),
    sc!("ws_5", G_TABS, "Switch to workspace 5", ["Alt+5"]),
    sc!("ws_6", G_TABS, "Switch to workspace 6", ["Alt+6"]),
    sc!("ws_7", G_TABS, "Switch to workspace 7", ["Alt+7"]),
    sc!("ws_8", G_TABS, "Switch to workspace 8", ["Alt+8"]),
    sc!("ws_9", G_TABS, "Switch to workspace 9", ["Alt+9"]),
    sc!("mv_1", G_TABS, "Move tab to workspace 1", ["Alt+Shift+1"]),
    sc!("mv_2", G_TABS, "Move tab to workspace 2", ["Alt+Shift+2"]),
    sc!("mv_3", G_TABS, "Move tab to workspace 3", ["Alt+Shift+3"]),
    sc!("mv_4", G_TABS, "Move tab to workspace 4", ["Alt+Shift+4"]),
    sc!("mv_5", G_TABS, "Move tab to workspace 5", ["Alt+Shift+5"]),
    sc!("mv_6", G_TABS, "Move tab to workspace 6", ["Alt+Shift+6"]),
    sc!("mv_7", G_TABS, "Move tab to workspace 7", ["Alt+Shift+7"]),
    sc!("mv_8", G_TABS, "Move tab to workspace 8", ["Alt+Shift+8"]),
    sc!("mv_9", G_TABS, "Move tab to workspace 9", ["Alt+Shift+9"]),
    sc!("settings", G_PANELS, "Toggle Settings panel", ["Alt+S"]),
    sc!("profiles", G_PANELS, "Profiles / profile switcher", ["Alt+P"]),
    sc!("data", G_PANELS, "Data & Backup", ["Alt+I"]),
    sc!("devtools", G_PANELS, "Toggle DevTools", ["F12"]),
    sc!("devtools_dock", G_PANELS, "Cycle DevTools dock position", ["Alt+Shift+D"]),
    sc!("map", G_PANELS, "Toggle Roadmap / Minimap", ["Alt+M"]),
    sc!("overview", G_PANELS, "Toggle Spatial Overview", ["Alt+O"]),
    sc!("sidebar", G_PANELS, "Toggle Sidebar", ["Alt+B"]),
    sc!("touchpad", G_PANELS, "Toggle Touchpad Canvas Navigation", ["Alt+Shift+T"]),
    sc!("back", G_PANELS, "Browser Back", ["Alt+Left"]),
    sc!("forward", G_PANELS, "Browser Forward", ["Alt+Right"]),
    sc!("fullscreen", G_PANELS, "Toggle Fullscreen", ["F11"]),
    sc!("quit", G_PANELS, "Quit Rail", ["Alt+Q"]),
];

#[derive(Clone, Default)]
struct ShortcutConfig {
    overrides: std::collections::HashMap<String, Vec<String>>,
    disabled: std::collections::HashSet<String>,
    site_wins: bool,
}

enum ScResolve {
    Action(String),
    Disabled,
    Unknown,
}

thread_local! {
    static SHORTCUTS: RefCell<ShortcutConfig> = RefCell::new(ShortcutConfig::default());
    static KEY_CTL: RefCell<Option<gtk::EventControllerKey>> = RefCell::new(None);
    /// Re-entrancy guard: Inspector::show() emits bring-to-front, which called show_devtools() again (stack overflow).
    static IN_SHOW_DEVTOOLS: Cell<bool> = Cell::new(false);
    /// The view whose inspector is currently shown in the Rail panel/window (may differ from the focused view after Ctrl+Tab).
    static DT_VIEW: RefCell<Option<WebView>> = RefCell::new(None);
    /// True while Rail itself closes the undocked DevTools window (so close-request must not tear DevTools down).
    static DT_CLOSING_WIN: Cell<bool> = Cell::new(false);
    /// Toolbar slot (0-based, 24px pitch) last clicked in the inspector toolbar; -1 = none.
    static LAST_DT_SLOT: Cell<i32> = Cell::new(-1);
}

fn shortcuts_path() -> PathBuf {
    customize_dir().join("shortcuts-customize.json")
}

/// Re-reads everything Customize stores on disk (used after a Data & Backup import).
fn reload_customize_from_disk() {
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    apply_shortcut_priority();
    apply_html_css(&load_html_css());
}

fn load_shortcuts_config() -> ShortcutConfig {
    let mut cfg = ShortcutConfig::default();
    let Ok(text) = std::fs::read_to_string(shortcuts_path()) else {
        return cfg;
    };
    let Some(root) = minijson::parse(&text) else {
        return cfg;
    };
    cfg.site_wins = root.get("priority").and_then(|v| v.as_str()) == Some("site");
    if let Some(minijson::Json::Obj(items)) = root.get("shortcuts") {
        for (id, v) in items {
            let Some(def) = SHORTCUT_DEFS.iter().find(|d| d.id == id.as_str()) else {
                continue;
            };
            if v.get("enabled").and_then(|b| b.as_bool()) == Some(false) {
                cfg.disabled.insert(def.id.to_string());
            }
            if let Some(arr) = v.get("keys").and_then(|k| k.as_arr()) {
                let list: Vec<String> = arr
                    .iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|c| sc_valid_combo(c))
                    .collect();
                cfg.overrides.insert(def.id.to_string(), list);
            }
        }
    }
    sc_normalize(&mut cfg);
    cfg
}

fn save_shortcuts_config(cfg: &ShortcutConfig) -> bool {
    let mut out = String::from("{\n  \"version\": 1,\n");
    out.push_str(&format!(
        "  \"priority\": \"{}\",\n",
        if cfg.site_wins { "site" } else { "rail" }
    ));
    out.push_str("  \"shortcuts\": {\n");
    let total = SHORTCUT_DEFS.len();
    for (i, def) in SHORTCUT_DEFS.iter().enumerate() {
        let keys: Vec<String> = sc_effective(def, cfg)
            .iter()
            .map(|k| format!("\"{}\"", minijson::escape(k)))
            .collect();
        out.push_str(&format!(
            "    \"{}\": {{ \"description\": \"{}\", \"enabled\": {}, \"keys\": [{}] }}{}\n",
            def.id,
            minijson::escape(def.desc),
            sc_enabled(cfg, def.id),
            keys.join(", "),
            if i + 1 < total { "," } else { "" }
        ));
    }
    out.push_str("  }\n}\n");
    let path = shortcuts_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out).is_ok()
}

fn sc_effective(def: &ShortcutDef, cfg: &ShortcutConfig) -> Vec<String> {
    match cfg.overrides.get(def.id) {
        Some(list) => list.clone(),
        None => def.defaults.iter().map(|s| s.to_string()).collect(),
    }
}

fn sc_enabled(cfg: &ShortcutConfig, id: &str) -> bool {
    id == SC_LOCKED || !cfg.disabled.contains(id)
}

fn sc_normalize(cfg: &mut ShortcutConfig) {
    cfg.disabled.remove(SC_LOCKED);
    cfg.disabled
        .retain(|id| SHORTCUT_DEFS.iter().any(|d| d.id == id.as_str()));
    for def in SHORTCUT_DEFS {
        let is_default = cfg
            .overrides
            .get(def.id)
            .map(|l| l.iter().map(|s| s.as_str()).eq(def.defaults.iter().copied()))
            .unwrap_or(false);
        if is_default {
            cfg.overrides.remove(def.id);
        }
    }
    // The Settings shortcut can never end up unbound.
    if cfg.overrides.get(SC_LOCKED).map(|l| l.is_empty()).unwrap_or(false) {
        cfg.overrides.remove(SC_LOCKED);
    }
}

/// Puts `combo` into slot `slot` of `def`. Takes it away from any other shortcut that
/// used it. Returns a short note about what moved, or why it was refused.
fn sc_set_slot(
    cfg: &mut ShortcutConfig,
    def: &ShortcutDef,
    slot: usize,
    combo: &str,
) -> Result<String, String> {
    if !sc_valid_combo(combo) {
        return Err(tr("Pick Alt or Ctrl with a key (F-keys can be used alone).").to_string());
    }
    if def.id != SC_LOCKED {
        if let Some(locked) = SHORTCUT_DEFS.iter().find(|d| d.id == SC_LOCKED) {
            if sc_effective(locked, cfg).iter().any(|c| c == combo) {
                return Err(trf("{} is reserved for opening Settings.", &[&combo]));
            }
        }
    }
    let mut note = String::new();
    for other in SHORTCUT_DEFS {
        if other.id == def.id {
            continue;
        }
        let eff = sc_effective(other, cfg);
        if eff.iter().any(|c| c == combo) {
            let rest: Vec<String> = eff.into_iter().filter(|c| c != combo).collect();
            cfg.overrides.insert(other.id.to_string(), rest);
            note = trf("{} was taken from \"{}\".", &[&combo, tr(other.desc)]);
        }
    }
    let mut list = sc_effective(def, cfg);
    if slot < list.len() {
        list[slot] = combo.to_string();
    } else {
        list.push(combo.to_string());
    }
    let mut seen: Vec<String> = Vec::new();
    list.retain(|c| {
        if seen.contains(c) {
            false
        } else {
            seen.push(c.clone());
            true
        }
    });
    cfg.overrides.insert(def.id.to_string(), list);
    Ok(note)
}

fn sc_valid_combo(c: &str) -> bool {
    let parts: Vec<&str> = c.split('+').collect();
    let has_mod = parts.iter().any(|p| matches!(*p, "Ctrl" | "Alt" | "Super"));
    let last = parts.last().copied().unwrap_or("");
    let is_fkey = last.len() >= 2
        && last.len() <= 3
        && last.starts_with('F')
        && last[1..].chars().all(|ch| ch.is_ascii_digit());
    has_mod || is_fkey
}

fn is_modifier_key(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::Hyper_L
            | gdk::Key::Hyper_R
            | gdk::Key::Caps_Lock
            | gdk::Key::Num_Lock
            | gdk::Key::ISO_Level3_Shift
    )
}

/// Layout-independent name of a key press, e.g. "Alt+Shift+T" or "F12".
fn shortcut_combo(key: gdk::Key, keycode: u32, state: gdk::ModifierType) -> Option<String> {
    use gdk::ModifierType as M;
    if is_modifier_key(key) {
        return None;
    }
    let mut label: Option<String> = None;
    if let Some(display) = gdk::Display::default() {
        if let Some(entries) = display.map_keycode(keycode) {
            for (kk, k) in entries.iter() {
                if kk.level() != 0 {
                    continue;
                }
                if let Some(c) = k.to_unicode() {
                    if c.is_ascii_graphic() {
                        label = Some(if c == '+' {
                            "plus".to_string()
                        } else {
                            c.to_ascii_uppercase().to_string()
                        });
                        break;
                    }
                }
            }
            if label.is_none() {
                for (kk, k) in entries.iter() {
                    if kk.level() == 0 {
                        if let Some(n) = k.name() {
                            label = Some(n.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }
    let label = match label {
        Some(l) => l,
        None => key.to_lower().name()?.to_string(),
    };
    let mut out = String::new();
    if state.contains(M::CONTROL_MASK) {
        out.push_str("Ctrl+");
    }
    if state.contains(M::ALT_MASK) {
        out.push_str("Alt+");
    }
    if state.contains(M::SHIFT_MASK) {
        out.push_str("Shift+");
    }
    if state.contains(M::SUPER_MASK) {
        out.push_str("Super+");
    }
    out.push_str(&label);
    Some(out)
}

/// Turns a default combo string back into a key event Rail's key handler understands.
fn sc_synth(combo: &str) -> Option<(gdk::Key, gdk::ModifierType)> {
    use gdk::ModifierType as M;
    let mut mods = M::empty();
    let mut name = "";
    for part in combo.split('+') {
        match part {
            "Ctrl" => mods |= M::CONTROL_MASK,
            "Alt" => mods |= M::ALT_MASK,
            "Shift" => mods |= M::SHIFT_MASK,
            "Super" => mods |= M::SUPER_MASK,
            other => name = other,
        }
    }
    let key = if name.chars().count() == 1 {
        gdk::Key::from_name(name.to_lowercase())?
    } else {
        gdk::Key::from_name(name)?
    };
    Some((key, mods))
}

fn sc_resolve(combo: &str) -> ScResolve {
    SHORTCUTS.with(|c| {
        let cfg = c.borrow();
        for def in SHORTCUT_DEFS {
            if sc_effective(def, &cfg).iter().any(|x| x == combo) {
                if !sc_enabled(&cfg, def.id) {
                    return ScResolve::Disabled;
                }
                return ScResolve::Action(def.defaults[0].to_string());
            }
        }
        for def in SHORTCUT_DEFS {
            if def.defaults.contains(&combo) {
                return ScResolve::Disabled;
            }
        }
        ScResolve::Unknown
    })
}

/// Rail intercepts keys in capture phase, respecting each shortcut's priority (Rail vs Website).
fn apply_shortcut_priority() {
    KEY_CTL.with(|k| {
        if let Some(ctl) = k.borrow().as_ref() {
            ctl.set_propagation_phase(gtk::PropagationPhase::Capture);
        }
    });
}

#[derive(Clone)]
struct ScUi {
    rows: gtk::Box,
    pending: Rc<RefCell<ShortcutConfig>>,
    status: gtk::Label,
    prio: gtk::CheckButton,
    is_syncing: Rc<Cell<bool>>,
}

const SC_MODS: &[&str] = &["Alt", "Ctrl", "Alt+Shift", "Ctrl+Shift", "None"];
const SC_KEY_ROWS: &[&str] = &["`1234567890-=", "QWERTYUIOP[]", "ASDFGHJKL;'", "ZXCVBNM,./\\"];

fn sc_key_label(v: &str) -> String {
    match v {
        "" => tr("Pick key").to_string(),
        "Left" => "\u{2190}".to_string(),
        "Right" => "\u{2192}".to_string(),
        "Up" => "\u{2191}".to_string(),
        "Down" => "\u{2193}".to_string(),
        other => other.to_string(),
    }
}

/// "Alt+Shift+T" -> (index into SC_MODS, "T")
fn sc_split(combo: &str) -> (u32, String) {
    let mut parts: Vec<&str> = combo.split('+').collect();
    let key = parts.pop().unwrap_or("").to_string();
    let ctrl = parts.contains(&"Ctrl");
    let alt = parts.contains(&"Alt");
    let shift = parts.contains(&"Shift");
    let idx = if ctrl {
        if shift { 3 } else { 1 }
    } else if alt {
        if shift { 2 } else { 0 }
    } else {
        4
    };
    (idx, key)
}

fn sc_join(mod_idx: u32, key: &str) -> String {
    let prefix = match mod_idx {
        0 => "Alt+",
        1 => "Ctrl+",
        2 => "Alt+Shift+",
        3 => "Ctrl+Shift+",
        _ => "",
    };
    format!("{}{}", prefix, key)
}

/// On-screen keyboard: the user taps a key instead of pressing it.
/// What it offers follows the modifier dropdown: "None" only makes sense for F1-F12
/// (a bare letter would steal normal typing), everything else gets the full keyboard.
fn sc_build_picker(sel: Rc<Cell<u32>>, on_pick: Rc<dyn Fn(String)>) -> gtk::Popover {
    let pop = gtk::Popover::new();
    let grid = gtk::Grid::new();
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    grid.set_margin_top(8);
    grid.set_margin_bottom(8);
    grid.set_margin_start(8);
    grid.set_margin_end(8);
    pop.set_child(Some(&grid));

    let fill = {
        let grid = grid.clone();
        let weak = pop.downgrade();
        move |only_f: bool| {
            while let Some(child) = grid.first_child() {
                grid.remove(&child);
            }
            let make = |label: &str, value: &str| -> gtk::Button {
                let b = gtk::Button::with_label(label);
                b.add_css_class("settings-option-item");
                b.set_size_request(if only_f { 52 } else { 34 }, 30);
                let v = value.to_string();
                let cb = on_pick.clone();
                let w = weak.clone();
                b.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.popdown();
                    }
                    cb(v.clone());
                });
                b
            };
            if only_f {
                for n in 1..=12i32 {
                    let s = format!("F{}", n);
                    grid.attach(&make(&s, &s), (n - 1) % 6, (n - 1) / 6, 1, 1);
                }
                return;
            }
            for (r, chars) in SC_KEY_ROWS.iter().enumerate() {
                for (c, ch) in chars.chars().enumerate() {
                    let s = ch.to_string();
                    grid.attach(&make(&s, &s), c as i32, r as i32, 1, 1);
                }
            }
            for n in 1..=12 {
                let s = format!("F{}", n);
                grid.attach(&make(&s, &s), n - 1, 4, 1, 1);
            }
            let arrows = [
                ("\u{2190}", "Left"),
                ("\u{2191}", "Up"),
                ("\u{2193}", "Down"),
                ("\u{2192}", "Right"),
            ];
            for (i, (l, v)) in arrows.iter().enumerate() {
                grid.attach(&make(l, v), i as i32, 5, 1, 1);
            }
        }
    };
    fill(sel.get() == 4);
    pop.connect_show(move |_| fill(sel.get() == 4));
    pop
}

fn sc_commit(ui: &ScUi) {
    let cfg = {
        let mut p = ui.pending.borrow_mut();
        sc_normalize(&mut p);
        p.clone()
    };
    if save_shortcuts_config(&cfg) {
        SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
        apply_shortcut_priority();
    } else {
        ui.status.set_text(tr("Could not write shortcuts-customize.json"));
    }
}

/// Rebuilds the rows after the current event finishes, keeping the scroll position.
fn sc_refresh(ui: &ScUi) {
    let ui = ui.clone();
    glib::idle_add_local_once(move || {
        let adj = ui
            .rows
            .ancestor(gtk::ScrolledWindow::static_type())
            .and_then(|w| w.downcast::<gtk::ScrolledWindow>().ok())
            .map(|sw| sw.vadjustment());
        let pos = adj.as_ref().map(|a| a.value());
        sc_build(&ui);
        if let (Some(adj), Some(pos)) = (adj, pos) {
            glib::idle_add_local_once(move || adj.set_value(pos));
        }
    });
}

fn sc_apply_slot(ui: &ScUi, def: &'static ShortcutDef, slot: usize, combo: String) {
    let res = sc_set_slot(&mut ui.pending.borrow_mut(), def, slot, &combo);
    match res {
        Ok(note) => {
            sc_commit(ui);
            ui.status.set_text(&note);
        }
        Err(e) => ui.status.set_text(&e),
    }
    sc_refresh(ui);
}

fn sc_build(ui: &ScUi) {
    let container = &ui.rows;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let cfg = ui.pending.borrow().clone();
    let mut last_group = "";
    for def in SHORTCUT_DEFS {
        if def.group != last_group {
            last_group = def.group;
            let h = gtk::Label::new(Some(tr(def.group)));
            h.add_css_class("settings-card-title");
            h.set_xalign(0.0);
            h.set_margin_top(10);
            h.set_margin_bottom(4);
            container.append(&h);
        }
        let enabled = sc_enabled(&cfg, def.id);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("shortcut-row");
        row.set_margin_top(3);
        row.set_margin_bottom(3);

        let d = gtk::Label::new(Some(tr(def.desc)));
        d.add_css_class("shortcut-desc");
        d.set_xalign(0.0);
        d.set_hexpand(true);
        d.set_wrap(true);
        d.set_valign(gtk::Align::Center);
        d.set_opacity(if enabled { 1.0 } else { 0.45 });
        row.append(&d);

        if def.id == SC_LOCKED {
            let lock = gtk::Image::from_icon_name("changes-prevent-symbolic");
            lock.set_valign(gtk::Align::Center);
            lock.set_tooltip_text(Some(
                tr("Always on: this opens Settings, so it can't be turned off."),
            ));
            row.append(&lock);
        } else {
            let sw = gtk::Switch::new();
            sw.set_active(enabled);
            sw.set_valign(gtk::Align::Center);
            sw.set_tooltip_text(Some(tr("Rail shortcuts win over website shortcuts")));
            let ui2 = ui.clone();
            sw.connect_active_notify(move |s| {
                if ui2.is_syncing.get() {
                    return;
                }
                let is_on = s.is_active();
                {
                    let mut c = ui2.pending.borrow_mut();
                    if is_on {
                        c.disabled.remove(def.id);
                    } else {
                        c.disabled.insert(def.id.to_string());
                    }
                }
                let all_on = {
                    let c = ui2.pending.borrow();
                    SHORTCUT_DEFS.iter().all(|d| sc_enabled(&c, d.id))
                };
                ui2.pending.borrow_mut().site_wins = !all_on;

                ui2.is_syncing.set(true);
                ui2.prio.set_active(all_on);
                ui2.is_syncing.set(false);

                sc_commit(&ui2);
                sc_refresh(&ui2);
            });
            row.append(&sw);
        }

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        line.set_sensitive(enabled);
        line.set_valign(gtk::Align::Center);
        let eff = sc_effective(def, &cfg);
        let combo = eff.first().cloned().unwrap_or_default();
        let (mod_idx, key) = if combo.is_empty() {
            (0u32, String::new())
        } else {
            sc_split(&combo)
        };
        let sel = Rc::new(Cell::new(mod_idx));

        let dd = gtk::DropDown::from_strings(SC_MODS);
        dd.set_selected(mod_idx);
        {
            let ui3 = ui.clone();
            let sel = sel.clone();
            let key = key.clone();
            dd.connect_selected_notify(move |d| {
                sel.set(d.selected());
                let is_f = key.len() >= 2 && key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit());
                if sel.get() == 4 && !is_f {
                    ui3.status.set_text(tr("No modifier: tap the key button and pick F1-F12"));
                } else if !key.is_empty() {
                    sc_apply_slot(&ui3, def, 0, sc_join(sel.get(), &key));
                }
            });
        }
        line.append(&dd);

        let kb = gtk::MenuButton::new();
        kb.set_label(&sc_key_label(&key));
        kb.set_size_request(80, -1);
        {
            let ui4 = ui.clone();
            let sel = sel.clone();
            let pop = sc_build_picker(sel.clone(), Rc::new(move |v: String| {
                sc_apply_slot(&ui4, def, 0, sc_join(sel.get(), &v));
            }));
            kb.set_popover(Some(&pop));
        }
        line.append(&kb);
        row.append(&line);

        if cfg.overrides.contains_key(def.id) || cfg.disabled.contains(def.id) {
            let reset = gtk::Button::from_icon_name("edit-undo-symbolic");
            reset.add_css_class("flat");
            reset.set_valign(gtk::Align::Center);
            reset.set_tooltip_text(Some(tr("Reset to default")));
            let ui6 = ui.clone();
            reset.connect_clicked(move |_| {
                {
                    let mut c = ui6.pending.borrow_mut();
                    c.overrides.remove(def.id);
                    c.disabled.remove(def.id);
                }
                sc_commit(&ui6);
                sc_refresh(&ui6);
            });
            row.append(&reset);
        } else {
            let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spacer.set_size_request(32, -1);
            row.append(&spacer);
        }
        container.append(&row);
    }
}

fn to_uri(input: &str, search_engine: &str) -> String {
    let s = input.trim();
    let s = s.strip_prefix("view-source:").unwrap_or(s).trim();
    if s.contains("://") || s.starts_with("about:") || s.starts_with("file:") {
        return s.into();
    }
    if s.starts_with("localhost") || s.starts_with("127.0.0.1") {
        return format!("http://{s}");
    }
    if !s.contains(' ') && s.contains('.') {
        return format!("https://{s}");
    }
    let encoded = glib::Uri::escape_string(s, None, false);
    match search_engine.to_lowercase().as_str() {
        "duckduckgo" | "ddg" => format!("https://duckduckgo.com/?q={}", encoded),
        "brave" => format!("https://search.brave.com/search?q={}", encoded),
        "bing" => format!("https://www.bing.com/search?q={}", encoded),
        "ecosia" => format!("https://www.ecosia.org/search?q={}", encoded),
        _ => format!("https://www.google.com/search?q={}", encoded),
    }
}

fn host_of(uri: &str) -> Option<String> {
    let u = glib::Uri::parse(uri, glib::UriFlags::NONE).ok()?;
    let h = u.host()?;
    Some(h.trim_start_matches("www.").to_string())
}

#[derive(Clone, Debug, PartialEq)]
struct SessionTabItem {
    row: usize,
    col: usize,
    full: bool,
    profile_id: String,
    url: String,
    custom_width: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
struct SessionState {
    focus: (usize, usize),
    sidebar_open: Option<bool>,
    touchpad_mode: Option<bool>,
    search_engine: Option<String>,
    win_size: Option<(i32, i32, bool)>,
    tabs: Vec<SessionTabItem>,
}

fn parse_session_content(content: &str) -> SessionState {
    let mut focus = (0usize, 0usize);
    let mut sidebar_open = None;
    let mut touchpad_mode = None;
    let mut search_engine = None;
    let mut win_size = None;
    let mut tabs: Vec<SessionTabItem> = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "focus" if parts.len() >= 3 => {
                let r = parts[1].parse().unwrap_or(0);
                let c = parts[2].parse().unwrap_or(0);
                focus = (r, c);
            }
            "sidebar" if parts.len() >= 2 => {
                sidebar_open = Some(parts[1] == "1");
            }
            "touchpad" if parts.len() >= 2 => {
                touchpad_mode = Some(parts[1] == "1");
            }
            "engine" | "search_engine" if parts.len() >= 2 => {
                search_engine = Some(parts[1].trim().to_string());
            }
            "win" if parts.len() >= 4 => {
                let w = parts[1].parse().unwrap_or(1400);
                let h = parts[2].parse().unwrap_or(850);
                let max = parts[3] == "1";
                win_size = Some((w, h, max));
            }
            "tab" if parts.len() >= 6 => {
                let r = parts[1].parse().unwrap_or(0);
                let c = parts[2].parse().unwrap_or(0);
                let full = parts[3] == "1";
                let pid = parts[4].to_string();
                let url = parts[5].to_string();
                let custom_width = parts.get(6).and_then(|s| s.parse::<f64>().ok());
                tabs.push(SessionTabItem { row: r, col: c, full, profile_id: pid, url, custom_width });
            }
            "tab" if parts.len() == 4 => {
                let c = parts[1].parse().unwrap_or(0);
                let pid = parts[2].to_string();
                let url = parts[3].to_string();
                tabs.push(SessionTabItem { row: 0, col: c, full: true, profile_id: pid, url, custom_width: None });
            }
            _ => {}
        }
    }
    SessionState {
        focus,
        sidebar_open,
        touchpad_mode,
        search_engine,
        win_size,
        tabs,
    }
}

impl App {
    fn focused_view(&self) -> Option<WebView> {
        let rows = self.rows.borrow();
        rows.get(self.focus_row.get())
            .and_then(|r| r.tabs.get(self.focus_col.get()))
            .map(|t| t.view.clone())
    }

    /// True when the clipboard offers an image but no text. WebKitGTK does not expose
    /// clipboard images through DataTransfer (WebKit bug 218519), so such a paste must be
    /// replayed into the page by Rail.
    fn clipboard_is_image_only(&self) -> bool {
        let f = self.win.clipboard().formats();
        f.contains_type(<gdk::Texture as glib::prelude::StaticType>::static_type())
            && !f.contain_mime_type("text/plain")
            && !f.contain_mime_type("text/plain;charset=utf-8")
    }

    /// Reads the clipboard image natively and dispatches a synthetic `paste` event
    /// carrying it as a File, so pages (Gemini etc.) see it in clipboardData.files.
    fn paste_clipboard_image(self: &Rc<Self>, view: WebView) {
        self.win.clipboard().read_texture_async(gio::Cancellable::NONE, move |res| {
            let Ok(Some(tex)) = res else { return };
            let png = tex.save_to_png_bytes();
            let b64 = glib::base64_encode(&png[..]);
            let js = format!(
                "(function(){{try{{var b=atob('{}');var u=new Uint8Array(b.length);\
                 for(var i=0;i<b.length;i++)u[i]=b.charCodeAt(i);\
                 var f=new File([u],'image.png',{{type:'image/png'}});\
                 var dt=new DataTransfer();dt.items.add(f);\
                 var t=document.activeElement||document.body;\
                 t.dispatchEvent(new ClipboardEvent('paste',{{clipboardData:dt,bubbles:true,cancelable:true,composed:true}}));\
                 }}catch(e){{}}}})();",
                b64
            );
            view.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        });
    }

    /// Ctrl+V with an image-only clipboard while a page has focus: handled by Rail.
    fn try_paste_image(self: &Rc<Self>) -> bool {
        let Some(view) = self.focused_view() else { return false };
        let page_focused = match gtk::prelude::GtkWindowExt::focus(&self.win) {
            Some(w) => w == *view.upcast_ref::<gtk::Widget>() || w.is_ancestor(&view),
            None => false,
        };
        if !page_focused || !self.clipboard_is_image_only() {
            return false;
        }
        self.paste_clipboard_image(view);
        true
    }

    fn palette_open(&self) -> bool {
        gtk::prelude::WidgetExt::is_visible(&self.palette)
    }

    fn overview_open(&self) -> bool {
        self.ov.get()
    }

    fn create_tab_widget(view: &WebView) -> (gtk::Box, gtk::Box, gtk::Picture) {
        view.add_css_class("tab");
        view.set_overflow(gtk::Overflow::Hidden);
        view.set_vexpand(true);
        view.set_hexpand(true);

        let freeze = gtk::Picture::new();
        freeze.add_css_class("tab-freeze");
        freeze.set_can_target(false);
        freeze.set_content_fit(gtk::ContentFit::Fill);
        freeze.set_hexpand(true);
        freeze.set_vexpand(true);
        freeze.set_visible(false);

        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_child(Some(view));
        overlay.add_overlay(&freeze);

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.add_css_class("rail-col");
        container.add_css_class("full");
        container.set_vexpand(true);
        container.set_hexpand(true);

        let resize_handle = gtk::Box::new(gtk::Orientation::Vertical, 0);
        resize_handle.add_css_class("tab-resize-handle");
        resize_handle.set_cursor_from_name(Some("col-resize"));

        container.append(&overlay);
        container.append(&resize_handle);
        (container, resize_handle, freeze)
    }

    fn set_tab_width(&self, r: usize, c: usize, width: f64) {
        let vw = self.strip.width() as f64;
        let clamped = width.clamp(280.0, vw);
        let rows = self.rows.borrow();
        if let Some(row) = rows.get(r) {
            if let Some(tab) = row.tabs.get(c) {
                tab.custom_width.set(Some(clamped));
                if (clamped - vw).abs() < 10.0 {
                    tab.container.add_css_class("full");
                } else {
                    tab.container.remove_css_class("full");
                }
            }
        }
        drop(rows);
        self.layout.set_column_width(r, c, Some(clamped));
        self.strip.queue_allocate();
    }

    fn update_window_title(&self) {
        let focused_title = self.focused_view().and_then(|v| {
            v.title()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty())
                .or_else(|| v.uri().and_then(|u| host_of(&u)))
        });
        let title = match focused_title {
            Some(t) => {
                let clean = t.trim();
                if clean.to_lowercase().contains("rail") {
                    clean.to_string()
                } else {
                    format!("{} — rail", clean)
                }
            }
            None => "rail".to_string(),
        };
        self.win.set_title(Some(&title));
    }

    fn sync_strip(&self) {
        let rows = self.rows.borrow();
        let mut grid = Vec::new();
        let mut custom_widths = Vec::new();
        let mut prev: Option<gtk::Widget> = None;
        for row in rows.iter() {
            let mut row_grid = Vec::new();
            let mut row_widths = Vec::new();
            for tab in row.tabs.iter() {
                row_grid.push(tab.container.has_css_class("full"));
                row_widths.push(tab.custom_width.get());
                if tab.container.parent().as_ref() != Some(self.strip.upcast_ref()) {
                    self.strip.append(&tab.container);
                }
                self.strip.reorder_child_after(&tab.container, prev.as_ref());
                prev = Some(tab.container.clone().upcast());
            }
            grid.push(row_grid);
            custom_widths.push(row_widths);
        }
        drop(rows);
        self.layout.set_grid(grid);
        self.layout.set_custom_widths(custom_widths);
        self.strip.queue_allocate();
    }

    // ---- strip scrolling -------------------------------------------------

    /// Normal mode: scroll 2D minimum needed to show focused row and column.
    /// Overview: center the 2D grid in the zoomed-out plane.
    fn reveal_focus(self: &Rc<Self>) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let z = self.target_zoom.get();
        let visible_w = vw / z;
        let visible_h = vh / z;
        let g = gap(z);
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();

        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }

        let total_h = rows.len() as f64 * vh + (rows.len().saturating_sub(1) as f64) * g;
        let mut max_w: f64 = 0.0;
        let mut active_span: Option<(f64, f64)> = None;
        let mut active_row_total_w = 0.0;

        for (ri, row) in rows.iter().enumerate() {
            let mut x = 0.0;
            for (ci, tab) in row.tabs.iter().enumerate() {
                let w = tab.custom_width.get().unwrap_or_else(|| col_width(tab.container.has_css_class("full"), vw));
                if ri == cur_r && ci == cur_c {
                    active_span = Some((x, x + w));
                }
                x += w + g;
            }
            let row_w = (x - g).max(0.0);
            if ri == cur_r {
                active_row_total_w = row_w;
            }
            if row_w > max_w {
                max_w = row_w;
            }
        }
        drop(rows);

        let (target_x, target_y) = if self.ov.get() {
            let ty = if total_h <= visible_h {
                -(visible_h - total_h) / 2.0
            } else {
                let row_center_y = cur_r as f64 * (vh + g) + vh / 2.0;
                let max_y = (total_h - visible_h).max(0.0);
                (row_center_y - visible_h / 2.0).clamp(0.0, max_y)
            };

            let tx = if max_w <= visible_w {
                -(visible_w - max_w) / 2.0
            } else {
                let max_x = (max_w - visible_w).max(0.0);
                let col_center_x = active_span.map_or(0.0, |(l, r)| (l + r) / 2.0);
                (col_center_x - visible_w / 2.0).clamp(0.0, max_x)
            };
            (tx, ty)
        } else {
            let ty = cur_r as f64 * (vh + g);
            let tx = if let Some((l, r)) = active_span {
                let max_x = (active_row_total_w - visible_w).max(0.0);
                let mut t = self.target_x.get().clamp(0.0, max_x);
                if l < t {
                    t = l;
                } else if r > t + visible_w {
                    t = r - visible_w;
                }
                t.clamp(0.0, max_x)
            } else {
                0.0
            };
            (tx, ty)
        };

        self.target_x.set(target_x);
        self.target_y.set(target_y);
        self.animate();
    }

    /// Ease both the scroll offsets (X, Y) and zoom toward their targets.
    fn animate(self: &Rc<Self>) {
        if self.animating.get() {
            return;
        }
        let near = |a: f64, b: f64, eps: f64| (a - b).abs() < eps;
        let cam_ready = near(self.target_x.get(), self.layout.offset_x(), 0.5)
            && near(self.target_y.get(), self.layout.offset_y(), 0.5)
            && near(self.target_zoom.get(), self.layout.zoom(), 0.002);

        if cam_ready && !self.layout.grid_needs_animation() {
            self.layout.set_offset_x(self.target_x.get());
            self.layout.set_offset_y(self.target_y.get());
            self.layout.set_zoom(self.target_zoom.get());
            self.strip.queue_allocate();
            return;
        }
        self.animating.set(true);
        let app = self.clone();
        let last = Cell::new(None::<i64>);
        self.strip.add_tick_callback(move |strip, clock| {
            let now = clock.frame_time();
            let dt = last.replace(Some(now)).map_or(1.0 / 60.0, |l| (now - l) as f64 / 1e6);
            let k = 1.0 - (-dt / 0.05).exp();

            let (oxc, oxt) = (app.layout.offset_x(), app.target_x.get());
            let (oyc, oyt) = (app.layout.offset_y(), app.target_y.get());
            let (zc, zt) = (app.layout.zoom(), app.target_zoom.get());

            let mut nx = oxc + (oxt - oxc) * k;
            let mut ny = oyc + (oyt - oyc) * k;
            let mut nz = zc + (zt - zc) * k;

            if near(oxt, nx, 0.5) { nx = oxt; }
            if near(oyt, ny, 0.5) { ny = oyt; }
            if near(zt, nz, 0.002) { nz = zt; }

            let grid_busy = app.layout.step_grid_progress(k);

            app.layout.set_offset_x(nx);
            app.layout.set_offset_y(ny);
            app.layout.set_zoom(nz);
            strip.queue_allocate();

            if nx == oxt && ny == oyt && nz == zt && !grid_busy {
                app.animating.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }


    /// Live 1:1 horizontal pan (finger tracks content). Used by touchpad slide mode.
    /// Offsets update every event; strip allocate is coalesced to one per frame.
    fn pan_canvas_x(self: &Rc<Self>, dx: f64) {
        let vw = self.strip.width() as f64;
        if vw <= 0.0 || dx == 0.0 {
            return;
        }
        let g = gap(self.target_zoom.get());
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r >= rows.len() {
            return;
        }
        let mut row_w = 0.0;
        let n = rows[cur_r].tabs.len();
        for (ci, tab) in rows[cur_r].tabs.iter().enumerate() {
            let w = tab.custom_width.get().unwrap_or_else(|| col_width(tab.container.has_css_class("full"), vw));
            row_w += w;
            if ci + 1 < n {
                row_w += g;
            }
        }
        drop(rows);
        let max_x = (row_w - vw).max(0.0);
        let nx = (self.layout.offset_x() + dx).clamp(0.0, max_x);
        self.layout.set_offset_x(nx);
        self.target_x.set(nx);
        // One allocate per display frame — per-event allocate freezes WebKit tabs
        if !self.pan_alloc_pending.get() {
            self.pan_alloc_pending.set(true);
            let app = self.clone();
            self.strip.add_tick_callback(move |strip, _clock| {
                app.pan_alloc_pending.set(false);
                strip.queue_allocate();
                glib::ControlFlow::Break
            });
        }
    }

    /// After a finger swipe ends, snap focus to the column under the viewport center.
    fn snap_canvas_to_nearest_column(self: &Rc<Self>) {
        let vw = self.strip.width() as f64;
        if vw <= 0.0 {
            return;
        }
        let g = gap(self.target_zoom.get());
        let cur_r = self.focus_row.get();
        let ox = self.layout.offset_x();
        let center = ox + vw / 2.0;
        let rows = self.rows.borrow();
        if cur_r >= rows.len() || rows[cur_r].tabs.is_empty() {
            return;
        }
        let mut best_c = 0usize;
        let mut best_d = f64::MAX;
        let mut x = 0.0;
        for (ci, tab) in rows[cur_r].tabs.iter().enumerate() {
            let w = tab.custom_width.get().unwrap_or_else(|| col_width(tab.container.has_css_class("full"), vw));
            let mid = x + w / 2.0;
            let d = (mid - center).abs();
            if d < best_d {
                best_d = d;
                best_c = ci;
            }
            x += w + g;
        }
        drop(rows);
        if best_c != self.focus_col.get() {
            self.focus_coords(cur_r, best_c);
        } else {
            // Same tab: still settle so edges align cleanly
            self.reveal_focus();
        }
    }

    // ---- tabs & workspaces -----------------------------------------------

    #[allow(dead_code)]
    fn focused_tab_profile(&self) -> Option<String> {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        rows.get(cur_r)
            .and_then(|r| r.tabs.get(cur_c))
            .map(|t| t.profile_id.clone())
    }

    fn find_view_profile(&self, view: &WebView) -> Option<String> {
        let rows = self.rows.borrow();
        for row in rows.iter() {
            for tab in row.tabs.iter() {
                if &tab.view == view {
                    return Some(tab.profile_id.clone());
                }
            }
        }
        None
    }

    fn create_web_view(session: &webkit6::NetworkSession, profile_id: &str) -> WebView {
        let persona = ProfilePersona::for_id(profile_id);

        let settings = webkit6::Settings::new();
        settings.set_user_agent(Some(persona.user_agent));
        settings.set_enable_webrtc(false);
        settings.set_enable_hyperlink_auditing(false);
        settings.set_enable_dns_prefetching(false);
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_webgl(true); // needed for smooth compositing on many sites
        settings.set_enable_developer_extras(true);
        settings.set_enable_back_forward_navigation_gestures(true);
        settings.set_enable_webaudio(false);
        settings.set_enable_html5_local_storage(true);
        settings.set_enable_html5_database(true);
        settings.set_enable_site_specific_quirks(true);
        settings.set_javascript_can_access_clipboard(true);
        // Always: smooth page scroll/compositing; falls back when GPU unavailable
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
        // Page cache helps back/forward; modest RAM cost
        settings.set_enable_page_cache(true);

        let stealth_js = persona.stealth_script();
        let script = webkit6::UserScript::new(
            &stealth_js,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        );
        let ucm = webkit6::UserContentManager::new();
        ucm.add_script(&script);
        let html_css = load_html_css();
        if !html_css.trim().is_empty() {
            ucm.add_style_sheet(&user_style_sheet(&html_css));
        }
        STYLE_MANAGERS.with(|l| l.borrow_mut().push(ucm.downgrade()));

        WebView::builder()
            .network_session(session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .build()
    }

    fn new_tab(self: &Rc<Self>) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_html(&start_page(), Some("rail://start/"));
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, prof_id.clone(), false);
        self.select_palette_profile(&prof_id);
        self.show_palette();
    }

    fn split_vertical(self: &Rc<Self>) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_html(&start_page(), Some("rail://start/"));
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, prof_id.clone(), true);
        self.select_palette_profile(&prof_id);
        self.show_palette();
    }

    fn new_tab_with_url_and_profile(self: &Rc<Self>, url: &str, profile_id: &str) {
        self.exit_overview();
        let session = self.profiles.borrow_mut().get_or_create_session(profile_id);
        let view = Self::create_web_view(&session, profile_id);
        view.load_uri(url);
        self.add_view(view, profile_id.to_string(), false);
    }

    fn new_tab_with_url(self: &Rc<Self>, url: &str) {
        let prof_id = self.profiles.borrow().active_id().to_string();
        self.new_tab_with_url_and_profile(url, &prof_id);
    }

    fn split_with_url(self: &Rc<Self>, url: &str) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_uri(url);
        self.add_view(view, prof_id, true);
    }

    // ---- DevTools (Alt+Shift+I / F12 / Alt+Shift+D) ----

    fn devtools_open(&self) -> bool {
        self.devtools_open.get()
    }

    fn dt_view(&self) -> Option<WebView> {
        DT_VIEW.with(|v| v.borrow().clone()).or_else(|| self.focused_view())
    }

    /// Close the undocked DevTools window without holding the RefCell borrow (close-request re-enters it).
    fn dt_close_window(&self) {
        let win = self.devtools_window.borrow_mut().take();
        if let Some(win) = win {
            DT_CLOSING_WIN.with(|f| f.set(true));
            win.set_child(None::<&gtk::Widget>);
            win.close();
            DT_CLOSING_WIN.with(|f| f.set(false));
        }
    }

    fn toggle_devtools(self: &Rc<Self>) {
        if self.devtools_open() {
            // F12 on another tab than the one being inspected: move DevTools to this tab instead of closing the wrong inspector.
            let other_tab = match (DT_VIEW.with(|v| v.borrow().clone()), self.focused_view()) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            };
            self.hide_devtools();
            if other_tab {
                self.show_devtools();
            }
        } else {
            self.show_devtools();
        }
    }

    fn cycle_devtools_dock(self: &Rc<Self>) {
        if !self.devtools_open() {
            self.show_devtools();
            return;
        }
        let next = match self.devtools_dock_side.get() {
            DevToolsDockSide::Bottom => DevToolsDockSide::Right,
            DevToolsDockSide::Right => DevToolsDockSide::Left,
            DevToolsDockSide::Left => DevToolsDockSide::Undocked,
            DevToolsDockSide::Undocked => DevToolsDockSide::Bottom,
        };
        self.set_devtools_dock_side(next);
    }

    fn set_devtools_dock_side(self: &Rc<Self>, side: DevToolsDockSide) {
        let prev = self.devtools_dock_side.get();
        self.devtools_dock_side.set(side);
        let Some(view) = self.dt_view() else { return };
        let Some(insp) = view.inspector() else { return };
        let Some(w) = insp.web_view() else { return };
        let widget: gtk::Widget = w.upcast();

        // Already docked on this side with the inspector widget in place: do not reparent.
        // Reparenting on every WebKit attach signal steals clicks and makes the panel feel "closed".
        if prev == side && self.devtools_open.get() {
            let already_in_panel = matches!(
                side,
                DevToolsDockSide::Bottom | DevToolsDockSide::Left | DevToolsDockSide::Right
            ) && widget
                .parent()
                .map(|p| p == self.devtools_content_box.upcast_ref::<gtk::Widget>().clone())
                .unwrap_or(false);
            let already_undocked = side == DevToolsDockSide::Undocked
                && self.devtools_window.borrow().is_some();
            if already_in_panel || already_undocked {
                return;
            }
        }

        match side {
            DevToolsDockSide::Bottom => {
                self.dt_close_window();
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                if widget.parent().is_some() {
                    widget.unparent();
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);

                self.devtools_panel.remove_css_class("dock-left");
                self.devtools_panel.remove_css_class("dock-right");
                self.devtools_panel.add_css_class("dock-bottom");

                self.devtools_panel.set_orientation(gtk::Orientation::Vertical);
                while let Some(c) = self.devtools_panel.first_child() {
                    self.devtools_panel.remove(&c);
                }
                self.devtools_panel.append(&self.devtools_resize_handle);
                self.devtools_panel.append(&self.devtools_content_box);

                self.devtools_panel.set_halign(gtk::Align::Fill);
                self.devtools_panel.set_valign(gtk::Align::End);
                self.devtools_panel.set_width_request(-1);
                self.devtools_panel.set_height_request(400);
                self.devtools_panel.set_visible(true);
                self.devtools_open.set(true);
            }
            DevToolsDockSide::Left => {
                self.dt_close_window();
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                if widget.parent().is_some() {
                    widget.unparent();
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);

                self.devtools_panel.remove_css_class("dock-bottom");
                self.devtools_panel.remove_css_class("dock-right");
                self.devtools_panel.add_css_class("dock-left");

                self.devtools_panel.set_orientation(gtk::Orientation::Horizontal);
                while let Some(c) = self.devtools_panel.first_child() {
                    self.devtools_panel.remove(&c);
                }
                self.devtools_panel.append(&self.devtools_content_box);
                self.devtools_panel.append(&self.devtools_resize_handle);

                self.devtools_panel.set_halign(gtk::Align::Start);
                self.devtools_panel.set_valign(gtk::Align::Fill);
                self.devtools_panel.set_height_request(-1);
                self.devtools_panel.set_width_request(560);
                self.devtools_panel.set_visible(true);
                self.devtools_open.set(true);
            }
            DevToolsDockSide::Right => {
                self.dt_close_window();
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                if widget.parent().is_some() {
                    widget.unparent();
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);

                self.devtools_panel.remove_css_class("dock-bottom");
                self.devtools_panel.remove_css_class("dock-left");
                self.devtools_panel.add_css_class("dock-right");

                self.devtools_panel.set_orientation(gtk::Orientation::Horizontal);
                while let Some(c) = self.devtools_panel.first_child() {
                    self.devtools_panel.remove(&c);
                }
                self.devtools_panel.append(&self.devtools_resize_handle);
                self.devtools_panel.append(&self.devtools_content_box);

                self.devtools_panel.set_halign(gtk::Align::End);
                self.devtools_panel.set_valign(gtk::Align::Fill);
                self.devtools_panel.set_height_request(-1);
                self.devtools_panel.set_width_request(560);
                self.devtools_panel.set_visible(true);
                self.devtools_open.set(true);
            }
            DevToolsDockSide::Undocked => {
                self.devtools_panel.set_visible(false);
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }

                let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
                let host = host_of(&cur_uri).unwrap_or_else(|| "DevTools".to_string());
                let mut win_opt = self.devtools_window.borrow_mut();
                if let Some(win) = win_opt.as_ref() {
                    if let Some(child) = win.child() {
                        if let Some(box_container) = child.downcast_ref::<gtk::Box>() {
                            if widget.parent().as_ref() != Some(box_container.upcast_ref()) {
                                if widget.parent().is_some() {
                                    widget.unparent();
                                }
                                while let Some(c) = box_container.first_child() {
                                    box_container.remove(&c);
                                }
                                widget.set_vexpand(true);
                                widget.set_hexpand(true);
                                box_container.append(&widget);
                            }
                        }
                    }
                    win.set_title(Some(&trf("Rail DevTools - {}", &[&host])));
                    win.present();
                } else {
                    let win_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    win_box.add_css_class("devtools-window-box");
                    if widget.parent().is_some() {
                        widget.unparent();
                    }
                    widget.set_vexpand(true);
                    widget.set_hexpand(true);
                    win_box.append(&widget);


                    let win = gtk::Window::builder()
                        .title(&trf("Rail DevTools - {}", &[&host]))
                        .default_width(1050)
                        .default_height(720)
                        // no transient_for: niri opens windows that have a parent as floating
                        .child(&win_box)
                        .build();
                    win.add_css_class("devtools-window");

                    let app_win = self.clone();
                    let win_box_ref = win_box.clone();
                    win.connect_close_request(move |w| {
                        if DT_CLOSING_WIN.with(|f| f.get()) {
                            return glib::Propagation::Proceed;
                        }
                        while let Some(c) = win_box_ref.first_child() {
                            win_box_ref.remove(&c);
                        }
                        w.set_child(None::<&gtk::Widget>);
                        app_win.devtools_window.borrow_mut().take();
                        app_win.hide_devtools();
                        glib::Propagation::Proceed
                    });

                    win.present();
                    *win_opt = Some(win);
                }
                self.devtools_open.set(true);
            }
        }
    }

    fn show_devtools(self: &Rc<Self>) {
        let Some(view) = self.focused_view() else { return };
        let Some(insp) = view.inspector() else { return };

        if IN_SHOW_DEVTOOLS.with(|f| f.replace(true)) {
            return;
        }
        DT_VIEW.with(|v| *v.borrow_mut() = Some(view.clone()));
        insp.show();

        // Update URL and host badge in devtools header if ever needed
        let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_else(|| "about:blank".to_string());
        let host = host_of(&cur_uri).unwrap_or_else(|| "localhost".to_string());
        self.devtools_url_lbl.set_markup(&format!("<span foreground='#fafafa'>{}</span> <span color='#64748b'>•</span> <span color='#94a3b8'>{}</span>", glib::markup_escape_text(&host), glib::markup_escape_text(&cur_uri)));

        let side = self.devtools_dock_side.get();
        self.set_devtools_dock_side(side);
        IN_SHOW_DEVTOOLS.with(|f| f.set(false));
    }

    fn hide_devtools(&self) {
        self.hide_devtools_impl(true);
    }

    /// `close_inspector=false` is used from the inspector's own `closed` signal: it is already closing, don't re-enter WebKit.
    fn hide_devtools_impl(&self, close_inspector: bool) {
        if !self.devtools_open.get() {
            return;
        }
        self.devtools_open.set(false);

        let dt_view = DT_VIEW.with(|v| v.borrow_mut().take()).or_else(|| self.focused_view());
        if close_inspector {
            if let Some(view) = dt_view {
                if let Some(insp) = view.inspector() {
                    if let Some(w) = insp.web_view() {
                        let widget: gtk::Widget = w.upcast();
                        if widget.parent().is_some() {
                            widget.unparent();
                        }
                    }
                    insp.close();
                }
            }
        }
        self.dt_close_window();
        while let Some(c) = self.devtools_content_box.first_child() {
            self.devtools_content_box.remove(&c);
        }
        self.devtools_panel.set_visible(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    fn toggle_devtools_undocked(self: &Rc<Self>) {
        if self.devtools_dock_side.get() == DevToolsDockSide::Undocked {
            self.set_devtools_dock_side(DevToolsDockSide::Bottom);
        } else {
            self.set_devtools_dock_side(DevToolsDockSide::Undocked);
        }
    }

    fn show_script_dialog(self: &Rc<Self>, view: &WebView, dialog: &ScriptDialog) {
        let dialog_type = dialog.dialog_type();
        let message = dialog.message().map(|s| s.to_string()).unwrap_or_default();
        let default_prompt = dialog.prompt_get_default_text().map(|s| s.to_string()).unwrap_or_default();

        let host = view.uri()
            .and_then(|u| glib::Uri::parse(&u, glib::UriFlags::NONE).ok())
            .and_then(|u| u.host().map(|h| h.to_string()))
            .unwrap_or_else(|| tr("This page").to_string());

        let title_text = match dialog_type {
            ScriptDialogType::Alert => trf("{} says:", &[&host]),
            ScriptDialogType::Confirm => trf("Confirm from {}", &[&host]),
            ScriptDialogType::Prompt => trf("Prompt from {}", &[&host]),
            ScriptDialogType::BeforeUnloadConfirm => tr("Leave Site?").to_string(),
            _ => trf("Message from {}", &[&host]),
        };

        let dlg_win = gtk::Window::builder()
            .title(&title_text)
            .transient_for(&self.win)
            .modal(true)
            .resizable(false)
            .default_width(380)
            .build();
        dlg_win.add_css_class("js-dialog-window");

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 14);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let header_lbl = gtk::Label::new(Some(&title_text));
        header_lbl.add_css_class("js-dialog-title");
        header_lbl.set_xalign(0.0);
        vbox.append(&header_lbl);

        let msg_text = if dialog_type == ScriptDialogType::BeforeUnloadConfirm && message.is_empty() {
            tr("Changes you made may not be saved. Are you sure you want to leave?").to_string()
        } else {
            message
        };
        let msg_lbl = gtk::Label::new(Some(&msg_text));
        msg_lbl.add_css_class("js-dialog-message");
        msg_lbl.set_wrap(true);
        msg_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        msg_lbl.set_selectable(true);
        msg_lbl.set_xalign(0.0);
        msg_lbl.set_max_width_chars(50);
        vbox.append(&msg_lbl);

        let prompt_entry = if dialog_type == ScriptDialogType::Prompt {
            let entry = gtk::Entry::new();
            entry.add_css_class("js-dialog-entry");
            entry.set_text(&default_prompt);
            vbox.append(&entry);
            Some(entry)
        } else {
            None
        };

        let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        btn_row.set_halign(gtk::Align::End);
        btn_row.set_margin_top(6);

        let handled = Rc::new(Cell::new(false));

        let close_dialog = {
            let handled = handled.clone();
            let dialog = dialog.clone();
            let dlg_win = dlg_win.clone();
            let prompt_entry = prompt_entry.clone();
            move |confirmed: bool| {
                if !handled.get() {
                    handled.set(true);
                    match dialog_type {
                        ScriptDialogType::Alert => {
                            dialog.close();
                        }
                        ScriptDialogType::Confirm | ScriptDialogType::BeforeUnloadConfirm => {
                            dialog.confirm_set_confirmed(confirmed);
                            dialog.close();
                        }
                        ScriptDialogType::Prompt => {
                            if confirmed {
                                let text = prompt_entry.as_ref().map(|e| e.text().to_string()).unwrap_or_default();
                                dialog.prompt_set_text(&text);
                                dialog.confirm_set_confirmed(true);
                            } else {
                                dialog.confirm_set_confirmed(false);
                            }
                            dialog.close();
                        }
                        _ => {
                            dialog.close();
                        }
                    }
                    dlg_win.close();
                }
            }
        };

        if dialog_type != ScriptDialogType::Alert {
            let cancel_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm { tr("Stay") } else { tr("Cancel") };
            let cancel_btn = gtk::Button::with_label(cancel_label);
            cancel_btn.add_css_class("js-dialog-btn-cancel");
            let cd = close_dialog.clone();
            cancel_btn.connect_clicked(move |_| cd(false));
            btn_row.append(&cancel_btn);
        }

        let ok_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm { tr("Leave") } else { tr("OK") };
        let ok_btn = gtk::Button::with_label(ok_label);
        ok_btn.add_css_class("js-dialog-btn-ok");
        let cd_ok = close_dialog.clone();
        ok_btn.connect_clicked(move |_| cd_ok(true));
        btn_row.append(&ok_btn);

        vbox.append(&btn_row);
        dlg_win.set_child(Some(&vbox));

        if let Some(entry) = &prompt_entry {
            let cd_ent = close_dialog.clone();
            entry.connect_activate(move |_| cd_ent(true));
        }

        let key_ctl = gtk::EventControllerKey::new();
        let cd_key = close_dialog.clone();
        key_ctl.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                cd_key(false);
                glib::Propagation::Stop
            } else if keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter {
                cd_key(true);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        dlg_win.add_controller(key_ctl);

        let cd_close = close_dialog.clone();
        dlg_win.connect_close_request(move |_| {
            cd_close(false);
            glib::Propagation::Proceed
        });

        dlg_win.present();
        if let Some(entry) = &prompt_entry {
            entry.grab_focus();
        } else {
            ok_btn.grab_focus();
        }
    }

    fn customize_context_menu(self: &Rc<Self>, view: &WebView, menu: &ContextMenu, hit: &HitTestResult) {
        let is_link = hit.context_is_link();
        let link_uri = hit.link_uri().map(|s| s.to_string());
        let is_image = hit.context_is_image();
        let image_uri = hit.image_uri().map(|s| s.to_string());
        let is_media = hit.context_is_media();
        let is_editable = hit.context_is_editable();
        let is_selection = hit.context_is_selection();

        menu.remove_all();

        // 1. Link items
        if is_link {
            if let Some(uri) = &link_uri {
                let uri_t = uri.clone();
                let app_t = self.clone();
                let act_tab = gio::SimpleAction::new("ctx-open-tab", None);
                act_tab.connect_activate(move |_, _| {
                    app_t.new_tab_with_url(&uri_t);
                });
                self.win.add_action(&act_tab);
                menu.append(&ContextMenuItem::from_gaction(&act_tab, tr("Open Link in New Tab"), None));

                let uri_s = uri.clone();
                let app_s = self.clone();
                let act_split = gio::SimpleAction::new("ctx-open-split", None);
                act_split.connect_activate(move |_, _| {
                    app_s.split_with_url(&uri_s);
                });
                self.win.add_action(&act_split);
                menu.append(&ContextMenuItem::from_gaction(&act_split, tr("Open Link in Split View"), None));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyLinkToClipboard,
                    tr("Copy Link Address"),
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::DownloadLinkToDisk,
                    tr("Save Link As..."),
                ));

                menu.append(&ContextMenuItem::new_separator());
            }
        }

        // 2. Image items
        if is_image {
            if let Some(img_url) = &image_uri {
                let img_t = img_url.clone();
                let app_t = self.clone();
                let act_img = gio::SimpleAction::new("ctx-open-img", None);
                act_img.connect_activate(move |_, _| {
                    app_t.new_tab_with_url(&img_t);
                });
                self.win.add_action(&act_img);
                menu.append(&ContextMenuItem::from_gaction(&act_img, tr("Open Image in New Tab"), None));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyImageUrlToClipboard,
                    tr("Copy Image Address"),
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyImageToClipboard,
                    tr("Copy Image"),
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::DownloadImageToDisk,
                    tr("Save Image As..."),
                ));

                menu.append(&ContextMenuItem::new_separator());
            }
        }

        // 3. Media items
        if is_media {
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::OpenVideoInNewWindow,
                tr("Open Media in New Tab"),
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::CopyVideoLinkToClipboard,
                tr("Copy Media Address"),
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::DownloadVideoToDisk,
                tr("Save Media As..."),
            ));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 4. Editable text
        if is_editable {
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Cut, tr("Cut")));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Copy, tr("Copy")));
            if self.clipboard_is_image_only() {
                let app_p = self.clone();
                let v_p = view.clone();
                let act_paste = gio::SimpleAction::new("ctx-paste-image", None);
                act_paste.connect_activate(move |_, _| app_p.paste_clipboard_image(v_p.clone()));
                self.win.add_action(&act_paste);
                menu.append(&ContextMenuItem::from_gaction(&act_paste, tr("Paste"), None));
            } else {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Paste, tr("Paste")));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::PasteAsPlainText, tr("Paste as Plain Text")));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::SelectAll, tr("Select All")));
            menu.append(&ContextMenuItem::new_separator());
        } else if is_selection {
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Copy, tr("Copy")));

            let app_sel = self.clone();
            let v_sel = view.clone();
            let act_search = gio::SimpleAction::new("ctx-search-sel", None);
            act_search.connect_activate(move |_, _| {
                let app_s = app_sel.clone();
                v_sel.evaluate_javascript(
                    "window.getSelection() ? window.getSelection().toString() : ''",
                    None,
                    None,
                    None::<&gio::Cancellable>,
                    move |res| {
                        if let Ok(js_val) = res {
                            let text = js_val.to_string();
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                let search_url = format!("https://www.google.com/search?q={}", glib::Uri::escape_string(trimmed, None, false));
                                app_s.new_tab_with_url(&search_url);
                            }
                        }
                    },
                );
            });
            self.win.add_action(&act_search);
            menu.append(&ContextMenuItem::from_gaction(&act_search, tr("Search Google for Selection"), None));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 5. Page navigation
        if !is_link && !is_editable && !is_selection {
            if view.can_go_back() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::GoBack, tr("Back")));
            }
            if view.can_go_forward() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::GoForward, tr("Forward")));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Reload, tr("Reload")));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 6. Developer Tools (Inspect Element / View Source)
        let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
        if !cur_uri.is_empty() && !cur_uri.starts_with("about:") {
            let app_src = self.clone();
            let act_src = gio::SimpleAction::new("ctx-view-source", None);
            act_src.connect_activate(move |_, _| {
                app_src.show_devtools();
            });
            self.win.add_action(&act_src);
            menu.append(&ContextMenuItem::from_gaction(&act_src, tr("View Page Source  (F12)"), None));
        }

        menu.append(&ContextMenuItem::from_stock_action_with_label(
            ContextMenuAction::InspectElement,
            tr("Inspect Element  (F12)"),
        ));
    }

    fn setup_view_signals(self: &Rc<Self>, view: &WebView) {
        let app = self.clone();
        let v = view.clone();
        let click = gtk::GestureClick::new();
        click.set_button(0); // Listen to any mouse click
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            if !app.ov.get() {
                if let Some((r, c)) = app.find_view_coords(&v) {
                    if app.focus_row.get() != r || app.focus_col.get() != c {
                        app.focus_coords(r, c);
                    }
                }
            }
        });
        view.add_controller(click);

        // Track developer tools window state & auto-dock to Rail panel
        if let Some(insp) = view.inspector() {
            let app_insp = self.clone();
            insp.connect_open_window(move |_| {
                app_insp.set_devtools_dock_side(DevToolsDockSide::Undocked);
                true
            });
            let app_detach = self.clone();
            insp.connect_detach(move |_| {
                if app_detach.devtools_dock_side.get() != DevToolsDockSide::Undocked {
                    app_detach.set_devtools_dock_side(DevToolsDockSide::Undocked);
                }
                true
            });
            let app_attach = self.clone();
            insp.connect_attach(move |_| {
                let cur = app_attach.devtools_dock_side.get();
                let slot = LAST_DT_SLOT.with(|c| c.replace(-1));
                // Visible dock buttons sit after the close button, in order Left, Bottom, Right, minus the current side.
                let visible: Vec<DevToolsDockSide> = [DevToolsDockSide::Left, DevToolsDockSide::Bottom, DevToolsDockSide::Right]
                    .into_iter()
                    .filter(|d| *d != cur)
                    .collect();
                let target = if slot >= 1 && ((slot - 1) as usize) < visible.len() && cur != DevToolsDockSide::Undocked {
                    visible[(slot - 1) as usize]
                } else if cur == DevToolsDockSide::Undocked {
                    DevToolsDockSide::Bottom
                } else {
                    cur
                };
                eprintln!("[rail] inspector attach: slot={} cur={:?} -> {:?}", slot, cur, target);
                // Same side: WebKit still emits attach; re-docking would unparent the webview and drop the click.
                if target != cur || !app_attach.devtools_open.get() {
                    app_attach.set_devtools_dock_side(target);
                }
                true
            });
            let a_closed = self.clone();
            insp.connect_closed(move |_| {
                a_closed.hide_devtools_impl(false);
            });
            let app_front = self.clone();
            insp.connect_bring_to_front(move |_| {
                app_front.show_devtools();
                true
            });
        }

        // JavaScript dialogs (alert, confirm, prompt, beforeunload)
        let app = self.clone();
        view.connect_script_dialog(move |v, dialog| {
            app.show_script_dialog(v, dialog);
            true
        });

        // HTML <input type="color"> color chooser dialog
        let app = self.clone();
        view.connect_run_color_chooser(move |_v, request| {
            let dialog = gtk::ColorDialog::new();
            dialog.set_modal(true);
            dialog.set_title(&tr("Select Color"));
            dialog.set_with_alpha(false);
            let initial = request.rgba();
            let req = request.clone();
            let cancellable = gio::Cancellable::new();
            let c_clone = cancellable.clone();
            request.connect_finished(move |_| {
                c_clone.cancel();
            });
            dialog.choose_rgba(Some(&app.win), Some(&initial), Some(&cancellable), move |res| {
                if let Ok(rgba) = res {
                    req.set_rgba(&rgba);
                    req.finish();
                } else {
                    req.cancel();
                }
            });
            true
        });

        // Developer context menu (Right-click & 2-finger touchpad tap)
        let app = self.clone();
        view.connect_context_menu(move |v, menu, hit| {
            app.customize_context_menu(v, menu, hit);
            false
        });

        let app = self.clone();
        let v = view.clone();
        let focus = gtk::EventControllerFocus::new();
        focus.connect_enter(move |_| {
            if !app.ov.get() {
                if let Some((r, c)) = app.find_view_coords(&v) {
                    if app.focus_row.get() != r || app.focus_col.get() != c {
                        app.focus_coords(r, c);
                    }
                }
            }
        });
        view.add_controller(focus);

        let app = self.clone();
        view.connect_title_notify(move |v| {
            app.refresh_sidebar();
            app.update_window_title();
            if let Some(u) = v.uri() {
                let title = v.title().unwrap_or_default().to_string();
                app.history.borrow_mut().record_visit(&u, &title);
            }
        });
        let app = self.clone();
        view.connect_uri_notify(move |v| {
            app.remember_uri(v);
            app.refresh_sidebar();
            app.update_window_title();
            app.schedule_save_session();
            if let Some(u) = v.uri() {
                let title = v.title().unwrap_or_default().to_string();
                app.history.borrow_mut().record_visit(&u, &title);
            }
        });
        let app = self.clone();
        view.connect_favicon_notify(move |_| app.refresh_sidebar());
        let app = self.clone();
        view.connect_is_loading_notify(move |v| {
            app.refresh_sidebar();
            app.update_load_bar();
            // Freeze faqat yuklash tugagach ochiladi (uxlagan tab uyg'onganda)
            if !v.is_loading() {
                app.clear_tab_freeze(v);
            }
        });
        let app = self.clone();
        view.connect_estimated_load_progress_notify(move |_| app.update_load_bar());
        let app = self.clone();
        view.connect_create(move |v, _| {
            let parent_prof = app.find_view_profile(v).unwrap_or_else(|| app.profiles.borrow().active_profile().id);
            let nv = WebView::builder().related_view(v).build();
            app.add_view(nv.clone(), parent_prof, false);
            app.mark_keep_alive(v);
            app.mark_keep_alive(&nv);
            Some(nv.upcast())
        });
        let app = self.clone();
        view.connect_close(move |v| app.close_view(v));
        let app_term = self.clone();
        view.connect_web_process_terminated(move |v, _reason| {
            // Background tabs are intentionally suspended (terminate) to free RAM.
            // Only the focused tab auto-reloads so the user keeps browsing.
            if app_term.focused_view().as_ref() == Some(v) {
                let saved = app_term.saved_uri_of(v);
                if saved.is_empty() {
                    v.reload();
                } else {
                    v.load_uri(&saved);
                }
            }
        });
    }

    fn setup_tab_signals(self: &Rc<Self>, container: &gtk::Box, view: &WebView, resize_handle: &gtk::Box) {
        self.setup_view_signals(view);

        let app = self.clone();
        let v = view.clone();
        let click = gtk::GestureClick::new();
        click.set_button(0);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            if !app.ov.get() {
                if let Some((r, c)) = app.find_view_coords(&v) {
                    if app.focus_row.get() != r || app.focus_col.get() != c {
                        app.focus_coords(r, c);
                    }
                }
            }
        });
        container.add_controller(click);

        // Interactive splitter resize drag gesture
        let drag = gtk::GestureDrag::new();
        let app = self.clone();
        let v = view.clone();
        let h = resize_handle.clone();
        let start_w0 = Rc::new(Cell::new(0.0f64));
        let start_w1 = Rc::new(Cell::new(Option::<f64>::None));
        let start_root_x = Rc::new(Cell::new(0.0f64));

        drag.connect_drag_begin({
            let app = app.clone();
            let v = v.clone();
            let h = h.clone();
            let start_w0 = start_w0.clone();
            let start_w1 = start_w1.clone();
            let start_root_x = start_root_x.clone();
            move |gesture, _start_x, _start_y| {
                h.add_css_class("active");
                if let Some(ev) = gesture.current_event() {
                    if let Some((rx, _)) = ev.position() {
                        start_root_x.set(rx);
                    }
                }
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let vw = app.strip.width() as f64;
                    let rows = app.rows.borrow();
                    if let Some(row) = rows.get(r) {
                        let w0 = row.tabs.get(c).and_then(|t| t.custom_width.get()).unwrap_or_else(|| {
                            let is_full = row.tabs.get(c).map(|t| t.container.has_css_class("full")).unwrap_or(false);
                            col_width(is_full, vw)
                        });
                        start_w0.set(w0);

                        let w1 = row.tabs.get(c + 1).map(|t| {
                            t.custom_width.get().unwrap_or_else(|| {
                                let is_full = t.container.has_css_class("full");
                                col_width(is_full, vw)
                            })
                        });
                        start_w1.set(w1);
                    }
                }
            }
        });

        drag.connect_drag_update({
            let app = app.clone();
            let v = v.clone();
            let start_w0 = start_w0.clone();
            let start_w1 = start_w1.clone();
            let start_root_x = start_root_x.clone();
            move |gesture, offset_x, _offset_y| {
                let dx = if let Some(ev) = gesture.current_event() {
                    if let Some((rx, _)) = ev.position() {
                        rx - start_root_x.get()
                    } else {
                        offset_x
                    }
                } else {
                    offset_x
                };

                if let Some((r, c)) = app.find_view_coords(&v) {
                    let vw = app.strip.width() as f64;
                    let min_w = 280.0;
                    let max_w = vw; // Fullscreen max: left edge to right edge
                    let initial0 = start_w0.get();
                    let new_w0 = (initial0 + dx).clamp(min_w, max_w);

                    if let Some(initial1) = start_w1.get() {
                        let total_pair = initial0 + initial1;
                        if total_pair <= vw + 15.0 {
                            let clamped_w0 = new_w0.clamp(min_w, (total_pair - min_w).min(max_w));
                            let new_w1 = (total_pair - clamped_w0).max(min_w);
                            app.set_tab_width(r, c, clamped_w0);
                            app.set_tab_width(r, c + 1, new_w1);
                            return;
                        }
                    }

                    app.set_tab_width(r, c, new_w0);
                }
            }
        });

        drag.connect_drag_end({
            let app = app.clone();
            let h = h.clone();
            move |_gesture, _offset_x, _offset_y| {
                h.remove_css_class("active");
                app.schedule_save_session();
                app.reveal_focus();
            }
        });

        let dbl_click = gtk::GestureClick::new();
        dbl_click.set_button(gdk::BUTTON_PRIMARY);
        let app_dbl = self.clone();
        let v_dbl = view.clone();
        dbl_click.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2 {
                if let Some((r, c)) = app_dbl.find_view_coords(&v_dbl) {
                    let rows = app_dbl.rows.borrow();
                    if let Some(tab) = rows.get(r).and_then(|row| row.tabs.get(c)) {
                        tab.custom_width.set(None);
                    }
                    drop(rows);
                    app_dbl.sync_strip();
                    app_dbl.schedule_save_session();
                }
            }
        });

        resize_handle.add_controller(drag);
        resize_handle.add_controller(dbl_click);
    }

    fn add_view(self: &Rc<Self>, view: WebView, profile_id: String, new_row: bool) {
        view.set_can_target(!self.ov.get());
        let (container, resize_handle, freeze) = Self::create_tab_widget(&view);

        let (target_r, target_c) = {
            let mut rows = self.rows.borrow_mut();
            if rows.is_empty() {
                rows.push(Row { tabs: Vec::new() });
            }
            if new_row {
                let cur_r = self.focus_row.get();
                let next_r = cur_r + 1;
                while rows.len() <= next_r {
                    rows.push(Row { tabs: Vec::new() });
                }
                rows[next_r].tabs.push(Tab { container: container.clone(), view: view.clone(), freeze: freeze.clone(), profile_id, custom_width: Cell::new(None), saved_uri: RefCell::new(String::new()), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) });
                let c_idx = rows[next_r].tabs.len() - 1;
                (next_r, c_idx)
            } else {
                let cur_r = self.focus_row.get().min(rows.len() - 1);
                let cur_c = self.focus_col.get();
                let col_idx = if rows[cur_r].tabs.is_empty() {
                    0
                } else {
                    (cur_c + 1).min(rows[cur_r].tabs.len())
                };
                rows[cur_r].tabs.insert(col_idx, Tab { container: container.clone(), view: view.clone(), freeze: freeze.clone(), profile_id, custom_width: Cell::new(None), saved_uri: RefCell::new(String::new()), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) });
                (cur_r, col_idx)
            }
        };

        self.setup_tab_signals(&container, &view, &resize_handle);
        self.sync_strip();
        self.focus_coords(target_r, target_c);
    }

    fn saved_uri_of(&self, view: &WebView) -> String {
        let Ok(rows) = self.rows.try_borrow() else { return String::new() };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    return tab.saved_uri.borrow().clone();
                }
            }
        }
        String::new()
    }

    /// Joriy http(s) manzilni eslab qoladi (web process to'xtatilganda uri() tozalanadi)
    fn remember_uri(&self, view: &WebView) {
        let Some(u) = view.uri() else { return };
        let u = u.to_string();
        if !(u.starts_with("http://") || u.starts_with("https://")) {
            return;
        }
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    *tab.saved_uri.borrow_mut() = u;
                    return;
                }
            }
        }
    }

    fn mark_keep_alive(&self, view: &WebView) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view {
                    tab.keep_alive.set(true);
                }
            }
        }
    }

    fn clear_tab_freeze(&self, view: &WebView) {
        let Ok(rows) = self.rows.try_borrow() else { return };
        for row in rows.iter() {
            for tab in &row.tabs {
                if &tab.view == view && tab.freeze.is_visible() && !tab.suspended.get() {
                    tab.freeze.set_visible(false);
                    tab.freeze.set_paintable(Option::<&gdk::Paintable>::None);
                    return;
                }
            }
        }
    }

    fn schedule_suspend(self: &Rc<Self>, view: &WebView) {
        let app = self.clone();
        let v = view.clone();
        glib::timeout_add_local_once(Duration::from_secs(SUSPEND_AFTER_SECS), move || {
            let Ok(rows) = app.rows.try_borrow() else { return };
            for (ri, row) in rows.iter().enumerate() {
                for (ci, tab) in row.tabs.iter().enumerate() {
                    if tab.view != v {
                        continue;
                    }
                    let is_focused = ri == app.focus_row.get() && ci == app.focus_col.get();
                    if is_focused
                        || tab.suspended.get()
                        || tab.keep_alive.get()
                        || tab.last_active.get().elapsed() < Duration::from_secs(SUSPEND_AFTER_SECS)
                        || tab.view.is_loading()
                        || tab.view.is_playing_audio()
                    {
                        return;
                    }
                    let uri = tab.view.uri().map(|s| s.to_string()).unwrap_or_default();
                    if uri.starts_with("http://") || uri.starts_with("https://") {
                        *tab.saved_uri.borrow_mut() = uri;
                        // Oxirgi kadrni muzlatib qo'yamiz — web process o'lganda ham ko'rinish qoladi
                        let wp = gtk::WidgetPaintable::new(Some(&tab.view));
                        let frame = gdk::prelude::PaintableExt::current_image(&wp);
                        tab.freeze.set_paintable(Some(&frame));
                        tab.freeze.set_visible(true);
                        tab.suspended.set(true);
                        tab.view.terminate_web_process();
                    }
                    return;
                }
            }
        });
    }

    fn find_view_coords(&self, view: &WebView) -> Option<(usize, usize)> {
        let rows = self.rows.borrow();
        for (r_idx, row) in rows.iter().enumerate() {
            for (c_idx, tab) in row.tabs.iter().enumerate() {
                if &tab.view == view {
                    return Some((r_idx, c_idx));
                }
            }
        }
        None
    }

    fn close_view(self: &Rc<Self>, view: &WebView) {
        let Some((r_idx, c_idx)) = self.find_view_coords(view) else { return };
        if self.fresh.borrow().as_ref() == Some(view) {
            *self.fresh.borrow_mut() = None;
        }

        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let is_closing_focused = r_idx == cur_r && c_idx == cur_c;

        let mut rows = self.rows.borrow_mut();
        if r_idx >= rows.len() || c_idx >= rows[r_idx].tabs.len() {
            return;
        }
        let tab = rows[r_idx].tabs.remove(c_idx);
        self.strip.remove(&tab.container);

        let row_removed = if rows[r_idx].tabs.is_empty() && rows.len() > 1 {
            rows.remove(r_idx);
            true
        } else {
            false
        };

        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let total_rows = rows.len();
        drop(rows);

        self.sync_strip();

        if total_tabs == 0 {
            self.save_session_immediate();
            self.win.close();
            return;
        }

        if is_closing_focused {
            let next_r = r_idx.min(total_rows.saturating_sub(1));
            let rows = self.rows.borrow();
            if rows.is_empty() || rows[next_r].tabs.is_empty() {
                if let Some((first_r, _)) = rows.iter().enumerate().find(|(_, r)| !r.tabs.is_empty()) {
                    let next_c = c_idx.min(rows[first_r].tabs.len().saturating_sub(1));
                    drop(rows);
                    self.focus_coords(first_r, next_c);
                } else {
                    drop(rows);
                    self.new_tab();
                }
            } else {
                let next_c = c_idx.min(rows[next_r].tabs.len().saturating_sub(1));
                drop(rows);
                self.focus_coords(next_r, next_c);
            }
        } else {
            let mut new_r = cur_r;
            let mut new_c = cur_c;

            if row_removed && r_idx < cur_r {
                new_r = cur_r.saturating_sub(1);
            }
            if r_idx == cur_r && c_idx < cur_c {
                new_c = cur_c.saturating_sub(1);
            }

            self.focus_row.set(new_r);
            self.focus_col.set(new_c);

            self.refresh_sidebar();
            self.update_window_title();
            self.schedule_save_session();
            if self.map_open() {
                self.trigger_map_animation();
            }
        }
    }

    fn focus_coords(self: &Rc<Self>, r_idx: usize, c_idx: usize) {
        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }
        let r_clamped = r_idx.min(rows.len() - 1);
        if rows[r_clamped].tabs.is_empty() {
            return;
        }
        let c_clamped = c_idx.min(rows[r_clamped].tabs.len() - 1);
        self.focus_row.set(r_clamped);
        self.focus_col.set(c_clamped);

        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                if ri == r_clamped && ci == c_clamped {
                    tab.view.add_css_class("focused");
                    tab.last_active.set(std::time::Instant::now());
                    // Uyg'otish: sahifani qayta yukla; freeze load tugaguncha turadi
                    if tab.suspended.get() {
                        tab.suspended.set(false);
                        let uri = tab.saved_uri.borrow().clone();
                        if !uri.is_empty() {
                            tab.view.load_uri(&uri);
                        } else {
                            tab.freeze.set_visible(false);
                            tab.freeze.set_paintable(Option::<&gdk::Paintable>::None);
                        }
                    }
                } else {
                    if tab.view.has_css_class("focused") {
                        tab.last_active.set(std::time::Instant::now());
                    }
                    tab.view.remove_css_class("focused");
                    // Fon tabni darhol emas, SUSPEND_AFTER_SECS dan keyin to'xtatamiz
                    if !tab.suspended.get() && !tab.keep_alive.get() {
                        self.schedule_suspend(&tab.view);
                    }
                }
            }
        }

        if let Some(tab) = rows[r_clamped].tabs.get(c_clamped) {
            if !tab.view.is_loading() {
                self.load_bar.set_visible(false);
            }
        }
        drop(rows);

        self.update_load_bar();
        self.reveal_focus();
        self.refresh_sidebar();
        self.update_window_title();
        self.schedule_save_session();

        if self.map_open() {
            self.trigger_map_animation();
        }

        if self.devtools_open.get() {
            self.show_devtools();
        }

        if !self.palette_open() && !self.ov.get() {
            if let Some(v) = self.focused_view() {
                v.grab_focus();
            }
        }
    }

    fn focus_column_left(self: &Rc<Self>) {
        let cur_c = self.focus_col.get();
        if cur_c > 0 {
            self.focus_coords(self.focus_row.get(), cur_c - 1);
        }
    }

    fn focus_column_right(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        if cur_r < rows.len() && cur_c + 1 < rows[cur_r].tabs.len() {
            drop(rows);
            self.focus_coords(cur_r, cur_c + 1);
        }
    }

    fn focus_row_up(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        if cur_r > 0 {
            let cur_c = self.focus_col.get();
            let rows = self.rows.borrow();
            let dest_c = cur_c.min(rows[cur_r - 1].tabs.len().saturating_sub(1));
            drop(rows);
            self.focus_coords(cur_r - 1, dest_c);
        }
    }

    fn focus_row_down(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r + 1 < rows.len() {
            let cur_c = self.focus_col.get();
            let dest_c = cur_c.min(rows[cur_r + 1].tabs.len().saturating_sub(1));
            drop(rows);
            self.focus_coords(cur_r + 1, dest_c);
        }
    }

    fn focus_row(self: &Rc<Self>, target_r: usize) {
        let rows = self.rows.borrow();
        if target_r < rows.len() {
            let cur_c = self.focus_col.get();
            let dest_c = cur_c.min(rows[target_r].tabs.len().saturating_sub(1));
            drop(rows);
            self.focus_coords(target_r, dest_c);
        } else if target_r == rows.len() {
            drop(rows);
            self.split_vertical();
        }
    }

    fn focus_tab_index(self: &Rc<Self>, target_c: usize) {
        if self.overview_open() {
            self.exit_overview();
        }
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r < rows.len() && !rows[cur_r].tabs.is_empty() {
            let num_tabs = rows[cur_r].tabs.len();
            if target_c < num_tabs {
                drop(rows);
                self.focus_coords(cur_r, target_c);
            }
        }
    }

    fn focus_last_tab(self: &Rc<Self>) {
        if self.overview_open() {
            self.exit_overview();
        }
        let cur_r = self.focus_row.get();
        let rows = self.rows.borrow();
        if cur_r < rows.len() && !rows[cur_r].tabs.is_empty() {
            let last_c = rows[cur_r].tabs.len() - 1;
            drop(rows);
            self.focus_coords(cur_r, last_c);
        }
    }

    /// Ctrl+Tab / Ctrl+Shift+Tab: cycle tabs inside the current workspace (row), wrapping around.
    fn cycle_tab(self: &Rc<Self>, dir: isize) {
        if self.overview_open() {
            self.exit_overview();
        }
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let n = {
            let rows = self.rows.borrow();
            if cur_r >= rows.len() { return; }
            rows[cur_r].tabs.len()
        };
        if n < 2 { return; }
        let next = (cur_c as isize + dir).rem_euclid(n as isize) as usize;
        self.focus_coords(cur_r, next);
    }

    fn move_column(self: &Rc<Self>, dir: isize) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let mut rows = self.rows.borrow_mut();
        if cur_r >= rows.len() || cur_c >= rows[cur_r].tabs.len() {
            return;
        }
        let Some(target_c) = cur_c.checked_add_signed(dir).filter(|&t| t < rows[cur_r].tabs.len()) else { return };
        rows[cur_r].tabs.swap(cur_c, target_c);
        drop(rows);
        self.sync_strip();
        self.focus_coords(cur_r, target_c);
    }

    fn reorder_tab_in_row(self: &Rc<Self>, r: usize, from_c: usize, to_c: usize) {
        if from_c == to_c {
            return;
        }
        let mut rows = self.rows.borrow_mut();
        if r >= rows.len() || from_c >= rows[r].tabs.len() || to_c >= rows[r].tabs.len() {
            return;
        }
        let tab = rows[r].tabs.remove(from_c);
        rows[r].tabs.insert(to_c, tab);
        drop(rows);
        self.sync_strip();
        self.focus_coords(r, to_c);
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    fn move_tab_to_row(self: &Rc<Self>, target_r: usize) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        if cur_r == target_r {
            return;
        }

        let mut rows = self.rows.borrow_mut();
        if cur_r >= rows.len() || cur_c >= rows[cur_r].tabs.len() {
            return;
        }

        let tab = rows[cur_r].tabs.remove(cur_c);
        let cur_row_empty = rows[cur_r].tabs.is_empty();

        while rows.len() <= target_r {
            rows.push(Row { tabs: Vec::new() });
        }
        rows[target_r].tabs.push(tab);
        let dest_c = rows[target_r].tabs.len() - 1;

        let need_fresh_in_row_0 = cur_row_empty && cur_r == 0;
        drop(rows);

        if need_fresh_in_row_0 {
            let prof_id = self.profiles.borrow().active_profile().id;
            let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
            let view = Self::create_web_view(&session, &prof_id);
            view.load_html(&start_page(), Some("rail://start/"));
            view.set_can_target(!self.ov.get());
            let (container, resize_handle, freeze) = Self::create_tab_widget(&view);
            self.setup_tab_signals(&container, &view, &resize_handle);

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab { container, view, freeze, profile_id: prof_id, custom_width: Cell::new(None), saved_uri: RefCell::new(String::new()), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) });
            drop(rows);
        } else if cur_row_empty && cur_r > 0 {
            let mut rows = self.rows.borrow_mut();
            while rows.len() > 1 && rows.last().map_or(false, |r| r.tabs.is_empty()) {
                rows.pop();
            }
            drop(rows);
        }

        self.sync_strip();
        self.focus_coords(target_r, dest_c);
    }

    fn move_tab_to_row_pos(self: &Rc<Self>, from_r: usize, from_c: usize, to_r: usize, to_c: usize) {
        if from_r == to_r {
            self.reorder_tab_in_row(from_r, from_c, to_c);
            return;
        }

        let mut rows = self.rows.borrow_mut();
        if from_r >= rows.len() || from_c >= rows[from_r].tabs.len() {
            return;
        }

        let tab = rows[from_r].tabs.remove(from_c);
        let cur_row_empty = rows[from_r].tabs.is_empty();

        while rows.len() <= to_r {
            rows.push(Row { tabs: Vec::new() });
        }
        let insert_c = to_c.min(rows[to_r].tabs.len());
        rows[to_r].tabs.insert(insert_c, tab);

        let need_fresh_in_row_0 = cur_row_empty && from_r == 0;
        drop(rows);

        if need_fresh_in_row_0 {
            let prof_id = self.profiles.borrow().active_profile().id;
            let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
            let view = Self::create_web_view(&session, &prof_id);
            view.load_html(&start_page(), Some("rail://start/"));
            view.set_can_target(!self.ov.get());
            let (container, resize_handle, freeze) = Self::create_tab_widget(&view);
            self.setup_tab_signals(&container, &view, &resize_handle);

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab { container, view, freeze, profile_id: prof_id, custom_width: Cell::new(None), saved_uri: RefCell::new(String::new()), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) });
            drop(rows);
        } else if cur_row_empty && from_r > 0 {
            let mut rows = self.rows.borrow_mut();
            while rows.len() > 1 && rows.last().map_or(false, |r| r.tabs.is_empty()) {
                rows.pop();
            }
            drop(rows);
        }

        self.sync_strip();
        self.focus_coords(to_r, insert_c);
        self.schedule_save_session();
    }

    fn toggle_touchpad_mode(self: &Rc<Self>) {
        let new_state = !self.touchpad_mode.get();
        self.touchpad_mode.set(new_state);
        self.tp_switch.set_active(new_state);
        self.schedule_save_session();
        self.show_touchpad_toast(new_state);
    }

    fn show_touchpad_toast(self: &Rc<Self>, enabled: bool) {
        if enabled {
            self.touchpad_toast.set_markup(&format!("<span weight='bold' foreground='#fafafa'>{}</span>  <span foreground='#94a3b8'>· {}</span>", tr("Touchpad Slide: ON"), tr("2-finger swipe to slide tabs")));
        } else {
            self.touchpad_toast.set_markup(&format!("<span weight='bold' foreground='#94a3b8'>{}</span>  <span foreground='#64748b'>· {}</span>", tr("Touchpad Slide: OFF"), tr("Standard page scroll")));
        }
        self.touchpad_toast.set_visible(true);
        let toast = self.touchpad_toast.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(1800), move || {
            toast.set_visible(false);
        });
    }

    fn save_session_internal(&self, sync: bool) {
        let rows = self.rows.borrow();
        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let dir = rail_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.txt");
        let temp_path = dir.join("session.txt.tmp");

        if total_tabs == 0 {
            *self.last_saved_session.borrow_mut() = String::new();
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(&temp_path);
            return;
        }

        let mut out = String::new();
        out.push_str(&format!("focus\t{}\t{}\n", self.focus_row.get(), self.focus_col.get()));
        out.push_str(&format!("sidebar\t{}\n", if self.side_open.get() { 1 } else { 0 }));
        out.push_str(&format!("touchpad\t{}\n", if self.touchpad_mode.get() { 1 } else { 0 }));
        out.push_str(&format!("engine\t{}\n", self.search_engine.borrow()));
        let win_w = self.win.width();
        let win_h = self.win.height();
        let is_max = if self.win.is_maximized() { 1 } else { 0 };
        out.push_str(&format!("win\t{}\t{}\t{}\n", win_w, win_h, is_max));

        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                let is_full = if tab.container.has_css_class("full") { 1 } else { 0 };
                let uri = tab.real_uri();
                if let Some(cw) = tab.custom_width.get() {
                    out.push_str(&format!("tab\t{}\t{}\t{}\t{}\t{}\t{:.1}\n", ri, ci, is_full, tab.profile_id, uri, cw));
                } else {
                    out.push_str(&format!("tab\t{}\t{}\t{}\t{}\t{}\n", ri, ci, is_full, tab.profile_id, uri));
                }
            }
        }
        drop(rows);

        // Optimization: if session content hasn't changed, skip disk write completely (prevents SSD wear)
        if *self.last_saved_session.borrow() == out {
            return;
        }

        // Atomic write via temp file + atomic rename
        let write_ok = if let Ok(mut file) = std::fs::File::create(&temp_path) {
            use std::io::Write;
            if file.write_all(out.as_bytes()).is_ok() {
                if sync {
                    let _ = file.sync_all(); // Flush physical hardware to SSD before exit / reboot
                }
                std::fs::rename(&temp_path, &path).is_ok()
            } else {
                false
            }
        } else {
            false
        };

        if !write_ok {
            let _ = std::fs::write(&path, &out);
        }

        *self.last_saved_session.borrow_mut() = out;
    }

    fn schedule_save_session(self: &Rc<Self>) {
        let current_gen = self.save_session_gen.get().wrapping_add(1);
        self.save_session_gen.set(current_gen);
        let app = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(1000), move || {
            if app.save_session_gen.get() == current_gen {
                app.save_session_internal(false);
            }
        });
    }

    fn save_session_immediate(&self) {
        self.save_session_gen.set(self.save_session_gen.get().wrapping_add(1));
        self.save_session_internal(true);
    }

    fn restore_session(self: &Rc<Self>) -> bool {
        let path = rail_data_dir().join("session.txt");
        let Ok(content) = std::fs::read_to_string(path) else { return false; };
        let state = parse_session_content(&content);
        if state.tabs.is_empty() {
            return false;
        }
        *self.last_saved_session.borrow_mut() = content;

        if let Some((w, h, max)) = state.win_size {
            if w > 400 && h > 300 {
                self.win.set_default_size(w, h);
            }
            if max {
                self.win.maximize();
            }
        }

        if let Some(tp) = state.touchpad_mode {
            self.touchpad_mode.set(tp);
            self.tp_switch.set_active(tp);
        }

        if let Some(engine) = state.search_engine {
            *self.search_engine.borrow_mut() = engine;
        }

        if let Some(open) = state.sidebar_open {
            self.side_open.set(open);
            self.side.set_visible(open);
            self.shell_layout.set_side(if open { SIDE_W as f64 } else { 0.0 });
        }

        let mut tab_items = state.tabs;
        tab_items.sort_by_key(|t| (t.row, t.col));

        let mut max_row = 0;
        for t in &tab_items {
            if t.row > max_row { max_row = t.row; }
        }

        // Clear any existing tabs before loading restored session
        {
            let mut rows = self.rows.borrow_mut();
            for r in rows.iter_mut() {
                for t in r.tabs.drain(..) {
                    self.strip.remove(&t.container);
                }
            }
            rows.clear();
        }

        let mut loaded_tabs: Vec<(usize, Tab)> = Vec::new();
        for item in tab_items {
            let session = self.profiles.borrow_mut().get_or_create_session(&item.profile_id);
            let view = Self::create_web_view(&session, &item.profile_id);
            if item.url.is_empty() || item.url.starts_with("about:") {
                view.load_html(&start_page(), Some("rail://start/"));
            } else {
                view.load_uri(&item.url);
            }
            let (container, resize_handle, freeze) = Self::create_tab_widget(&view);
            if !item.full {
                container.remove_css_class("full");
            }
            self.setup_tab_signals(&container, &view, &resize_handle);
            loaded_tabs.push((item.row, Tab { container, view, freeze, profile_id: item.profile_id, custom_width: Cell::new(item.custom_width), saved_uri: RefCell::new(if item.url.starts_with("http") { item.url.clone() } else { String::new() }), suspended: Cell::new(false), keep_alive: Cell::new(false), last_active: Cell::new(std::time::Instant::now()) }));
        }

        let mut rows = self.rows.borrow_mut();
        while rows.len() <= max_row {
            rows.push(Row { tabs: Vec::new() });
        }
        for (row_idx, tab) in loaded_tabs {
            rows[row_idx].tabs.push(tab);
        }
        drop(rows);

        self.sync_strip();
        self.focus_coords(state.focus.0, state.focus.1);
        true
    }

    fn toggle_full(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        let Some(tab) = rows.get(cur_r).and_then(|r| r.tabs.get(cur_c)) else { return };
        tab.custom_width.set(None);
        if tab.container.has_css_class("full") {
            tab.container.remove_css_class("full");
        } else {
            tab.container.add_css_class("full");
        }
        drop(rows);
        self.layout.set_column_width(cur_r, cur_c, None);
        self.sync_strip();
        self.reveal_focus();
        self.schedule_save_session();
    }

    // ---- sidebar ---------------------------------------------------------

    fn tab_label(t: &Tab) -> String {
        match t.view.title().filter(|s| !s.is_empty()) {
            Some(s) => s.to_string(),
            None => {
                let u = t.real_uri();
                if u.is_empty() || u.starts_with("about:") {
                    tr("New tab").into()
                } else {
                    host_of(&u).unwrap_or_else(|| u.clone())
                }
            }
        }
    }

    fn tab_icon(t: &Tab, size: i32) -> gtk::Widget {
        if t.view.is_loading() {
            let s = gtk::Spinner::new();
            s.set_size_request(size, size);
            s.start();
            return s.upcast();
        }
        if let Some(f) = t.view.favicon() {
            let img = gtk::Image::from_paintable(Some(&f));
            img.set_pixel_size(size);
            return img.upcast();
        }
        let label = Self::tab_label(t);
        let letter = label.chars().find(|c| c.is_alphanumeric()).unwrap_or('R');
        let seed = label.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
        let badge = gtk::Label::new(Some(&letter.to_uppercase().to_string()));
        badge.add_css_class("fav");
        badge.add_css_class(&format!("fav-{}", seed % 8));
        badge.set_size_request(size, size);
        badge.set_valign(gtk::Align::Center);
        badge.set_halign(gtk::Align::Center);
        badge.upcast()
    }

    fn refresh_sidebar(self: &Rc<Self>) {
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        let rows = self.rows.borrow();
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let tab_items_registry: Rc<RefCell<Vec<(usize, usize, gtk::Box)>>> = Rc::new(RefCell::new(Vec::new()));

        for (ri, row) in rows.iter().enumerate() {
            // Workspace Header
            let ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            ws_box.add_css_class("workspace-header");
            ws_box.set_overflow(gtk::Overflow::Hidden);
            if ri == cur_r {
                ws_box.add_css_class("active-workspace");
            }

            let ws_title = gtk::Label::new(Some(&trf("Workspace {}", &[&(ri + 1).to_string()])));
            ws_title.add_css_class("workspace-title");
            ws_title.set_xalign(0.0);
            ws_title.set_hexpand(true);
            ws_title.set_width_chars(1);
            ws_title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            ws_box.append(&ws_title);

            if ri == cur_r {
                let active_badge = gtk::Label::new(Some(tr("ACTIVE")));
                active_badge.add_css_class("workspace-badge");
                ws_box.append(&active_badge);
            }

            let count_text = if row.tabs.len() == 1 {
                tr("1 tab").to_string()
            } else {
                trf("{} tabs", &[&row.tabs.len().to_string()])
            };
            let count_lbl = gtk::Label::new(Some(&count_text));
            count_lbl.add_css_class("workspace-tab-count");
            count_lbl.set_width_chars(1);
            count_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            ws_box.append(&count_lbl);

            let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
            add_btn.add_css_class("btn-ws-add");
            add_btn.set_tooltip_text(Some(&trf("Add tab to Workspace {}", &[&(ri + 1).to_string()])));
            let app = self.clone();
            add_btn.connect_clicked(move |_| {
                app.focus_row(ri);
                app.open_new_tab_palette();
            });
            ws_box.append(&add_btn);

            let app = self.clone();
            let ws_click = gtk::GestureClick::new();
            ws_click.connect_released(move |_, _, _, _| {
                app.focus_row(ri);
            });
            ws_box.add_controller(ws_click);

            self.list.append(&ws_box);

            // Tabs inside this workspace
            for (ci, t) in row.tabs.iter().enumerate() {
                let item = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                item.add_css_class("tabrow");
                item.set_overflow(gtk::Overflow::Hidden);
                if ri == cur_r && ci == cur_c {
                    item.add_css_class("active-tab");
                }
                tab_items_registry.borrow_mut().push((ri, ci, item.clone()));

                let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                content_box.set_hexpand(true);
                content_box.set_overflow(gtk::Overflow::Hidden);
                content_box.append(&Self::tab_icon(t, 16));

                let mgr = self.profiles.borrow();
                if mgr.profiles().len() > 1 {
                    if let Some(p) = mgr.get_profile(&t.profile_id) {
                        let p_dot = gtk::Label::new(None);
                        let fg = p.text_color();
                        p_dot.set_markup(&format!(
                            "<span foreground='{}' font_weight='800'>{}</span>",
                            fg,
                            glib::markup_escape_text(&p.initial_letter())
                        ));
                        p_dot.add_css_class("profile-badge-mini");
                        p_dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
                        p_dot.set_valign(gtk::Align::Center);
                        p_dot.set_halign(gtk::Align::Center);
                        p_dot.set_width_request(18);
                        p_dot.set_height_request(16);
                        p_dot.set_tooltip_text(Some(&trf("Profile: {}", &[&p.display_name()])));
                        content_box.append(&p_dot);
                    }
                }
                drop(mgr);

                let index_lbl = gtk::Label::new(Some(&format!("{}", ci + 1)));
                index_lbl.add_css_class("tab-index");
                index_lbl.set_valign(gtk::Align::Center);
                content_box.append(&index_lbl);

                let title_lbl = gtk::Label::new(Some(&Self::tab_label(t)));
                title_lbl.add_css_class("tab-title");
                title_lbl.set_xalign(0.0);
                title_lbl.set_hexpand(true);
                title_lbl.set_width_chars(1);
                title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                content_box.append(&title_lbl);

                let app = self.clone();
                let drag = gtk::GestureDrag::new();
                drag.set_button(gdk::BUTTON_PRIMARY);

                let item_drag = item.clone();
                let drag_dist = Rc::new(Cell::new(0.0f64));

                drag.connect_drag_begin({
                    let item_drag = item_drag.clone();
                    let drag_dist = drag_dist.clone();
                    move |_gesture, _start_x, _start_y| {
                        drag_dist.set(0.0);
                        item_drag.add_css_class("dragging");
                        item_drag.set_cursor_from_name(Some("grabbing"));
                    }
                });

                let tab_reg_update = tab_items_registry.clone();
                let app_update = self.clone();
                drag.connect_drag_update({
                    let drag_dist = drag_dist.clone();
                    let tab_reg_update = tab_reg_update.clone();
                    let app_update = app_update.clone();
                    move |_gesture, offset_x, offset_y| {
                        let dist = offset_x.hypot(offset_y);
                        drag_dist.set(dist);

                        if dist < 4.0 {
                            let items = tab_reg_update.borrow();
                            for &(_, _, ref b) in items.iter() {
                                b.remove_css_class("drop-target-above");
                                b.remove_css_class("drop-target-below");
                            }
                            return;
                        }

                        let tab_h = 32.0f64;
                        let delta = (offset_y / tab_h).round() as isize;
                        let rows_borrow = app_update.rows.borrow();
                        let cur_tabs_len = rows_borrow.get(ri).map_or(0, |r| r.tabs.len());
                        let total_rows = rows_borrow.len();

                        let target = if delta == 0 {
                            None
                        } else {
                            let new_col_idx = (ci as isize) + delta;
                            if new_col_idx >= 0 && (new_col_idx as usize) < cur_tabs_len {
                                Some((ri, new_col_idx as usize))
                            } else if new_col_idx < 0 && ri > 0 {
                                let tr = ri - 1;
                                let target_len = rows_borrow.get(tr).map_or(0, |r| r.tabs.len());
                                Some((tr, target_len.saturating_sub(1)))
                            } else if new_col_idx >= cur_tabs_len as isize && ri + 1 < total_rows {
                                Some((ri + 1, 0))
                            } else {
                                Some((ri, if delta > 0 { cur_tabs_len.saturating_sub(1) } else { 0 }))
                            }
                        };
                        drop(rows_borrow);

                        let items = tab_reg_update.borrow();
                        for &(r, c, ref b) in items.iter() {
                            if let Some((tr, tc)) = target {
                                if (r, c) == (tr, tc) && (tr, tc) != (ri, ci) {
                                    if delta < 0 {
                                        b.add_css_class("drop-target-above");
                                        b.remove_css_class("drop-target-below");
                                    } else {
                                        b.add_css_class("drop-target-below");
                                        b.remove_css_class("drop-target-above");
                                    }
                                } else {
                                    b.remove_css_class("drop-target-above");
                                    b.remove_css_class("drop-target-below");
                                }
                            } else {
                                b.remove_css_class("drop-target-above");
                                b.remove_css_class("drop-target-below");
                            }
                        }
                    }
                });

                let item_end = item.clone();
                let app_end = app.clone();
                let drag_dist_end = drag_dist.clone();
                let tab_reg_end = tab_items_registry.clone();

                drag.connect_drag_end(move |_gesture, _offset_x, offset_y| {
                    item_end.remove_css_class("dragging");
                    item_end.set_cursor_from_name(Some("grab"));

                    let items = tab_reg_end.borrow();
                    for &(_, _, ref b) in items.iter() {
                        b.remove_css_class("drop-target-above");
                        b.remove_css_class("drop-target-below");
                    }
                    drop(items);

                    let d = drag_dist_end.get();
                    if d < 6.0 {
                        app_end.focus_coords(ri, ci);
                        return;
                    }

                    let tab_h = 32.0f64;
                    let delta = (offset_y / tab_h).round() as isize;
                    let rows_borrow = app_end.rows.borrow();
                    let cur_tabs_len = rows_borrow.get(ri).map_or(0, |r| r.tabs.len());
                    let total_rows = rows_borrow.len();
                    drop(rows_borrow);

                    if delta != 0 {
                        let new_col_idx = (ci as isize) + delta;
                        if new_col_idx >= 0 && (new_col_idx as usize) < cur_tabs_len {
                            app_end.reorder_tab_in_row(ri, ci, new_col_idx as usize);
                        } else if new_col_idx < 0 && ri > 0 {
                            let target_r = ri - 1;
                            let rows_borrow = app_end.rows.borrow();
                            let target_len = rows_borrow.get(target_r).map_or(0, |r| r.tabs.len());
                            drop(rows_borrow);
                            app_end.move_tab_to_row_pos(ri, ci, target_r, target_len);
                        } else if new_col_idx >= cur_tabs_len as isize && ri + 1 < total_rows {
                            let target_r = ri + 1;
                            app_end.move_tab_to_row_pos(ri, ci, target_r, 0);
                        }
                    }
                });

                content_box.add_controller(drag);

                let close = gtk::Button::from_icon_name("window-close-symbolic");
                close.add_css_class("tab-close");
                close.set_valign(gtk::Align::Center);
                close.set_cursor_from_name(Some("pointer"));
                close.set_hexpand(false);
                close.set_halign(gtk::Align::End);
                let (app, v) = (self.clone(), t.view.clone());
                close.connect_clicked(move |_| {
                    app.close_view(&v);
                });

                item.append(&content_box);
                item.append(&close);

                self.list.append(&item);
            }
        }
        drop(rows);

        // Bottom "+ New Workspace" button
        let new_ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        new_ws_box.add_css_class("btn-new-workspace");
        new_ws_box.set_halign(gtk::Align::Fill);
        new_ws_box.set_overflow(gtk::Overflow::Hidden);
        let new_ws_lbl = gtk::Label::new(Some(tr("+ New Workspace (Alt+Shift+V)")));
        new_ws_lbl.set_xalign(0.0);
        new_ws_lbl.set_hexpand(true);
        new_ws_lbl.set_width_chars(1);
        new_ws_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        new_ws_box.append(&new_ws_lbl);
        let app = self.clone();
        let new_ws_click = gtk::GestureClick::new();
        new_ws_click.connect_released(move |_, _, _, _| {
            app.split_vertical();
        });
        new_ws_box.add_controller(new_ws_click);
        self.list.append(&new_ws_box);

        let text = match self.focused_view().and_then(|v| v.uri()) {
            Some(u) if !u.starts_with("about:") => host_of(&u).unwrap_or(u.to_string()),
            _ => tr("Search or enter address").into(),
        };
        self.pill_label.set_text(&text);
    }

    fn toggle_sidebar(self: &Rc<Self>) {
        self.set_sidebar(!self.side_open.get());
    }

    fn set_sidebar(self: &Rc<Self>, open: bool) {
        self.side_open.set(open);
        self.schedule_save_session();
        let epoch = self.side_gen.get().wrapping_add(1);
        self.side_gen.set(epoch);
        if open {
            self.side.set_visible(true);
        }
        let start = self.shell_layout.side();
        let end = if open { SIDE_W as f64 } else { 0.0 };
        let app = self.clone();
        let t0 = Cell::new(None::<i64>);
        self.shell.add_tick_callback(move |shell, clock| {
            if app.side_gen.get() != epoch {
                return glib::ControlFlow::Break;
            }
            let now = clock.frame_time();
            let begin = t0.get().unwrap_or_else(|| {
                t0.set(Some(now));
                now
            });
            let p = ((now - begin) as f64 / 200_000.0).min(1.0);
            let e = 1.0 - (1.0 - p).powi(3);
            app.shell_layout.set_side(start + (end - start) * e);
            shell.queue_allocate();
            if p >= 1.0 {
                if !open {
                    app.side.set_visible(false);
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    // ---- load feedback ---------------------------------------------------

    fn update_load_bar(self: &Rc<Self>) {
        let bar = &self.load_bar;
        let Some(v) = self.focused_view() else {
            bar.set_visible(false);
            return;
        };
        if v.is_loading() {
            bar.set_visible(true);
            bar.set_fraction(v.estimated_load_progress().max(0.08));
        } else if gtk::prelude::WidgetExt::is_visible(bar) {
            bar.set_fraction(1.0);
            let app = self.clone();
            glib::timeout_add_local_once(Duration::from_millis(250), move || {
                if !app.focused_view().is_some_and(|v| v.is_loading()) {
                    app.load_bar.set_visible(false);
                }
            });
        }
    }

    // ---- address palette & profiles --------------------------------------

    fn select_palette_profile(&self, id: &str) {
        *self.selected_palette_profile.borrow_mut() = id.to_string();
        if let Some(p) = self.profiles.borrow().get_profile(id) {
            let fg = p.text_color();
            self.profile_btn_lbl.set_markup(&format!(
                "<span foreground='{}' font_weight='800'>{}</span>",
                fg,
                glib::markup_escape_text(&p.initial_letter())
            ));
            for i in 0..8 {
                self.profile_btn.remove_css_class(&format!("prof-bg-{}", i));
            }
            self.profile_btn.add_css_class(&format!("prof-bg-{}", p.color_idx));
            let persona = ProfilePersona::for_id(&p.id);
            self.profile_btn.set_tooltip_text(Some(&trf("Profile: {} [{}] (100% Isolated)", &[&p.display_name(), &persona.summary()])));
        }
    }

    fn rebuild_profile_popover(self: &Rc<Self>) {
        let box_menu = gtk::Box::new(gtk::Orientation::Vertical, 4);
        box_menu.set_margin_top(4);
        box_menu.set_margin_bottom(4);
        box_menu.set_margin_start(4);
        box_menu.set_margin_end(4);

        let title = gtk::Label::new(Some(tr("Select Profile")));
        title.add_css_class("profile-popover-title");
        title.set_xalign(0.0);
        title.set_margin_start(8);
        title.set_margin_bottom(4);
        box_menu.append(&title);

        let (profiles, selected_id) = {
            let mgr = self.profiles.borrow();
            (mgr.profiles().to_vec(), self.selected_palette_profile.borrow().clone())
        };

        for p in profiles {
            let row_btn = gtk::Button::new();
            row_btn.add_css_class("profile-item-btn");
            if p.id == selected_id {
                row_btn.add_css_class("selected");
            }

            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);

            let dot = gtk::Label::new(None);
            let fg = p.text_color();
            dot.set_markup(&format!(
                "<span foreground='{}' font_weight='800'>{}</span>",
                fg,
                glib::markup_escape_text(&p.initial_letter())
            ));
            dot.add_css_class("profile-item-dot");
            dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
            dot.set_valign(gtk::Align::Center);
            dot.set_halign(gtk::Align::Center);
            dot.set_width_request(24);
            dot.set_height_request(24);

            let persona = ProfilePersona::for_id(&p.id);
            let name_col = gtk::Box::new(gtk::Orientation::Vertical, 1);
            name_col.set_hexpand(true);

            let name_lbl = gtk::Label::new(Some(&p.display_name()));
            name_lbl.add_css_class("profile-item-name");
            name_lbl.set_xalign(0.0);

            let persona_lbl = gtk::Label::new(Some(&persona.summary()));
            persona_lbl.add_css_class("profile-item-persona");
            persona_lbl.set_xalign(0.0);

            name_col.append(&name_lbl);
            name_col.append(&persona_lbl);

            row_box.append(&dot);
            row_box.append(&name_col);

            if p.id == selected_id {
                let check = gtk::Image::from_icon_name("object-select-symbolic");
                check.add_css_class("profile-item-check");
                row_box.append(&check);
            }

            row_btn.set_child(Some(&row_box));

            let app = self.clone();
            let pid = p.id.clone();
            row_btn.connect_clicked(move |_| {
                let is_new = app.palette_for_new_tab.get();
                app.select_palette_profile(&pid);
                if !is_new {
                    app.profiles.borrow_mut().set_active(&pid);
                    app.switch_focused_tab_profile(&pid);
                }
                app.rebuild_profile_popover();
                app.profile_popover.popdown();
                app.entry.grab_focus();
            });

            box_menu.append(&row_btn);
        }

        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep.set_margin_top(6);
        sep.set_margin_bottom(6);
        box_menu.append(&sep);

        let manage_btn = gtk::Button::new();
        manage_btn.add_css_class("profile-item-btn");
        let manage_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let manage_icon = gtk::Image::from_icon_name("preferences-system-symbolic");
        let manage_lbl = gtk::Label::new(Some(tr("Manage Profiles in Settings (Alt+P)")));
        manage_lbl.add_css_class("profile-item-name");
        manage_lbl.set_xalign(0.0);
        manage_box.append(&manage_icon);
        manage_box.append(&manage_lbl);
        manage_btn.set_child(Some(&manage_box));

        let app_c = self.clone();
        manage_btn.connect_clicked(move |_| {
            app_c.profile_popover.popdown();
            app_c.open_settings_tab(SettingsTab::Profiles);
        });
        box_menu.append(&manage_btn);

        self.profile_popover.set_child(Some(&box_menu));
    }

    fn update_palette_size(&self) {
        let win_w = self.win.width();
        let target_w = if win_w < 900 {
            (win_w - 48).clamp(280, 520)
        } else {
            580
        };
        self.palette.set_size_request(target_w, -1);
        let win_h = self.win.height();
        let target_top = if win_h < 600 { 50 } else { 90 };
        self.palette.set_margin_top(target_top);
    }

    fn open_new_tab_palette(self: &Rc<Self>) {
        self.exit_overview();
        self.palette_for_new_tab.set(true);

        self.entry.set_text("");

        let prof_id = self.profiles.borrow().active_id().to_string();
        self.select_palette_profile(&prof_id);
        self.rebuild_profile_popover();

        self.palette_suggestions_box.set_visible(false);
        self.clear_suggestions_ui();
        *self.current_suggestions.borrow_mut() = Vec::new();
        self.selected_suggestion_idx.set(None);
        self.last_entry_len.set(0);

        self.update_palette_size();
        self.palette.set_visible(true);
        self.entry.grab_focus();
    }

    fn show_palette(self: &Rc<Self>) {
        self.palette_for_new_tab.set(false);
        let uri = self.focused_view().and_then(|v| v.uri()).filter(|u| !u.starts_with("about:"));
        self.entry.set_text(uri.as_deref().unwrap_or(""));

        let prof_id = self.profiles.borrow().active_id().to_string();
        self.select_palette_profile(&prof_id);
        self.rebuild_profile_popover();

        self.palette_suggestions_box.set_visible(false);
        self.clear_suggestions_ui();
        *self.current_suggestions.borrow_mut() = Vec::new();
        self.selected_suggestion_idx.set(None);
        self.last_entry_len.set(self.entry.text().chars().count());

        self.update_palette_size();
        self.palette.set_visible(true);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    fn hide_palette(&self) {
        self.palette_suggestions_box.set_visible(false);
        self.clear_suggestions_ui();
        *self.current_suggestions.borrow_mut() = Vec::new();
        self.selected_suggestion_idx.set(None);
        self.profile_popover.popdown();
        self.palette.set_visible(false);
        if !self.profiles_modal_open() {
            if let Some(v) = self.focused_view() {
                v.grab_focus();
            }
        }
    }

    fn clear_suggestions_ui(&self) {
        while let Some(child) = self.palette_suggestions_box.first_child() {
            self.palette_suggestions_box.remove(&child);
        }
    }

    fn render_suggestions_ui(self: &Rc<Self>, suggestions: &[HistorySuggestion]) {
        self.clear_suggestions_ui();
        if suggestions.is_empty() {
            self.palette_suggestions_box.set_visible(false);
            return;
        }

        let sel_idx = self.selected_suggestion_idx.get();

        for (idx, sug) in suggestions.iter().enumerate() {
            let item_btn = gtk::Button::new();
            item_btn.add_css_class("palette-sug-item");
            if Some(idx) == sel_idx {
                item_btn.add_css_class("selected");
            }

            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row_box.set_valign(gtk::Align::Center);

            let icon_name = match sug.kind {
                SuggestionKind::History => "document-open-recent-symbolic",
                SuggestionKind::Popular => "starred-symbolic",
                SuggestionKind::Search => "system-search-symbolic",
            };
            let icon = gtk::Image::from_icon_name(icon_name);
            icon.add_css_class("palette-sug-icon");
            row_box.append(&icon);

            let title_lbl = gtk::Label::new(Some(&sug.display_title));
            title_lbl.add_css_class("palette-sug-title");
            title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title_lbl.set_xalign(0.0);
            row_box.append(&title_lbl);

            if sug.kind != SuggestionKind::Search {
                let url_lbl = gtk::Label::new(Some(&sug.display_url));
                url_lbl.add_css_class("palette-sug-url");
                url_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                url_lbl.set_xalign(0.0);
                url_lbl.set_hexpand(true);
                row_box.append(&url_lbl);
            } else {
                title_lbl.set_hexpand(true);
            }

            let badge_text = match sug.kind {
                SuggestionKind::History => tr("History"),
                SuggestionKind::Popular => tr("Top"),
                SuggestionKind::Search => tr("Search"),
            };
            let badge = gtk::Label::new(Some(badge_text));
            badge.add_css_class("palette-sug-badge");
            badge.set_halign(gtk::Align::End);
            row_box.append(&badge);

            item_btn.set_child(Some(&row_box));

            let app = self.clone();
            let target = sug.target_url.clone();
            item_btn.connect_clicked(move |_| {
                app.navigate(&target);
            });

            self.palette_suggestions_box.append(&item_btn);
        }

        self.palette_suggestions_box.set_visible(true);
    }

    fn handle_entry_changed(self: &Rc<Self>) {
        if self.is_autofilling.get() {
            return;
        }

        let text = self.entry.text().to_string();
        let cur_len = text.chars().count();
        let prev_len = self.last_entry_len.get();
        self.last_entry_len.set(cur_len);

        let is_typing_forward = cur_len > prev_len && !self.delete_key.get();
        self.selected_suggestion_idx.set(None);

        if text.trim().is_empty() {
            self.palette_suggestions_box.set_visible(false);
            self.clear_suggestions_ui();
            *self.current_suggestions.borrow_mut() = Vec::new();
            return;
        }

        let suggestions = self.history.borrow().suggest(&text);
        if suggestions.is_empty() {
            self.palette_suggestions_box.set_visible(false);
            self.clear_suggestions_ui();
            *self.current_suggestions.borrow_mut() = Vec::new();
            return;
        }

        // 1. Inline autofill if typing forward (not on backspace/deletion)
        if is_typing_forward {
            if let Some(first) = suggestions.first() {
                if first.kind != SuggestionKind::Search {
                    let clean_q = text.trim().to_lowercase();
                    let autofill = &first.autofill_text;
                    let autofill_lower = autofill.to_lowercase();
                    if autofill_lower.starts_with(&clean_q) && autofill.chars().count() > cur_len {
                        self.is_autofilling.set(true);
                        self.entry.set_text(autofill);
                        self.entry.select_region(cur_len as i32, -1);
                        self.is_autofilling.set(false);
                        // Foydalanuvchi yozgan uzunlikni eslaymiz (to'ldirilgan uzunlikni emas),
                        // aks holda keyingi harf "o'chirish" deb hisoblanib to'ldirish bir harf oralatib ishlardi.
                        self.last_entry_len.set(cur_len);
                    }
                }
            }
        }

        // 2. Render suggestions UI
        self.render_suggestions_ui(&suggestions);
        *self.current_suggestions.borrow_mut() = suggestions;
    }

    fn move_suggestion_selection(self: &Rc<Self>, delta: isize) -> bool {
        let sugs = self.current_suggestions.borrow();
        if sugs.is_empty() {
            return false;
        }
        let total = sugs.len() as isize;
        let cur = self.selected_suggestion_idx.get();

        let new_idx = match cur {
            None => {
                if delta > 0 {
                    0
                } else {
                    (total - 1) as usize
                }
            }
            Some(i) => {
                let next = i as isize + delta;
                if next < 0 {
                    (total - 1) as usize
                } else if next >= total {
                    0
                } else {
                    next as usize
                }
            }
        };
        drop(sugs);

        self.selected_suggestion_idx.set(Some(new_idx));

        let sugs = self.current_suggestions.borrow();
        if let Some(s) = sugs.get(new_idx) {
            self.is_autofilling.set(true);
            let display_text = if s.kind == SuggestionKind::Search {
                s.autofill_text.clone()
            } else {
                s.display_url.clone()
            };
            self.entry.set_text(&display_text);
            self.entry.set_position(-1);
            self.last_entry_len.set(display_text.chars().count());
            self.is_autofilling.set(false);
        }
        let sugs_clone = sugs.clone();
        drop(sugs);

        self.render_suggestions_ui(&sugs_clone);
        true
    }

    fn accept_autofill_or_next_suggestion(self: &Rc<Self>) -> bool {
        if let Some((start, end)) = self.entry.selection_bounds() {
            if start != end {
                self.entry.set_position(-1);
                self.entry.select_region(-1, -1);
                return true;
            }
        }
        self.move_suggestion_selection(1)
    }

    fn cancel_palette(self: &Rc<Self>) {
        let is_new_tab = self.palette_for_new_tab.get();
        self.palette_for_new_tab.set(false);
        self.hide_palette();
        if !is_new_tab {
            let view = self.focused_view();
            let fresh = self.fresh.borrow().clone();
            if let (Some(v), Some(f)) = (view, fresh) {
                let total_tabs: usize = self.rows.borrow().iter().map(|r| r.tabs.len()).sum();
                if v == f && total_tabs > 1 {
                    self.close_view(&v);
                }
            }
        }
    }

    fn switch_focused_tab_profile(self: &Rc<Self>, target_profile_id: &str) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        let tab_info = rows.get(cur_r).and_then(|r| r.tabs.get(cur_c)).map(|t| (t.profile_id.clone(), t.view.uri()));
        drop(rows);

        let Some((current_pid, current_uri)) = tab_info else { return };
        if current_pid == target_profile_id {
            return;
        }

        let session = self.profiles.borrow_mut().get_or_create_session(target_profile_id);
        let new_view = Self::create_web_view(&session, target_profile_id);
        new_view.set_can_target(!self.ov.get());
        self.setup_view_signals(&new_view);

        let mut rows = self.rows.borrow_mut();
        if let Some(row) = rows.get_mut(cur_r) {
            if let Some(tab) = row.tabs.get_mut(cur_c) {
                tab.container.remove(&tab.view);
                tab.container.prepend(&new_view);
                new_view.add_css_class("tab");
                new_view.set_overflow(gtk::Overflow::Hidden);
                new_view.set_vexpand(true);
                new_view.set_hexpand(true);
                tab.view = new_view.clone();
                tab.profile_id = target_profile_id.to_string();
            }
        }
        drop(rows);

        if let Some(uri) = current_uri {
            if uri.starts_with("about:") || uri.is_empty() {
                new_view.load_html(&start_page(), Some("rail://start/"));
            } else {
                new_view.load_uri(&uri);
            }
        } else {
            new_view.load_html(&start_page(), Some("rail://start/"));
        }

        self.refresh_sidebar();
        self.schedule_save_session();
        self.focus_coords(cur_r, cur_c);
    }

    fn navigate(self: &Rc<Self>, text: &str) {
        let is_new_tab = self.palette_for_new_tab.get();
        self.palette_for_new_tab.set(false);

        if !text.trim().is_empty() {
            let target_profile_id = self.selected_palette_profile.borrow().clone();
            let uri = to_uri(text, &self.search_engine.borrow());
            self.history.borrow_mut().record_visit(&uri, "");

            if is_new_tab {
                self.profiles.borrow_mut().set_active(&target_profile_id);
                self.new_tab_with_url_and_profile(&uri, &target_profile_id);
            } else {
                let cur_r = self.focus_row.get();
                let cur_c = self.focus_col.get();

                let needs_view_recreation = {
                    let rows = self.rows.borrow();
                    rows.get(cur_r)
                        .and_then(|r| r.tabs.get(cur_c))
                        .map(|t| t.profile_id != target_profile_id)
                        .unwrap_or(false)
                };

                if needs_view_recreation {
                    self.profiles.borrow_mut().set_active(&target_profile_id);
                    self.switch_focused_tab_profile(&target_profile_id);
                }

                if let Some(v) = self.focused_view() {
                    self.pill_label.set_text(&host_of(&uri).unwrap_or(uri.clone()));
                    v.load_uri(&uri);
                    *self.fresh.borrow_mut() = None;
                    self.refresh_sidebar();
                    self.schedule_save_session();
                }
            }
        }
        self.hide_palette();
    }

    // ---- floating settings modal (Alt+Shift+S / Alt+P / Alt+I) --------

    fn settings_modal_open(&self) -> bool {
        self.settings_modal.is_visible()
    }

    fn open_settings_tab(self: &Rc<Self>, tab: SettingsTab) {
        if self.palette_open() {
            self.hide_palette();
        }
        self.current_settings_tab.set(tab);
        let tab_name = match tab {
            SettingsTab::General => "general",
            SettingsTab::Profiles => "profiles",
            SettingsTab::Data => "data",
            SettingsTab::Privacy => "privacy",
            SettingsTab::Customize => "customize",
            SettingsTab::Update => "update",
            SettingsTab::About => "about",
        };
        self.settings_stack.set_visible_child_name(tab_name);

        for (btn_tab, btn) in self.settings_tab_buttons.borrow().iter() {
            if *btn_tab == tab {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }

        match tab {
            SettingsTab::General => {
                self.tp_switch.set_active(self.touchpad_mode.get());
            }
            SettingsTab::Profiles => {
                self.rebuild_profiles_modal_list();
                self.new_profile_entry.set_text("");
                self.select_new_profile_color(0);
                self.update_create_profile_btn_state();
                self.new_profile_entry.grab_focus();
            }
            SettingsTab::Data => {
                self.refresh_data_modal_info();
                let timestamp = glib::DateTime::now_local()
                    .ok()
                    .and_then(|d| d.format("%Y%m%d_%H%M%S").ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "backup".to_string());
                let default_export = glib::home_dir().join(format!("rail_backup_{}.tar.gz", timestamp));
                self.data_export_entry.set_text(&default_export.to_string_lossy());
                self.data_modal_status_lbl.set_text("");
                self.data_export_entry.grab_focus();
            }
            SettingsTab::Customize => {
                self.reload_css_editor();
            }
            _ => {}
        }

        self.settings_modal.set_visible(true);
    }

    fn reload_css_editor(&self) {
        self.css_buffer.set_text(&load_html_css());
    }

    fn toggle_settings_modal(self: &Rc<Self>) {
        if self.settings_modal_open() {
            self.hide_settings_modal();
        } else {
            self.open_settings_tab(self.current_settings_tab.get());
        }
    }

    fn hide_settings_modal(&self) {
        self.settings_modal.set_visible(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    fn profiles_modal_open(&self) -> bool {
        self.settings_modal.is_visible() && self.current_settings_tab.get() == SettingsTab::Profiles
    }

    fn toggle_profiles_modal(self: &Rc<Self>) {
        if self.settings_modal_open() && self.current_settings_tab.get() == SettingsTab::Profiles {
            self.hide_settings_modal();
        } else {
            self.open_settings_tab(SettingsTab::Profiles);
        }
    }

    fn toggle_data_modal(self: &Rc<Self>) {
        if self.settings_modal_open() && self.current_settings_tab.get() == SettingsTab::Data {
            self.hide_settings_modal();
        } else {
            self.open_settings_tab(SettingsTab::Data);
        }
    }

    #[allow(dead_code)]
    fn clear_history_action(self: &Rc<Self>) {
        self.history.borrow_mut().clear();
        self.data_modal_status_lbl.set_markup(&format!(
            "<span foreground='#4ade80'><b>{}</b> {}</span>",
            tr("History Cleared:"), tr("All Omnibar history and suggestions have been purged.")
        ));
    }

    #[allow(dead_code)]
    fn clear_cache_action(self: &Rc<Self>) {
        let dir = rail_data_dir();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("profile_")).unwrap_or(false) {
                    let cache_dir = p.join("cache");
                    let _ = std::fs::remove_dir_all(&cache_dir);
                }
            }
        }
        self.data_modal_status_lbl.set_markup(&format!(
            "<span foreground='#4ade80'><b>{}</b> {}</span>",
            tr("Cache Cleared:"), tr("Profile network disk caches purged successfully.")
        ));
    }

    fn refresh_data_modal_info(&self) {
        let dir = rail_data_dir();
        let rows = self.rows.borrow();
        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let ws_count = rows.len();
        drop(rows);
        let prof_count = self.profiles.borrow().profiles().len();

        let info_text = trf(
            "Directory: {}\nStatus:    {} open tab(s) in {} workspace(s) | {} profiles (cookies/logins active)",
            &[&dir.display().to_string(), &total_tabs.to_string(), &ws_count.to_string(), &prof_count.to_string()],
        );
        self.data_modal_info_lbl.set_text(&info_text);
    }

    fn show_password_dialog<F>(
        &self,
        title: &'static str,
        prompt_text: &'static str,
        btn_label: &'static str,
        need_confirm: bool,
        on_confirm: F,
    ) where
        F: Fn(String) + 'static,
    {
        let dialog = gtk::Window::builder()
            .title(tr(title))
            .modal(true)
            .transient_for(&self.win)
            .default_width(380)
            .resizable(false)
            .build();
        dialog.add_css_class("settings-modal-window");

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(18);
        vbox.set_margin_end(18);

        let lbl_title = gtk::Label::new(Some(tr(title)));
        lbl_title.add_css_class("settings-card-title");
        lbl_title.set_xalign(0.0);
        vbox.append(&lbl_title);

        let lbl_desc = gtk::Label::new(Some(tr(prompt_text)));
        lbl_desc.add_css_class("data-section-desc");
        lbl_desc.set_xalign(0.0);
        lbl_desc.set_wrap(true);
        vbox.append(&lbl_desc);

        let entry_pass = gtk::PasswordEntry::new();
        entry_pass.add_css_class("profile-entry-box");
        entry_pass.set_show_peek_icon(true);
        entry_pass.set_placeholder_text(Some(tr("Password")));
        vbox.append(&entry_pass);

        let entry_confirm = if need_confirm {
            let e = gtk::PasswordEntry::new();
            e.add_css_class("profile-entry-box");
            e.set_show_peek_icon(true);
            e.set_placeholder_text(Some(tr("Confirm password (optional)")));
            vbox.append(&e);
            Some(e)
        } else {
            None
        };

        let err_lbl = gtk::Label::new(None);
        err_lbl.add_css_class("data-status-lbl");
        err_lbl.set_xalign(0.0);
        err_lbl.set_visible(false);
        vbox.append(&err_lbl);

        let btns_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        btns_row.set_halign(gtk::Align::End);
        btns_row.set_margin_top(8);

        let btn_cancel = gtk::Button::with_label(tr("Cancel"));
        btn_cancel.add_css_class("btn-data-action");

        let btn_ok = gtk::Button::with_label(tr(btn_label));
        btn_ok.add_css_class("btn-data-primary");

        btns_row.append(&btn_cancel);
        btns_row.append(&btn_ok);
        vbox.append(&btns_row);

        dialog.set_child(Some(&vbox));

        let d = dialog.clone();
        btn_cancel.connect_clicked(move |_| d.close());

        let d = dialog.clone();
        let on_confirm = std::rc::Rc::new(on_confirm);
        let ep = entry_pass.clone();
        let ec = entry_confirm.clone();
        let el = err_lbl.clone();
        let do_submit = move || {
            let p1 = ep.text().to_string();
            // Spec: 1-inputda kamida 1 belgi bo'lsa shifrlash ishlasin; 2-input bo'sh bo'lishi mumkin
            if p1.is_empty() {
                el.set_markup(&format!("<span foreground='#f87171'>{}</span>", tr("Password cannot be empty.")));
                el.set_visible(true);
                return;
            }
            if let Some(ref c) = ec {
                let p2 = c.text().to_string();
                if !p2.is_empty() && p1 != p2 {
                    el.set_markup(&format!("<span foreground='#f87171'>{}</span>", tr("Passwords do not match.")));
                    el.set_visible(true);
                    return;
                }
            }
            d.close();
            on_confirm(p1);
        };

        let ds1 = do_submit.clone();
        btn_ok.connect_clicked(move |_| ds1());

        // Enter on 1st field -> focus 2nd if present, else submit
        if let Some(ref conf) = entry_confirm {
            let conf2 = conf.clone();
            entry_pass.connect_activate(move |_| {
                conf2.grab_focus();
            });
            let ds3 = do_submit.clone();
            conf.connect_activate(move |_| ds3());
        } else {
            let ds2 = do_submit.clone();
            entry_pass.connect_activate(move |_| ds2());
        }

        dialog.present();
        entry_pass.grab_focus();
    }

    fn prompt_and_execute_sync(self: &Rc<Self>) {
        let app = self.clone();
        self.show_password_dialog(
            "Encryption Password",
            "Set a password to encrypt your data. Without this password, no one can restore or access your data.",
            "Encrypt & Sync",
            true,
            move |password| {
                app.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#38bdf8'><b>{}</b></span>",
                    tr("Syncing: Encrypting and uploading data to cloud...")
                ));

                app.save_session_immediate();
                let mut s = load_app_settings();
                s.search_engine = app.search_engine.borrow().clone();
                s.touchpad_mode = app.touchpad_mode.get();
                save_app_settings(&s);

                let data_dir = rail_data_dir();
                let server_url = sync::sync_server_url();
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let res = (|| -> Result<sync::SyncResult, String> {
                        let cipher_b64 = sync::pack_and_encrypt(&data_dir, &password)?;
                        sync::upload_sync_payload(&cipher_b64, &server_url)
                    })();
                    let _ = tx.send(res);
                });

                let app_check = app.clone();
                glib::timeout_add_local(Duration::from_millis(50), move || {
                    if let Ok(res) = rx.try_recv() {
                        match res {
                            Ok(r) => {
                                app_check.sync_url_result_entry.set_text(&r.url);
                                app_check.win.clipboard().set_text(&r.url);
                                app_check.data_modal_status_lbl.set_markup(&format!(
                                    "<span foreground='#4ade80'><b>{}</b> {} ({})</span>",
                                    tr("Sync Successful:"),
                                    tr("URL copied to clipboard! Remember your password to restore."),
                                    glib::markup_escape_text(&r.url)
                                ));
                            }
                            Err(e) => {
                                app_check.data_modal_status_lbl.set_markup(&format!(
                                    "<span foreground='#f87171'><b>{}</b> {}</span>",
                                    tr("Sync Failed:"),
                                    glib::markup_escape_text(&e)
                                ));
                            }
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            },
        );
    }

    fn prompt_and_execute_restore(self: &Rc<Self>) {
        let raw = self.sync_restore_entry.text().to_string();
        if raw.trim().is_empty() {
            self.data_modal_status_lbl.set_markup(&format!(
                "<span foreground='#f87171'><b>{}</b> {}</span>",
                tr("Error:"),
                tr("Please enter a valid Sync URL or ID.")
            ));
            return;
        }

        let app = self.clone();
        self.show_password_dialog(
            "Enter Decryption Password",
            "Enter the password used to encrypt this backup:",
            "Decrypt & Restore",
            false,
            move |password| {
                app.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#38bdf8'><b>{}</b></span>",
                    tr("Restoring: Downloading and decrypting cloud data...")
                ));

                let raw_input = raw.clone();
                let server_url = sync::sync_server_url();
                let rand_id = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 10000000).to_string();
                let temp_tar = std::env::temp_dir().join(format!("rail_cloud_restore_{}.tar.gz", rand_id));
                let temp_tar_thread = temp_tar.clone();
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let res = sync::download_and_decrypt(&raw_input, &password, &server_url, &temp_tar_thread);
                    let _ = tx.send(res);
                });

                let app_check = app.clone();
                glib::timeout_add_local(Duration::from_millis(50), move || {
                    if let Ok(res) = rx.try_recv() {
                        match res {
                            Ok(()) => {
                                let imp_res = import_data(&temp_tar);
                                let _ = std::fs::remove_file(&temp_tar);
                                match imp_res {
                                    Ok(()) => {
                                        let imported_settings = load_app_settings();
                                        *app_check.search_engine.borrow_mut() = imported_settings.search_engine.clone();
                                        app_check.touchpad_mode.set(imported_settings.touchpad_mode);
                                        app_check.tp_switch.set_active(imported_settings.touchpad_mode);
                                        reload_customize_from_disk();
                                        app_check.reload_css_editor();
                                        *app_check.profiles.borrow_mut() = ProfileManager::new(rail_data_dir());
                                        app_check.rebuild_profiles_modal_list();
                                        let active_id = app_check.profiles.borrow().active_profile().id;
                                        app_check.select_palette_profile(&active_id);
                                        app_check.restore_session();
                                        app_check.refresh_sidebar();
                                        app_check.refresh_data_modal_info();
                                        app_check.data_modal_status_lbl.set_markup(&format!(
                                            "<span foreground='#4ade80'><b>{}</b> {}</span>",
                                            tr("Restore Successful:"),
                                            tr("All workspaces, tabs, accounts & cookies restored!")
                                        ));
                                    }
                                    Err(e) => {
                                        app_check.data_modal_status_lbl.set_markup(&format!(
                                            "<span foreground='#f87171'><b>{}</b> {}</span>",
                                            tr("Restore Failed:"),
                                            glib::markup_escape_text(&e)
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = std::fs::remove_file(&temp_tar);
                                if e == "INCORRECT_PASSWORD" {
                                    app_check.data_modal_status_lbl.set_markup(&format!(
                                        "<span foreground='#f87171'><b>{}</b> {}</span>",
                                        tr("Error:"),
                                        tr("Incorrect password! Data cannot be restored without the correct password.")
                                    ));
                                } else {
                                    app_check.data_modal_status_lbl.set_markup(&format!(
                                        "<span foreground='#f87171'><b>{}</b> {}</span>",
                                        tr("Restore Failed:"),
                                        glib::markup_escape_text(&e)
                                    ));
                                }
                            }
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            },
        );
    }

    fn execute_export_action(self: &Rc<Self>) {
        self.save_session_immediate();
        let mut s = load_app_settings();
        s.search_engine = self.search_engine.borrow().clone();
        s.touchpad_mode = self.touchpad_mode.get();
        save_app_settings(&s);
        let path_str = self.data_export_entry.text().to_string();
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            self.data_modal_status_lbl.set_markup(&format!("<span foreground='#f87171'><b>{}</b> {}</span>", tr("Error:"), tr("Please specify an export destination file.")));
            return;
        }

        match export_data(&path) {
            Ok(()) => {
                let size_str = std::fs::metadata(&path)
                    .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
                    .unwrap_or_else(|_| tr("saved").to_string());
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#4ade80'><b>{}</b> {} ({})</span>",
                    tr("Export Successful:"), glib::markup_escape_text(&path.to_string_lossy()), size_str
                ));
            }
            Err(e) => {
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#f87171'><b>{}</b> {}</span>",
                    tr("Export Failed:"), glib::markup_escape_text(&e)
                ));
            }
        }
    }

    fn execute_import_action(self: &Rc<Self>) {
        let path_str = self.data_import_entry.text().to_string();
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() || !path.exists() {
            self.data_modal_status_lbl.set_markup(&format!("<span foreground='#f87171'><b>{}</b> {}</span>", tr("Error:"), tr("Please select a valid .tar.gz backup file.")));
            return;
        }

        match import_data(&path) {
            Ok(()) => {
                // Reload settings from imported settings.json
                let imported_settings = load_app_settings();
                *self.search_engine.borrow_mut() = imported_settings.search_engine.clone();
                self.touchpad_mode.set(imported_settings.touchpad_mode);
                self.tp_switch.set_active(imported_settings.touchpad_mode);

                // Reload profile manager from imported profiles.txt
                reload_customize_from_disk();
                self.reload_css_editor();
                *self.profiles.borrow_mut() = ProfileManager::new(rail_data_dir());
                self.rebuild_profiles_modal_list();
                let active_id = self.profiles.borrow().active_profile().id;
                self.select_palette_profile(&active_id);

                // Reload session tabs
                self.restore_session();
                self.refresh_sidebar();
                self.refresh_data_modal_info();

                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#4ade80'><b>{}</b> {}</span>",
                    tr("Import Successful:"), tr("All workspaces, tabs, accounts & cookies restored!")
                ));
            }
            Err(e) => {
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#f87171'><b>{}</b> {}</span>",
                    tr("Import Failed:"), glib::markup_escape_text(&e)
                ));
            }
        }
    }

    // ---- Minimap / Roadmap (Alt+M) ----

    fn map_open(&self) -> bool {
        self.map_container.is_visible()
    }

    fn toggle_map(self: &Rc<Self>) {
        if self.map_open() {
            self.hide_map();
        } else {
            self.show_map();
        }
    }

    fn show_map(self: &Rc<Self>) {
        let rows = self.rows.borrow();
        let (cx, cy) = self.compute_active_tab_center(&rows);
        let (rx, ry, rw, rh) = self.compute_active_tab_rect(&rows);
        drop(rows);

        {
            let mut st = self.map_state.borrow_mut();
            st.cam_x = cx;
            st.cam_y = cy;
            st.offset_x = 0.0;
            st.offset_y = 0.0;
            st.target_offset_x = 0.0;
            st.target_offset_y = 0.0;
            st.zoom = 1.0;
            st.target_zoom = 1.0;
            st.user_panned = false;
            st.vel_x = 0.0;
            st.vel_y = 0.0;
            st.cursor_x = rx;
            st.cursor_y = ry;
            st.cursor_w = rw;
            st.cursor_h = rh;
            st.cursor_initialized = true;
            st.animating = false;
        }
        self.map_location_btn.set_visible(false);
        self.map_container.set_visible(true);
        self.map_drawing_area.queue_draw();
    }

    fn hide_map(&self) {
        self.map_container.set_visible(false);
    }

    fn update_map_location_btn_visibility(&self) {
        let st = self.map_state.borrow();
        let width = (self.map_drawing_area.width() as f64).max(380.0);
        let height = (self.map_drawing_area.height() as f64).max(260.0);
        let active_sx = width / 2.0 - st.target_offset_x * st.zoom;
        let active_sy = height / 2.0 - st.target_offset_y * st.zoom;
        let off_screen = active_sx < 20.0 || active_sx > width - 20.0 || active_sy < 20.0 || active_sy > height - 20.0;
        let far_from_center = (st.target_offset_x * st.zoom).hypot(st.target_offset_y * st.zoom) > 40.0;
        let should_show = st.user_panned && (off_screen || far_from_center);
        self.map_location_btn.set_visible(should_show);
    }

    fn minimap_tab_width(&self, tab: &Tab) -> f64 {
        if let Some(cw) = tab.custom_width.get() {
            let vw = self.strip.width().max(600) as f64;
            (cw / vw * MAP_TAB_W_FULL).clamp(MAP_TAB_W * 0.7, MAP_TAB_W_FULL)
        } else if tab.container.has_css_class("full") {
            MAP_TAB_W_FULL
        } else {
            MAP_TAB_W
        }
    }

    fn compute_active_tab_center(&self, rows: &[Row]) -> (f64, f64) {
        let fr = self.focus_row.get();
        let fc = self.focus_col.get();
        if fr < rows.len() {
            let row = &rows[fr];
            if fc < row.tabs.len() {
                let y = fr as f64 * (MAP_TAB_H + MAP_GAP_Y);
                let mut x = 0.0;
                for (i, tab) in row.tabs.iter().enumerate() {
                    let w = self.minimap_tab_width(tab);
                    if i == fc {
                        return (x + w / 2.0, y + MAP_TAB_H / 2.0);
                    }
                    x += w + MAP_GAP_X;
                }
            }
        }
        (0.0, 0.0)
    }

    fn compute_active_tab_rect(&self, rows: &[Row]) -> (f64, f64, f64, f64) {
        let fr = self.focus_row.get();
        let fc = self.focus_col.get();
        if fr < rows.len() {
            let row = &rows[fr];
            if fc < row.tabs.len() {
                let y = fr as f64 * (MAP_TAB_H + MAP_GAP_Y);
                let mut x = 0.0;
                for (i, tab) in row.tabs.iter().enumerate() {
                    let w = self.minimap_tab_width(tab);
                    if i == fc {
                        return (x, y, w, MAP_TAB_H);
                    }
                    x += w + MAP_GAP_X;
                }
            }
        }
        (0.0, 0.0, MAP_TAB_W, MAP_TAB_H)
    }

    fn trigger_map_animation(self: &Rc<Self>) {
        if !self.map_open() {
            return;
        }
        {
            let mut st = self.map_state.borrow_mut();
            if st.animating {
                return;
            }
            st.animating = true;
        }

        let app = self.clone();
        let last_time = Cell::new(None::<i64>);
        let drawing_area = app.map_drawing_area.clone();

        drawing_area.add_tick_callback(move |area, clock| {
            if !app.map_open() {
                app.map_state.borrow_mut().animating = false;
                return glib::ControlFlow::Break;
            }

            let now = clock.frame_time();
            let dt = last_time.replace(Some(now)).map_or(1.0 / 60.0, |l| {
                ((now - l) as f64 / 1e6).clamp(0.001, 0.1)
            });

            let k_pos = 1.0 - (-dt / 0.065).exp();
            let k_zoom = 1.0 - (-dt / 0.075).exp();

            let rows = app.rows.borrow();
            let (target_active_cx, target_active_cy) = app.compute_active_tab_center(&rows);
            let (target_rect_x, target_rect_y, target_rect_w, target_rect_h) = app.compute_active_tab_rect(&rows);
            drop(rows);

            let mut st = app.map_state.borrow_mut();

            // Inertia decay
            if st.vel_x.abs() > 0.05 || st.vel_y.abs() > 0.05 {
                st.target_offset_x += st.vel_x * dt;
                st.target_offset_y += st.vel_y * dt;
                let friction = (-dt * 7.5).exp();
                st.vel_x *= friction;
                st.vel_y *= friction;
                if st.vel_x.abs() < 0.05 { st.vel_x = 0.0; }
                if st.vel_y.abs() < 0.05 { st.vel_y = 0.0; }
                st.user_panned = true;
            }

            let target_cam_x = target_active_cx + st.target_offset_x;
            let target_cam_y = target_active_cy + st.target_offset_y;

            let mut diff_cx = target_cam_x - st.cam_x;
            let mut diff_cy = target_cam_y - st.cam_y;
            if diff_cx.abs() < 0.2 {
                st.cam_x = target_cam_x;
                diff_cx = 0.0;
            } else {
                st.cam_x += diff_cx * k_pos;
            }
            if diff_cy.abs() < 0.2 {
                st.cam_y = target_cam_y;
                diff_cy = 0.0;
            } else {
                st.cam_y += diff_cy * k_pos;
            }

            let diff_ox = st.target_offset_x - st.offset_x;
            let diff_oy = st.target_offset_y - st.offset_y;
            if diff_ox.abs() < 0.2 {
                st.offset_x = st.target_offset_x;
            } else {
                st.offset_x += diff_ox * k_pos;
            }
            if diff_oy.abs() < 0.2 {
                st.offset_y = st.target_offset_y;
            } else {
                st.offset_y += diff_oy * k_pos;
            }

            let mut diff_zoom = st.target_zoom - st.zoom;
            if diff_zoom.abs() < 0.002 {
                st.zoom = st.target_zoom;
                diff_zoom = 0.0;
            } else {
                st.zoom += diff_zoom * k_zoom;
            }

            let mut diff_rx = target_rect_x - st.cursor_x;
            let mut diff_ry = target_rect_y - st.cursor_y;
            let mut diff_rw = target_rect_w - st.cursor_w;
            let mut diff_rh = target_rect_h - st.cursor_h;

            if diff_rx.abs() < 0.2 { st.cursor_x = target_rect_x; diff_rx = 0.0; } else { st.cursor_x += diff_rx * k_pos; }
            if diff_ry.abs() < 0.2 { st.cursor_y = target_rect_y; diff_ry = 0.0; } else { st.cursor_y += diff_ry * k_pos; }
            if diff_rw.abs() < 0.2 { st.cursor_w = target_rect_w; diff_rw = 0.0; } else { st.cursor_w += diff_rw * k_pos; }
            if diff_rh.abs() < 0.2 { st.cursor_h = target_rect_h; diff_rh = 0.0; } else { st.cursor_h += diff_rh * k_pos; }

            let still_animating = diff_cx != 0.0
                || diff_cy != 0.0
                || diff_zoom != 0.0
                || diff_rx != 0.0
                || diff_ry != 0.0
                || diff_rw != 0.0
                || diff_rh != 0.0
                || st.vel_x != 0.0
                || st.vel_y != 0.0;

            drop(st);

            app.update_map_location_btn_visibility();
            area.queue_draw();

            if still_animating {
                glib::ControlFlow::Continue
            } else {
                app.map_state.borrow_mut().animating = false;
                glib::ControlFlow::Break
            }
        });
    }

    fn draw_rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let r = r.min(w / 2.0).min(h / 2.0);
        cr.new_sub_path();
        cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
        cr.close_path();
    }

    fn draw_minimap(&self, cr: &gtk::cairo::Context, width: f64, height: f64) {
        // Rail deep obsidian canvas background (#060913)
        cr.set_source_rgb(0.03, 0.03, 0.03);
        let _ = cr.paint();

        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }

        let (cam_x, cam_y, zoom, cur_rx, cur_ry, cur_rw, cur_rh, cursor_initialized) = {
            let st = self.map_state.borrow();
            (
                st.cam_x,
                st.cam_y,
                st.zoom,
                st.cursor_x,
                st.cursor_y,
                st.cursor_w,
                st.cursor_h,
                st.cursor_initialized,
            )
        };

        for (r, row) in rows.iter().enumerate() {
            let y = r as f64 * (MAP_TAB_H + MAP_GAP_Y);
            let sy = height / 2.0 + (y - cam_y) * zoom;
            let sh = MAP_TAB_H * zoom;

            if sy + sh < -10.0 || sy > height + 10.0 {
                continue;
            }

            let mut x = 0.0;
            for (_c, tab) in row.tabs.iter().enumerate() {
                let w = self.minimap_tab_width(tab);
                let sx = width / 2.0 + (x - cam_x) * zoom;
                let sw = w * zoom;
                x += w + MAP_GAP_X;

                if sx + sw < -10.0 || sx > width + 10.0 {
                    continue;
                }

                let corner_r = (3.0 * zoom).clamp(1.5, 4.0);

                Self::draw_rounded_rect(cr, sx, sy, sw, sh, corner_r);

                // Sleek dark slate tile (#0f172a)
                cr.set_source_rgb(0.08, 0.08, 0.08);
                let _ = cr.fill_preserve();
                // Subtle border (#1e293b)
                cr.set_source_rgb(0.15, 0.15, 0.15);
                cr.set_line_width((1.2 * zoom).clamp(0.8, 1.8));
                let _ = cr.stroke();
            }
        }

        // Draw the silky-smooth gliding active tab reticle / cursor
        if cursor_initialized {
            let cur_sx = width / 2.0 + (cur_rx - cam_x) * zoom;
            let cur_sy = height / 2.0 + (cur_ry - cam_y) * zoom;
            let cur_sw = cur_rw * zoom;
            let cur_sh = cur_rh * zoom;
            let corner_r = (3.0 * zoom).clamp(1.5, 4.0);

            Self::draw_rounded_rect(cr, cur_sx, cur_sy, cur_sw, cur_sh, corner_r);

            // Dark fill
            cr.set_source_rgba(0.22, 0.22, 0.22, 0.95);
            let _ = cr.fill_preserve();

            // Bright white border
            cr.set_source_rgb(0.93, 0.93, 0.93);
            cr.set_line_width((2.0 * zoom).clamp(1.4, 2.8));
            let _ = cr.stroke();

            // Bright center core indicator
            let dot_r = (2.2 * zoom).clamp(1.5, 3.2);
            cr.arc(cur_sx + cur_sw / 2.0, cur_sy + cur_sh / 2.0, dot_r, 0.0, std::f64::consts::PI * 2.0);
            cr.set_source_rgb(1.0, 1.0, 1.0);
            let _ = cr.fill();
        }
    }

    fn handle_minimap_click(self: &Rc<Self>, click_x: f64, click_y: f64) {
        let rows = self.rows.borrow();
        if rows.is_empty() {
            return;
        }
        let (cam_x, cam_y, zoom) = {
            let st = self.map_state.borrow();
            (st.cam_x, st.cam_y, st.zoom)
        };

        let width = (self.map_drawing_area.width() as f64).max(380.0);
        let height = (self.map_drawing_area.height() as f64).max(260.0);
        let world_x = cam_x + (click_x - width / 2.0) / zoom;
        let world_y = cam_y + (click_y - height / 2.0) / zoom;

        for (r, row) in rows.iter().enumerate() {
            let y = r as f64 * (MAP_TAB_H + MAP_GAP_Y);
            if world_y < y || world_y > y + MAP_TAB_H {
                continue;
            }
            let mut x = 0.0;
            for (c, tab) in row.tabs.iter().enumerate() {
                let w = self.minimap_tab_width(tab);
                if world_x >= x && world_x <= x + w {
                    drop(rows);
                    self.focus_coords(r, c);
                    return;
                }
                x += w + MAP_GAP_X;
            }
        }
    }

    fn rebuild_profiles_modal_list(self: &Rc<Self>) {
        while let Some(r) = self.profiles_list_box.row_at_index(0) {
            self.profiles_list_box.remove(&r);
        }

        let (profiles, active_id, can_delete) = {
            let mgr = self.profiles.borrow();
            (mgr.profiles().to_vec(), mgr.active_id().to_string(), mgr.profiles().len() > 1)
        };
        let total_profiles = profiles.len();
        let rows_vec: Rc<RefCell<Vec<gtk::Box>>> = Rc::new(RefCell::new(Vec::new()));
        let drag_provider = Rc::new(gtk::CssProvider::new());
        if let Some(disp) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &disp,
                &*drag_provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        for (idx, p) in profiles.into_iter().enumerate() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.add_css_class("profile-card");
            if p.id == active_id {
                row.add_css_class("active-profile");
            }
            rows_vec.borrow_mut().push(row.clone());

            let drag_handle = gtk::Label::new(Some("⋮⋮"));
            drag_handle.add_css_class("profile-drag-handle");
            drag_handle.set_cursor_from_name(Some("grab"));
            drag_handle.set_valign(gtk::Align::Center);
            drag_handle.set_tooltip_text(Some(tr("Drag up or down to reorder profiles")));

            if total_profiles > 1 {
                let drag = gtk::GestureDrag::new();
                drag.set_button(gdk::BUTTON_PRIMARY);

                let row_drag = row.clone();
                let handle_drag = drag_handle.clone();
                let drag_provider_begin = drag_provider.clone();
                drag.connect_drag_begin(move |_gesture, _start_x, _start_y| {
                    row_drag.add_css_class("dragging");
                    row_drag.set_cursor_from_name(Some("grabbing"));
                    handle_drag.set_cursor_from_name(Some("grabbing"));
                    drag_provider_begin.load_from_string(".profile-card.dragging { z-index: 1000 !important; }");
                });

                let rows_vec_update = rows_vec.clone();
                let drag_provider_update = drag_provider.clone();
                let from_idx = idx;
                drag.connect_drag_update(move |_gesture, _offset_x, offset_y| {
                    drag_provider_update.load_from_string(&format!(
                        ".profile-card.dragging {{ transform: translateY({:.1}px) !important; z-index: 1000 !important; }}",
                        offset_y
                    ));

                    let stride = 60.0f64;
                    let delta = (offset_y / stride).round() as isize;
                    let target_idx = ((from_idx as isize) + delta).clamp(0, (total_profiles - 1) as isize) as usize;

                    let rows_ref = rows_vec_update.borrow();
                    for (i, r_box) in rows_ref.iter().enumerate() {
                        if i == from_idx {
                            continue;
                        }
                        if from_idx < target_idx {
                            if i > from_idx && i <= target_idx {
                                r_box.add_css_class("shift-up");
                                r_box.remove_css_class("shift-down");
                            } else {
                                r_box.remove_css_class("shift-up");
                                r_box.remove_css_class("shift-down");
                            }
                        } else if from_idx > target_idx {
                            if i >= target_idx && i < from_idx {
                                r_box.add_css_class("shift-down");
                                r_box.remove_css_class("shift-up");
                            } else {
                                r_box.remove_css_class("shift-up");
                                r_box.remove_css_class("shift-down");
                            }
                        } else {
                            r_box.remove_css_class("shift-up");
                            r_box.remove_css_class("shift-down");
                        }
                    }
                });

                let row_end = row.clone();
                let handle_end = drag_handle.clone();
                let app_end = self.clone();
                let from_idx = idx;
                let rows_vec_end = rows_vec.clone();
                let drag_provider_end = drag_provider.clone();
                drag.connect_drag_end(move |_gesture, _offset_x, offset_y| {
                    drag_provider_end.load_from_string("");
                    row_end.remove_css_class("dragging");
                    row_end.set_cursor_from_name(Some("default"));
                    handle_end.set_cursor_from_name(Some("grab"));

                    let rows_ref = rows_vec_end.borrow();
                    for r_box in rows_ref.iter() {
                        r_box.remove_css_class("shift-up");
                        r_box.remove_css_class("shift-down");
                    }
                    drop(rows_ref);

                    let stride = 60.0f64;
                    let delta = (offset_y / stride).round() as isize;
                    let target_idx = ((from_idx as isize) + delta).clamp(0, (total_profiles - 1) as isize) as usize;

                    if target_idx != from_idx {
                        if app_end.profiles.borrow_mut().reorder_profile(from_idx, target_idx) {
                            let a = app_end.clone();
                            glib::idle_add_local_once(move || {
                                a.rebuild_profiles_modal_list();
                                a.rebuild_profile_popover();
                                a.refresh_sidebar();
                            });
                        }
                    }
                });

                drag_handle.add_controller(drag);
            }

            let avatar = gtk::Label::new(None);
            let fg = p.text_color();
            avatar.set_markup(&format!(
                "<span foreground='{}' font_weight='800'>{}</span>",
                fg,
                glib::markup_escape_text(&p.initial_letter())
            ));
            avatar.add_css_class("profile-avatar-circle");
            avatar.add_css_class(&format!("prof-bg-{}", p.color_idx));
            avatar.set_valign(gtk::Align::Center);
            avatar.set_halign(gtk::Align::Center);
            avatar.set_width_request(32);
            avatar.set_height_request(32);

            let info_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
            info_box.set_hexpand(true);
            info_box.set_valign(gtk::Align::Center);

            let name_lbl = gtk::Label::new(Some(&p.display_name()));
            name_lbl.add_css_class("profile-card-name");
            name_lbl.set_xalign(0.0);

            let persona = ProfilePersona::for_id(&p.id);
            let sub_text = trf("@{} - {} (100% Isolated)", &[&p.id, &persona.summary()]);
            let sub_lbl = gtk::Label::new(Some(&sub_text));
            sub_lbl.add_css_class("profile-card-sub");
            sub_lbl.set_xalign(0.0);

            info_box.append(&name_lbl);
            info_box.append(&sub_lbl);

            let actions_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            actions_box.set_valign(gtk::Align::Center);

            if p.id == active_id {
                let active_badge = gtk::Label::new(Some(tr("Active")));
                active_badge.add_css_class("profile-badge-active");
                actions_box.append(&active_badge);
            } else {
                let set_act_btn = gtk::Button::with_label(tr("Make Active"));
                set_act_btn.add_css_class("btn-make-active");
                let app = self.clone();
                let pid = p.id.clone();
                set_act_btn.connect_clicked(move |_| {
                    app.profiles.borrow_mut().set_active(&pid);
                    let a = app.clone();
                    glib::idle_add_local_once(move || {
                        a.rebuild_profiles_modal_list();
                        a.rebuild_profile_popover();
                        a.refresh_sidebar();
                    });
                });
                actions_box.append(&set_act_btn);
            }

            if can_delete && p.id != "default" {
                let del_btn = gtk::Button::from_icon_name("window-close-symbolic");
                del_btn.add_css_class("btn-del-profile");
                del_btn.set_tooltip_text(Some(tr("Delete Profile")));
                let app = self.clone();
                let pid = p.id.clone();
                del_btn.connect_clicked(move |_| {
                    app.profiles.borrow_mut().delete_profile(&pid);
                    {
                        let mut rows = app.rows.borrow_mut();
                        for row in rows.iter_mut() {
                            for tab in row.tabs.iter_mut() {
                                if tab.profile_id == pid {
                                    tab.profile_id = "default".to_string();
                                }
                            }
                        }
                    }
                    let a = app.clone();
                    glib::idle_add_local_once(move || {
                        a.rebuild_profiles_modal_list();
                        a.rebuild_profile_popover();
                        a.refresh_sidebar();
                    });
                });
                actions_box.append(&del_btn);
            }

            row.append(&drag_handle);
            row.append(&avatar);
            row.append(&info_box);
            row.append(&actions_box);

            self.profiles_list_box.append(&row);
        }
    }

    fn select_new_profile_color(&self, idx: usize) {
        self.new_profile_color_idx.set(idx);
        let btns = self.color_buttons.borrow();
        for (i, b) in btns.iter().enumerate() {
            if i == idx {
                b.add_css_class("selected");
            } else {
                b.remove_css_class("selected");
            }
        }
    }

    fn update_create_profile_btn_state(self: &Rc<Self>) {
        let text = self.new_profile_entry.text().to_string();
        let clean = text.trim();
        if clean.is_empty() {
            self.new_profile_error_lbl.set_visible(false);
            self.new_profile_entry.remove_css_class("error");
            self.add_prof_btn.set_sensitive(false);
            self.add_prof_btn.remove_css_class("active-ready");
        } else if self.profiles.borrow().profile_exists(clean) {
            self.new_profile_error_lbl.set_text(tr("Profile already exists"));
            self.new_profile_error_lbl.set_visible(true);
            self.new_profile_entry.add_css_class("error");
            self.add_prof_btn.set_sensitive(false);
            self.add_prof_btn.remove_css_class("active-ready");
        } else {
            self.new_profile_error_lbl.set_visible(false);
            self.new_profile_entry.remove_css_class("error");
            self.add_prof_btn.set_sensitive(true);
            self.add_prof_btn.add_css_class("active-ready");
        }
    }

    fn create_new_profile_action(self: &Rc<Self>) {
        let name = self.new_profile_entry.text().to_string();
        let clean = name.trim().to_string();
        if clean.is_empty() {
            self.update_create_profile_btn_state();
            return;
        }

        let color_idx = self.new_profile_color_idx.get();
        let create_res = self.profiles.borrow_mut().create_profile(&clean, color_idx);
        match create_res {
            Ok(_new_p) => {
                self.new_profile_entry.set_text("");
                self.update_create_profile_btn_state();
                self.rebuild_profiles_modal_list();
                self.rebuild_profile_popover();
                self.refresh_sidebar();

                let scroll = self.profiles_modal_scroll.clone();
                glib::idle_add_local_once(move || {
                    let adj = scroll.vadjustment();
                    adj.set_value(adj.upper());
                });
            }
            Err(err) => {
                self.new_profile_error_lbl.set_text(tr(err));
                self.new_profile_error_lbl.set_visible(true);
                self.new_profile_entry.add_css_class("error");
                self.add_prof_btn.set_sensitive(false);
                self.add_prof_btn.remove_css_class("active-ready");
            }
        }
    }

    // ---- overview (niri-style 2D grid: zoomed out) -----------------------

    fn toggle_overview(self: &Rc<Self>) {
        if self.ov.get() {
            self.exit_overview();
        } else {
            self.enter_overview();
        }
    }

    fn enter_overview(self: &Rc<Self>) {
        if self.ov.get() || self.rows.borrow().is_empty() {
            return;
        }
        self.palette.set_visible(false);
        self.ov.set(true);
        self.ov_return.set((self.focus_row.get(), self.focus_col.get()));
        self.ov_saved_x.set(self.target_x.get());
        self.ov_saved_y.set(self.target_y.get());
        self.strip.add_css_class("zoomed");
        self.ov_hint.set_visible(true);
        for row in self.rows.borrow().iter() {
            for t in row.tabs.iter() {
                t.view.set_can_target(false);
            }
        }
        self.target_zoom.set(OV_ZOOM);
        self.reveal_focus();
    }

    fn exit_overview(self: &Rc<Self>) {
        if !self.ov.get() {
            return;
        }
        self.ov.set(false);
        self.strip.remove_css_class("zoomed");
        self.ov_hint.set_visible(false);
        for row in self.rows.borrow().iter() {
            for t in row.tabs.iter() {
                t.view.set_can_target(true);
            }
        }
        self.target_zoom.set(1.0);
        self.reveal_focus();
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    fn cancel_overview(self: &Rc<Self>) {
        let (r, c) = self.ov_return.get();
        self.focus_coords(r, c);
        self.exit_overview();
    }

    fn ov_click(self: &Rc<Self>, click_x: f64, click_y: f64) {
        let vw = self.strip.width() as f64;
        let vh = self.strip.height() as f64;
        let z = self.layout.zoom();
        let off_x = self.layout.offset_x();
        let off_y = self.layout.offset_y();
        let g = gap(z);

        let unscaled_x = click_x / z + off_x;
        let unscaled_y = click_y / z + off_y;

        let rows = self.rows.borrow();
        let mut hit = None;
        for (r_idx, row) in rows.iter().enumerate() {
            let row_y = r_idx as f64 * (vh + g);
            if unscaled_y >= row_y && unscaled_y <= row_y + vh {
                let mut col_x = 0.0;
                for (c_idx, tab) in row.tabs.iter().enumerate() {
                    let cw = tab.custom_width.get().unwrap_or_else(|| col_width(tab.container.has_css_class("full"), vw));
                    if unscaled_x >= col_x && unscaled_x <= col_x + cw {
                        hit = Some((r_idx, c_idx));
                        break;
                    }
                    col_x += cw + g;
                }
                if hit.is_some() {
                    break;
                }
            }
        }
        drop(rows);

        if let Some((r, c)) = hit {
            self.focus_coords(r, c);
        }
        self.exit_overview();
    }

    fn ov_key(self: &Rc<Self>, k: gdk::Key, m: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
        let plain = m.is_empty();
        if plain || m == M::ALT_MASK {
            match k {
                gdk::Key::h | gdk::Key::Left => {
                    self.focus_column_left();
                    return true;
                }
                gdk::Key::l | gdk::Key::Right => {
                    self.focus_column_right();
                    return true;
                }
                gdk::Key::j | gdk::Key::Down => {
                    self.focus_row_down();
                    return true;
                }
                gdk::Key::k | gdk::Key::Up => {
                    self.focus_row_up();
                    return true;
                }
                _ => {}
            }
        }
        if plain {
            match k {
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::space => self.exit_overview(),
                gdk::Key::x | gdk::Key::Delete | gdk::Key::BackSpace => {
                    if let Some(v) = self.focused_view() {
                        self.close_view(&v);
                    }
                }
                _ => {}
            }
        }
        plain
    }

    // ---- keys ------------------------------------------------------------

    fn on_key(self: &Rc<Self>, key: gdk::Key, keycode: u32, state: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let shift = state.contains(M::SHIFT_MASK);
        let alt = state.contains(M::ALT_MASK);
        let k = key.to_lower();

        if self.settings_modal_open() {
            if k == gdk::Key::Escape {
                self.hide_settings_modal();
                return true;
            }
            if alt && !ctrl && (k == gdk::Key::s || k == gdk::Key::S || keycode == 39) {
                self.hide_settings_modal();
                return true;
            }
            if alt && !shift && !ctrl && (k == gdk::Key::p || k == gdk::Key::P) {
                if self.current_settings_tab.get() == SettingsTab::Profiles {
                    self.hide_settings_modal();
                } else {
                    self.open_settings_tab(SettingsTab::Profiles);
                }
                return true;
            }
            if alt && !shift && !ctrl && (k == gdk::Key::i || k == gdk::Key::I) {
                if self.current_settings_tab.get() == SettingsTab::Data {
                    self.hide_settings_modal();
                } else {
                    self.open_settings_tab(SettingsTab::Data);
                }
                return true;
            }
            // All other keystrokes pass directly to settings_modal without triggering window shortcuts
            return false;
        }

        if self.map_open() {
            if k == gdk::Key::Escape {
                self.hide_map();
                return true;
            }
            if alt && !shift && !ctrl && (k == gdk::Key::m || k == gdk::Key::M) {
                self.hide_map();
                return true;
            }
        }

        if self.devtools_open() {
            if k == gdk::Key::Escape {
                self.hide_devtools();
                return true;
            }
            if !ctrl && !shift && !alt && k == gdk::Key::F12 {
                self.hide_devtools();
                return true;
            }
            if alt && shift && !ctrl && (k == gdk::Key::i || k == gdk::Key::I) {
                self.hide_devtools();
                return true;
            }
        }

        if alt && shift && !ctrl && (k == gdk::Key::d || k == gdk::Key::D) {
            self.cycle_devtools_dock();
            return true;
        }

        if !ctrl && !shift && !alt && k == gdk::Key::F12 {
            self.toggle_devtools();
            return true;
        }
        if alt && shift && !ctrl && (k == gdk::Key::i || k == gdk::Key::I) {
            self.toggle_devtools();
            return true;
        }

        if k == gdk::Key::Escape {
            if self.palette_open() {
                self.cancel_palette();
                return true;
            }
            if self.ov.get() {
                self.cancel_overview();
                return true;
            }
            return false;
        }
        let global = (ctrl && !alt && (k == gdk::Key::Tab || k == gdk::Key::ISO_Left_Tab || keycode == 23)) || (alt && !ctrl && (matches!(k, gdk::Key::o | gdk::Key::t | gdk::Key::T | gdk::Key::q | gdk::Key::p | gdk::Key::m | gdk::Key::i | gdk::Key::I | gdk::Key::s | gdk::Key::S | gdk::Key::d | gdk::Key::D) || keycode == 28 || keycode == 39))
            || (ctrl && shift && !alt && matches!(k, gdk::Key::t | gdk::Key::T))
            || (ctrl && !shift && !alt && (matches!(k, gdk::Key::_1 | gdk::Key::_2 | gdk::Key::_3 | gdk::Key::_4 | gdk::Key::_5 | gdk::Key::_6 | gdk::Key::_7 | gdk::Key::_8 | gdk::Key::_9 | gdk::Key::KP_1 | gdk::Key::KP_2 | gdk::Key::KP_3 | gdk::Key::KP_4 | gdk::Key::KP_5 | gdk::Key::KP_6 | gdk::Key::KP_7 | gdk::Key::KP_8 | gdk::Key::KP_9) || (keycode >= 10 && keycode <= 18) || (keycode >= 87 && keycode <= 95)));
        if self.overview_open() && !global && self.ov_key(k, state & gtk::accelerator_get_default_mod_mask()) {
            return true;
        }

        // Standard universal browser shortcuts (Ctrl+R, Ctrl+T, Ctrl+W, Ctrl+L, F5, Ctrl+1..9)
        if ctrl && !shift && !alt {
            match k {
                gdk::Key::r | gdk::Key::R => {
                    if let Some(v) = self.focused_view() {
                        v.reload();
                    }
                    return true;
                }
                gdk::Key::t | gdk::Key::T => {
                    self.open_new_tab_palette();
                    return true;
                }
                gdk::Key::w | gdk::Key::W => {
                    if !self.palette_open() {
                        if let Some(v) = self.focused_view() {
                            self.close_view(&v);
                        }
                    }
                    return true;
                }
                gdk::Key::l | gdk::Key::L => {
                    self.show_palette();
                    return true;
                }
                _ => {}
            }

            if k == gdk::Key::Tab || k == gdk::Key::ISO_Left_Tab || keycode == 23 {
                self.cycle_tab(1);
                return true;
            }
            // Tab switching within active workspace (Ctrl+1..8 to tabs 1..8, Ctrl+9 to last tab)
            if keycode == 10 || keycode == 87 || k == gdk::Key::_1 || k == gdk::Key::KP_1 || key.to_unicode() == Some('1') {
                self.focus_tab_index(0);
                return true;
            }
            if keycode == 11 || keycode == 88 || k == gdk::Key::_2 || k == gdk::Key::KP_2 || key.to_unicode() == Some('2') {
                self.focus_tab_index(1);
                return true;
            }
            if keycode == 12 || keycode == 89 || k == gdk::Key::_3 || k == gdk::Key::KP_3 || key.to_unicode() == Some('3') {
                self.focus_tab_index(2);
                return true;
            }
            if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                self.focus_tab_index(3);
                return true;
            }
            if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                self.focus_tab_index(4);
                return true;
            }
            if keycode == 15 || keycode == 92 || k == gdk::Key::_6 || k == gdk::Key::KP_6 || key.to_unicode() == Some('6') {
                self.focus_tab_index(5);
                return true;
            }
            if keycode == 16 || keycode == 93 || k == gdk::Key::_7 || k == gdk::Key::KP_7 || key.to_unicode() == Some('7') {
                self.focus_tab_index(6);
                return true;
            }
            if keycode == 17 || keycode == 94 || k == gdk::Key::_8 || k == gdk::Key::KP_8 || key.to_unicode() == Some('8') {
                self.focus_tab_index(7);
                return true;
            }
            if keycode == 18 || keycode == 95 || k == gdk::Key::_9 || k == gdk::Key::KP_9 || key.to_unicode() == Some('9') {
                self.focus_last_tab();
                return true;
            }
        }
        if !ctrl && !shift && !alt && (k == gdk::Key::F5 || keycode == 71) {
            if let Some(v) = self.focused_view() {
                v.reload();
            }
            return true;
        }

        if ctrl && shift && !alt && (k == gdk::Key::ISO_Left_Tab || k == gdk::Key::Tab || keycode == 23) {
            self.cycle_tab(-1);
            return true;
        }

        if ctrl && shift && !alt {
            match k {
                gdk::Key::t | gdk::Key::T => {
                    self.toggle_touchpad_mode();
                    return true;
                }
                _ => {}
            }
        }

        if !ctrl && !shift && !alt && k == gdk::Key::F11 {
            self.win.set_fullscreened(!self.win.is_fullscreen());
            return true;
        }

        if alt && !shift && !ctrl {
            match k {
                gdk::Key::Left => {
                    if let Some(v) = self.focused_view() {
                        v.go_back();
                    }
                }
                gdk::Key::Right => {
                    if let Some(v) = self.focused_view() {
                        v.go_forward();
                    }
                }
                gdk::Key::p | gdk::Key::P => {
                    if self.palette_open() {
                        if self.profile_popover.is_visible() {
                            self.profile_popover.popdown();
                        } else {
                            self.profile_popover.popup();
                        }
                    } else {
                        self.toggle_profiles_modal();
                    }
                }
                gdk::Key::i | gdk::Key::I => self.toggle_data_modal(),
                gdk::Key::t | gdk::Key::T => self.open_new_tab_palette(),
                gdk::Key::n => self.split_vertical(),
                gdk::Key::f => self.toggle_full(),
                gdk::Key::h => self.focus_column_left(),
                gdk::Key::l => self.focus_column_right(),
                gdk::Key::j | gdk::Key::Down => self.focus_row_down(),
                gdk::Key::k | gdk::Key::Up => self.focus_row_up(),
                gdk::Key::w => {
                    if !self.palette_open() {
                        if let Some(v) = self.focused_view() {
                            self.close_view(&v);
                        }
                    }
                }
                gdk::Key::b => self.toggle_sidebar(),
                gdk::Key::o => self.toggle_overview(),
                gdk::Key::m => self.toggle_map(),
                gdk::Key::g | gdk::Key::G => self.show_palette(),
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    if self.palette_open() {
                        return false;
                    }
                    self.show_palette();
                }
                gdk::Key::r => {
                    if let Some(v) = self.focused_view() {
                        v.reload();
                    }
                }
                gdk::Key::s | gdk::Key::S => {
                    self.toggle_settings_modal();
                    return true;
                }
                gdk::Key::q => self.win.close(),
                _ => {
                    if keycode == 39 {
                        self.toggle_settings_modal();
                        return true;
                    }
                    if keycode == 28 {
                        self.open_new_tab_palette();
                        return true;
                    }
                    if keycode == 42 {
                        self.show_palette();
                        return true;
                    }
                    // Vertical workspace selection (Alt+1..9)
                    if keycode == 10 || keycode == 87 || k == gdk::Key::_1 || k == gdk::Key::KP_1 || key.to_unicode() == Some('1') {
                        self.focus_row(0);
                        return true;
                    }
                    if keycode == 11 || keycode == 88 || k == gdk::Key::_2 || k == gdk::Key::KP_2 || key.to_unicode() == Some('2') {
                        self.focus_row(1);
                        return true;
                    }
                    if keycode == 12 || keycode == 89 || k == gdk::Key::_3 || k == gdk::Key::KP_3 || key.to_unicode() == Some('3') {
                        self.focus_row(2);
                        return true;
                    }
                    if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                        self.focus_row(3);
                        return true;
                    }
                    if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                        self.focus_row(4);
                        return true;
                    }
                    if keycode == 15 || keycode == 92 || k == gdk::Key::_6 || k == gdk::Key::KP_6 || key.to_unicode() == Some('6') {
                        self.focus_row(5);
                        return true;
                    }
                    if keycode == 16 || keycode == 93 || k == gdk::Key::_7 || k == gdk::Key::KP_7 || key.to_unicode() == Some('7') {
                        self.focus_row(6);
                        return true;
                    }
                    if keycode == 17 || keycode == 94 || k == gdk::Key::_8 || k == gdk::Key::KP_8 || key.to_unicode() == Some('8') {
                        self.focus_row(7);
                        return true;
                    }
                    if keycode == 18 || keycode == 95 || k == gdk::Key::_9 || k == gdk::Key::KP_9 || key.to_unicode() == Some('9') {
                        self.focus_row(8);
                        return true;
                    }
                    return false;
                }
            }
            return true;
        }

        if alt && shift && !ctrl {
            match k {
                gdk::Key::s | gdk::Key::S => {
                    self.toggle_settings_modal();
                    return true;
                }
                gdk::Key::i | gdk::Key::I => self.toggle_devtools(),
                gdk::Key::d | gdk::Key::D => {
                    self.cycle_devtools_dock();
                    return true;
                }
                gdk::Key::t | gdk::Key::T => self.toggle_touchpad_mode(),
                gdk::Key::v | gdk::Key::V | gdk::Key::n | gdk::Key::N => self.split_vertical(),
                gdk::Key::h | gdk::Key::Left => self.move_column(-1),
                gdk::Key::l | gdk::Key::Right => self.move_column(1),
                gdk::Key::j | gdk::Key::Down => self.move_tab_to_row(self.focus_row.get() + 1),
                gdk::Key::k | gdk::Key::Up => {
                    if self.focus_row.get() > 0 {
                        self.move_tab_to_row(self.focus_row.get() - 1);
                    }
                }
                _ => {
                    if keycode == 39 {
                        self.toggle_settings_modal();
                        return true;
                    }
                    // Alt+Shift+1 -> move to workspace/row 1
                    if keycode == 10 || keycode == 87
                        || k == gdk::Key::_1 || k == gdk::Key::exclam || k == gdk::Key::KP_1
                        || key == gdk::Key::_1 || key == gdk::Key::exclam || key == gdk::Key::KP_1
                        || key.to_unicode() == Some('1') || key.to_unicode() == Some('!')
                    {
                        self.move_tab_to_row(0);
                        return true;
                    }
                    // Alt+Shift+2 -> move to workspace/row 2
                    if keycode == 11 || keycode == 88
                        || k == gdk::Key::_2 || k == gdk::Key::at || k == gdk::Key::quotedbl || k == gdk::Key::KP_2
                        || key == gdk::Key::_2 || key == gdk::Key::at || key == gdk::Key::quotedbl || key == gdk::Key::KP_2
                        || key.to_unicode() == Some('2') || key.to_unicode() == Some('@') || key.to_unicode() == Some('"')
                    {
                        self.move_tab_to_row(1);
                        return true;
                    }
                    // Alt+Shift+3 -> move to workspace/row 3
                    if keycode == 12 || keycode == 89
                        || k == gdk::Key::_3 || k == gdk::Key::numbersign || k == gdk::Key::KP_3
                        || key == gdk::Key::_3 || key == gdk::Key::numbersign || key == gdk::Key::KP_3
                        || key.to_unicode() == Some('3') || key.to_unicode() == Some('#')
                    {
                        self.move_tab_to_row(2);
                        return true;
                    }
                    // Alt+Shift+4 -> move to workspace/row 4
                    if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                        self.move_tab_to_row(3);
                        return true;
                    }
                    // Alt+Shift+5 -> move to workspace/row 5
                    if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                        self.move_tab_to_row(4);
                        return true;
                    }
                    // Alt+Shift+6 -> move to workspace/row 6
                    if keycode == 15 || keycode == 92 || k == gdk::Key::_6 || k == gdk::Key::asciicircum || k == gdk::Key::KP_6 || key.to_unicode() == Some('6') || key.to_unicode() == Some('^') {
                        self.move_tab_to_row(5);
                        return true;
                    }
                    // Alt+Shift+7 -> move to workspace/row 7
                    if keycode == 16 || keycode == 93 || k == gdk::Key::_7 || k == gdk::Key::ampersand || k == gdk::Key::KP_7 || key.to_unicode() == Some('7') || key.to_unicode() == Some('&') {
                        self.move_tab_to_row(6);
                        return true;
                    }
                    // Alt+Shift+8 -> move to workspace/row 8
                    if keycode == 17 || keycode == 94 || k == gdk::Key::_8 || k == gdk::Key::asterisk || k == gdk::Key::KP_8 || key.to_unicode() == Some('8') || key.to_unicode() == Some('*') {
                        self.move_tab_to_row(7);
                        return true;
                    }
                    // Alt+Shift+9 -> move to workspace/row 9
                    if keycode == 18 || keycode == 95 || k == gdk::Key::_9 || k == gdk::Key::parenleft || k == gdk::Key::KP_9 || key.to_unicode() == Some('9') || key.to_unicode() == Some('(') {
                        self.move_tab_to_row(8);
                        return true;
                    }
                    return false;
                }
            }
            return true;
        }
        false
    }
}

fn build(gapp: &gtk::Application) {
    if let Some(s) = gtk::Settings::default() {
        s.set_gtk_application_prefer_dark_theme(true);
    }
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );

    let profile_mgr = ProfileManager::new(rail_data_dir());
    let history_mgr = HistoryManager::new(rail_data_dir());
    let initial_prof_id = profile_mgr.active_profile().id;

    // ---- sidebar ----
    let brand = gtk::Label::new(None);
    brand.set_markup("<span weight='bold' font_family='monospace' size='large' foreground='#f8fafc'>rail</span><span weight='bold' size='large' foreground='#38bdf8'>.</span>");
    brand.set_hexpand(true);
    brand.set_xalign(0.0);
    let add = gtk::Button::new();
    add.set_child(Some(&create_gear_icon(18)));
    add.add_css_class("btn-add-tab");
    add.set_tooltip_text(Some(tr("Settings")));
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    head.set_margin_start(4);
    head.append(&brand);
    head.append(&add);

    let pill_label = gtk::Label::new(Some(tr("Search or enter address")));
    pill_label.set_xalign(0.0);
    pill_label.set_hexpand(true);
    pill_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    pill_label.set_width_chars(1);
    let pill_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let s_icon = create_google_icon(18);
    s_icon.add_css_class("pill-icon");
    pill_inner.append(&s_icon);
    pill_inner.append(&pill_label);
    let pill_chip = gtk::Label::new(Some(tr("Alt+G")));
    pill_chip.add_css_class("pill-chip");
    pill_inner.append(&pill_chip);
    let pill = gtk::Button::new();
    pill.add_css_class("pill");
    pill.set_child(Some(&pill_inner));
    pill.set_tooltip_text(Some(tr("Go to address  (Alt+G)")));

    let list = gtk::ListBox::new();
    list.add_css_class("tablist");
    list.set_selection_mode(gtk::SelectionMode::None);
    list.set_overflow(gtk::Overflow::Hidden);
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_vexpand(true);
    list_scroll.set_overflow(gtk::Overflow::Hidden);
    list_scroll.set_child(Some(&list));

    let side = gtk::Box::new(gtk::Orientation::Vertical, 10);
    side.add_css_class("side-bar");
    side.set_size_request(SIDE_W, -1);
    side.set_overflow(gtk::Overflow::Hidden);
    side.set_margin_top(10);
    side.set_margin_bottom(10);
    side.set_margin_start(8);
    side.set_margin_end(2);
    side.append(&head);
    side.append(&pill);
    side.append(&list_scroll);

    // ---- strip + shell ----
    let layout = StripLayout::new();
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    strip.set_layout_manager(Some(layout.clone()));
    strip.set_overflow(gtk::Overflow::Hidden);
    strip.set_margin_top(8);
    strip.set_margin_bottom(8);
    strip.set_margin_end(8);
    strip.set_margin_start(6);

    let shell_layout = ShellLayout::new();
    shell_layout.set_side(SIDE_W as f64);
    let shell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    shell.set_layout_manager(Some(shell_layout.clone()));
    shell.set_overflow(gtk::Overflow::Hidden);
    shell.set_hexpand(true);
    shell.set_vexpand(true);
    shell.append(&side);
    shell.append(&strip);

    // ---- address palette ----
    let entry = gtk::Entry::new();
    entry.add_css_class("palette-entry");
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some(tr("Search Google or enter web address...")));

    let sicon = create_google_icon(20);

    let profile_btn = gtk::Button::new();
    profile_btn.add_css_class("profile-circle-btn");
    profile_btn.set_valign(gtk::Align::Center);
    profile_btn.set_halign(gtk::Align::End);
    let active_p = profile_mgr.active_profile();
    let profile_btn_lbl = gtk::Label::new(None);
    profile_btn_lbl.set_markup(&format!(
        "<span foreground='{}' font_weight='800'>{}</span>",
        active_p.text_color(),
        glib::markup_escape_text(&active_p.initial_letter())
    ));
    profile_btn_lbl.add_css_class("profile-circle-label");
    profile_btn.add_css_class(&format!("prof-bg-{}", active_p.color_idx));
    profile_btn.set_child(Some(&profile_btn_lbl));

    let profile_popover = gtk::Popover::new();
    profile_popover.add_css_class("profile-popover");
    profile_popover.set_parent(&profile_btn);
    profile_popover.set_has_arrow(true);

    let prow = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    prow.append(&sicon);
    prow.append(&entry);
    prow.append(&profile_btn);

    let palette_suggestions_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    palette_suggestions_box.add_css_class("palette-suggestions-box");
    palette_suggestions_box.set_visible(false);

    let phint = gtk::Label::new(Some(tr("Enter  go      Esc  cancel      Alt+P  profiles")));
    phint.add_css_class("palette-hint");
    phint.set_xalign(0.0);
    let palette = gtk::Box::new(gtk::Orientation::Vertical, 0);
    palette.add_css_class("palette");
    palette.set_halign(gtk::Align::Center);
    palette.set_valign(gtk::Align::Start);
    palette.set_margin_top(90);
    palette.set_margin_start(16);
    palette.set_margin_end(16);
    palette.append(&prow);
    palette.append(&palette_suggestions_box);
    palette.append(&phint);
    palette.set_visible(false);

    let pop_c = profile_popover.clone();
    profile_btn.connect_clicked(move |_| {
        pop_c.popup();
    });

    // ---- floating settings panel (Alt+Shift+S / Alt+P / Alt+I) ----
    let settings_modal = gtk::Box::new(gtk::Orientation::Vertical, 0);
    settings_modal.add_css_class("settings-modal");
    settings_modal.set_halign(gtk::Align::Center);
    settings_modal.set_valign(gtk::Align::Center);
    settings_modal.set_visible(false);
    settings_modal.set_size_request(820, 560);

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
    settings_close_btn.set_tooltip_text(Some(tr("Close Settings (Esc)")));

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
    settings_body.set_vexpand(true);

    let settings_sidebar = gtk::Box::new(gtk::Orientation::Vertical, 4);
    settings_sidebar.add_css_class("settings-sidebar");

    let settings_stack = gtk::Stack::new();
    settings_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    settings_stack.set_transition_duration(150);
    settings_stack.set_hexpand(true);
    settings_stack.set_vexpand(true);

    let sidebar_items = [
        (SettingsTab::General, "preferences-system-symbolic", tr("General")),
        (SettingsTab::Profiles, "system-users-symbolic", tr("Profiles")),
        (SettingsTab::Data, "drive-harddisk-symbolic", tr("Data")),
        (SettingsTab::Privacy, "security-high-symbolic", tr("Privacy & Shield")),
        (SettingsTab::Customize, "document-edit-symbolic", tr("Customize")),
        (SettingsTab::Update, "software-update-available-symbolic", tr("Updates")),
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

    let esc_hint = gtk::Label::new(Some(tr("Esc to close")));
    esc_hint.add_css_class("data-section-desc");
    esc_hint.set_xalign(0.5);
    esc_hint.set_margin_bottom(8);
    settings_sidebar.append(&esc_hint);

    settings_body.append(&settings_sidebar);
    settings_body.append(&settings_stack);
    settings_modal.append(&settings_body);

    // --- Page 1: General ---
    let page_general = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_general.add_css_class("settings-content-page");

    let gen_title = gtk::Label::new(Some(tr("General Preferences")));
    gen_title.add_css_class("settings-page-title");
    gen_title.set_xalign(0.0);
    let gen_subtitle = gtk::Label::new(Some(tr("Configure default search engine, new tab behavior, and browsing controls")));
    gen_subtitle.add_css_class("settings-page-subtitle");
    gen_subtitle.set_xalign(0.0);
    page_general.append(&gen_title);
    page_general.append(&gen_subtitle);

    // Search Engine Card
    let engine_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    engine_card.add_css_class("settings-card");
    let engine_title = gtk::Label::new(Some(tr("Default Search Engine")));
    engine_title.add_css_class("settings-card-title");
    engine_title.set_xalign(0.0);
    let engine_desc = gtk::Label::new(Some(tr("Applied when searching via the Omnibar (Alt+T / Alt+G)")));
    engine_desc.add_css_class("settings-card-desc");
    engine_desc.set_xalign(0.0);
    engine_card.append(&engine_title);
    engine_card.append(&engine_desc);

    let initial_settings = load_app_settings();
    let initial_engine = initial_settings.search_engine.clone();

    let engines_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let engine_choices = [
        ("google", "Google"),
        ("duckduckgo", "DuckDuckGo"),
        ("brave", "Brave"),
        ("bing", "Bing"),
        ("ecosia", "Ecosia"),
    ];
    let mut engine_buttons: Vec<(String, gtk::Button)> = Vec::new();
    for (id, label) in engine_choices {
        let btn = gtk::Button::with_label(label);
        btn.add_css_class("settings-option-item");
        if id == initial_engine {
            btn.add_css_class("active");
        }
        engines_row.append(&btn);
        engine_buttons.push((id.to_string(), btn));
    }
    engine_card.append(&engines_row);
    page_general.append(&engine_card);

    // Language Card
    let lang_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    lang_card.add_css_class("settings-card");
    let lang_title = gtk::Label::new(Some(tr("Interface Language")));
    lang_title.add_css_class("settings-card-title");
    lang_title.set_xalign(0.0);
    let lang_desc = gtk::Label::new(Some(tr("Choose the language of Rail menus and settings. Applies after restart.")));
    lang_desc.add_css_class("settings-card-desc");
    lang_desc.set_xalign(0.0);
    lang_desc.set_wrap(true);
    lang_card.append(&lang_title);
    lang_card.append(&lang_desc);
    let lang_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let lang_status = gtk::Label::new(None);
    lang_status.add_css_class("settings-card-desc");
    lang_status.set_xalign(0.0);
    lang_status.set_visible(false);
    let lang_restart = gtk::Button::with_label(tr("Restart now"));
    lang_restart.add_css_class("settings-option-item");
    lang_restart.set_halign(gtk::Align::Start);
    lang_restart.set_visible(false);
    lang_restart.connect_clicked(|b| {
        if let Some(w) = b.root().and_then(|r| r.downcast::<gtk::ApplicationWindow>().ok()) {
            update::restart(&w);
        }
    });
    let mut lang_buttons: Vec<(i18n::Lang, gtk::Button)> = Vec::new();
    for (lang, name) in [(i18n::Lang::Uz, "O'zbekcha"), (i18n::Lang::En, "English"), (i18n::Lang::Ru, "Русский")] {
        let btn = gtk::Button::with_label(name);
        btn.add_css_class("settings-option-item");
        if lang == i18n::current() {
            btn.add_css_class("active");
        }
        lang_row.append(&btn);
        lang_buttons.push((lang, btn));
    }
    for (lang, btn) in lang_buttons.iter() {
        let lang = *lang;
        let all = lang_buttons.clone();
        let status = lang_status.clone();
        let restart_btn = lang_restart.clone();
        btn.connect_clicked(move |_| {
            if i18n::set_language(lang) {
                for (l, b) in all.iter() {
                    if *l == lang {
                        b.add_css_class("active");
                    } else {
                        b.remove_css_class("active");
                    }
                }
                let changed = lang != i18n::current();
                status.set_text(tr("Language saved. Restart Rail to apply."));
                status.set_visible(changed);
                restart_btn.set_visible(changed);
            }
        });
    }
    lang_card.append(&lang_row);
    lang_card.append(&lang_status);
    lang_card.append(&lang_restart);
    page_general.append(&lang_card);

    // Browsing Controls Card
    let browse_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    browse_card.add_css_class("settings-card");
    let browse_title = gtk::Label::new(Some(tr("Browsing Ergonomics & Gestures")));
    browse_title.add_css_class("settings-card-title");
    browse_title.set_xalign(0.0);
    browse_card.append(&browse_title);

    let tp_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let tp_lbl = gtk::Label::new(Some(tr("Touchpad Canvas Navigation (Two-finger swipe navigates 2D canvas)")));
    tp_lbl.add_css_class("shortcut-desc");
    tp_lbl.set_hexpand(true);
    tp_lbl.set_xalign(0.0);
    let tp_switch = gtk::Switch::new();
    tp_switch.set_active(false);
    tp_switch.set_valign(gtk::Align::Center);
    tp_row.append(&tp_lbl);
    tp_row.append(&tp_switch);
    browse_card.append(&tp_row);

    let scroll_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let scroll_lbl = gtk::Label::new(Some(tr("WebKit Smooth Kinetic Scrolling")));
    scroll_lbl.add_css_class("shortcut-desc");
    scroll_lbl.set_hexpand(true);
    scroll_lbl.set_xalign(0.0);
    let scroll_switch = gtk::Switch::new();
    scroll_switch.set_active(true);
    scroll_switch.set_valign(gtk::Align::Center);
    scroll_row.append(&scroll_lbl);
    scroll_row.append(&scroll_switch);
    browse_card.append(&scroll_row);

    let hw_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let hw_lbl = gtk::Label::new(Some(tr("Hardware Acceleration Policy")));
    hw_lbl.add_css_class("shortcut-desc");
    hw_lbl.set_hexpand(true);
    hw_lbl.set_xalign(0.0);
    let hw_badge = gtk::Label::new(Some(tr("Always (GSK GPU)")));
    hw_badge.add_css_class("shortcut-kbd");
    hw_row.append(&hw_lbl);
    hw_row.append(&hw_badge);
    browse_card.append(&hw_row);

    page_general.append(&browse_card);

    let page_gen_scroll = gtk::ScrolledWindow::new();
    page_gen_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_gen_scroll.set_child(Some(&page_general));
    settings_stack.add_named(&page_gen_scroll, Some("general"));

    // --- Page: Customize (html.css editor) ---
    let page_css = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_css.set_vexpand(true);

    let css_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    let css_view = gtk::TextView::with_buffer(&css_buffer);
    css_view.add_css_class("css-editor");
    css_view.set_monospace(true);
    css_view.set_wrap_mode(gtk::WrapMode::Word);
    css_view.set_left_margin(10);
    css_view.set_right_margin(10);
    css_view.set_top_margin(8);
    css_view.set_bottom_margin(8);
    let css_scroll = gtk::ScrolledWindow::new();
    css_scroll.add_css_class("css-editor-frame");
    css_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    css_scroll.set_vexpand(true);
    css_scroll.set_min_content_height(300);
    css_scroll.set_child(Some(&css_view));
    page_css.append(&css_scroll);

    let css_save_btn = gtk::Button::with_label(tr("Save"));
    css_save_btn.add_css_class("settings-option-item");
    css_save_btn.add_css_class("active");
    css_save_btn.set_halign(gtk::Align::Start);
    page_css.append(&css_save_btn);

    {
        let buf = css_buffer.clone();
        css_save_btn.connect_clicked(move |b| save_css_editor(&buf, b));
    }
    {
        let buf = css_buffer.clone();
        let btn = css_save_btn.clone();
        let kc = gtk::EventControllerKey::new();
        kc.set_propagation_phase(gtk::PropagationPhase::Capture);
        kc.connect_key_pressed(move |_, key, _, state| {
            if state.contains(gdk::ModifierType::CONTROL_MASK) && key.to_lower() == gdk::Key::s {
                save_css_editor(&buf, &btn);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        css_view.add_controller(kc);
    }

    // --- Customize > Shortcuts subtab ---
    let sc_pending: Rc<RefCell<ShortcutConfig>> = Rc::new(RefCell::new(ShortcutConfig::default()));
    let sc_page = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sc_page.set_vexpand(true);

    let sc_prio = gtk::CheckButton::with_label(tr("Rail shortcuts win over website shortcuts"));
    sc_prio.set_active(true);
    let sc_prio_hint = gtk::Label::new(Some(
        tr("Unchecked: when a page uses the same keys, the page wins and Rail stays out of the way."),
    ));
    sc_prio_hint.add_css_class("settings-card-desc");
    sc_prio_hint.set_xalign(0.0);
    sc_prio_hint.set_wrap(true);
    sc_page.append(&sc_prio);
    sc_page.append(&sc_prio_hint);

    let sc_rows = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let sc_scroll = gtk::ScrolledWindow::new();
    sc_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    sc_scroll.set_vexpand(true);
    sc_scroll.set_min_content_height(260);
    sc_scroll.set_child(Some(&sc_rows));
    sc_page.append(&sc_scroll);

    let sc_status = gtk::Label::new(None);
    sc_status.add_css_class("settings-card-desc");
    sc_status.set_xalign(0.0);
    sc_status.set_wrap(true);
    sc_page.append(&sc_status);

    let sc_reset_all = gtk::Button::with_label(tr("Reset all to defaults"));
    sc_reset_all.add_css_class("settings-option-item");
    sc_reset_all.set_halign(gtk::Align::Start);
    sc_page.append(&sc_reset_all);

    let is_syncing = Rc::new(Cell::new(false));
    let sc_ui = ScUi {
        rows: sc_rows.clone(),
        pending: sc_pending.clone(),
        status: sc_status.clone(),
        prio: sc_prio.clone(),
        is_syncing: is_syncing.clone(),
    };
    {
        let ui = sc_ui.clone();
        sc_prio.connect_toggled(move |c| {
            if ui.is_syncing.get() {
                return;
            }
            let rail_wins = c.is_active();
            {
                let mut p = ui.pending.borrow_mut();
                p.site_wins = !rail_wins;
                if rail_wins {
                    p.disabled.clear();
                } else {
                    for def in SHORTCUT_DEFS {
                        if def.id != SC_LOCKED {
                            p.disabled.insert(def.id.to_string());
                        }
                    }
                }
            }
            sc_commit(&ui);
            sc_refresh(&ui);
        });
    }
    {
        let ui = sc_ui.clone();
        let prio = sc_prio.clone();
        sc_page.connect_map(move |_| {
            *ui.pending.borrow_mut() = SHORTCUTS.with(|c| c.borrow().clone());
            let all_on = {
                let c = ui.pending.borrow();
                !c.site_wins && SHORTCUT_DEFS.iter().all(|d| sc_enabled(&c, d.id))
            };
            ui.is_syncing.set(true);
            prio.set_active(all_on);
            ui.is_syncing.set(false);
            ui.status.set_text("");
            sc_build(&ui);
        });
    }
    {
        let ui = sc_ui.clone();
        sc_reset_all.connect_clicked(move |_| {
            {
                let mut c = ui.pending.borrow_mut();
                c.overrides.clear();
                c.disabled.clear();
                c.site_wins = false;
            }
            ui.is_syncing.set(true);
            ui.prio.set_active(true);
            ui.is_syncing.set(false);
            sc_commit(&ui);
            ui.status.set_text(tr("All shortcuts are back to their defaults."));
            sc_build(&ui);
        });
    }

    // --- Customize sub-tab bar ---
    let customize_stack = gtk::Stack::new();
    customize_stack.set_vexpand(true);
    customize_stack.add_named(&page_css, Some("html"));
    customize_stack.add_named(&sc_page, Some("shortcuts"));

    let sub_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sub_html = gtk::Button::with_label(tr("HTML Defaults"));
    sub_html.add_css_class("settings-option-item");
    sub_html.add_css_class("active");
    let sub_sc = gtk::Button::with_label(tr("Shortcuts"));
    sub_sc.add_css_class("settings-option-item");
    sub_bar.append(&sub_html);
    sub_bar.append(&sub_sc);
    {
        let stack = customize_stack.clone();
        let other = sub_sc.clone();
        sub_html.connect_clicked(move |b| {
            stack.set_visible_child_name("html");
            b.add_css_class("active");
            other.remove_css_class("active");
        });
    }
    {
        let stack = customize_stack.clone();
        let other = sub_html.clone();
        sub_sc.connect_clicked(move |b| {
            stack.set_visible_child_name("shortcuts");
            b.add_css_class("active");
            other.remove_css_class("active");
        });
    }

    let page_customize = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_customize.add_css_class("settings-content-page");
    page_customize.set_vexpand(true);
    page_customize.append(&sub_bar);
    page_customize.append(&customize_stack);
    settings_stack.add_named(&page_customize, Some("customize"));

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
    let prof_page_subtitle = gtk::Label::new(Some(tr("Isolated workspaces, sessions & cookie storage  (Alt+P)")));
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
    let data_page_subtitle = gtk::Label::new(Some(tr("End-to-end encrypted backup and instant cloud synchronization across devices  (Alt+I)")));
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

    // --- Page 4: Privacy & Shield ---
    let page_privacy = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_privacy.add_css_class("settings-content-page");

    let priv_title = gtk::Label::new(Some(tr("Privacy & Security Shield")));
    priv_title.add_css_class("settings-page-title");
    priv_title.set_xalign(0.0);
    let priv_subtitle = gtk::Label::new(Some(tr("Container isolation, fingerprint shielding, and network leak prevention")));
    priv_subtitle.add_css_class("settings-page-subtitle");
    priv_subtitle.set_xalign(0.0);
    page_privacy.append(&priv_title);
    page_privacy.append(&priv_subtitle);

    let iso_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    iso_card.add_css_class("settings-card");
    let iso_title = gtk::Label::new(Some(tr("Multi-Container Storage Isolation")));
    iso_title.add_css_class("settings-card-title");
    iso_title.set_xalign(0.0);
    let iso_desc = gtk::Label::new(Some(tr("Each profile runs in its own isolated WebKit WebsiteDataManager directory with completely independent cookie jars, IndexedDB databases, and localStorage partitions. Websites cannot cross-track you between personas.")));
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

    let dnt_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dnt_lbl = gtk::Label::new(Some(tr("Do Not Track & Global Privacy Control")));
    dnt_lbl.add_css_class("shortcut-desc");
    dnt_lbl.set_hexpand(true);
    dnt_lbl.set_xalign(0.0);
    let dnt_badge = gtk::Label::new(Some(tr("Active")));
    dnt_badge.add_css_class("shortcut-kbd");
    dnt_row.append(&dnt_lbl);
    dnt_row.append(&dnt_badge);
    leak_card.append(&dnt_row);

    page_privacy.append(&leak_card);

    let page_priv_scroll = gtk::ScrolledWindow::new();
    page_priv_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_priv_scroll.set_child(Some(&page_privacy));
    settings_stack.add_named(&page_priv_scroll, Some("privacy"));

    // --- Page: Updates ---
    let page_update = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_update.add_css_class("settings-content-page");

    let update_title = gtk::Label::new(Some(tr("Updates")));
    update_title.add_css_class("settings-page-title");
    update_title.set_xalign(0.0);
    let update_subtitle = gtk::Label::new(Some(tr("Check for new versions and install updates from GitHub releases")));
    update_subtitle.add_css_class("settings-page-subtitle");
    update_subtitle.set_xalign(0.0);
    update_subtitle.set_wrap(true);
    page_update.append(&update_title);
    page_update.append(&update_subtitle);

    let auto_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    auto_card.add_css_class("settings-card");
    let auto_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let auto_lbl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    auto_lbl_box.set_hexpand(true);
    let auto_title = gtk::Label::new(Some(tr("Automatic updates")));
    auto_title.add_css_class("settings-card-title");
    auto_title.set_xalign(0.0);
    let auto_desc = gtk::Label::new(Some(tr("When enabled, Rail checks for a new version every launch and installs it")));
    auto_desc.add_css_class("settings-card-desc");
    auto_desc.set_xalign(0.0);
    auto_desc.set_wrap(true);
    auto_lbl_box.append(&auto_title);
    auto_lbl_box.append(&auto_desc);
    let auto_switch = gtk::Switch::new();
    auto_switch.set_active(load_app_settings().auto_update);
    auto_switch.set_valign(gtk::Align::Center);
    auto_row.append(&auto_lbl_box);
    auto_row.append(&auto_switch);
    auto_card.append(&auto_row);
    page_update.append(&auto_card);

    {
        auto_switch.connect_state_set(move |_, state| {
            let mut s = load_app_settings();
            s.auto_update = state;
            save_app_settings(&s);
            glib::Propagation::Proceed
        });
    }

    let update_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    update_card.add_css_class("settings-card");

    let cur_ver_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let cur_lbl = gtk::Label::new(Some(tr("Current version")));
    cur_lbl.add_css_class("settings-card-title");
    cur_lbl.set_xalign(0.0);
    cur_lbl.set_hexpand(true);
    let cur_badge = gtk::Label::new(Some(&format!("v{}", update::current_version())));
    cur_badge.add_css_class("settings-badge");
    cur_ver_row.append(&cur_lbl);
    cur_ver_row.append(&cur_badge);
    update_card.append(&cur_ver_row);

    let status_lbl = gtk::Label::new(Some(tr("Checking for updates...")));
    status_lbl.add_css_class("settings-card-desc");
    status_lbl.set_xalign(0.0);
    status_lbl.set_wrap(true);
    update_card.append(&status_lbl);

    let versions_lbl = gtk::Label::new(Some(tr("Available versions (from GitHub releases)")));
    versions_lbl.add_css_class("settings-card-desc");
    versions_lbl.set_xalign(0.0);
    versions_lbl.set_margin_top(8);
    update_card.append(&versions_lbl);

    let dropdown = gtk::DropDown::from_strings(&["Loading..."]);
    dropdown.set_hexpand(true);
    update_card.append(&dropdown);

    let update_btn = gtk::Button::with_label(tr("Update now"));
    update_btn.add_css_class("settings-option-item");
    update_btn.set_halign(gtk::Align::Start);
    update_btn.set_margin_top(8);
    update_btn.set_sensitive(false);
    update_card.append(&update_btn);

    let err_lbl = gtk::Label::new(None);
    err_lbl.add_css_class("settings-card-desc");
    err_lbl.set_xalign(0.0);
    err_lbl.set_wrap(true);
    err_lbl.set_visible(false);
    update_card.append(&err_lbl);

    let update_info: Rc<std::cell::RefCell<Option<update::UpdateInfo>>> = Rc::new(std::cell::RefCell::new(None));

    {
        let update_info = update_info.clone();
        let err_lbl = err_lbl.clone();
        update_btn.connect_clicked(move |b| {
            let Some(inf) = update_info.borrow().clone() else { return; };
            b.set_sensitive(false);
            b.set_label(tr("Updating..."));
            err_lbl.set_visible(false);
            let b = b.clone();
            let err_lbl = err_lbl.clone();
            glib::spawn_future_local(async move {
                let url = inf.url.clone();
                let res = gio::spawn_blocking(move || update::download_and_install(&url)).await;
                b.set_sensitive(true);
                match res {
                    Ok(Ok(())) => {
                        b.set_label(tr("Restarting..."));
                        b.set_sensitive(false);
                        if let Some(w) = b.root().and_then(|r| r.downcast::<gtk::ApplicationWindow>().ok()) {
                            glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || update::restart(&w));
                        }
                    }
                    Ok(Err(e)) => {
                        err_lbl.set_text(&e);
                        err_lbl.set_visible(true);
                        b.set_label(tr("Retry"));
                    }
                    Err(_) => {
                        err_lbl.set_text(tr("Update failed unexpectedly."));
                        err_lbl.set_visible(true);
                        b.set_label(tr("Retry"));
                    }
                }
            });
        });
    }




    page_update.append(&update_card);

    let page_update_scroll = gtk::ScrolledWindow::new();
    page_update_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_update_scroll.set_child(Some(&page_update));
    settings_stack.add_named(&page_update_scroll, Some("update"));

    // Populate asynchronously
    {
        let status_lbl = status_lbl.clone();
        let dropdown = dropdown.clone();
        let update_btn = update_btn.clone();
        let err_lbl = err_lbl.clone();
        let update_info = update_info.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(update::fetch_releases).await;
            match res {
                Ok(Some((tags, info))) => {
                    let model = gtk::StringList::new(&[]);
                    for t in &tags {
                        model.append(t);
                    }
                    dropdown.set_model(Some(&model));
                    // Select latest or current
                    let current = update::current_version();
                    let mut selected = 0;
                    for (i, t) in tags.iter().enumerate() {
                        if t.trim_start_matches('v') == current || t == &format!("v{}", current) {
                            selected = i;
                            break;
                        }
                    }
                    dropdown.set_selected(selected as u32);

                    if let Some(inf) = info {
                        let behind = format!("{}{}", inf.newer_count, if inf.more { "+" } else { "" });
                        status_lbl.set_text(&trf(
                            if inf.newer_count == 1 { "You are on v{}, {} version behind. Latest: {}" } else { "You are on v{}, {} versions behind. Latest: {}" },
                            &[&current, &behind, &inf.latest_tag],
                        ));
                        update_btn.set_sensitive(true);
                        update_btn.set_label(tr("Update now"));
                        *update_info.borrow_mut() = Some(inf);
                    } else {
                        status_lbl.set_text(&trf("rail {} is up to date", &[&current]));
                        update_btn.set_sensitive(false);
                        update_btn.set_label(tr("Up to date"));
                    }
                }
                _ => {
                    status_lbl.set_text(tr("Could not check for updates. Check your connection."));
                    err_lbl.set_text(tr("Failed to fetch releases from GitHub."));
                    err_lbl.set_visible(true);
                }
            }
        });
    }

    {
        let dropdown = dropdown.clone();
        let update_btn = update_btn.clone();
        let update_info = update_info.clone();
        let err_lbl = err_lbl.clone();
        dropdown.connect_selected_notify(move |dd| {
            let Some(item) = dd.selected_item() else { return; };
            let Some(str_obj) = item.downcast_ref::<gtk::StringObject>() else { return; };
            let tag = str_obj.string().to_string();
            let current = update::current_version();
            let current_parsed = update::parse_ver(&current);
            let sel_parsed = update::parse_ver(&tag);
            err_lbl.set_visible(false);

            match (sel_parsed, current_parsed) {
                (Some(s), Some(c)) if s == c => {
                    update_btn.set_sensitive(false);
                    update_btn.set_label(tr("Up to date"));
                    *update_info.borrow_mut() = None;
                }
                (Some(s), Some(c)) if s < c => {
                    update_btn.set_sensitive(true);
                    update_btn.set_label(tr("Install this version"));
                    *update_info.borrow_mut() = Some(update::UpdateInfo {
                        latest_tag: tag.clone(),
                        newer_count: 0,
                        more: false,
                        url: update::asset_url_for_tag(&tag),
                    });
                }
                _ => {
                    update_btn.set_sensitive(true);
                    update_btn.set_label(tr("Update now"));
                    *update_info.borrow_mut() = Some(update::UpdateInfo {
                        latest_tag: tag.clone(),
                        newer_count: 1,
                        more: false,
                        url: update::asset_url_for_tag(&tag),
                    });
                }
            }
        });
    }

    // --- Page 6: About Rail ---
    let page_about = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_about.add_css_class("settings-content-page");

    let about_title = gtk::Label::new(Some(tr("About Rail Browser")));
    about_title.add_css_class("settings-page-title");
    about_title.set_xalign(0.0);
    let about_subtitle = gtk::Label::new(Some(tr("High-performance native browser engineered in Rust")));
    about_subtitle.add_css_class("settings-page-subtitle");
    about_subtitle.set_xalign(0.0);
    page_about.append(&about_title);
    page_about.append(&about_subtitle);

    let about_card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    about_card.add_css_class("settings-card");

    let brand_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let brand_logo = gtk::Label::new(None);
    brand_logo.set_markup("<span weight='bold' font_family='monospace' size='xx-large' foreground='#f8fafc'>rail</span><span weight='bold' size='xx-large' foreground='#38bdf8'>.</span>");
    let version_badge = gtk::Label::new(Some(&format!("v{}", update::current_version())));
    version_badge.add_css_class("settings-badge");
    version_badge.set_valign(gtk::Align::Center);
    brand_row.append(&brand_logo);
    brand_row.append(&version_badge);
    about_card.append(&brand_row);

    let specs = [
        (tr("Core Engine"), "WebKitGTK 4.1 / WebKit 6.0"),
        (tr("GUI Framework"), "GTK 4.0 + GSK GPU Compositor"),
        (tr("Language"), tr("100% Pure Native Rust")),
        (tr("Architecture"), tr("Zero-Electron / Multi-Persona Sandboxed Profiles")),
        (tr("Data Directory"), "~/.local/share/rail/"),
        (tr("License"), "MIT License"),
    ];
    for (k_txt, v_txt) in specs {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("shortcut-row");
        let k = gtk::Label::new(Some(k_txt));
        k.add_css_class("shortcut-desc");
        k.set_xalign(0.0);
        k.set_hexpand(true);
        let v = gtk::Label::new(Some(v_txt));
        v.add_css_class("shortcut-kbd");
        row.append(&k);
        row.append(&v);
        about_card.append(&row);
    }
    page_about.append(&about_card);

    let page_about_scroll = gtk::ScrolledWindow::new();
    page_about_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_about_scroll.set_child(Some(&page_about));
    settings_stack.add_named(&page_about_scroll, Some("about"));

    // ---- overview hint ----
    let ov_hint = gtk::Label::new(Some(tr("h/l/j/k  move      Enter  open      x  close      Esc  cancel")));
    ov_hint.add_css_class("ov-hint");
    ov_hint.set_halign(gtk::Align::Center);
    ov_hint.set_valign(gtk::Align::End);
    ov_hint.set_margin_bottom(28);
    ov_hint.set_can_target(false);
    ov_hint.set_visible(false);

    let load_bar = gtk::ProgressBar::new();
    load_bar.add_css_class("load-bar");
    load_bar.set_valign(gtk::Align::Start);
    load_bar.set_can_target(false);
    load_bar.set_visible(false);

    // ---- Minimap / Roadmap (Alt+M) ----
    let map_header_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    map_header_row.add_css_class("minimap-header");
    map_header_row.set_cursor_from_name(Some("grab"));

    let map_icon = gtk::Image::from_icon_name("view-grid-symbolic");
    map_icon.add_css_class("minimap-header-icon");

    let map_title = gtk::Label::new(None);
    map_title.set_markup("<span>roadmap</span><span foreground='#38bdf8'>.</span>");
    map_title.add_css_class("minimap-header-title");
    map_title.set_hexpand(true);
    map_title.set_xalign(0.0);

    let map_badge = gtk::Label::new(Some(tr("Alt+M")));
    map_badge.add_css_class("minimap-badge");

    let map_hint = gtk::Label::new(Some(tr("Drag header")));
    map_hint.add_css_class("minimap-header-hint");
    map_hint.set_xalign(0.0);

    let map_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    map_close_btn.add_css_class("flat-icon");
    map_close_btn.set_tooltip_text(Some(tr("Close Map (Alt+M / Esc)")));
    map_close_btn.set_cursor_from_name(Some("pointer"));

    map_header_row.append(&map_icon);
    map_header_row.append(&map_title);
    map_header_row.append(&map_badge);
    map_header_row.append(&map_hint);
    map_header_row.append(&map_close_btn);

    let map_drawing_area = gtk::DrawingArea::new();
    map_drawing_area.add_css_class("minimap-canvas");
    map_drawing_area.set_content_width(380);
    map_drawing_area.set_content_height(260);

    let map_location_btn = gtk::Button::new();
    let loc_box = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let loc_icon = gtk::Image::from_icon_name("find-location-symbolic");
    let loc_lbl = gtk::Label::new(Some(tr("recenter")));
    loc_box.append(&loc_icon);
    loc_box.append(&loc_lbl);
    map_location_btn.set_child(Some(&loc_box));
    map_location_btn.add_css_class("minimap-location-btn");
    map_location_btn.set_halign(gtk::Align::Center);
    map_location_btn.set_valign(gtk::Align::End);
    map_location_btn.set_margin_bottom(12);
    map_location_btn.set_visible(false);

    let map_overlay = gtk::Overlay::new();
    map_overlay.set_child(Some(&map_drawing_area));
    map_overlay.add_overlay(&map_location_btn);

    let map_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    map_container.add_css_class("minimap-window");
    map_container.set_halign(gtk::Align::End);
    map_container.set_valign(gtk::Align::End);
    map_container.set_margin_end(24);
    map_container.set_margin_bottom(24);
    map_container.set_visible(false);

    map_container.append(&map_header_row);
    map_container.append(&map_overlay);

    // ---- Native Rail Developer Console (F12 / Alt+Shift+I) ----
    let devtools_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    devtools_header.add_css_class("devtools-header");

    let devtools_icon = gtk::Image::from_icon_name("utilities-terminal-symbolic");
    devtools_icon.add_css_class("devtools-brand-icon");

    let devtools_title = gtk::Label::new(None);
    devtools_title.set_markup("<span font_family='ui-monospace' font_weight='800'>rail</span><span foreground='#38bdf8'>.</span><span font_family='ui-monospace' foreground='#a1a1a1'>devtools</span>");
    devtools_title.add_css_class("devtools-brand-label");

    let devtools_badge = gtk::Label::new(Some("F12"));
    devtools_badge.add_css_class("devtools-badge");

    let devtools_url_lbl = gtk::Label::new(None);
    devtools_url_lbl.add_css_class("devtools-url-lbl");
    devtools_url_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
    devtools_url_lbl.set_max_width_chars(60);

    let dt_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dt_spacer.set_hexpand(true);

    let devtools_detach_btn = gtk::Button::from_icon_name("view-restore-symbolic");
    devtools_detach_btn.add_css_class("flat-icon");
    devtools_detach_btn.set_tooltip_text(Some(tr("Detach to Separate Window / Dock")));

    let devtools_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    devtools_close_btn.add_css_class("flat-icon");
    devtools_close_btn.set_tooltip_text(Some(tr("Close DevTools (Esc / F12)")));

    devtools_header.append(&devtools_icon);
    devtools_header.append(&devtools_title);
    devtools_header.append(&devtools_badge);
    devtools_header.append(&devtools_url_lbl);
    devtools_header.append(&dt_spacer);
    devtools_header.append(&devtools_detach_btn);
    devtools_header.append(&devtools_close_btn);

    let devtools_resize_handle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    devtools_resize_handle.add_css_class("devtools-resize-handle");

    let devtools_content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    devtools_content_box.add_css_class("devtools-content-area");
    devtools_content_box.set_vexpand(true);
    devtools_content_box.set_hexpand(true);

    let devtools_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    devtools_panel.add_css_class("devtools-panel");
    devtools_panel.add_css_class("dock-bottom");
    devtools_panel.set_halign(gtk::Align::Fill);
    devtools_panel.set_valign(gtk::Align::End);
    devtools_panel.set_height_request(400);
    devtools_panel.set_visible(false);

    devtools_panel.append(&devtools_resize_handle);
    devtools_panel.append(&devtools_content_box);

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&shell));
    overlay.add_overlay(&load_bar);
    let touchpad_toast = gtk::Label::new(None);
    touchpad_toast.add_css_class("ov-hint");
    touchpad_toast.set_halign(gtk::Align::Center);
    touchpad_toast.set_valign(gtk::Align::End);
    touchpad_toast.set_margin_bottom(28);
    touchpad_toast.set_can_target(false);
    touchpad_toast.set_visible(false);

    overlay.add_overlay(&palette);
    overlay.add_overlay(&ov_hint);
    overlay.add_overlay(&touchpad_toast);
    overlay.add_overlay(&settings_modal);
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

    let app = Rc::new(App {
        win: win.clone(),
        shell: shell.clone(),
        shell_layout,
        side,
        strip,
        layout: layout.clone(),
        list: list.clone(),
        pill_label,
        load_bar,
        palette,
        entry: entry.clone(),
        palette_for_new_tab: Cell::new(false),
        palette_suggestions_box: palette_suggestions_box.clone(),
        history: RefCell::new(history_mgr),
        current_suggestions: RefCell::new(Vec::new()),
        selected_suggestion_idx: Cell::new(None),
        is_autofilling: Cell::new(false),
        last_entry_len: Cell::new(0),
        delete_key: Cell::new(false),
        ov_hint,
        ov: Cell::new(false),
        ov_return: Cell::new((0, 0)),
        ov_saved_x: Cell::new(0.0),
        ov_saved_y: Cell::new(0.0),
        target_zoom: Cell::new(1.0),
        rows: RefCell::new(Vec::new()),
        focus_row: Cell::new(0),
        focus_col: Cell::new(0),
        target_x: Cell::new(0.0),
        target_y: Cell::new(0.0),
        animating: Cell::new(false),
        pan_alloc_pending: Cell::new(false),
        side_open: Cell::new(true),
        side_gen: Cell::new(0),
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
        touchpad_mode: Cell::new(false),
        tp_switch: tp_switch.clone(),
        touchpad_toast,
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
        devtools_dock_side: Cell::new(DevToolsDockSide::Bottom),
        devtools_panel: devtools_panel.clone(),
        devtools_content_box: devtools_content_box.clone(),
        devtools_resize_handle: devtools_resize_handle.clone(),
        devtools_url_lbl: devtools_url_lbl.clone(),
        devtools_window: RefCell::new(None),
        settings_modal: settings_modal.clone(),
        settings_stack: settings_stack.clone(),
        settings_tab_buttons: RefCell::new(tab_buttons_vec.clone()),
        current_settings_tab: Cell::new(SettingsTab::General),
        css_buffer: css_buffer.clone(),
        search_engine: RefCell::new(initial_settings.search_engine.clone()),
    });

    app.touchpad_mode.set(initial_settings.touchpad_mode);
    tp_switch.set_active(initial_settings.touchpad_mode);
    app.select_palette_profile(&initial_prof_id);

    let a_close = app.clone();
    map_close_btn.connect_clicked(move |_| a_close.hide_map());

    let a_loc = app.clone();
    map_location_btn.connect_clicked(move |_| {
        {
            let mut st = a_loc.map_state.borrow_mut();
            st.target_offset_x = 0.0;
            st.target_offset_y = 0.0;
            st.vel_x = 0.0;
            st.vel_y = 0.0;
            st.user_panned = false;
        }
        a_loc.map_location_btn.set_visible(false);
        a_loc.trigger_map_animation();
    });

    let a_draw = app.clone();
    map_drawing_area.set_draw_func(move |_area, cr, width, height| {
        a_draw.draw_minimap(cr, width as f64, height as f64);
    });

    let drag_dist = Rc::new(Cell::new(0.0));
    let is_pinching = Rc::new(Cell::new(false));
    let pinch_end_time = Rc::new(Cell::new(0i64));
    let last_drag_pos = Rc::new(Cell::new((0.0f64, 0.0f64)));
    let last_drag_t = Rc::new(Cell::new(0i64));

    let drag = gtk::GestureDrag::new();
    drag.connect_drag_begin({
        let a = app.clone();
        let drag_dist = drag_dist.clone();
        let last_pos = last_drag_pos.clone();
        let last_t = last_drag_t.clone();
        move |_gesture, start_x, start_y| {
            drag_dist.set(0.0);
            last_pos.set((start_x, start_y));
            last_t.set(glib::monotonic_time());
            let mut st = a.map_state.borrow_mut();
            st.drag_start_offset_x = st.target_offset_x;
            st.drag_start_offset_y = st.target_offset_y;
            st.vel_x = 0.0;
            st.vel_y = 0.0;
        }
    });

    drag.connect_drag_update({
        let a = app.clone();
        let drag_dist = drag_dist.clone();
        let is_pinching = is_pinching.clone();
        let pinch_end_time = pinch_end_time.clone();
        let last_t = last_drag_t.clone();
        move |_gesture, offset_x, offset_y| {
            if is_pinching.get() || (glib::monotonic_time() - pinch_end_time.get()) < 200_000 {
                return;
            }
            drag_dist.set(offset_x.hypot(offset_y));

            let now = glib::monotonic_time();
            let dt = ((now - last_t.get()) as f64 / 1e6).max(0.001);

            let mut st = a.map_state.borrow_mut();
            let new_target_ox = st.drag_start_offset_x - offset_x / st.zoom;
            let new_target_oy = st.drag_start_offset_y - offset_y / st.zoom;

            let instant_vx = (new_target_ox - st.target_offset_x) / dt;
            let instant_vy = (new_target_oy - st.target_offset_y) / dt;
            st.vel_x = st.vel_x * 0.3 + instant_vx.clamp(-3000.0, 3000.0) * 0.7;
            st.vel_y = st.vel_y * 0.3 + instant_vy.clamp(-3000.0, 3000.0) * 0.7;

            st.target_offset_x = new_target_ox;
            st.target_offset_y = new_target_oy;
            st.offset_x = new_target_ox;
            st.offset_y = new_target_oy;
            st.user_panned = true;

            last_t.set(now);
            drop(st);
            a.update_map_location_btn_visibility();
            a.map_drawing_area.queue_draw();
        }
    });

    drag.connect_drag_end({
        let a = app.clone();
        let drag_dist = drag_dist.clone();
        move |_gesture, _offset_x, _offset_y| {
            if drag_dist.get() > 4.0 {
                a.trigger_map_animation();
            }
        }
    });
    map_drawing_area.add_controller(drag);

    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_PRIMARY);
    let a_click = app.clone();
    let drag_dist_for_click = drag_dist.clone();
    let is_pinching_click = is_pinching.clone();
    let pinch_end_time_click = pinch_end_time.clone();
    click.connect_released(move |_gesture, _n_press, x, y| {
        if is_pinching_click.get() || (glib::monotonic_time() - pinch_end_time_click.get()) < 250_000 {
            return;
        }
        if drag_dist_for_click.get() < 6.0 {
            a_click.handle_minimap_click(x, y);
        }
    });
    map_drawing_area.add_controller(click);

    let scroll = gtk::EventControllerScroll::new(
        gtk::EventControllerScrollFlags::BOTH_AXES,
    );
    let a_scroll = app.clone();
    scroll.connect_scroll(move |_ctrl, _dx, dy| {
        if dy.abs() > 0.001 {
            let mut st = a_scroll.map_state.borrow_mut();
            let factor = 1.0 - (dy * 0.12).clamp(-0.35, 0.35);
            st.target_zoom = (st.target_zoom * factor).clamp(0.2, 4.0);
            drop(st);
            a_scroll.trigger_map_animation();
        }
        glib::Propagation::Stop
    });
    map_drawing_area.add_controller(scroll);

    let zoom_gesture = gtk::GestureZoom::new();
    let prev_scale = Rc::new(Cell::new(1.0f64));

    {
        let prev_scale = prev_scale.clone();
        let is_pinching = is_pinching.clone();
        zoom_gesture.connect_begin(move |_gesture, _seq| {
            prev_scale.set(1.0);
            is_pinching.set(true);
        });
    }

    {
        let is_pinching = is_pinching.clone();
        let pinch_end_time = pinch_end_time.clone();
        let a = app.clone();
        zoom_gesture.connect_end(move |_gesture, _seq| {
            is_pinching.set(false);
            pinch_end_time.set(glib::monotonic_time());
            a.trigger_map_animation();
        });
    }

    {
        let a = app.clone();
        let prev_scale = prev_scale.clone();
        zoom_gesture.connect_scale_changed(move |_gesture, scale| {
            let p = prev_scale.get();
            if p > 0.0 && scale > 0.0 {
                let factor = scale / p;
                let mut st = a.map_state.borrow_mut();
                st.target_zoom = (st.target_zoom * factor).clamp(0.2, 4.0);
                st.zoom = st.target_zoom;
                drop(st);
                a.update_map_location_btn_visibility();
                a.map_drawing_area.queue_draw();
            }
            prev_scale.set(scale);
        });
    }
    map_drawing_area.add_controller(zoom_gesture);

    // ---- Minimap Header Dragging (reposition floating roadmap window) ----
    let header_drag = gtk::GestureDrag::new();
    header_drag.set_button(gdk::BUTTON_PRIMARY);

    let header_start_margin_end = Rc::new(Cell::new(24));
    let header_start_margin_bottom = Rc::new(Cell::new(24));
    let header_start_root_x = Rc::new(Cell::new(0.0f64));
    let header_start_root_y = Rc::new(Cell::new(0.0f64));

    {
        let start_e = header_start_margin_end.clone();
        let start_b = header_start_margin_bottom.clone();
        let start_rx = header_start_root_x.clone();
        let start_ry = header_start_root_y.clone();
        let map_container = map_container.clone();
        let map_header_row = map_header_row.clone();
        header_drag.connect_drag_begin(move |gesture, _start_x, _start_y| {
            start_e.set(map_container.margin_end());
            start_b.set(map_container.margin_bottom());
            if let Some(ev) = gesture.current_event() {
                if let Some((rx, ry)) = ev.position() {
                    start_rx.set(rx);
                    start_ry.set(ry);
                }
            }
            map_header_row.set_cursor_from_name(Some("grabbing"));
        });
    }

    {
        let start_e = header_start_margin_end.clone();
        let start_b = header_start_margin_bottom.clone();
        let start_rx = header_start_root_x.clone();
        let start_ry = header_start_root_y.clone();
        let map_container = map_container.clone();
        let overlay = overlay.clone();
        header_drag.connect_drag_update(move |gesture, offset_x, offset_y| {
            let (dx, dy) = if let Some(ev) = gesture.current_event() {
                if let Some((rx, ry)) = ev.position() {
                    (rx - start_rx.get(), ry - start_ry.get())
                } else {
                    (offset_x, offset_y)
                }
            } else {
                (offset_x, offset_y)
            };

            let new_m_end = start_e.get() - (dx.round() as i32);
            let new_m_bottom = start_b.get() - (dy.round() as i32);

            let overlay_w = overlay.width().max(600);
            let overlay_h = overlay.height().max(400);
            let map_w = map_container.width().max(380);
            let map_h = map_container.height().max(260);

            let max_margin_e = (overlay_w - map_w - 8).max(8);
            let max_margin_b = (overlay_h - map_h - 8).max(8);

            map_container.set_margin_end(new_m_end.clamp(8, max_margin_e));
            map_container.set_margin_bottom(new_m_bottom.clamp(8, max_margin_b));
        });
    }

    {
        let map_header_row = map_header_row.clone();
        header_drag.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
            map_header_row.set_cursor_from_name(Some("grab"));
        });
    }
    map_header_row.add_controller(header_drag);

    let a = app.clone();
    settings_close_btn.connect_clicked(move |_| a.hide_settings_modal());

    {
        let a = app.clone();
        settings_update_chip.connect_clicked(move |_| {
            a.open_settings_tab(SettingsTab::Update);
        });
    }

    // Populate header "New version" chip from GitHub (same source as Updates page)
    {
        let chip = settings_update_chip.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(update::fetch_update).await;
            if let Ok(Some(inf)) = res {
                chip.set_label(&trf("New: {}", &[&inf.latest_tag]));
                chip.set_visible(true);
            }
        });
    }

    for (tab_type, btn) in tab_buttons_vec.iter() {
        let a = app.clone();
        let t = *tab_type;
        btn.connect_clicked(move |_| a.open_settings_tab(t));
    }

    for (engine_id, btn) in engine_buttons.iter() {
        let a = app.clone();
        let id_str = engine_id.clone();
        let engine_btns_clone = engine_buttons.clone();
        btn.connect_clicked(move |_| {
            *a.search_engine.borrow_mut() = id_str.clone();
            for (eid, b) in engine_btns_clone.iter() {
                if eid == &id_str {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
            a.save_session_immediate();
            let mut s = load_app_settings();
            s.search_engine = id_str.clone();
            save_app_settings(&s);
        });
    }

    let a = app.clone();
    btn_clear_hist.connect_clicked(move |_| a.clear_history_action());

    let a = app.clone();
    btn_clear_cache.connect_clicked(move |_| a.clear_cache_action());

    let a = app.clone();
    tp_switch.connect_state_set(move |_, state| {
        a.touchpad_mode.set(state);
        a.schedule_save_session();
        let mut s = load_app_settings();
        s.touchpad_mode = state;
        save_app_settings(&s);
        glib::Propagation::Proceed
    });

    let a = app.clone();
    btn_sync_now.connect_clicked(move |_| a.prompt_and_execute_sync());

    let a = app.clone();
    btn_sync_restore.connect_clicked(move |_| a.prompt_and_execute_restore());

    let a = app.clone();
    sync_restore_entry.connect_activate(move |_| a.prompt_and_execute_restore());

    let a = app.clone();
    let entry_res = sync_url_result_entry.clone();
    btn_copy_sync_url.connect_clicked(move |btn| {
        let text = entry_res.text().to_string();
        if !text.is_empty() {
            a.win.clipboard().set_text(&text);
            btn.set_label(tr("Copied!"));
            let b = btn.clone();
            glib::timeout_add_local_once(Duration::from_millis(1500), move || {
                b.set_label(tr("Copy Link"));
            });
        }
    });

    let a = app.clone();
    let win_weak = win.downgrade();
    btn_export_local.connect_clicked(move |_| {
        let Some(win) = win_weak.upgrade() else { return };
        let timestamp = glib::DateTime::now_local()
            .ok()
            .and_then(|d| d.format("%Y%m%d_%H%M%S").ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "backup".to_string());
        let default_name = format!("rail_backup_{}.tar.gz", timestamp);
        let dialog = gtk::FileDialog::builder()
            .title(tr("Save Rail Backup Archive (.tar.gz)"))
            .initial_name(&default_name)
            .build();
        let a = a.clone();
        dialog.save(Some(&win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    a.data_export_entry.set_text(&path.to_string_lossy());
                    a.execute_export_action();
                }
            }
        });
    });

    let a = app.clone();
    let win_weak = win.downgrade();
    btn_import_local.connect_clicked(move |_| {
        let Some(win) = win_weak.upgrade() else { return };
        let dialog = gtk::FileDialog::builder()
            .title(tr("Select Rail Backup Archive (.tar.gz)"))
            .build();
        let a = a.clone();
        dialog.open(Some(&win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    a.data_import_entry.set_text(&path.to_string_lossy());
                    a.execute_import_action();
                }
            }
        });
    });

    for (i, btn) in app.color_buttons.borrow().iter().enumerate() {
        let a = app.clone();
        btn.connect_clicked(move |_| a.select_new_profile_color(i));
    }

    let a = app.clone();
    add_prof_btn.connect_clicked(move |_| a.create_new_profile_action());
    let a = app.clone();
    new_profile_entry.connect_activate(move |_| a.create_new_profile_action());

    let a = app.clone();
    new_profile_entry.connect_changed(move |_| {
        a.update_create_profile_btn_state();
    });

    let a = app.clone();
    layout.connect_resized(move || {
        a.reveal_focus();
        if a.palette_open() {
            a.update_palette_size();
        }
    });

    // WebKit's own inspector toolbar handles close/dock/undock via closed/attach/detach signals.
    // The attach signal carries no side, so remember which toolbar slot was clicked (never claims the click).
    let dt_click = gtk::GestureClick::new();
    dt_click.set_propagation_phase(gtk::PropagationPhase::Capture);
    dt_click.connect_pressed(move |g, _, x, y| {
        g.set_state(gtk::EventSequenceState::None);
        if (0.0..=36.0).contains(&y) {
            LAST_DT_SLOT.with(|c| c.set((x / 24.0).floor() as i32));
        } else {
            LAST_DT_SLOT.with(|c| c.set(-1));
        }
    });
    devtools_content_box.add_controller(dt_click);

    // DevTools resize drag handle
    let dt_drag = gtk::GestureDrag::new();
    let app_drag = app.clone();
    let start_size = Rc::new(Cell::new(0i32));
    let st_sz_begin = start_size.clone();
    dt_drag.connect_drag_begin(move |_, _, _| {
        let side = app_drag.devtools_dock_side.get();
        let cur = match side {
            DevToolsDockSide::Bottom => app_drag.devtools_panel.height(),
            DevToolsDockSide::Left | DevToolsDockSide::Right => app_drag.devtools_panel.width(),
            _ => 0,
        };
        st_sz_begin.set(cur);
    });

    let app_drag_up = app.clone();
    let st_sz_update = start_size.clone();
    dt_drag.connect_drag_update(move |_, offset_x, offset_y| {
        let side = app_drag_up.devtools_dock_side.get();
        let base = st_sz_update.get() as f64;
        let win_w = app_drag_up.win.width() as f64;
        let win_h = app_drag_up.win.height() as f64;
        match side {
            DevToolsDockSide::Bottom => {
                let nh = (base - offset_y).clamp(180.0, (win_h - 100.0).max(200.0)) as i32;
                app_drag_up.devtools_panel.set_height_request(nh);
            }
            DevToolsDockSide::Left => {
                let nw = (base + offset_x).clamp(280.0, (win_w - 200.0).max(300.0)) as i32;
                app_drag_up.devtools_panel.set_width_request(nw);
            }
            DevToolsDockSide::Right => {
                let nw = (base - offset_x).clamp(280.0, (win_w - 200.0).max(300.0)) as i32;
                app_drag_up.devtools_panel.set_width_request(nw);
            }
            _ => {}
        }
    });
    devtools_resize_handle.add_controller(dt_drag);

    let a_dt_close = app.clone();
    devtools_close_btn.connect_clicked(move |_| a_dt_close.hide_devtools());

    let a_dt_detach = app.clone();
    devtools_detach_btn.connect_clicked(move |_| a_dt_detach.toggle_devtools_undocked());


    let a = app.clone();
    add.connect_clicked(move |_| a.open_settings_tab(SettingsTab::General));
    let a = app.clone();
    pill.connect_clicked(move |_| a.show_palette());

    let a = app.clone();
    entry.connect_changed(move |_| {
        a.handle_entry_changed();
    });

    let entry_keys = gtk::EventControllerKey::new();
    entry_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let a = app.clone();
    let ent = entry.clone();
    entry_keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let shift = state.contains(M::SHIFT_MASK);
        let alt = state.contains(M::ALT_MASK);
        let k = key.to_lower();

        a.delete_key.set(
            matches!(key, gdk::Key::BackSpace | gdk::Key::Delete)
                || (ctrl && !alt && (k == gdk::Key::x || k == gdk::Key::w || k == gdk::Key::u || k == gdk::Key::k || k == gdk::Key::h)),
        );

        if ctrl && !shift && !alt && (k == gdk::Key::v || k == gdk::Key::V || keycode == 55) {
            let clipboard = ent.clipboard();
            let entry_clone = ent.clone();
            clipboard.read_text_async(gio::Cancellable::NONE, move |res| {
                if let Ok(Some(text)) = res {
                    let text = text.to_string();
                    let (start, end) = entry_clone.selection_bounds().unwrap_or_else(|| {
                        let pos = entry_clone.position();
                        (pos, pos)
                    });
                    entry_clone.delete_text(start, end);
                    let mut insert_pos = start;
                    entry_clone.insert_text(&text, &mut insert_pos);
                    entry_clone.set_position(insert_pos);
                }
            });
            return glib::Propagation::Stop;
        }

        match key {
            gdk::Key::Down => {
                if a.move_suggestion_selection(1) {
                    return glib::Propagation::Stop;
                }
            }
            gdk::Key::Up => {
                if a.move_suggestion_selection(-1) {
                    return glib::Propagation::Stop;
                }
            }
            gdk::Key::Tab => {
                if a.accept_autofill_or_next_suggestion() {
                    return glib::Propagation::Stop;
                }
            }
            _ => {}
        }
        glib::Propagation::Proceed
    });
    entry.add_controller(entry_keys);

    let a = app.clone();
    entry.connect_activate(move |e| {
        if let Some(idx) = a.selected_suggestion_idx.get() {
            let sugs = a.current_suggestions.borrow();
            if let Some(sug) = sugs.get(idx) {
                let target = sug.target_url.clone();
                drop(sugs);
                a.navigate(&target);
                return;
            }
        }
        a.navigate(&e.text());
    });

    let pick = gtk::GestureClick::new();
    let a = app.clone();
    pick.connect_released(move |_, _, x, y| {
        if a.ov.get() {
            a.ov_click(x, y);
        }
    });
    app.strip.add_controller(pick);

    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let a = app.clone();
    click.connect_pressed(move |_, _, _, _| {
        if a.palette_open() {
            a.hide_palette();
        }
        if a.settings_modal_open() {
            a.hide_settings_modal();
        }
    });
    shell.add_controller(click);

    // BOTH_AXES only: KINETIC+DISCRETE on a Capture window controller makes page scroll sticky/janky
    // (steals momentum from WebKit). Overview + touchpad slide still work via dx/dy deltas.
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let accum_x = Rc::new(Cell::new(0.0f64));
    let accum_y = Rc::new(Cell::new(0.0f64));
    let last_scroll = Rc::new(Cell::new(0i64));
    let last_swipe = Rc::new(Cell::new(0i64));
    let a = app.clone();
    let a_dec = app.clone();
    let ax = accum_x.clone();
    let ay = accum_y.clone();
    let ls = last_scroll.clone();
    let l_swipe = last_swipe.clone();
    scroll.connect_scroll(move |_, dx, dy| {
        // Fast path: normal browsing — do not touch scroll at all (WebKit keeps smooth kinetic scroll)
        if !a.ov.get() && !a.touchpad_mode.get() {
            return glib::Propagation::Proceed;
        }
        if a.settings_modal_open() || a.palette_open() || a.map_open() {
            return glib::Propagation::Proceed;
        }

        let now = glib::monotonic_time();

        // 2. Overview mode: 2D canvas pan navigation on zoomed-out workspace grid
        if a.ov.get() {
            // Cooldown prevents jumping across multiple rows/columns in a single swipe stroke
            if now - l_swipe.get() < 280_000 {
                ax.set(0.0);
                ay.set(0.0);
                return glib::Propagation::Stop;
            }

            if now - ls.get() > 250_000 {
                ax.set(0.0);
                ay.set(0.0);
            }
            ls.set(now);

            let nx = ax.get() + dx;
            let ny = ay.get() + dy;
            ax.set(nx);
            ay.set(ny);

            const OV_THRESHOLD: f64 = 6.0;
            if nx > OV_THRESHOLD {
                ax.set(0.0);
                l_swipe.set(now);
                a.focus_column_right();
                return glib::Propagation::Stop;
            } else if nx < -OV_THRESHOLD {
                ax.set(0.0);
                l_swipe.set(now);
                a.focus_column_left();
                return glib::Propagation::Stop;
            }

            if ny > OV_THRESHOLD {
                ay.set(0.0);
                l_swipe.set(now);
                a.focus_row_down();
                return glib::Propagation::Stop;
            } else if ny < -OV_THRESHOLD {
                ay.set(0.0);
                l_swipe.set(now);
                a.focus_row_up();
                return glib::Propagation::Stop;
            }
            return glib::Propagation::Proceed;
        }

        // 3. Touchpad slide mode (Alt+Shift+T): 1:1 finger tracking, not discrete tab jumps
        if !a.touchpad_mode.get() {
            return glib::Propagation::Proceed;
        }

        // Only clearly horizontal gestures pan the ribbon; everything else (incl. diagonal
        // reading scroll) must reach WebKit or the page feels sticky.
        if dx.abs() > 0.8 && dx.abs() > 2.0 * dy.abs() {
            ls.set(now);
            l_swipe.set(now);
            a.pan_canvas_x(dx);
            return glib::Propagation::Stop;
        }

        glib::Propagation::Proceed
    });
    let ax2 = accum_x.clone();
    let ay2 = accum_y.clone();
    let l_swipe2 = last_swipe.clone();
    scroll.connect_decelerate(move |_, _vx, _vy| {
        if a_dec.touchpad_mode.get() && !a_dec.ov.get() {
            a_dec.snap_canvas_to_nearest_column();
        }
        ax2.set(0.0);
        ay2.set(0.0);
        l_swipe2.set(0);
    });
    win.add_controller(scroll);

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    KEY_CTL.with(|k| *k.borrow_mut() = Some(keys.clone()));
    apply_shortcut_priority();
    let held_shortcut: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));

    let a = app.clone();
    let held = held_shortcut.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let alt = state.contains(M::ALT_MASK);
        let super_key = state.contains(M::SUPER_MASK);

        // Image paste: WebKitGTK gives pages an empty clipboardData for images
        if ctrl && !alt && !state.contains(M::SHIFT_MASK)
            && (key == gdk::Key::v || key == gdk::Key::V || keycode == 55)
            && a.try_paste_image()
        {
            return glib::Propagation::Stop;
        }

        // Autorepeat loop suppressor: block machine-gun repeat when shortcut is held down
        if ctrl || alt || super_key {
            if held.get() == Some(keycode) {
                return glib::Propagation::Stop;
            }
        }

        let combo = shortcut_combo(key, keycode, state);
        let handled = match combo.as_deref().map(sc_resolve) {
            Some(ScResolve::Action(def_combo)) => {
                if Some(def_combo.as_str()) == combo.as_deref() {
                    a.on_key(key, keycode, state)
                } else if let Some((k2, m2)) = sc_synth(&def_combo) {
                    a.on_key(k2, 0, m2)
                } else {
                    false
                }
            }
            Some(ScResolve::Disabled) => false,
            _ => a.on_key(key, keycode, state),
        };
        if handled {
            if ctrl || alt || super_key {
                held.set(Some(keycode));
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    let held_rel = held_shortcut.clone();
    keys.connect_key_released(move |_, _, keycode, _| {
        if held_rel.get() == Some(keycode) {
            held_rel.set(None);
        }
    });

    let held_focus = held_shortcut.clone();
    win.connect_has_focus_notify(move |_| {
        held_focus.set(None);
    });
    win.add_controller(keys);

    let a = app.clone();
    win.connect_close_request(move |_| {
        a.save_session_immediate();
        glib::Propagation::Proceed
    });

    let restored = app.restore_session();
    if let Some(url) = TARGET_URL.get() {
        if restored {
            app.new_tab();
            app.navigate(url);
        } else {
            app.navigate(url);
        }
    } else if !restored {
        app.new_tab();
    }
    app.update_window_title();
    win.present();
}

fn export_data(archive_path: &Path) -> Result<(), String> {
    let dir = rail_data_dir();
    if !dir.exists() {
        return Err(trf("Storage directory does not exist: {}", &[&dir.display().to_string()]));
    }
    let parent = dir.parent().unwrap_or(&dir);
    let name = dir.file_name().ok_or_else(|| tr("Invalid directory name").to_string())?;

    let abs_archive = if archive_path.is_absolute() {
        archive_path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?.join(archive_path)
    };

    let status = std::process::Command::new("tar")
        .arg("--exclude=*/cache")
        .arg("--exclude=*/cache/*")
        .arg("-czf")
        .arg(&abs_archive)
        .arg("-C")
        .arg(parent)
        .arg(name)
        .status()
        .map_err(|e| trf("Failed to run tar command: {}", &[&e.to_string()]))?;

    if status.success() {
        println!("{}", trf("Successfully exported all Rail data to: {}", &[&abs_archive.display().to_string()]));
        println!("{}", tr("This file contains all your tabs, workspaces, profiles, logins, and cookies."));
        Ok(())
    } else {
        Err(trf("tar command failed with status: {}", &[&status.to_string()]))
    }
}

fn import_data(archive_path: &Path) -> Result<(), String> {
    let abs_archive = if archive_path.is_absolute() {
        archive_path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?.join(archive_path)
    };

    if !abs_archive.exists() {
        return Err(trf("Archive file does not exist: {}", &[&abs_archive.display().to_string()]));
    }

    let dest_dir = rail_data_dir();
    let parent = dest_dir.parent().unwrap_or(&dest_dir);
    let _ = std::fs::create_dir_all(parent);

    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&abs_archive)
        .arg("-C")
        .arg(parent)
        .status()
        .map_err(|e| trf("Failed to run tar command: {}", &[&e.to_string()]))?;

    if status.success() {
        println!("{}", trf("Successfully imported all Rail data into: {}", &[&dest_dir.display().to_string()]));
        println!("{}", tr("You can now launch 'rail' and all tabs, workspaces, logins, and accounts will be active."));
        Ok(())
    } else {
        Err(trf("tar command failed with status: {}", &[&status.to_string()]))
    }
}

fn print_data_info() {
    let dir = rail_data_dir();
    println!("{}", trf("Rail Unified Storage Directory: {}", &[&dir.display().to_string()]));
    println!("--------------------------------------------------");
    if !dir.exists() {
        println!("{}", tr("Status: Directory does not exist yet (clean slate)"));
        return;
    }

    let session_file = dir.join("session.txt");
    if session_file.exists() {
        if let Ok(c) = std::fs::read_to_string(&session_file) {
            let tabs = c.lines().filter(|l| l.starts_with("tab\t")).count();
            println!("{}", trf("  Session:       {} saved tabs", &[&tabs.to_string()]));
        }
    } else {
        println!("{}", tr("  Session:       No saved session (clean start on next launch)"));
    }

    let profiles_file = dir.join("profiles.txt");
    if profiles_file.exists() {
        if let Ok(c) = std::fs::read_to_string(&profiles_file) {
            let count = c.lines().filter(|l| !l.trim().is_empty()).count();
            println!("{}", trf("  Profiles:      {} profiles configured", &[&count.to_string()]));
        }
    }

    let profiles_dir = dir.join("profiles");
    if profiles_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    let has_cookies = p.join("data").join("cookies.sqlite").exists();
                    let has_storage = p.join("data").join("storage").exists();
                    println!("{}", trf("    - Profile '{}': accounts/cookies={}, localStorage/IndexedDB={}", &[&name, &has_cookies.to_string(), &has_storage.to_string()]));
                }
            }
        }
    }

    println!();
    println!("{}", tr("To share your exact environment with a friend:"));
    println!("  Export:   rail --export rail_backup.tar.gz");
    println!("  Import:   rail --import rail_backup.tar.gz");
    println!("{}", trf("  Or copy:  cp -r {} /path/to/destination", &[&dir.display().to_string()]));
}

fn main() -> glib::ExitCode {
    // RAM: constrain WebKit early (before any NetworkSession / WebView)
    // DocumentViewer cache model uses far less memory than full WebBrowser.
    unsafe {
        // Prefer software path / less compositor memory when many processes spawn
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    {
        let mut mps = webkit6::MemoryPressureSettings::new();
        // Soft limit ~384 MiB of process memory before WebKit starts reclaiming aggressively
        mps.set_memory_limit(384);
        mps.set_conservative_threshold(0.33);
        mps.set_strict_threshold(0.50);
        mps.set_kill_threshold(0.0); // 0 = disabled kill; we suspend tabs ourselves
        mps.set_poll_interval(5.0);
        webkit6::NetworkSession::set_memory_pressure_settings(&mut mps);
        if let Some(ctx) = webkit6::WebContext::default() {
            ctx.set_cache_model(webkit6::CacheModel::DocumentViewer);
        }
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
            println!("{}", tr("  -d, --data-dir <PATH>      Use custom storage directory for 100% portable environment"));
            println!("{}", tr("      --export <FILE.tar.gz> Export all tabs, profiles, logins & cookies into archive"));
            println!("{}", tr("      --import <FILE.tar.gz> Import all tabs, profiles, logins & cookies from archive"));
            println!("{}", tr("      --data-info            Show current storage directory and environment details"));
            println!("{}", tr("      --update               Download and install the newest release"));
            println!("{}", tr("  -v, --version              Show version"));
            println!("{}", tr("  -h, --help                 Show help message"));
            println!();
            println!("{}", tr("Environment Variables:"));
            println!("{}", tr("  RAIL_DATA_DIR              Override default storage directory (~/.local/share/rail)"));
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
                eprintln!("{}", tr("Error: --export requires an output filename (e.g. rail --export backup.tar.gz)"));
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
                eprintln!("{}", tr("Error: --import requires an archive filename (e.g. rail --import backup.tar.gz)"));
                return glib::ExitCode::FAILURE;
            }
        }
        if arg == "-d" || arg == "--data-dir" {
            i += 2;
            continue;
        } else if arg.starts_with("--data-dir=") {
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

    let app = gtk::Application::builder()
        .application_id("dev.rail.browser")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build);
    app.run_with_args::<&str>(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_content() {
        let sample = "focus\t1\t0\nsidebar\t1\ntouchpad\t0\nwin\t1400\t850\t0\ntab\t0\t0\t1\tdefault\thttps://google.com\ntab\t1\t0\t1\twork\thttps://github.com\n";
        let state = parse_session_content(sample);
        assert_eq!(state.focus, (1, 0));
        assert_eq!(state.sidebar_open, Some(true));
        assert_eq!(state.touchpad_mode, Some(false));
        assert_eq!(state.win_size, Some((1400, 850, false)));
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.tabs[0], SessionTabItem {
            row: 0,
            col: 0,
            full: true,
            profile_id: "default".to_string(),
            url: "https://google.com".to_string(),
            custom_width: None,
        });
        assert_eq!(state.tabs[1], SessionTabItem {
            row: 1,
            col: 0,
            full: true,
            profile_id: "work".to_string(),
            url: "https://github.com".to_string(),
            custom_width: None,
        });
    }

    #[test]
    fn test_parse_empty_session() {
        let state = parse_session_content("");
        assert_eq!(state.tabs.len(), 0);
        assert_eq!(state.focus, (0, 0));
        assert_eq!(state.sidebar_open, None);
        assert_eq!(state.touchpad_mode, None);
        assert_eq!(state.win_size, None);
    }

    #[test]
    fn test_parse_legacy_session() {
        let sample = "tab\t0\twork\thttps://news.ycombinator.com\n";
        let state = parse_session_content(sample);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0], SessionTabItem {
            row: 0,
            col: 0,
            full: true,
            profile_id: "work".to_string(),
            url: "https://news.ycombinator.com".to_string(),
            custom_width: None,
        });
    }

    #[test]
    fn test_parse_session_with_custom_width() {
        let sample = "tab\t0\t0\t0\tdefault\thttps://claude.ai\t850.5\n";
        let state = parse_session_content(sample);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0].custom_width, Some(850.5));
    }

    #[test]
    fn test_parse_session_with_search_engine() {
        let sample = "focus\t0\t0\nengine\tduckduckgo\ntab\t0\t0\t1\tdefault\thttps://duckduckgo.com\n";
        let state = parse_session_content(sample);
        assert_eq!(state.search_engine, Some("duckduckgo".to_string()));
        assert_eq!(state.tabs.len(), 1);
    }

    #[test]
    fn test_to_uri_search_engines() {
        assert_eq!(to_uri("rust lang", "google"), "https://www.google.com/search?q=rust%20lang");
        assert_eq!(to_uri("rust lang", "duckduckgo"), "https://duckduckgo.com/?q=rust%20lang");
        assert_eq!(to_uri("rust lang", "brave"), "https://search.brave.com/search?q=rust%20lang");
        assert_eq!(to_uri("rust lang", "bing"), "https://www.bing.com/search?q=rust%20lang");
        assert_eq!(to_uri("rust lang", "ecosia"), "https://www.ecosia.org/search?q=rust%20lang");
        assert_eq!(to_uri("https://github.com", "google"), "https://github.com");
        assert_eq!(to_uri("github.com", "duckduckgo"), "https://github.com");
    }

    #[test]
    fn test_settings_tab_enum() {
        assert_ne!(SettingsTab::General, SettingsTab::Profiles);
        assert_ne!(SettingsTab::Data, SettingsTab::Customize);
        assert_eq!(SettingsTab::General, SettingsTab::General);
    }

    #[test]
    fn test_devtools_dock_side_cycling() {
        let cycle = |side: DevToolsDockSide| match side {
            DevToolsDockSide::Bottom => DevToolsDockSide::Right,
            DevToolsDockSide::Right => DevToolsDockSide::Left,
            DevToolsDockSide::Left => DevToolsDockSide::Undocked,
            DevToolsDockSide::Undocked => DevToolsDockSide::Bottom,
        };
        assert_eq!(cycle(DevToolsDockSide::Bottom), DevToolsDockSide::Right);
        assert_eq!(cycle(DevToolsDockSide::Right), DevToolsDockSide::Left);
        assert_eq!(cycle(DevToolsDockSide::Left), DevToolsDockSide::Undocked);
        assert_eq!(cycle(DevToolsDockSide::Undocked), DevToolsDockSide::Bottom);
    }

    #[test]
    fn test_tab_width_pair_clamping() {
        let min_w = 280.0;
        let max_w = 1200.0;
        let initial0 = 500.0;
        let initial1 = 500.0;
        let total_pair = initial0 + initial1;

        let clamp_pair = |req_w0: f64| {
            let clamped_w0 = req_w0.clamp(min_w, (total_pair - min_w).min(max_w));
            let new_w1 = (total_pair - clamped_w0).max(min_w);
            (clamped_w0, new_w1)
        };

        // Test normal resize
        let (w0, w1) = clamp_pair(600.0);
        assert_eq!(w0, 600.0);
        assert_eq!(w1, 400.0);
        assert_eq!(w0 + w1, 1000.0);

        // Test below min_w constraint
        let (w0, w1) = clamp_pair(100.0);
        assert_eq!(w0, 280.0);
        assert_eq!(w1, 720.0);
        assert_eq!(w0 + w1, 1000.0);

        // Test above maximum allowed by neighbour's min_w
        let (w0, w1) = clamp_pair(950.0);
        assert_eq!(w0, 720.0);
        assert_eq!(w1, 280.0);
        assert_eq!(w0 + w1, 1000.0);
    }

    #[test]
    fn test_app_settings_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rail_test_settings_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        let s = AppSettings {
            search_engine: "duckduckgo".to_string(),
            touchpad_mode: true,
            smooth_scrolling: false,
            webrtc_enabled: true,
            auto_update: false,
        };
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"search_engine\": \"{}\",\n", minijson::escape(&s.search_engine)));
        out.push_str(&format!("  \"touchpad_navigation\": {},\n", s.touchpad_mode));
        out.push_str(&format!("  \"smooth_scrolling\": {},\n", s.smooth_scrolling));
        out.push_str(&format!("  \"webrtc_enabled\": {}\n", s.webrtc_enabled));
        out.push_str("}\n");
        std::fs::write(&path, &out).unwrap();

        let parsed = minijson::parse(&out).unwrap();
        assert_eq!(parsed.get("search_engine").and_then(|v| v.as_str()), Some("duckduckgo"));
        assert_eq!(parsed.get("touchpad_navigation").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(parsed.get("smooth_scrolling").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(parsed.get("webrtc_enabled").and_then(|v| v.as_bool()), Some(true));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_shortcut_master_and_individual_priority() {
        let mut cfg = ShortcutConfig::default();
        // Initially all enabled
        assert!(SHORTCUT_DEFS.iter().all(|d| sc_enabled(&cfg, d.id)));

        // User disables one shortcut (e.g. "data" Alt+I)
        cfg.disabled.insert("data".to_string());
        assert!(!sc_enabled(&cfg, "data"));
        assert!(!SHORTCUT_DEFS.iter().all(|d| sc_enabled(&cfg, d.id)));

        // Master checkbox turned ON -> forces all shortcuts enabled
        cfg.disabled.clear();
        cfg.site_wins = false;
        assert!(SHORTCUT_DEFS.iter().all(|d| sc_enabled(&cfg, d.id)));

        // Master checkbox turned OFF -> forces all shortcuts disabled except locked settings
        for def in SHORTCUT_DEFS {
            if def.id != SC_LOCKED {
                cfg.disabled.insert(def.id.to_string());
            }
        }
        cfg.site_wins = true;
        assert!(!sc_enabled(&cfg, "data"));
        assert!(!sc_enabled(&cfg, "ws_1"));
        assert!(sc_enabled(&cfg, SC_LOCKED)); // Settings remains accessible
    }

    #[test]
    fn test_ctrl_tab_shortcuts_definitions() {
        for num in 1..=9 {
            let id = format!("tab_{}", num);
            let def = SHORTCUT_DEFS.iter().find(|d| d.id == id).expect("tab shortcut exists");
            let expected = format!("Ctrl+{}", num);
            assert_eq!(def.defaults[0], expected.as_str());
        }
    }
}

