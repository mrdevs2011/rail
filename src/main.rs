mod profile;
mod strip;

use gtk::glib;
use gtk::{gdk, gio};
use gtk4 as gtk;
use profile::ProfileManager;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use strip::{col_width, gap, ShellLayout, StripLayout, SIDE_W};
use webkit6::prelude::*;
use webkit6::WebView;

const START_PAGE: &str = r#"<!doctype html><meta charset=utf-8><meta name=color-scheme content=dark>
<style>
html,body{height:100%;margin:0;box-sizing:border-box}
body{display:grid;place-items:center;background:#000000;color:#64748b;font:13px/1.5 ui-monospace,monospace,system-ui;padding:16px;overflow-y:auto;overflow-x:hidden}
.w{text-align:center;max-width:540px;width:100%;margin:auto;box-sizing:border-box}
h1{font-weight:800;font-size:clamp(32px, 7vw, 54px);letter-spacing:-.04em;margin:0;color:#f8fafc;font-family:ui-monospace,monospace}
h1 b{color:#38bdf8}
p{margin:4px 0 24px;font-size:clamp(12px, 2.5vw, 13px);color:#64748b}
.k{display:grid;grid-template-columns:auto 1fr;gap:8px 16px;text-align:left;justify-content:center;align-items:center;font-size:12px}
.k span:nth-child(odd){text-align:right;white-space:nowrap}
.k span:nth-child(even){text-align:left;color:#94a3b8}
kbd{font:11px ui-monospace,monospace;background:#090a0f;border:1px solid #1c1d29;border-bottom-width:2px;
    border-radius:5px;padding:2px 6px;color:#f1f5f9;margin-left:3px;white-space:nowrap}
@media (max-width: 440px) {
  body{padding:8px}
  .k{font-size:11px;gap:6px 10px}
  kbd{padding:1px 4px;font-size:10px}
}
</style>
<div class=w><h1>rail<b>.</b></h1><p>a niri-style 2D tiled browser (workspaces, horizontal strips & isolated profiles)</p><div class=k>
<span><kbd>Ctrl</kbd><kbd>G</kbd></span><span>go to address</span>
<span><kbd>Ctrl</kbd><kbd>Enter</kbd></span><span>edit current URL</span>
<span><kbd>Ctrl</kbd><kbd>T</kbd></span><span>new tab (current row)</span>
<span><kbd>Ctrl</kbd><kbd>V</kbd></span><span>new row tab (workspace below)</span>
<span><kbd>Ctrl</kbd><kbd>1</kbd> <kbd>2</kbd></span><span>focus workspace 1 / 2 (row)</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>2</kbd> <kbd>1</kbd></span><span>move tab to workspace 2 / 1</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>T</kbd></span><span>touchpad mode (2-finger slide tabs)</span>
<span><kbd>Ctrl</kbd><kbd>P</kbd> / <kbd>Shift</kbd><kbd>P</kbd></span><span>manage profiles</span>
<span><kbd>Ctrl</kbd><kbd>H</kbd> <kbd>L</kbd></span><span>focus left / right tab</span>
<span><kbd>Ctrl</kbd><kbd>J</kbd> <kbd>K</kbd></span><span>focus workspace down / up</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>H</kbd> <kbd>L</kbd></span><span>move tab left / right</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>J</kbd> <kbd>K</kbd></span><span>move tab down / up workspace</span>
<span><kbd>Ctrl</kbd><kbd>F</kbd></span><span>full / half width</span>
<span><kbd>Ctrl</kbd><kbd>O</kbd></span><span>overview (2D grid zoom out)</span>
<span><kbd>Ctrl</kbd><kbd>B</kbd></span><span>sidebar</span>
</div></div>"#;

const CSS: &str = "
window { background: #000000; border-radius: 0; box-shadow: none; }
label { color: #94a3b8; }

/* ---- columns & tabs ---- */
.rail-col { background: transparent; }
.tab { border-radius: 8px; border: 1.5px solid #14141c; background: #000000; }
.tab.focused { border-color: #38bdf8; box-shadow: 0 0 16px rgba(56, 189, 248, 0.2); }

/* ---- sidebar (pure pitch black 000 dev style) ---- */
.side-bar { background: #000000; border-right: 1px solid #14141c; padding-right: 6px; }
.brand { font-family: ui-monospace, monospace; font-weight: 800; font-size: 16px; letter-spacing: -0.04em; color: #f8fafc; }
.brand-dot { color: #38bdf8; }
button.btn-add-tab {
    background: #090a0f;
    border: 1px solid #1c1d29;
    border-radius: 6px;
    min-width: 26px;
    min-height: 26px;
    color: #64748b;
    padding: 0;
    transition: all 120ms;
}
button.btn-add-tab:hover {
    background: #141622;
    border-color: #38bdf8;
    color: #f8fafc;
}
button.flat-icon {
    background: transparent;
    border: none;
    box-shadow: none;
    min-width: 26px;
    min-height: 26px;
    padding: 0;
    border-radius: 6px;
    color: #64748b;
}
button.flat-icon:hover {
    background: #13141f;
    color: #f1f5f9;
}
button.pill {
    background: #07080c;
    border: 1px solid #171822;
    border-radius: 8px;
    padding: 7px 10px;
    box-shadow: none;
    color: #64748b;
    transition: all 120ms;
}
button.pill:hover {
    background: #0d0e14;
    border-color: #26293a;
    color: #cbd5e1;
}
button.pill label {
    color: #64748b;
    font-size: 12.5px;
}
button.pill:hover label {
    color: #cbd5e1;
}
.pill-icon {
    color: #475569;
}
button.pill:hover .pill-icon {
    color: #38bdf8;
}
.pill-chip {
    font-family: ui-monospace, monospace;
    font-size: 10px;
    font-weight: 700;
    color: #475569;
    background: #0f1017;
    border: 1px solid #1c1d29;
    border-radius: 4px;
    padding: 1px 5px;
}
button.pill:hover .pill-chip {
    color: #94a3b8;
    border-color: #2b2e40;
}
.tablist, .tablist row {
    background: transparent;
}
.tablist row {
    padding: 0;
    margin: 2px 0;
    border-radius: 7px;
    border: 1px solid transparent;
    transition: all 100ms ease;
}
.tablist row:hover {
    background: #090a0f;
    border-color: #171824;
}
.tablist row:selected {
    background: transparent;
    border-color: transparent;
}
.workspace-header {
    padding: 6px 8px;
    margin: 8px 0 2px 0;
    border-radius: 6px;
    background: #0d0e15;
    border: 1px solid #1a1c29;
    cursor: pointer;
    transition: all 120ms ease;
}
.workspace-header:hover {
    background: #121420;
    border-color: #24293d;
}
.workspace-header.active-workspace {
    background: #111422;
    border-color: #263352;
    border-left: 3px solid #38bdf8;
}
.workspace-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: #94a3b8;
    text-transform: uppercase;
}
.workspace-header.active-workspace .workspace-title {
    color: #38bdf8;
}
.workspace-badge {
    font-size: 9px;
    font-weight: 700;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.15);
    border-radius: 3px;
    padding: 1px 5px;
    margin-right: 2px;
}
.workspace-tab-count {
    font-size: 10px;
    color: #475569;
    margin-right: 4px;
}
.btn-ws-add {
    min-width: 20px;
    min-height: 20px;
    padding: 0;
    border-radius: 4px;
    background: transparent;
    border: none;
    color: #64748b;
    transition: all 100ms;
}
.btn-ws-add:hover {
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
}

.tabrow {
    padding: 6px 8px 6px 16px;
    margin: 1px 0;
    border-radius: 6px;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 100ms ease;
}
.tabrow:hover {
    background: #090a0f;
    border-color: #171824;
}
.tabrow.active-tab {
    background: #0e121e;
    border-color: #222c42;
    border-left: 3px solid #38bdf8;
}
.tabrow.active-tab .tab-title {
    color: #f8fafc;
    font-weight: 600;
}
.tabrow.active-tab .tab-index {
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border-color: rgba(56, 189, 248, 0.35);
}
.tab-index {
    font-family: ui-monospace, monospace;
    font-size: 10.5px;
    font-weight: 700;
    color: #475569;
    background: #08090d;
    border: 1px solid #14151f;
    border-radius: 4px;
    padding: 1px 5px;
    min-width: 14px;
}
.tab-title {
    font-size: 12.5px;
    font-weight: 500;
    color: #8892b0;
}
.tab-full-badge {
    font-size: 8.5px;
    font-weight: 700;
    color: #a78bfa;
    background: rgba(167, 139, 250, 0.12);
    border: 1px solid rgba(167, 139, 250, 0.3);
    border-radius: 3px;
    padding: 0 4px;
}
.tabrow image {
    color: #64748b;
}
.tab-close {
    opacity: 0;
    min-width: 20px;
    min-height: 20px;
    padding: 0;
    border-radius: 4px;
    background: transparent;
    border: none;
    box-shadow: none;
    color: #64748b;
    transition: all 100ms;
}
.tab-close:hover {
    background: rgba(244, 63, 94, 0.2);
    color: #fb7185;
}
.tabrow:hover .tab-close, .tabrow.active-tab .tab-close {
    opacity: 1;
}
.btn-new-workspace {
    margin: 12px 4px 6px 4px;
    padding: 6px 10px;
    background: #0b0d14;
    border: 1px dashed #202436;
    border-radius: 6px;
    color: #64748b;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 120ms ease;
}
.btn-new-workspace:hover {
    background: #121524;
    border-color: #38bdf8;
    color: #38bdf8;
}
.side-hint {
    font-family: ui-monospace, monospace;
    font-size: 9.5px;
    color: #334155;
    letter-spacing: -0.01em;
}

.fav, .tabrow label.fav, row:selected .tabrow label.fav {
    border-radius: 4px;
    font-size: 10px;
    font-weight: 800;
    color: #000000;
    font-family: ui-monospace, monospace;
}
.fav-0 { background: #38bdf8; } .fav-1 { background: #c084fc; } .fav-2 { background: #4ade80; }
.fav-3 { background: #facc15; } .fav-4 { background: #f87171; } .fav-5 { background: #22d3ee; }
.fav-6 { background: #fb923c; } .fav-7 { background: #2dd4bf; }

/* ---- load progress ---- */
progressbar.load-bar { min-height: 2.5px; margin: 0 8px; }
progressbar.load-bar trough { min-height: 2.5px; background: transparent; border: none; }
progressbar.load-bar progress { min-height: 2.5px; background: #38bdf8; border-radius: 2px;
                                box-shadow: 0 0 10px rgba(56,189,248,0.8); }

/* ---- address palette ---- */
.palette {
    background: #06070a;
    border-radius: 12px;
    border: 1px solid #1e202d;
    padding: 12px 16px 10px;
    box-shadow: 0 24px 64px rgba(0,0,0,0.95);
}
.palette image {
    color: #64748b;
}
entry.palette-entry, entry.palette-entry:focus-within {
    background: none;
    border: none;
    box-shadow: none;
    outline: none;
    padding: 0;
    min-height: 0;
    font-size: 15px;
    color: #f1f5f9;
    font-family: ui-monospace, monospace, system-ui;
}
.palette-hint {
    color: #475569;
    font-size: 10.5px;
    font-family: ui-monospace, monospace;
    margin-top: 6px;
}

/* ---- overview (the live strip, zoomed out in 2D) ---- */
.zoomed .tab { opacity: 0.85; border-radius: 16px; border: 1.5px solid #20222e; }
.zoomed .tab.focused { opacity: 1; outline: 4px solid #38bdf8; outline-offset: -2px; }
.ov-hint {
    background: #08090d;
    color: #94a3b8;
    border: 1px solid #1e202d;
    border-radius: 8px;
    padding: 6px 14px;
    font-size: 11px;
    font-family: ui-monospace, monospace;
    box-shadow: 0 8px 24px rgba(0,0,0,0.7);
}

/* ---- profile colors (True Dark Dev Neon) ---- */
.prof-bg-0 { background-color: #38bdf8; color: #000000; }
.prof-bg-1 { background-color: #c084fc; color: #000000; }
.prof-bg-2 { background-color: #4ade80; color: #000000; }
.prof-bg-3 { background-color: #facc15; color: #000000; }
.prof-bg-4 { background-color: #f87171; color: #000000; }
.prof-bg-5 { background-color: #22d3ee; color: #000000; }
.prof-bg-6 { background-color: #fb923c; color: #000000; }
.prof-bg-7 { background-color: #2dd4bf; color: #000000; }

/* ---- profile circle button in URL palette ---- */
button.profile-circle-btn {
    min-width: 30px;
    min-height: 30px;
    padding: 0;
    border-radius: 9999px;
    border: 1.5px solid #1e202d;
    box-shadow: 0 4px 12px rgba(0,0,0,0.4);
    transition: transform 120ms, border-color 120ms;
}
button.profile-circle-btn:hover {
    transform: scale(1.08);
    border-color: #38bdf8;
}
.profile-circle-label {
    font-weight: 800;
    font-size: 12px;
    color: #000000;
    font-family: ui-monospace, monospace;
}

/* ---- profile popover dropdown ---- */
.profile-popover {
    background: #06070a;
    border: 1px solid #1e202d;
    border-radius: 12px;
    padding: 6px;
    box-shadow: 0 16px 48px rgba(0,0,0,0.9);
}
.profile-popover-title {
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #475569;
    font-family: ui-monospace, monospace;
    padding: 4px 8px 2px;
}
button.profile-item-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    border-radius: 8px;
    padding: 5px 8px;
    transition: background 100ms;
}
button.profile-item-btn:hover {
    background: #0f111a;
}
button.profile-item-btn.selected {
    background: #131622;
    border-left: 2px solid #38bdf8;
}
.profile-item-dot {
    min-width: 22px;
    min-height: 22px;
    border-radius: 9999px;
    font-size: 11px;
    font-weight: 800;
    color: #000000;
    font-family: ui-monospace, monospace;
    padding: 0;
}
.profile-item-name {
    font-size: 12.5px;
    font-weight: 600;
    color: #f1f5f9;
}
.profile-item-check {
    font-size: 12px;
    font-weight: 700;
    color: #38bdf8;
}

/* ---- profiles manager modal (Ctrl+P / Ctrl+Shift+P) ---- */
.profiles-modal {
    background: #06070a;
    border: 1px solid #1e202d;
    border-radius: 16px;
    padding: 22px 26px;
    box-shadow: 0 32px 80px rgba(0,0,0,0.98);
    min-width: 440px;
    margin-left: 16px;
    margin-right: 16px;
}
.profiles-modal-title {
    font-size: 18px;
    font-weight: 800;
    color: #f8fafc;
    letter-spacing: -0.03em;
    font-family: ui-monospace, monospace, system-ui;
}
.profiles-modal-subtitle {
    font-size: 11.5px;
    color: #64748b;
    margin-top: 2px;
}
.profile-card {
    background: #0a0b10;
    border: 1px solid #161722;
    border-radius: 10px;
    padding: 9px 12px;
    margin-bottom: 6px;
    transition: all 120ms ease;
}
.profile-card:hover {
    border-color: #242738;
}
.profile-card.active-profile {
    border-color: #38bdf8;
    background: #0d101a;
    box-shadow: 0 0 16px rgba(56, 189, 248, 0.12);
}
.profile-avatar-circle {
    min-width: 30px;
    min-height: 30px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 800;
    color: #000000;
    font-family: ui-monospace, monospace;
    margin: 0;
    padding: 0;
}
.profile-card-name {
    font-size: 13.5px;
    font-weight: 600;
    color: #f1f5f9;
}
.profile-card-sub {
    font-size: 10.5px;
    color: #475569;
    font-family: ui-monospace, monospace;
}
.profile-badge-active {
    font-size: 10px;
    font-weight: 700;
    border-radius: 9999px;
    padding: 2px 8px;
    background: rgba(56, 189, 248, 0.12);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.35);
    font-family: ui-monospace, monospace;
}
button.btn-make-active {
    background: #11131c;
    color: #94a3b8;
    border: 1px solid #222536;
    border-radius: 6px;
    padding: 3px 10px;
    font-size: 11.5px;
    font-weight: 600;
    transition: all 120ms;
}
button.btn-make-active:hover {
    background: #38bdf8;
    color: #000000;
    border-color: #38bdf8;
}
button.btn-del-profile {
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 4px;
    color: #475569;
    transition: all 120ms;
}
button.btn-del-profile:hover {
    background: rgba(244, 63, 94, 0.18);
    color: #fb7185;
}
.profile-section-label {
    font-size: 10.5px;
    font-weight: 700;
    color: #475569;
    letter-spacing: 0.06em;
    font-family: ui-monospace, monospace;
    margin-top: 4px;
}
.profile-entry-box {
    background: #090a0e;
    border: 1px solid #1c1d29;
    border-radius: 8px;
    padding: 7px 11px;
    color: #f1f5f9;
    font-size: 13px;
    font-family: ui-monospace, monospace, system-ui;
    transition: all 120ms;
}
.profile-entry-box:focus {
    border-color: #38bdf8;
    box-shadow: 0 0 0 1px #38bdf8;
}
.profile-entry-box.error {
    border-color: #f87171;
    box-shadow: 0 0 0 1px #f87171;
}
.profile-error-label {
    font-size: 11px;
    font-weight: 600;
    color: #f87171;
    margin-top: 2px;
}
button.btn-create-profile {
    background: #0d0e14;
    border: 1px solid #1c1d29;
    border-radius: 7px;
    padding: 6px 16px;
    transition: all 120ms;
}
button.btn-create-profile label {
    color: #475569;
    font-weight: 700;
    font-size: 12px;
}
button.btn-create-profile:disabled {
    background: #0a0b10;
    border-color: #161720;
}
button.btn-create-profile:disabled label {
    color: #475569;
}
button.btn-create-profile.active-ready {
    background: #38bdf8;
    border-color: #38bdf8;
}
button.btn-create-profile.active-ready label {
    color: #000000;
    font-weight: 800;
}
button.btn-create-profile.active-ready:hover {
    background: #7dd3fc;
    border-color: #7dd3fc;
}
button.btn-create-profile.active-ready:hover label {
    color: #000000;
}

.profile-badge-mini {
    font-size: 9px;
    font-weight: 800;
    min-width: 16px;
    min-height: 16px;
    border-radius: 9999px;
    margin-left: 2px;
    margin-right: 4px;
    color: #000000;
    padding: 0;
    font-family: ui-monospace, monospace;
}
button.color-choice {
    min-width: 22px;
    min-height: 22px;
    padding: 0;
    border-radius: 9999px;
    border: 2px solid transparent;
    transition: transform 100ms, border-color 100ms;
}
button.color-choice:hover {
    transform: scale(1.15);
}
button.color-choice.selected {
    border-color: #ffffff;
    box-shadow: 0 0 0 2px #06070a;
    transform: scale(1.15);
}
";

/// How far the strip zooms out in overview (niri uses a similar factor).
const OV_ZOOM: f64 = 0.4;

struct Tab {
    container: gtk::Box,
    view: WebView,
    profile_id: String,
}

struct Row {
    tabs: Vec<Tab>,
}

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
    profiles_modal: gtk::Box,
    profiles_list_box: gtk::ListBox,
    new_profile_entry: gtk::Entry,
    new_profile_error_lbl: gtk::Label,
    add_prof_btn: gtk::Button,
    new_profile_color_idx: Cell<usize>,
    color_buttons: RefCell<Vec<gtk::Button>>,
    touchpad_mode: Cell<bool>,
}

fn to_uri(input: &str) -> String {
    let s = input.trim();
    if s.contains("://") || s.starts_with("about:") || s.starts_with("file:") {
        return s.into();
    }
    if s.starts_with("localhost") || s.starts_with("127.0.0.1") {
        return format!("http://{s}");
    }
    if !s.contains(' ') && s.contains('.') {
        return format!("https://{s}");
    }
    format!("https://duckduckgo.com/?q={}", glib::Uri::escape_string(s, None, false))
}

fn host_of(uri: &str) -> Option<String> {
    let u = glib::Uri::parse(uri, glib::UriFlags::NONE).ok()?;
    let h = u.host()?;
    Some(h.trim_start_matches("www.").to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionTabItem {
    row: usize,
    col: usize,
    full: bool,
    profile_id: String,
    url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionState {
    focus: (usize, usize),
    sidebar_open: Option<bool>,
    touchpad_mode: Option<bool>,
    win_size: Option<(i32, i32, bool)>,
    tabs: Vec<SessionTabItem>,
}

fn parse_session_content(content: &str) -> SessionState {
    let mut focus = (0usize, 0usize);
    let mut sidebar_open = None;
    let mut touchpad_mode = None;
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
                tabs.push(SessionTabItem { row: r, col: c, full, profile_id: pid, url });
            }
            "tab" if parts.len() == 4 => {
                let c = parts[1].parse().unwrap_or(0);
                let pid = parts[2].to_string();
                let url = parts[3].to_string();
                tabs.push(SessionTabItem { row: 0, col: c, full: true, profile_id: pid, url });
            }
            _ => {}
        }
    }
    SessionState {
        focus,
        sidebar_open,
        touchpad_mode,
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

    fn palette_open(&self) -> bool {
        gtk::prelude::WidgetExt::is_visible(&self.palette)
    }

    fn overview_open(&self) -> bool {
        self.ov.get()
    }

    fn create_tab_widget(view: &WebView) -> gtk::Box {
        view.add_css_class("tab");
        view.set_overflow(gtk::Overflow::Hidden);
        view.set_vexpand(true);
        view.set_hexpand(true);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("rail-col");
        container.add_css_class("full");
        container.set_vexpand(true);
        container.set_hexpand(true);
        container.append(view);
        container
    }

    fn sync_strip(&self) {
        let rows = self.rows.borrow();
        let mut grid = Vec::new();
        let mut prev: Option<gtk::Widget> = None;
        for row in rows.iter() {
            let mut row_grid = Vec::new();
            for tab in row.tabs.iter() {
                row_grid.push(tab.container.has_css_class("full"));
                if tab.container.parent().as_ref() != Some(self.strip.upcast_ref()) {
                    self.strip.append(&tab.container);
                }
                self.strip.reorder_child_after(&tab.container, prev.as_ref());
                prev = Some(tab.container.clone().upcast());
            }
            grid.push(row_grid);
        }
        drop(rows);
        self.layout.set_grid(grid);
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
                let w = col_width(tab.container.has_css_class("full"), vw);
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
        if near(self.target_x.get(), self.layout.offset_x(), 0.5)
            && near(self.target_y.get(), self.layout.offset_y(), 0.5)
            && near(self.target_zoom.get(), self.layout.zoom(), 0.002)
        {
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
            let k = 1.0 - (-dt / 0.06).exp();

            let (oxc, oxt) = (app.layout.offset_x(), app.target_x.get());
            let (oyc, oyt) = (app.layout.offset_y(), app.target_y.get());
            let (zc, zt) = (app.layout.zoom(), app.target_zoom.get());

            let mut nx = oxc + (oxt - oxc) * k;
            let mut ny = oyc + (oyt - oyc) * k;
            let mut nz = zc + (zt - zc) * k;

            if near(oxt, nx, 0.5) { nx = oxt; }
            if near(oyt, ny, 0.5) { ny = oyt; }
            if near(zt, nz, 0.002) { nz = zt; }

            app.layout.set_offset_x(nx);
            app.layout.set_offset_y(ny);
            app.layout.set_zoom(nz);
            strip.queue_allocate();

            if nx == oxt && ny == oyt && nz == zt {
                app.animating.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    // ---- tabs & workspaces -----------------------------------------------

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

    fn new_tab(self: &Rc<Self>) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_profile().id;
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = WebView::builder().network_session(&session).build();
        view.load_html(START_PAGE, None);
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, prof_id.clone(), false);
        self.select_palette_profile(&prof_id);
        self.show_palette();
    }

    fn split_vertical(self: &Rc<Self>) {
        self.exit_overview();
        let prof_id = self.focused_tab_profile().unwrap_or_else(|| self.profiles.borrow().active_profile().id);
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = WebView::builder().network_session(&session).build();
        view.load_html(START_PAGE, None);
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, prof_id.clone(), true);
        self.select_palette_profile(&prof_id);
        self.show_palette();
    }

    fn setup_view_signals(self: &Rc<Self>, view: &WebView) {
        let app = self.clone();
        view.connect_title_notify(move |_| app.refresh_sidebar());
        let app = self.clone();
        view.connect_uri_notify(move |_| {
            app.refresh_sidebar();
            app.save_session();
        });
        let app = self.clone();
        view.connect_favicon_notify(move |_| app.refresh_sidebar());
        let app = self.clone();
        view.connect_is_loading_notify(move |_| {
            app.refresh_sidebar();
            app.update_load_bar();
        });
        let app = self.clone();
        view.connect_estimated_load_progress_notify(move |_| app.update_load_bar());
        let app = self.clone();
        view.connect_create(move |v, _| {
            let parent_prof = app.find_view_profile(v).unwrap_or_else(|| app.profiles.borrow().active_profile().id);
            let nv = WebView::builder().related_view(v).build();
            app.add_view(nv.clone(), parent_prof, false);
            Some(nv.upcast())
        });
        let app = self.clone();
        view.connect_close(move |v| app.close_view(v));
        view.connect_web_process_terminated(move |v, _reason| {
            v.reload();
        });
    }

    fn add_view(self: &Rc<Self>, view: WebView, profile_id: String, new_row: bool) {
        view.set_can_target(!self.ov.get());
        let container = Self::create_tab_widget(&view);

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
                rows[next_r].tabs.push(Tab { container, view: view.clone(), profile_id });
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
                rows[cur_r].tabs.insert(col_idx, Tab { container, view: view.clone(), profile_id });
                (cur_r, col_idx)
            }
        };

        self.setup_view_signals(&view);
        self.sync_strip();
        self.focus_coords(target_r, target_c);
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

        let mut rows = self.rows.borrow_mut();
        if r_idx >= rows.len() || c_idx >= rows[r_idx].tabs.len() {
            return;
        }
        let tab = rows[r_idx].tabs.remove(c_idx);
        self.strip.remove(&tab.container);

        if rows[r_idx].tabs.is_empty() && rows.len() > 1 {
            rows.remove(r_idx);
        }

        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let total_rows = rows.len();
        drop(rows);

        self.sync_strip();

        if total_tabs == 0 {
            self.save_session();
            self.win.close();
            return;
        } else {
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
                } else {
                    tab.view.remove_css_class("focused");
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
        self.save_session();

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
            let view = WebView::builder().network_session(&session).build();
            view.load_html(START_PAGE, None);
            view.set_can_target(!self.ov.get());
            let container = Self::create_tab_widget(&view);
            self.setup_view_signals(&view);

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab { container, view, profile_id: prof_id });
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

    fn toggle_touchpad_mode(self: &Rc<Self>) {
        self.touchpad_mode.set(!self.touchpad_mode.get());
        self.save_session();
    }

    fn save_session(&self) {
        let rows = self.rows.borrow();
        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let path = glib::user_data_dir().join("rail").join("session.txt");
        let temp_path = glib::user_data_dir().join("rail").join("session.txt.tmp");

        if total_tabs == 0 {
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(&temp_path);
            return;
        }

        let mut out = String::new();
        out.push_str(&format!("focus\t{}\t{}\n", self.focus_row.get(), self.focus_col.get()));
        out.push_str(&format!("sidebar\t{}\n", if self.side_open.get() { 1 } else { 0 }));
        out.push_str(&format!("touchpad\t{}\n", if self.touchpad_mode.get() { 1 } else { 0 }));
        let win_w = self.win.width();
        let win_h = self.win.height();
        let is_max = if self.win.is_maximized() { 1 } else { 0 };
        out.push_str(&format!("win\t{}\t{}\t{}\n", win_w, win_h, is_max));

        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                let is_full = if tab.container.has_css_class("full") { 1 } else { 0 };
                let uri = tab.view.uri().unwrap_or_default();
                out.push_str(&format!("tab\t{}\t{}\t{}\t{}\t{}\n", ri, ci, is_full, tab.profile_id, uri));
            }
        }
        drop(rows);

        // Atomic write with sync_all (fsync) ensuring data is immediately flushed to physical hard disk
        if let Ok(mut file) = std::fs::File::create(&temp_path) {
            use std::io::Write;
            if file.write_all(out.as_bytes()).is_ok() && file.sync_all().is_ok() {
                let _ = std::fs::rename(&temp_path, &path);
                return;
            }
        }
        let _ = std::fs::write(&path, &out);
    }

    fn restore_session(self: &Rc<Self>) -> bool {
        let path = glib::user_data_dir().join("rail").join("session.txt");
        let Ok(content) = std::fs::read_to_string(path) else { return false; };
        let state = parse_session_content(&content);
        if state.tabs.is_empty() {
            return false;
        }

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

        let mut loaded_tabs: Vec<(usize, Tab)> = Vec::new();
        for item in tab_items {
            let session = self.profiles.borrow_mut().get_or_create_session(&item.profile_id);
            let view = WebView::builder().network_session(&session).build();
            if item.url.is_empty() || item.url.starts_with("about:") {
                view.load_html(START_PAGE, None);
            } else {
                view.load_uri(&item.url);
            }
            let container = Self::create_tab_widget(&view);
            if !item.full {
                container.remove_css_class("full");
            }
            self.setup_view_signals(&view);
            loaded_tabs.push((item.row, Tab { container, view, profile_id: item.profile_id }));
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
        if tab.container.has_css_class("full") {
            tab.container.remove_css_class("full");
        } else {
            tab.container.add_css_class("full");
        }
        drop(rows);
        self.sync_strip();
        self.reveal_focus();
        self.save_session();
    }

    // ---- sidebar ---------------------------------------------------------

    fn tab_label(t: &Tab) -> String {
        match t.view.title().filter(|s| !s.is_empty()) {
            Some(s) => s.to_string(),
            None => t
                .view
                .uri()
                .filter(|u| !u.starts_with("about:"))
                .map_or("New tab".into(), |u| host_of(&u).unwrap_or(u.to_string())),
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
        let letter = label.chars().find(|c| c.is_alphanumeric()).unwrap_or('·');
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

        for (ri, row) in rows.iter().enumerate() {
            // Workspace Header
            let ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            ws_box.add_css_class("workspace-header");
            if ri == cur_r {
                ws_box.add_css_class("active-workspace");
            }

            let ws_title = gtk::Label::new(Some(&format!("Workspace {}", ri + 1)));
            ws_title.add_css_class("workspace-title");
            ws_title.set_xalign(0.0);
            ws_title.set_hexpand(true);
            ws_box.append(&ws_title);

            if ri == cur_r {
                let active_dot = gtk::Label::new(Some("● Active"));
                active_dot.add_css_class("workspace-badge");
                ws_box.append(&active_dot);
            }

            let count_text = if row.tabs.len() == 1 {
                "1 tab".to_string()
            } else {
                format!("{} tabs", row.tabs.len())
            };
            let count_lbl = gtk::Label::new(Some(&count_text));
            count_lbl.add_css_class("workspace-tab-count");
            ws_box.append(&count_lbl);

            let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
            add_btn.add_css_class("btn-ws-add");
            add_btn.set_tooltip_text(Some(&format!("Add tab to Workspace {}", ri + 1)));
            let app = self.clone();
            add_btn.connect_clicked(move |_| {
                app.focus_row(ri);
                app.new_tab();
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
                let item = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                item.add_css_class("tabrow");
                if ri == cur_r && ci == cur_c {
                    item.add_css_class("active-tab");
                }

                item.append(&Self::tab_icon(t, 16));

                let mgr = self.profiles.borrow();
                if mgr.profiles().len() > 1 {
                    if let Some(p) = mgr.get_profile(&t.profile_id) {
                        let p_dot = gtk::Label::new(Some(&p.initial_letter()));
                        p_dot.add_css_class("profile-badge-mini");
                        p_dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
                        p_dot.set_valign(gtk::Align::Center);
                        p_dot.set_halign(gtk::Align::Center);
                        p_dot.set_width_request(16);
                        p_dot.set_height_request(16);
                        p_dot.set_tooltip_text(Some(&format!("Profile: {}", p.name)));
                        item.append(&p_dot);
                    }
                }
                drop(mgr);

                let index_lbl = gtk::Label::new(Some(&format!("{}", ci + 1)));
                index_lbl.add_css_class("tab-index");
                index_lbl.set_valign(gtk::Align::Center);
                item.append(&index_lbl);

                let title_lbl = gtk::Label::new(Some(&Self::tab_label(t)));
                title_lbl.add_css_class("tab-title");
                title_lbl.set_xalign(0.0);
                title_lbl.set_hexpand(true);
                title_lbl.set_width_chars(1);
                title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                item.append(&title_lbl);

                if t.container.has_css_class("full") {
                    let full_tag = gtk::Label::new(Some("FULL"));
                    full_tag.add_css_class("tab-full-badge");
                    full_tag.set_valign(gtk::Align::Center);
                    item.append(&full_tag);
                }

                let close = gtk::Button::from_icon_name("window-close-symbolic");
                close.add_css_class("tab-close");
                close.set_valign(gtk::Align::Center);
                let (app, v) = (self.clone(), t.view.clone());
                close.connect_clicked(move |_| app.close_view(&v));
                item.append(&close);

                let app = self.clone();
                let tab_click = gtk::GestureClick::new();
                tab_click.connect_released(move |_, _, _, _| {
                    app.focus_coords(ri, ci);
                });
                item.add_controller(tab_click);

                self.list.append(&item);
            }
        }
        drop(rows);

        // Bottom "+ New Workspace" button
        let new_ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        new_ws_box.add_css_class("btn-new-workspace");
        new_ws_box.set_halign(gtk::Align::Fill);
        let new_ws_icon = gtk::Image::from_icon_name("list-add-symbolic");
        let new_ws_lbl = gtk::Label::new(Some("+ New Workspace (Ctrl+V)"));
        new_ws_lbl.set_xalign(0.0);
        new_ws_lbl.set_hexpand(true);
        new_ws_box.append(&new_ws_icon);
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
            _ => "Search or enter address".into(),
        };
        self.pill_label.set_text(&text);
    }

    fn toggle_sidebar(self: &Rc<Self>) {
        self.set_sidebar(!self.side_open.get());
    }

    fn set_sidebar(self: &Rc<Self>, open: bool) {
        self.side_open.set(open);
        self.save_session();
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
            self.profile_btn_lbl.set_text(&p.initial_letter());
            for i in 0..8 {
                self.profile_btn.remove_css_class(&format!("prof-bg-{}", i));
            }
            self.profile_btn.add_css_class(&format!("prof-bg-{}", p.color_idx));
            self.profile_btn.set_tooltip_text(Some(&format!("Profile: {} (Click to switch)", p.name)));
        }
    }

    fn rebuild_profile_popover(self: &Rc<Self>) {
        let box_menu = gtk::Box::new(gtk::Orientation::Vertical, 4);
        box_menu.set_margin_top(4);
        box_menu.set_margin_bottom(4);
        box_menu.set_margin_start(4);
        box_menu.set_margin_end(4);

        let title = gtk::Label::new(Some("Select Profile"));
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

            let dot = gtk::Label::new(Some(&p.initial_letter()));
            dot.add_css_class("profile-item-dot");
            dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
            dot.set_valign(gtk::Align::Center);
            dot.set_halign(gtk::Align::Center);
            dot.set_width_request(24);
            dot.set_height_request(24);

            let name_lbl = gtk::Label::new(Some(&p.name));
            name_lbl.add_css_class("profile-item-name");
            name_lbl.set_hexpand(true);
            name_lbl.set_xalign(0.0);

            row_box.append(&dot);
            row_box.append(&name_lbl);

            if p.id == selected_id {
                let check = gtk::Label::new(Some("✓"));
                check.add_css_class("profile-item-check");
                row_box.append(&check);
            }

            row_btn.set_child(Some(&row_box));

            let app = self.clone();
            let pid = p.id.clone();
            row_btn.connect_clicked(move |_| {
                app.select_palette_profile(&pid);
                app.profile_popover.popdown();
                app.entry.grab_focus();
            });

            box_menu.append(&row_btn);
        }

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

    fn show_palette(self: &Rc<Self>) {
        let uri = self.focused_view().and_then(|v| v.uri()).filter(|u| !u.starts_with("about:"));
        self.entry.set_text(uri.as_deref().unwrap_or(""));

        let prof_id = self.focused_tab_profile().unwrap_or_else(|| self.profiles.borrow().active_profile().id);
        self.select_palette_profile(&prof_id);
        self.rebuild_profile_popover();

        self.update_palette_size();
        self.palette.set_visible(true);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    fn hide_palette(&self) {
        self.profile_popover.popdown();
        self.palette.set_visible(false);
        if !self.profiles_modal_open() {
            if let Some(v) = self.focused_view() {
                v.grab_focus();
            }
        }
    }

    fn cancel_palette(self: &Rc<Self>) {
        let view = self.focused_view();
        let fresh = self.fresh.borrow().clone();
        self.hide_palette();
        if let (Some(v), Some(f)) = (view, fresh) {
            let total_tabs: usize = self.rows.borrow().iter().map(|r| r.tabs.len()).sum();
            if v == f && total_tabs > 1 {
                self.close_view(&v);
            }
        }
    }

    fn navigate(self: &Rc<Self>, text: &str) {
        if !text.trim().is_empty() {
            let target_profile_id = self.selected_palette_profile.borrow().clone();
            let cur_r = self.focus_row.get();
            let cur_c = self.focus_col.get();
            let uri = to_uri(text);

            let needs_view_recreation = {
                let rows = self.rows.borrow();
                rows.get(cur_r)
                    .and_then(|r| r.tabs.get(cur_c))
                    .map(|t| t.profile_id != target_profile_id)
                    .unwrap_or(false)
            };

            if needs_view_recreation {
                let session = self.profiles.borrow_mut().get_or_create_session(&target_profile_id);
                let new_view = WebView::builder().network_session(&session).build();
                new_view.set_can_target(!self.ov.get());
                self.setup_view_signals(&new_view);

                let mut rows = self.rows.borrow_mut();
                if let Some(row) = rows.get_mut(cur_r) {
                    if let Some(tab) = row.tabs.get_mut(cur_c) {
                        tab.container.remove(&tab.view);
                        tab.container.append(&new_view);
                        new_view.add_css_class("tab");
                        new_view.set_overflow(gtk::Overflow::Hidden);
                        new_view.set_vexpand(true);
                        new_view.set_hexpand(true);
                        tab.view = new_view.clone();
                        tab.profile_id = target_profile_id;
                    }
                }
                drop(rows);

                self.pill_label.set_text(&host_of(&uri).unwrap_or(uri.clone()));
                new_view.load_uri(&uri);
                *self.fresh.borrow_mut() = None;
                self.refresh_sidebar();
                self.focus_coords(cur_r, cur_c);
            } else if let Some(v) = self.focused_view() {
                self.pill_label.set_text(&host_of(&uri).unwrap_or(uri.clone()));
                v.load_uri(&uri);
                *self.fresh.borrow_mut() = None;
            }
        }
        self.hide_palette();
    }

    // ---- profiles manager modal (Ctrl+P / Ctrl+Shift+P) ------------------

    fn profiles_modal_open(&self) -> bool {
        self.profiles_modal.is_visible()
    }

    fn toggle_profiles_modal(self: &Rc<Self>) {
        if self.profiles_modal_open() {
            self.hide_profiles_modal();
        } else {
            self.show_profiles_modal();
        }
    }

    fn show_profiles_modal(self: &Rc<Self>) {
        self.hide_palette();
        self.rebuild_profiles_modal_list();
        self.new_profile_entry.set_text("");
        self.select_new_profile_color(0);
        self.update_create_profile_btn_state();
        self.profiles_modal.set_visible(true);
        self.new_profile_entry.grab_focus();
    }

    fn hide_profiles_modal(&self) {
        self.profiles_modal.set_visible(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
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

        for p in profiles {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.add_css_class("profile-card");
            if p.id == active_id {
                row.add_css_class("active-profile");
            }

            let avatar = gtk::Label::new(Some(&p.initial_letter()));
            avatar.add_css_class("profile-avatar-circle");
            avatar.add_css_class(&format!("prof-bg-{}", p.color_idx));
            avatar.set_valign(gtk::Align::Center);
            avatar.set_halign(gtk::Align::Center);
            avatar.set_width_request(32);
            avatar.set_height_request(32);

            let info_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
            info_box.set_hexpand(true);
            info_box.set_valign(gtk::Align::Center);

            let name_lbl = gtk::Label::new(Some(&p.name));
            name_lbl.add_css_class("profile-card-name");
            name_lbl.set_xalign(0.0);

            let sub_text = if p.id == "default" {
                "@default • System Default".to_string()
            } else {
                format!("@{}", p.id)
            };
            let sub_lbl = gtk::Label::new(Some(&sub_text));
            sub_lbl.add_css_class("profile-card-sub");
            sub_lbl.set_xalign(0.0);

            info_box.append(&name_lbl);
            info_box.append(&sub_lbl);

            let actions_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            actions_box.set_valign(gtk::Align::Center);

            if p.id == active_id {
                let active_badge = gtk::Label::new(Some("Active"));
                active_badge.add_css_class("profile-badge-active");
                actions_box.append(&active_badge);
            } else {
                let set_act_btn = gtk::Button::with_label("Make Active");
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
                del_btn.set_tooltip_text(Some("Delete Profile"));
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
            self.new_profile_error_lbl.set_text("⚠ Profile already exists");
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
            Ok(_) => {
                self.new_profile_entry.set_text("");
                self.update_create_profile_btn_state();
                self.rebuild_profiles_modal_list();
                self.rebuild_profile_popover();
                self.refresh_sidebar();
            }
            Err(err) => {
                self.new_profile_error_lbl.set_text(&format!("⚠ {}", err));
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
                    let cw = col_width(tab.container.has_css_class("full"), vw);
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
        if plain || m == M::CONTROL_MASK {
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

        if self.profiles_modal_open() {
            if k == gdk::Key::Escape {
                self.hide_profiles_modal();
                return true;
            }
            if ctrl && !shift && !alt && (k == gdk::Key::p || k == gdk::Key::P) {
                self.hide_profiles_modal();
                return true;
            }
            // All other keystrokes pass directly to profiles_modal without triggering window shortcuts
            return false;
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
        let global = ctrl && !shift && !alt && matches!(k, gdk::Key::o | gdk::Key::t | gdk::Key::q | gdk::Key::p);
        if self.overview_open() && !global && self.ov_key(k, state & gtk::accelerator_get_default_mod_mask()) {
            return true;
        }
        if !ctrl && !shift && !alt && k == gdk::Key::F11 {
            self.win.set_fullscreened(!self.win.is_fullscreen());
            return true;
        }
        if alt && !ctrl && !shift {
            let v = self.focused_view();
            match k {
                gdk::Key::Left => v.map(|v| v.go_back()),
                gdk::Key::Right => v.map(|v| v.go_forward()),
                _ => return false,
            };
            return true;
        }

        if ctrl && !shift && !alt {
            match k {
                gdk::Key::p => self.toggle_profiles_modal(),
                gdk::Key::t => self.new_tab(),
                gdk::Key::v => self.split_vertical(),
                gdk::Key::f => self.toggle_full(),
                gdk::Key::h | gdk::Key::Left => self.focus_column_left(),
                gdk::Key::l | gdk::Key::Right => self.focus_column_right(),
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
                gdk::Key::g => self.show_palette(),
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
                gdk::Key::q => self.win.close(),
                _ => {
                    // Vertical workspace selection (Ctrl+1..9)
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

        if ctrl && shift && !alt {
            match k {
                gdk::Key::p | gdk::Key::P => self.toggle_profiles_modal(),
                gdk::Key::t | gdk::Key::T => self.toggle_touchpad_mode(),
                gdk::Key::h | gdk::Key::Left => self.move_column(-1),
                gdk::Key::l | gdk::Key::Right => self.move_column(1),
                gdk::Key::j | gdk::Key::Down => self.move_tab_to_row(self.focus_row.get() + 1),
                gdk::Key::k | gdk::Key::Up => {
                    if self.focus_row.get() > 0 {
                        self.move_tab_to_row(self.focus_row.get() - 1);
                    }
                }
                _ => {
                    // Ctrl+Shift+1 -> move to workspace/row 1
                    if keycode == 10 || keycode == 87
                        || k == gdk::Key::_1 || k == gdk::Key::exclam || k == gdk::Key::KP_1
                        || key == gdk::Key::_1 || key == gdk::Key::exclam || key == gdk::Key::KP_1
                        || key.to_unicode() == Some('1') || key.to_unicode() == Some('!')
                    {
                        self.move_tab_to_row(0);
                        return true;
                    }
                    // Ctrl+Shift+2 -> move to workspace/row 2
                    if keycode == 11 || keycode == 88
                        || k == gdk::Key::_2 || k == gdk::Key::at || k == gdk::Key::quotedbl || k == gdk::Key::KP_2
                        || key == gdk::Key::_2 || key == gdk::Key::at || key == gdk::Key::quotedbl || key == gdk::Key::KP_2
                        || key.to_unicode() == Some('2') || key.to_unicode() == Some('@') || key.to_unicode() == Some('"')
                    {
                        self.move_tab_to_row(1);
                        return true;
                    }
                    // Ctrl+Shift+3 -> move to workspace/row 3
                    if keycode == 12 || keycode == 89
                        || k == gdk::Key::_3 || k == gdk::Key::numbersign || k == gdk::Key::KP_3
                        || key == gdk::Key::_3 || key == gdk::Key::numbersign || key == gdk::Key::KP_3
                        || key.to_unicode() == Some('3') || key.to_unicode() == Some('#')
                    {
                        self.move_tab_to_row(2);
                        return true;
                    }
                    // Ctrl+Shift+4 -> move to workspace/row 4
                    if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                        self.move_tab_to_row(3);
                        return true;
                    }
                    // Ctrl+Shift+5 -> move to workspace/row 5
                    if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                        self.move_tab_to_row(4);
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
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let profile_mgr = ProfileManager::new();
    let initial_prof_id = profile_mgr.active_profile().id;

    // ---- sidebar ----
    let brand = gtk::Label::new(None);
    brand.set_markup("<span weight='bold' font_family='monospace' size='large' foreground='#f8fafc'>rail</span><span weight='bold' size='large' foreground='#38bdf8'>.</span>");
    brand.set_hexpand(true);
    brand.set_xalign(0.0);
    let add = gtk::Button::from_icon_name("list-add-symbolic");
    add.add_css_class("btn-add-tab");
    add.set_tooltip_text(Some("New tab  (Ctrl+T)"));
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    head.set_margin_start(4);
    head.append(&brand);
    head.append(&add);

    let pill_label = gtk::Label::new(Some("Search or enter address"));
    pill_label.set_xalign(0.0);
    pill_label.set_hexpand(true);
    pill_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    pill_label.set_width_chars(1);
    let pill_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let s_icon = gtk::Image::from_icon_name("system-search-symbolic");
    s_icon.add_css_class("pill-icon");
    pill_inner.append(&s_icon);
    pill_inner.append(&pill_label);
    let pill_chip = gtk::Label::new(Some("Ctrl+G"));
    pill_chip.add_css_class("pill-chip");
    pill_inner.append(&pill_chip);
    let pill = gtk::Button::new();
    pill.add_css_class("pill");
    pill.set_child(Some(&pill_inner));
    pill.set_tooltip_text(Some("Go to address  (Ctrl+G)"));

    let list = gtk::ListBox::new();
    list.add_css_class("tablist");
    list.set_selection_mode(gtk::SelectionMode::None);
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_vexpand(true);
    list_scroll.set_child(Some(&list));

    let hint = gtk::Label::new(Some("ctrl+h/l tabs  ·  ctrl+j/k rows  ·  ctrl+v new row"));
    hint.add_css_class("side-hint");
    hint.set_xalign(0.0);
    hint.set_margin_start(4);

    let side = gtk::Box::new(gtk::Orientation::Vertical, 10);
    side.add_css_class("side-bar");
    side.set_margin_top(10);
    side.set_margin_bottom(10);
    side.set_margin_start(10);
    side.set_margin_end(2);
    side.append(&head);
    side.append(&pill);
    side.append(&list_scroll);
    side.append(&hint);

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
    entry.set_placeholder_text(Some("Search or enter address"));

    let sicon = gtk::Image::from_icon_name("system-search-symbolic");
    sicon.set_pixel_size(20);

    let profile_btn = gtk::Button::new();
    profile_btn.add_css_class("profile-circle-btn");
    profile_btn.set_valign(gtk::Align::Center);
    profile_btn.set_halign(gtk::Align::End);
    let profile_btn_lbl = gtk::Label::new(Some("D"));
    profile_btn_lbl.add_css_class("profile-circle-label");
    profile_btn.set_child(Some(&profile_btn_lbl));

    let profile_popover = gtk::Popover::new();
    profile_popover.add_css_class("profile-popover");
    profile_popover.set_parent(&profile_btn);
    profile_popover.set_has_arrow(true);

    let prow = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    prow.append(&sicon);
    prow.append(&entry);
    prow.append(&profile_btn);

    let phint = gtk::Label::new(Some("Enter  go      Esc  cancel      Ctrl+P  profiles"));
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
    palette.append(&phint);
    palette.set_visible(false);

    let pop_c = profile_popover.clone();
    profile_btn.connect_clicked(move |_| {
        pop_c.popup();
    });

    // ---- profiles modal (Ctrl+P / Ctrl+Shift+P) ----
    let modal_title = gtk::Label::new(Some("Profiles"));
    modal_title.add_css_class("profiles-modal-title");
    modal_title.set_xalign(0.0);
    modal_title.set_hexpand(true);

    let modal_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    modal_close_btn.add_css_class("flat-icon");
    modal_close_btn.set_tooltip_text(Some("Close (Esc)"));

    let modal_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    modal_header.append(&modal_title);
    modal_header.append(&modal_close_btn);

    let modal_subtitle = gtk::Label::new(Some("Isolated workspaces, sessions & cookie storage  (Ctrl+P)"));
    modal_subtitle.add_css_class("profiles-modal-subtitle");
    modal_subtitle.set_xalign(0.0);

    let profiles_list_box = gtk::ListBox::new();
    profiles_list_box.add_css_class("tablist");
    profiles_list_box.set_selection_mode(gtk::SelectionMode::None);

    let modal_list_scroll = gtk::ScrolledWindow::new();
    modal_list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    modal_list_scroll.set_max_content_height(240);
    modal_list_scroll.set_propagate_natural_height(true);
    modal_list_scroll.set_child(Some(&profiles_list_box));

    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(8);
    sep.set_margin_bottom(8);

    let new_lbl = gtk::Label::new(Some("CREATE NEW PROFILE"));
    new_lbl.add_css_class("profile-section-label");
    new_lbl.set_xalign(0.0);

    let new_profile_entry = gtk::Entry::new();
    new_profile_entry.add_css_class("profile-entry-box");
    new_profile_entry.set_placeholder_text(Some("Enter unique profile name (e.g. Work, School)..."));
    new_profile_entry.set_hexpand(true);

    let new_profile_error_lbl = gtk::Label::new(None);
    new_profile_error_lbl.add_css_class("profile-error-label");
    new_profile_error_lbl.set_xalign(0.0);
    new_profile_error_lbl.set_visible(false);

    let add_prof_btn = gtk::Button::with_label("Create Profile");
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

    let profiles_modal = gtk::Box::new(gtk::Orientation::Vertical, 8);
    profiles_modal.add_css_class("profiles-modal");
    profiles_modal.set_halign(gtk::Align::Center);
    profiles_modal.set_valign(gtk::Align::Center);
    profiles_modal.set_visible(false);

    profiles_modal.append(&modal_header);
    profiles_modal.append(&modal_subtitle);
    profiles_modal.append(&modal_list_scroll);
    profiles_modal.append(&sep);
    profiles_modal.append(&new_lbl);
    profiles_modal.append(&new_profile_entry);
    profiles_modal.append(&new_profile_error_lbl);
    profiles_modal.append(&create_bottom_row);

    // ---- overview hint ----
    let ov_hint = gtk::Label::new(Some("h/l/j/k  move      Enter  open      x  close      Esc  cancel"));
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

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&shell));
    overlay.add_overlay(&load_bar);
    overlay.add_overlay(&palette);
    overlay.add_overlay(&ov_hint);
    overlay.add_overlay(&profiles_modal);

    let win = gtk::ApplicationWindow::builder()
        .application(gapp)
        .default_width(1400)
        .default_height(850)
        .decorated(false)
        .child(&overlay)
        .build();

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
        side_open: Cell::new(true),
        side_gen: Cell::new(0),
        fresh: RefCell::new(None),
        profiles: RefCell::new(profile_mgr),
        selected_palette_profile: RefCell::new(initial_prof_id),
        profile_btn: profile_btn.clone(),
        profile_btn_lbl: profile_btn_lbl.clone(),
        profile_popover: profile_popover.clone(),
        profiles_modal: profiles_modal.clone(),
        profiles_list_box: profiles_list_box.clone(),
        new_profile_entry: new_profile_entry.clone(),
        new_profile_error_lbl: new_profile_error_lbl.clone(),
        add_prof_btn: add_prof_btn.clone(),
        new_profile_color_idx,
        color_buttons: RefCell::new(color_btns),
        touchpad_mode: Cell::new(false),
    });

    for (i, btn) in app.color_buttons.borrow().iter().enumerate() {
        let a = app.clone();
        btn.connect_clicked(move |_| a.select_new_profile_color(i));
    }

    let a = app.clone();
    modal_close_btn.connect_clicked(move |_| a.hide_profiles_modal());
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


    let a = app.clone();
    add.connect_clicked(move |_| a.new_tab());
    let a = app.clone();
    pill.connect_clicked(move |_| a.show_palette());
    let a = app.clone();
    entry.connect_activate(move |e| a.navigate(&e.text()));

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
        if a.profiles_modal_open() {
            a.hide_profiles_modal();
        }
    });
    shell.add_controller(click);

    let scroll = gtk::EventControllerScroll::new(
        gtk::EventControllerScrollFlags::BOTH_AXES
            | gtk::EventControllerScrollFlags::KINETIC
            | gtk::EventControllerScrollFlags::DISCRETE,
    );
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let accum_x = Rc::new(Cell::new(0.0f64));
    let accum_y = Rc::new(Cell::new(0.0f64));
    let last_scroll = Rc::new(Cell::new(0i64));
    let a = app.clone();
    let ax = accum_x.clone();
    let ay = accum_y.clone();
    let ls = last_scroll.clone();
    scroll.connect_scroll(move |_, dx, dy| {
        if !a.touchpad_mode.get() {
            return glib::Propagation::Proceed;
        }
        let now = glib::monotonic_time();
        if now - ls.get() > 400_000 {
            ax.set(0.0);
            ay.set(0.0);
        }
        ls.set(now);

        let nx = ax.get() + dx;
        let ny = ay.get() + dy;
        ax.set(nx);
        ay.set(ny);

        const THRESHOLD: f64 = 2.5;
        if nx > THRESHOLD {
            ax.set(0.0);
            a.focus_column_right();
        } else if nx < -THRESHOLD {
            ax.set(0.0);
            a.focus_column_left();
        }

        if ny > THRESHOLD {
            ay.set(0.0);
            a.focus_row_down();
        } else if ny < -THRESHOLD {
            ay.set(0.0);
            a.focus_row_up();
        }
        glib::Propagation::Stop
    });
    let ax2 = accum_x.clone();
    let ay2 = accum_y.clone();
    scroll.connect_decelerate(move |_, _, _| {
        ax2.set(0.0);
        ay2.set(0.0);
    });
    win.add_controller(scroll);

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let a = app.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        if a.on_key(key, keycode, state) { glib::Propagation::Stop } else { glib::Propagation::Proceed }
    });
    win.add_controller(keys);

    let a = app.clone();
    win.connect_close_request(move |_| {
        a.save_session();
        glib::Propagation::Proceed
    });

    let restored = app.restore_session();
    if let Some(url) = std::env::args().nth(1).filter(|u| !u.starts_with('-')) {
        if restored {
            app.new_tab();
            app.navigate(&url);
        } else {
            app.navigate(&url);
        }
    } else if !restored {
        app.new_tab();
    }
    win.present();
}

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("rail {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: rail [URL]");
        return glib::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a.starts_with("--appimage-")) {
        return glib::ExitCode::FAILURE;
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
            url: "https://google.com".to_string()
        });
        assert_eq!(state.tabs[1], SessionTabItem {
            row: 1,
            col: 0,
            full: true,
            profile_id: "work".to_string(),
            url: "https://github.com".to_string()
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
            url: "https://news.ycombinator.com".to_string()
        });
    }
}
