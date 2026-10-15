mod history;
mod profile;
mod strip;

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

const START_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<title>Rail Browser</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: radial-gradient(circle at 50% 15%, #13192b 0%, #080a10 70%, #05060a 100%);
  color: #94a3b8;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "SF Pro Display", Inter, system-ui, sans-serif;
  padding: 32px 20px;
  overflow-y: auto;
  user-select: none;
}
.container {
  width: 100%;
  max-width: 780px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 26px;
  animation: fadeIn 250ms ease-out;
}
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}
.brand-area {
  text-align: center;
}
.brand-title {
  font-size: clamp(42px, 7vw, 60px);
  font-weight: 800;
  letter-spacing: -0.04em;
  color: #f8fafc;
  line-height: 1.1;
  display: inline-flex;
  align-items: baseline;
}
.brand-title .dot {
  color: #38bdf8;
  text-shadow: 0 0 24px rgba(56, 189, 248, 0.6);
}
.brand-subtitle {
  margin-top: 6px;
  font-size: 13.5px;
  color: #64748b;
  font-weight: 400;
  letter-spacing: 0.01em;
}
.search-wrapper {
  width: 100%;
  max-width: 600px;
  position: relative;
}
.search-bar {
  width: 100%;
  background: rgba(18, 22, 36, 0.85);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 14px;
  padding: 13px 18px 13px 44px;
  font-size: 14.5px;
  color: #f1f5f9;
  outline: none;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  transition: all 180ms ease;
  backdrop-filter: blur(12px);
}
.search-bar:focus {
  background: rgba(22, 28, 46, 0.95);
  border-color: #38bdf8;
  box-shadow: 0 0 0 3px rgba(56, 189, 248, 0.2), 0 16px 44px rgba(0, 0, 0, 0.6);
}
.search-bar::placeholder {
  color: #55657e;
  font-weight: 400;
}
.search-icon {
  position: absolute;
  left: 16px;
  top: 50%;
  transform: translateY(-50%);
  width: 16px;
  height: 16px;
  color: #64748b;
  pointer-events: none;
}
.search-hint {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 10px;
  font-weight: 600;
  color: #64748b;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 5px;
  padding: 2px 7px;
  pointer-events: none;
}
.cards-grid {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 14px;
}
@media (max-width: 640px) {
  .cards-grid { grid-template-columns: 1fr; }
}
.card {
  background: rgba(15, 18, 30, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 12px;
  padding: 16px 18px;
  backdrop-filter: blur(8px);
  transition: all 150ms ease;
}
.card:hover {
  background: rgba(20, 25, 42, 0.85);
  border-color: rgba(56, 189, 248, 0.3);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}
.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.card-icon {
  width: 16px;
  height: 16px;
  color: #38bdf8;
}
.card-title {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: #f1f5f9;
}
.shortcuts-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
}
.shortcut-desc {
  color: #8896ab;
}
.keys {
  display: flex;
  gap: 4px;
}
kbd {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10.5px;
  font-weight: 600;
  color: #e2e8f0;
  background: #111422;
  border: 1px solid #22293e;
  border-bottom: 2px solid #1a2032;
  border-radius: 4px;
  padding: 2px 6px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.footer {
  margin-top: 6px;
  font-size: 11px;
  color: #475569;
}
</style>
</head>
<body>
<div class="container">
  <div class="brand-area">
    <div class="brand-title">rail<span class="dot">.</span></div>
    <div class="brand-subtitle">Spatial 2D Tiled Web Browser • Clean & Isolated</div>
  </div>

  <div class="search-wrapper">
    <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="11" cy="11" r="8"></circle>
      <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
    </svg>
    <input
      type="text"
      class="search-bar"
      placeholder="Search DuckDuckGo or enter web address..."
      autocomplete="off"
      spellcheck="false"
      onkeydown="if(event.key==='Enter'){var q=this.value.trim();if(q){if(q.indexOf('://')!==-1||q.indexOf('.')!==-1&&q.indexOf(' ')===-1){location.href=q.indexOf('://')===-1?'https://'+q:q;}else{location.href='https://duckduckgo.com/?q='+encodeURIComponent(q);}}}"
    />
    <span class="search-hint">Ctrl + G</span>
  </div>

  <div class="cards-grid">
    <div class="card">
      <div class="card-header">
        <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/></svg>
        <div class="card-title">Workspaces & Strips</div>
      </div>
      <div class="shortcuts-list">
        <div class="shortcut-row"><span class="shortcut-desc">New Tab in row</span><div class="keys"><kbd>Ctrl</kbd><kbd>T</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">New Workspace Row</span><div class="keys"><kbd>Ctrl</kbd><kbd>V</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Close Current Tab</span><div class="keys"><kbd>Ctrl</kbd><kbd>W</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Switch Workspace</span><div class="keys"><kbd>Ctrl</kbd><kbd>1..9</kbd></div></div>
      </div>
    </div>

    <div class="card">
      <div class="card-header">
        <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>
        <div class="card-title">Spatial Navigation</div>
      </div>
      <div class="shortcuts-list">
        <div class="shortcut-row"><span class="shortcut-desc">Focus Left / Right Tab</span><div class="keys"><kbd>Ctrl</kbd><kbd>H</kbd><kbd>L</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Focus Up / Down Row</span><div class="keys"><kbd>Ctrl</kbd><kbd>K</kbd><kbd>J</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Full / Half Width Toggle</span><div class="keys"><kbd>Ctrl</kbd><kbd>F</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">2D Grid Overview</span><div class="keys"><kbd>Ctrl</kbd><kbd>O</kbd></div></div>
      </div>
    </div>

    <div class="card">
      <div class="card-header">
        <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
        <div class="card-title">Profiles & Data</div>
      </div>
      <div class="shortcuts-list">
        <div class="shortcut-row"><span class="shortcut-desc">Profiles Manager</span><div class="keys"><kbd>Ctrl</kbd><kbd>P</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Quick Profile Switcher</span><div class="keys"><kbd>Avatar</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Backup / Restore (.tar.gz)</span><div class="keys"><kbd>Ctrl</kbd><kbd>I</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Toggle Left Sidebar</span><div class="keys"><kbd>Ctrl</kbd><kbd>B</kbd></div></div>
      </div>
    </div>

    <div class="card">
      <div class="card-header">
        <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
        <div class="card-title">Developer & Tools</div>
      </div>
      <div class="shortcuts-list">
        <div class="shortcut-row"><span class="shortcut-desc">Rail DevTools Console</span><div class="keys"><kbd>F12</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Inspect Element</span><div class="keys"><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>I</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Floating Minimap HUD</span><div class="keys"><kbd>Ctrl</kbd><kbd>M</kbd></div></div>
        <div class="shortcut-row"><span class="shortcut-desc">Fullscreen View</span><div class="keys"><kbd>F11</kbd></div></div>
      </div>
    </div>
  </div>

  <div class="footer">Press <kbd>Ctrl</kbd> + <kbd>G</kbd> anytime to open the Command Palette</div>
</div>
</body>
</html>"#;

const CSS: &str = "
window {
    background: #090b10;
    border-radius: 0;
    box-shadow: none;
    color: #f1f5f9;
    font-family: system-ui, -apple-system, 'SF Pro Display', 'Inter', 'Segoe UI', Roboto, sans-serif;
}
label {
    color: #94a3b8;
}

/* ---- columns & tabs (Spatial 2D Strip) ---- */
.rail-col {
    background: transparent;
}
.tab {
    border-radius: 12px;
    border: 1px solid #1a2030;
    background: #0d1017;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    transition: border-color 150ms ease, box-shadow 150ms ease;
}
.tab.focused {
    border-color: #38bdf8;
    box-shadow: 0 0 0 1px rgba(56, 189, 248, 0.35), 0 12px 36px rgba(0, 0, 0, 0.7);
}

/* ---- interactive tab resize splitter handle ---- */
.tab-resize-handle {
    min-width: 6px;
    width: 6px;
    background: transparent;
    cursor: col-resize;
    transition: background-color 150ms ease, box-shadow 150ms ease;
}
.tab-resize-handle:hover,
.tab-resize-handle.active {
    background: #38bdf8;
    box-shadow: 0 0 10px rgba(56, 189, 248, 0.7);
}

/* ---- overview (the live strip, zoomed out in 2D) ---- */
.zoomed .tab {
    opacity: 0.88;
    border-radius: 14px;
    border: 1px solid #23283c;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.7);
}
.zoomed .tab.focused {
    opacity: 1;
    border-color: #38bdf8;
    outline: 2px solid #38bdf8;
    outline-offset: 1px;
    box-shadow: 0 0 24px rgba(56, 189, 248, 0.4);
}
.ov-hint {
    background: #0e121d;
    color: #cbd5e1;
    border: 1px solid #232c44;
    border-radius: 8px;
    padding: 7px 16px;
    font-size: 11.5px;
    font-family: ui-monospace, monospace;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.8);
}

/* ---- sidebar (Modern Obsidian Acrylic) ---- */
.side-bar {
    background: #0a0d15;
    border-right: 1px solid #141824;
    padding-right: 4px;
}
.brand {
    font-family: system-ui, -apple-system, sans-serif;
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.02em;
    color: #f8fafc;
}
.brand-dot {
    color: #38bdf8;
}
button.btn-add-tab {
    background: #111422;
    border: 1px solid #1c2234;
    border-radius: 7px;
    min-width: 28px;
    min-height: 28px;
    color: #94a3b8;
    padding: 0;
    transition: all 120ms ease;
}
button.btn-add-tab:hover {
    background: #181d2e;
    border-color: #38bdf8;
    color: #38bdf8;
}
button.flat-icon {
    background: transparent;
    border: none;
    box-shadow: none;
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border-radius: 6px;
    color: #64748b;
    transition: all 100ms ease;
}
button.flat-icon:hover {
    background: rgba(255, 255, 255, 0.06);
    color: #f1f5f9;
}
button.pill {
    background: #101422;
    border: 1px solid #1c2336;
    border-radius: 9px;
    padding: 7px 11px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
    color: #94a3b8;
    transition: all 120ms ease;
}
button.pill:hover {
    background: #151a2b;
    border-color: #28334d;
    color: #f1f5f9;
}
button.pill label {
    color: #94a3b8;
    font-size: 12px;
    font-weight: 500;
}
button.pill:hover label {
    color: #f1f5f9;
}
.pill-icon {
    color: #64748b;
}
button.pill:hover .pill-icon {
    color: #38bdf8;
}
.pill-chip {
    font-family: ui-monospace, monospace;
    font-size: 9.5px;
    font-weight: 600;
    color: #64748b;
    background: #090c14;
    border: 1px solid #1e2438;
    border-radius: 4px;
    padding: 1px 5px;
}
button.pill:hover .pill-chip {
    color: #38bdf8;
    border-color: rgba(56, 189, 248, 0.3);
}

/* ---- tab list & workspaces ---- */
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
    background: #101422;
    border-color: #1a2034;
}
.tablist row:selected {
    background: transparent;
    border-color: transparent;
}
.workspace-header {
    padding: 6px 10px;
    margin: 10px 4px 3px 4px;
    border-radius: 8px;
    background: #0d101a;
    border: 1px solid #161b2b;
    cursor: pointer;
    transition: all 120ms ease;
}
.workspace-header:hover {
    background: #121726;
    border-color: #222a42;
}
.workspace-header.active-workspace {
    background: #14192a;
    border-color: #273454;
    border-left: 3px solid #38bdf8;
}
.workspace-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: #94a3b8;
    text-transform: uppercase;
}
.workspace-header.active-workspace .workspace-title {
    color: #38bdf8;
}
.workspace-badge {
    font-size: 8.5px;
    font-weight: 700;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border-radius: 3px;
    padding: 1px 4px;
    margin-right: 2px;
}
.workspace-tab-count {
    font-size: 10px;
    color: #475569;
    margin-right: 4px;
}
.btn-ws-add {
    min-width: 22px;
    min-height: 22px;
    padding: 0;
    border-radius: 5px;
    background: transparent;
    border: none;
    color: #64748b;
    transition: all 100ms ease;
}
.btn-ws-add:hover {
    background: rgba(56, 189, 248, 0.12);
    color: #38bdf8;
}

.tabrow {
    padding: 6px 10px 6px 14px;
    margin: 2px 4px;
    border-radius: 8px;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 120ms ease;
}
.tabrow:hover {
    background: #111420;
    border-color: #1a2032;
}
.tabrow.active-tab {
    background: #151a2b;
    border-color: #232d48;
    border-left: 3px solid #38bdf8;
}
.tabrow.active-tab .tab-title {
    color: #f8fafc;
    font-weight: 600;
}
.tabrow.active-tab .tab-index {
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border-color: rgba(56, 189, 248, 0.3);
}
.tabrow.dragging {
    opacity: 0.88;
    background: #18223c !important;
    border: 1px solid #38bdf8 !important;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.65), 0 0 14px rgba(56, 189, 248, 0.35) !important;
}
.tabrow.drop-target-above {
    border-top: 3px solid #38bdf8 !important;
    background: rgba(56, 189, 248, 0.14) !important;
    box-shadow: 0 -3px 10px rgba(56, 189, 248, 0.3) !important;
}
.tabrow.drop-target-below {
    border-bottom: 3px solid #38bdf8 !important;
    background: rgba(56, 189, 248, 0.14) !important;
    box-shadow: 0 3px 10px rgba(56, 189, 248, 0.3) !important;
}
.tab-index {
    font-family: ui-monospace, monospace;
    font-size: 10px;
    font-weight: 600;
    color: #475569;
    background: #090c14;
    border: 1px solid #181d2e;
    border-radius: 4px;
    padding: 1px 5px;
    min-width: 14px;
}
.tab-title {
    font-size: 12.5px;
    font-weight: 500;
    color: #94a3b8;
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
    border-radius: 5px;
    background: transparent;
    border: none;
    box-shadow: none;
    color: #64748b;
    cursor: pointer;
    transition: all 120ms ease;
}
.tab-close:hover {
    background: rgba(244, 63, 94, 0.18);
    color: #fb7185;
    opacity: 1 !important;
}
.tabrow:hover .tab-close, .tabrow.active-tab .tab-close {
    opacity: 0.7;
}
.btn-new-workspace {
    margin: 12px 6px 6px 6px;
    padding: 7px 12px;
    background: #0d101a;
    border: 1px dashed #20273d;
    border-radius: 7px;
    color: #64748b;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 120ms ease;
}
.btn-new-workspace:hover {
    background: #14192b;
    border-color: #38bdf8;
    color: #38bdf8;
}
.side-hint {
    font-family: ui-monospace, monospace;
    font-size: 9.5px;
    color: #475569;
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

/* ---- load progress bar ---- */
progressbar.load-bar { min-height: 2px; margin: 0; }
progressbar.load-bar trough { min-height: 2px; background: transparent; border: none; }
progressbar.load-bar progress {
    min-height: 2px;
    background: #38bdf8;
    border-radius: 1px;
    box-shadow: 0 0 10px rgba(56, 189, 248, 0.7);
}

/* ---- address & command palette (Spotlight / Raycast style) ---- */
.palette {
    background: #0d101a;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 14px 18px 12px;
    box-shadow: 0 24px 72px rgba(0, 0, 0, 0.85), 0 0 0 1px rgba(255, 255, 255, 0.05);
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
    font-weight: 500;
    color: #f8fafc;
    font-family: system-ui, -apple-system, sans-serif;
}
.palette-hint {
    color: #64748b;
    font-size: 10.5px;
    font-family: ui-monospace, monospace;
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
    border-radius: 8px;
    background: transparent;
    transition: background 100ms ease;
    border: none;
    box-shadow: none;
}
.palette-sug-item:hover, .palette-sug-item.selected {
    background: rgba(56, 189, 248, 0.12);
}
.palette-sug-icon {
    color: #64748b;
    margin-end: 8px;
}
.palette-sug-item:hover .palette-sug-icon, .palette-sug-item.selected .palette-sug-icon {
    color: #38bdf8;
}
.palette-sug-title {
    color: #f1f5f9;
    font-size: 13px;
    font-weight: 500;
}
.palette-sug-url {
    color: #64748b;
    font-size: 11.5px;
    margin-start: 8px;
}
.palette-sug-badge {
    color: #38bdf8;
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    background: rgba(56, 189, 248, 0.1);
    border: 1px solid rgba(56, 189, 248, 0.25);
    border-radius: 4px;
    padding: 1px 5px;
    margin-start: 8px;
}

/* ---- profile colors (True Dark Multi-Persona) ---- */
.prof-bg-0 { background-color: #38bdf8; color: #000000; }
.prof-bg-1 { background-color: #c084fc; color: #000000; }
.prof-bg-2 { background-color: #4ade80; color: #000000; }
.prof-bg-3 { background-color: #facc15; color: #000000; }
.prof-bg-4 { background-color: #f87171; color: #000000; }
.prof-bg-5 { background-color: #22d3ee; color: #000000; }
.prof-bg-6 { background-color: #fb923c; color: #000000; }
.prof-bg-7 { background-color: #2dd4bf; color: #000000; }

.prof-bg-0 label, .prof-bg-1 label, .prof-bg-2 label, .prof-bg-3 label,
.prof-bg-4 label, .prof-bg-5 label, .prof-bg-6 label, .prof-bg-7 label {
    color: #000000;
}

/* ---- profile circle button in URL palette ---- */
button.profile-circle-btn {
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border-radius: 9999px;
    border: 2px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    transition: transform 120ms ease, border-color 120ms ease;
    color: #000000;
}
button.profile-circle-btn:hover {
    transform: scale(1.08);
    border-color: #38bdf8;
}
button.profile-circle-btn label,
.profile-circle-label {
    color: #000000;
    font-weight: 800;
    font-size: 11.5px;
    font-family: system-ui, sans-serif;
}

/* ---- profile popover dropdown ---- */
.profile-popover {
    background: #0d101a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    padding: 6px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.9);
}
.profile-popover-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #64748b;
    font-family: ui-monospace, monospace;
    padding: 4px 8px 2px;
}
button.profile-item-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    border-radius: 7px;
    padding: 6px 9px;
    transition: background 100ms;
}
button.profile-item-btn:hover {
    background: #14192b;
}
button.profile-item-btn.selected {
    background: #182036;
    border-left: 2px solid #38bdf8;
}
.profile-item-dot {
    min-width: 20px;
    min-height: 20px;
    border-radius: 9999px;
    font-size: 10.5px;
    font-weight: 800;
    color: #000000;
    padding: 0;
}
.profile-item-name {
    font-size: 12.5px;
    font-weight: 600;
    color: #f1f5f9;
}
.profile-item-persona {
    font-size: 9.5px;
    color: #64748b;
    font-family: ui-monospace, monospace;
}
.profile-item-check {
    font-size: 12px;
    font-weight: 700;
    color: #38bdf8;
}

/* ---- profiles manager modal (Ctrl+P) ---- */
.profiles-modal {
    background: #0d101a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    padding: 24px 28px;
    box-shadow: 0 32px 84px rgba(0, 0, 0, 0.95), 0 0 0 1px rgba(255, 255, 255, 0.06);
    min-width: 440px;
    margin-left: 16px;
    margin-right: 16px;
}
.profiles-modal-title {
    font-size: 17px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: -0.02em;
    font-family: system-ui, sans-serif;
}
.profiles-modal-subtitle {
    font-size: 11.5px;
    color: #64748b;
    margin-top: 2px;
}
.profile-card {
    background: #111422;
    border: 1px solid #1b2135;
    border-radius: 10px;
    padding: 10px 14px;
    margin-bottom: 6px;
    transition: all 120ms ease;
}
.profile-card:hover {
    border-color: #283350;
    background: #141829;
}
.profile-card.active-profile {
    border-color: #38bdf8;
    background: #13192c;
    box-shadow: 0 0 20px rgba(56, 189, 248, 0.15);
}
.profile-card.dragging {
    opacity: 0.88;
    border-color: #38bdf8 !important;
    background: #182238 !important;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.65), 0 0 16px rgba(56, 189, 248, 0.35) !important;
}
.profile-card.drop-target-above {
    border-top: 3px solid #38bdf8 !important;
    background: rgba(56, 189, 248, 0.14) !important;
    box-shadow: 0 -3px 10px rgba(56, 189, 248, 0.3) !important;
}
.profile-card.drop-target-below {
    border-bottom: 3px solid #38bdf8 !important;
    background: rgba(56, 189, 248, 0.14) !important;
    box-shadow: 0 3px 10px rgba(56, 189, 248, 0.3) !important;
}
.profile-drag-handle {
    font-size: 15px;
    font-weight: bold;
    color: #475569;
    padding: 0 4px 0 2px;
    cursor: grab;
    transition: color 150ms ease, transform 150ms ease;
}
.profile-drag-handle:hover {
    color: #38bdf8;
    transform: scale(1.15);
}
.profile-avatar-circle {
    min-width: 30px;
    min-height: 30px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 800;
    color: #000000;
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
    color: #64748b;
    font-family: ui-monospace, monospace;
}
.profile-badge-active {
    font-size: 9.5px;
    font-weight: 700;
    border-radius: 9999px;
    padding: 2px 8px;
    background: rgba(56, 189, 248, 0.12);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.35);
    font-family: ui-monospace, monospace;
}
button.btn-make-active {
    background: #141828;
    color: #94a3b8;
    border: 1px solid #242c44;
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    transition: all 120ms ease;
}
button.btn-make-active:hover {
    background: #38bdf8;
    color: #021220;
    border-color: #38bdf8;
}
button.btn-del-profile {
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 4px;
    color: #64748b;
    transition: all 120ms ease;
}
button.btn-del-profile:hover {
    background: rgba(244, 63, 94, 0.18);
    color: #fb7185;
}
.profile-section-label {
    font-size: 10.5px;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.05em;
    font-family: ui-monospace, monospace;
    margin-top: 6px;
}
.profile-entry-box {
    background: #0f121d;
    border: 1px solid #1f253a;
    border-radius: 8px;
    padding: 8px 12px;
    color: #f1f5f9;
    font-size: 13px;
    transition: all 120ms ease;
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
    font-weight: 500;
    color: #f87171;
    margin-top: 2px;
}
button.btn-create-profile {
    background: #121626;
    border: 1px solid #1f273d;
    border-radius: 8px;
    padding: 7px 18px;
    transition: all 120ms ease;
}
button.btn-create-profile label {
    color: #64748b;
    font-weight: 600;
    font-size: 12px;
}
button.btn-create-profile:disabled {
    background: #0f121d;
    border-color: #1a2032;
}
button.btn-create-profile.active-ready {
    background: #38bdf8;
    border-color: #38bdf8;
}
button.btn-create-profile.active-ready label {
    color: #021220;
    font-weight: 700;
}
button.btn-create-profile.active-ready:hover {
    background: #7dd3fc;
    border-color: #7dd3fc;
}

.profile-badge-mini {
    font-size: 9px;
    font-weight: 800;
    min-width: 16px;
    min-height: 16px;
    border-radius: 9999px;
    margin-left: 2px;
    margin-right: 4px;
    padding: 0;
    font-family: ui-monospace, monospace;
    color: #000000;
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
    box-shadow: 0 0 0 2px #0d101a;
    transform: scale(1.15);
}

/* ---- JS script dialogs ---- */
.js-dialog-window {
    background: #0d101a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.9);
}
.js-dialog-title {
    color: #38bdf8;
    font-size: 13px;
    font-weight: 700;
}
.js-dialog-message {
    color: #e2e8f0;
    font-size: 13px;
    line-height: 1.45;
}
.js-dialog-entry {
    background: #111422;
    color: #ffffff;
    border: 1px solid #232c44;
    border-radius: 7px;
    padding: 7px 11px;
    font-size: 13px;
}
.js-dialog-entry:focus {
    border-color: #38bdf8;
    outline: none;
}
.js-dialog-btn-ok {
    background: #0284c7;
    color: #ffffff;
    border-radius: 7px;
    padding: 6px 18px;
    font-size: 12px;
    font-weight: 600;
    border: none;
}
.js-dialog-btn-ok:hover {
    background: #0369a1;
}
.js-dialog-btn-cancel {
    background: #1a2034;
    color: #94a3b8;
    border-radius: 7px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border: none;
}
.js-dialog-btn-cancel:hover {
    background: #252e4a;
    color: #ffffff;
}

/* ---- data backup & migration modal (Ctrl+I) ---- */
.data-modal {
    background: #0d101a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    padding: 24px 28px;
    box-shadow: 0 32px 84px rgba(0, 0, 0, 0.95), 0 0 0 1px rgba(255, 255, 255, 0.06);
    min-width: 520px;
    max-width: 620px;
    margin-left: 16px;
    margin-right: 16px;
}
.data-modal-title {
    font-size: 17px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: -0.02em;
    font-family: system-ui, sans-serif;
}
.data-modal-subtitle {
    font-size: 11.5px;
    color: #64748b;
    margin-top: 2px;
}
.data-section-box {
    background: #111422;
    border: 1px solid #1b2135;
    border-radius: 10px;
    padding: 12px 16px;
    margin-top: 8px;
}
.data-section-title {
    font-size: 12px;
    font-weight: 700;
    color: #38bdf8;
    letter-spacing: 0.02em;
}
.data-section-desc {
    font-size: 11px;
    color: #64748b;
    margin-top: 2px;
    margin-bottom: 8px;
}
.data-info-card {
    background: #0d0f19;
    border: 1px solid #1e263d;
    border-radius: 8px;
    padding: 8px 12px;
    font-family: ui-monospace, monospace;
    font-size: 11px;
    color: #94a3b8;
}
.btn-data-action {
    background: #151a2c;
    border: 1px solid #273350;
    border-radius: 7px;
    color: #f1f5f9;
    font-size: 12px;
    font-weight: 600;
    padding: 5px 12px;
    transition: all 120ms ease;
}
.btn-data-action:hover {
    background: #1e2640;
    border-color: #38bdf8;
    color: #38bdf8;
}
.btn-data-primary {
    background: #0284c7;
    border: 1px solid #38bdf8;
    border-radius: 7px;
    color: #ffffff;
    font-size: 12px;
    font-weight: 700;
    padding: 5px 14px;
    transition: all 120ms ease;
}
.btn-data-primary:hover {
    background: #0369a1;
    border-color: #7dd3fc;
}
.data-status-lbl {
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    margin-top: 6px;
}

/* ---- floating minimap / roadmap (Ctrl+M) ---- */
.minimap-window {
    background: #0b0e17;
    border: 1px solid #1c2438;
    border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.9), 0 0 0 1px rgba(56, 189, 248, 0.15);
    overflow: hidden;
}
.minimap-header {
    background: #0e121e;
    border-bottom: 1px solid #181e2e;
    padding: 7px 12px;
    cursor: grab;
}
.minimap-header:active {
    cursor: grabbing;
}
.minimap-header-icon {
    color: #38bdf8;
    margin-right: 4px;
}
.minimap-header-title {
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: 0.03em;
    font-family: system-ui, sans-serif;
}
.minimap-header-title b {
    color: #38bdf8;
}
.minimap-badge {
    font-family: ui-monospace, monospace;
    font-size: 9px;
    font-weight: 700;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.25);
    border-radius: 4px;
    padding: 1px 5px;
    margin: 0 4px;
}
.minimap-header-hint {
    font-size: 9.5px;
    color: #64748b;
    font-family: ui-monospace, monospace;
}
.minimap-canvas {
    border-radius: 0 0 11px 11px;
    cursor: grab;
}
.minimap-canvas:active {
    cursor: grabbing;
}
.minimap-location-btn {
    background: #111524;
    border: 1px solid #38bdf8;
    border-radius: 20px;
    padding: 3px 10px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.85);
    transition: all 120ms ease;
}
.minimap-location-btn image {
    color: #38bdf8;
    margin-right: 4px;
}
.minimap-location-btn label {
    font-size: 10px;
    font-weight: 700;
    color: #38bdf8;
    font-family: ui-monospace, monospace;
    letter-spacing: 0.02em;
}
.minimap-location-btn:hover {
    background: #0284c7;
    border-color: #7dd3fc;
}
.minimap-location-btn:hover image,
.minimap-location-btn:hover label {
    color: #ffffff;
}

/* ---- Rail DevTools Panel (F12 / Ctrl+Shift+I) ---- */
.devtools-panel {
    background: #0b0e17;
    border-top: 1px solid #1a2032;
    box-shadow: 0 -16px 48px rgba(0, 0, 0, 0.85);
}
.devtools-header {
    background: #0e121e;
    border-bottom: 1px solid #181e2e;
    padding: 6px 14px;
}
.devtools-brand-icon {
    color: #38bdf8;
    margin-right: 6px;
}
.devtools-brand-label {
    font-family: ui-monospace, monospace;
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    margin-right: 14px;
}
.devtools-badge {
    font-family: ui-monospace, monospace;
    font-size: 9px;
    font-weight: 700;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.25);
    border-radius: 4px;
    padding: 1px 5px;
    margin-right: 12px;
}
.devtools-tab-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    padding: 3px 10px;
    color: #94a3b8;
    font-family: ui-monospace, monospace;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 120ms ease;
}
.devtools-tab-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #f8fafc;
}
.devtools-tab-btn.active {
    background: rgba(56, 189, 248, 0.14);
    color: #38bdf8;
    border-color: rgba(56, 189, 248, 0.3);
}
.devtools-log-area {
    background: #0b0e17;
}
.devtools-log-box {
    padding: 6px 12px;
}
.devtools-log-row {
    padding: 3px 10px;
    border-radius: 4px;
    margin-bottom: 2px;
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
}
.devtools-log-row.level-in {
    background: rgba(255, 255, 255, 0.03);
    color: #94a3b8;
    border-left: 2px solid #64748b;
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
    background: rgba(56, 189, 248, 0.08);
    color: #38bdf8;
    border-left: 2px solid #38bdf8;
}
.devtools-input-bar {
    background: #0d111d;
    border-top: 1px solid #1a2032;
    padding: 5px 12px;
}
.devtools-prompt-char {
    font-family: ui-monospace, monospace;
    font-size: 13px;
    font-weight: 800;
    color: #38bdf8;
    margin-right: 8px;
}
.devtools-entry {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #f8fafc;
    font-family: ui-monospace, monospace;
    font-size: 12px;
}
.devtools-text-view {
    background: #0b0e17;
    color: #cbd5e1;
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    padding: 12px;
}
.devtools-card {
    background: #0f1320;
    border: 1px solid #1b2236;
    border-radius: 8px;
    padding: 12px 16px;
    margin: 8px 12px;
}
.devtools-card-title {
    font-family: ui-monospace, monospace;
    font-size: 11px;
    font-weight: 700;
    color: #38bdf8;
    margin-bottom: 6px;
}
.devtools-card-row {
    font-family: ui-monospace, monospace;
    font-size: 11px;
    color: #94a3b8;
    padding: 2px 0;
}
.devtools-card-row b {
    color: #f8fafc;
}
";

/// How far the strip zooms out in overview (niri uses a similar factor).
const OV_ZOOM: f64 = 0.4;

struct Tab {
    container: gtk::Box,
    view: WebView,
    profile_id: String,
    custom_width: Cell<Option<f64>>,
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
    profiles_modal_scroll: gtk::ScrolledWindow,
    profiles_list_box: gtk::ListBox,
    new_profile_entry: gtk::Entry,
    new_profile_error_lbl: gtk::Label,
    add_prof_btn: gtk::Button,
    new_profile_color_idx: Cell<usize>,
    color_buttons: RefCell<Vec<gtk::Button>>,
    touchpad_mode: Cell<bool>,
    last_saved_session: RefCell<String>,
    save_session_gen: Cell<u64>,
    data_modal: gtk::Box,
    data_modal_info_lbl: gtk::Label,
    data_modal_status_lbl: gtk::Label,
    data_export_entry: gtk::Entry,
    data_import_entry: gtk::Entry,
    // Minimap / Roadmap (Ctrl+M)
    map_container: gtk::Box,
    map_drawing_area: gtk::DrawingArea,
    map_location_btn: gtk::Button,
    map_state: RefCell<MinimapState>,
    // DevTools Panel (F12 / Ctrl+Shift+I)
    devtools_open: Cell<bool>,
    devtools_panel: gtk::Box,
    devtools_content_box: gtk::Box,
    devtools_url_lbl: gtk::Label,
    devtools_window: RefCell<Option<gtk::Window>>,
}

fn to_uri(input: &str) -> String {
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
    format!("https://duckduckgo.com/?q={}", glib::Uri::escape_string(s, None, false))
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

    fn create_tab_widget(view: &WebView) -> (gtk::Box, gtk::Box) {
        view.add_css_class("tab");
        view.set_overflow(gtk::Overflow::Hidden);
        view.set_vexpand(true);
        view.set_hexpand(true);

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.add_css_class("rail-col");
        container.add_css_class("full");
        container.set_vexpand(true);
        container.set_hexpand(true);

        let resize_handle = gtk::Box::new(gtk::Orientation::Vertical, 0);
        resize_handle.add_css_class("tab-resize-handle");
        resize_handle.set_cursor_from_name(Some("col-resize"));

        container.append(view);
        container.append(&resize_handle);
        (container, resize_handle)
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
        settings.set_enable_webrtc(true);
        settings.set_enable_hyperlink_auditing(false);
        settings.set_enable_dns_prefetching(false);
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_webgl(true);
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
        settings.set_enable_developer_extras(true);
        settings.set_enable_back_forward_navigation_gestures(true);

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
        view.load_html(START_PAGE, None);
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
        view.load_html(START_PAGE, None);
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, prof_id.clone(), true);
        self.select_palette_profile(&prof_id);
        self.show_palette();
    }

    fn new_tab_with_url(self: &Rc<Self>, url: &str) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_uri(url);
        self.add_view(view, prof_id, false);
    }

    fn split_with_url(self: &Rc<Self>, url: &str) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_uri(url);
        self.add_view(view, prof_id, true);
    }

    // ---- DevTools (Ctrl+Shift+I) ----

    fn devtools_open(&self) -> bool {
        self.devtools_open.get()
    }

    fn toggle_devtools(self: &Rc<Self>) {
        if self.devtools_open() {
            self.hide_devtools();
        } else {
            self.show_devtools();
        }
    }

    fn show_devtools(self: &Rc<Self>) {
        let Some(view) = self.focused_view() else { return };
        let Some(insp) = view.inspector() else { return };

        // If inspector is inside detached window and that window is open, present it
        if let Some(win) = self.devtools_window.borrow().as_ref() {
            if win.is_visible() {
                insp.show();
                win.present();
                self.devtools_open.set(true);
                return;
            }
        }

        insp.show();

        // Update URL and host badge in devtools header
        let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_else(|| "about:blank".to_string());
        let host = host_of(&cur_uri).unwrap_or_else(|| "localhost".to_string());
        self.devtools_url_lbl.set_markup(&format!("<span foreground='#38bdf8'>{}</span> <span color='#64748b'>•</span> <span color='#94a3b8'>{}</span>", glib::markup_escape_text(&host), glib::markup_escape_text(&cur_uri)));

        // Attach WebKit's full WebInspector widget to content_box
        if let Some(w) = insp.web_view() {
            let widget: gtk::Widget = w.upcast();
            if widget.parent().as_ref() != Some(self.devtools_content_box.upcast_ref()) {
                while let Some(c) = self.devtools_content_box.first_child() {
                    self.devtools_content_box.remove(&c);
                }
                widget.set_vexpand(true);
                widget.set_hexpand(true);
                self.devtools_content_box.append(&widget);
            }
        }

        self.devtools_panel.set_visible(true);
        self.devtools_open.set(true);
    }

    fn hide_devtools(&self) {
        if let Some(view) = self.focused_view() {
            if let Some(insp) = view.inspector() {
                insp.close();
            }
        }
        if let Some(win) = self.devtools_window.borrow().as_ref() {
            win.set_visible(false);
        }
        self.devtools_panel.set_visible(false);
        self.devtools_open.set(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    fn toggle_devtools_undocked(self: &Rc<Self>) {
        let Some(view) = self.focused_view() else { return };
        let Some(insp) = view.inspector() else { return };
        let Some(w) = insp.web_view() else { return };
        let widget: gtk::Widget = w.upcast();

        let mut win_opt = self.devtools_window.borrow_mut();
        if let Some(win) = win_opt.take() {
            // Re-dock into bottom drawer
            win.set_child(None::<&gtk::Widget>);
            win.close();
            while let Some(c) = self.devtools_content_box.first_child() {
                self.devtools_content_box.remove(&c);
            }
            widget.set_vexpand(true);
            widget.set_hexpand(true);
            self.devtools_content_box.append(&widget);
            self.devtools_panel.set_visible(true);
        } else {
            // Detach into dedicated window
            self.devtools_panel.set_visible(false);
            while let Some(c) = self.devtools_content_box.first_child() {
                self.devtools_content_box.remove(&c);
            }

            let cur_uri = view.uri().map(|s| s.to_string()).unwrap_or_default();
            let host = host_of(&cur_uri).unwrap_or_else(|| "DevTools".to_string());
            let win = gtk::Window::builder()
                .title(&format!("Rail DevTools - {}", host))
                .default_width(1050)
                .default_height(720)
                .child(&widget)
                .build();
            win.add_css_class("devtools-window");

            let app_re_dock = self.clone();
            win.connect_close_request(move |_| {
                app_re_dock.hide_devtools();
                glib::Propagation::Proceed
            });

            win.present();
            *win_opt = Some(win);
        }
    }

    fn show_script_dialog(self: &Rc<Self>, view: &WebView, dialog: &ScriptDialog) {
        let dialog_type = dialog.dialog_type();
        let message = dialog.message().map(|s| s.to_string()).unwrap_or_default();
        let default_prompt = dialog.prompt_get_default_text().map(|s| s.to_string()).unwrap_or_default();

        let host = view.uri()
            .and_then(|u| glib::Uri::parse(&u, glib::UriFlags::NONE).ok())
            .and_then(|u| u.host().map(|h| h.to_string()))
            .unwrap_or_else(|| "This page".to_string());

        let title_text = match dialog_type {
            ScriptDialogType::Alert => format!("{} says:", host),
            ScriptDialogType::Confirm => format!("Confirm from {}", host),
            ScriptDialogType::Prompt => format!("Prompt from {}", host),
            ScriptDialogType::BeforeUnloadConfirm => "Leave Site?".to_string(),
            _ => format!("Message from {}", host),
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
            "Changes you made may not be saved. Are you sure you want to leave?".to_string()
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
            let cancel_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm { "Stay" } else { "Cancel" };
            let cancel_btn = gtk::Button::with_label(cancel_label);
            cancel_btn.add_css_class("js-dialog-btn-cancel");
            let cd = close_dialog.clone();
            cancel_btn.connect_clicked(move |_| cd(false));
            btn_row.append(&cancel_btn);
        }

        let ok_label = if dialog_type == ScriptDialogType::BeforeUnloadConfirm { "Leave" } else { "OK" };
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
                menu.append(&ContextMenuItem::from_gaction(&act_tab, "Open Link in New Tab", None));

                let uri_s = uri.clone();
                let app_s = self.clone();
                let act_split = gio::SimpleAction::new("ctx-open-split", None);
                act_split.connect_activate(move |_, _| {
                    app_s.split_with_url(&uri_s);
                });
                self.win.add_action(&act_split);
                menu.append(&ContextMenuItem::from_gaction(&act_split, "Open Link in Split View", None));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyLinkToClipboard,
                    "Copy Link Address",
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::DownloadLinkToDisk,
                    "Save Link As...",
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
                menu.append(&ContextMenuItem::from_gaction(&act_img, "Open Image in New Tab", None));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyImageUrlToClipboard,
                    "Copy Image Address",
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::CopyImageToClipboard,
                    "Copy Image",
                ));

                menu.append(&ContextMenuItem::from_stock_action_with_label(
                    ContextMenuAction::DownloadImageToDisk,
                    "Save Image As...",
                ));

                menu.append(&ContextMenuItem::new_separator());
            }
        }

        // 3. Media items
        if is_media {
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::OpenVideoInNewWindow,
                "Open Media in New Tab",
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::CopyVideoLinkToClipboard,
                "Copy Media Address",
            ));
            menu.append(&ContextMenuItem::from_stock_action_with_label(
                ContextMenuAction::DownloadVideoToDisk,
                "Save Media As...",
            ));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 4. Editable text
        if is_editable {
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Cut, "Cut"));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Copy, "Copy"));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Paste, "Paste"));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::PasteAsPlainText, "Paste as Plain Text"));
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::SelectAll, "Select All"));
            menu.append(&ContextMenuItem::new_separator());
        } else if is_selection {
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Copy, "Copy"));

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
            menu.append(&ContextMenuItem::from_gaction(&act_search, "Search Google for Selection", None));
            menu.append(&ContextMenuItem::new_separator());
        }

        // 5. Page navigation
        if !is_link && !is_editable && !is_selection {
            if view.can_go_back() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::GoBack, "Back"));
            }
            if view.can_go_forward() {
                menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::GoForward, "Forward"));
            }
            menu.append(&ContextMenuItem::from_stock_action_with_label(ContextMenuAction::Reload, "Reload"));
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
            menu.append(&ContextMenuItem::from_gaction(&act_src, "View Page Source  (F12)", None));
        }

        menu.append(&ContextMenuItem::from_stock_action_with_label(
            ContextMenuAction::InspectElement,
            "Inspect Element  (F12 / Ctrl+Shift+I)",
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
                app_insp.show_devtools();
                true
            });
            let a_closed = self.clone();
            insp.connect_closed(move |_| {
                a_closed.hide_devtools();
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

        drag.connect_drag_begin({
            let app = app.clone();
            let v = v.clone();
            let h = h.clone();
            let start_w0 = start_w0.clone();
            let start_w1 = start_w1.clone();
            move |_gesture, _start_x, _start_y| {
                h.add_css_class("active");
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
            move |_gesture, offset_x, _offset_y| {
                if let Some((r, c)) = app.find_view_coords(&v) {
                    let vw = app.strip.width() as f64;
                    let min_w = 280.0;
                    let max_w = vw; // Fullscreen max: left edge to right edge
                    let initial0 = start_w0.get();
                    let new_w0 = (initial0 + offset_x).clamp(min_w, max_w);

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
        let (container, resize_handle) = Self::create_tab_widget(&view);

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
                rows[next_r].tabs.push(Tab { container: container.clone(), view: view.clone(), profile_id, custom_width: Cell::new(None) });
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
                rows[cur_r].tabs.insert(col_idx, Tab { container: container.clone(), view: view.clone(), profile_id, custom_width: Cell::new(None) });
                (cur_r, col_idx)
            }
        };

        self.setup_tab_signals(&container, &view, &resize_handle);
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
            view.load_html(START_PAGE, None);
            view.set_can_target(!self.ov.get());
            let (container, resize_handle) = Self::create_tab_widget(&view);
            self.setup_tab_signals(&container, &view, &resize_handle);

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab { container, view, profile_id: prof_id, custom_width: Cell::new(None) });
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
            view.load_html(START_PAGE, None);
            view.set_can_target(!self.ov.get());
            let (container, resize_handle) = Self::create_tab_widget(&view);
            self.setup_tab_signals(&container, &view, &resize_handle);

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab { container, view, profile_id: prof_id, custom_width: Cell::new(None) });
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
        self.touchpad_mode.set(!self.touchpad_mode.get());
        self.schedule_save_session();
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
        let win_w = self.win.width();
        let win_h = self.win.height();
        let is_max = if self.win.is_maximized() { 1 } else { 0 };
        out.push_str(&format!("win\t{}\t{}\t{}\n", win_w, win_h, is_max));

        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                let is_full = if tab.container.has_css_class("full") { 1 } else { 0 };
                let uri = tab.view.uri().unwrap_or_default();
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
                view.load_html(START_PAGE, None);
            } else {
                view.load_uri(&item.url);
            }
            let (container, resize_handle) = Self::create_tab_widget(&view);
            if !item.full {
                container.remove_css_class("full");
            }
            self.setup_tab_signals(&container, &view, &resize_handle);
            loaded_tabs.push((item.row, Tab { container, view, profile_id: item.profile_id, custom_width: Cell::new(item.custom_width) }));
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
                let active_badge = gtk::Label::new(Some("ACTIVE"));
                active_badge.add_css_class("workspace-badge");
                ws_box.append(&active_badge);
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
                if ri == cur_r && ci == cur_c {
                    item.add_css_class("active-tab");
                }
                tab_items_registry.borrow_mut().push((ri, ci, item.clone()));

                let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                content_box.set_hexpand(true);
                content_box.append(&Self::tab_icon(t, 16));

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

                if t.container.has_css_class("full") {
                    let full_tag = gtk::Label::new(Some("FULL"));
                    full_tag.add_css_class("tab-full-badge");
                    full_tag.set_valign(gtk::Align::Center);
                    content_box.append(&full_tag);
                }

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
        let new_ws_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        new_ws_box.add_css_class("btn-new-workspace");
        new_ws_box.set_halign(gtk::Align::Fill);
        let new_ws_icon = gtk::Image::from_icon_name("list-add-symbolic");
        let new_ws_lbl = gtk::Label::new(Some("+ New Workspace (Ctrl+Shift+V)"));
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
            self.profile_btn_lbl.set_text(&p.initial_letter());
            for i in 0..8 {
                self.profile_btn.remove_css_class(&format!("prof-bg-{}", i));
            }
            self.profile_btn.add_css_class(&format!("prof-bg-{}", p.color_idx));
            let persona = ProfilePersona::for_id(&p.id);
            self.profile_btn.set_tooltip_text(Some(&format!("Profile: {} [{}] (100% Isolated)", p.name, persona.summary())));
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

            let persona = ProfilePersona::for_id(&p.id);
            let name_col = gtk::Box::new(gtk::Orientation::Vertical, 1);
            name_col.set_hexpand(true);

            let name_lbl = gtk::Label::new(Some(&p.name));
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
                app.profiles.borrow_mut().set_active(&pid);
                app.select_palette_profile(&pid);
                app.switch_focused_tab_profile(&pid);
                app.rebuild_profile_popover();
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
                SuggestionKind::History => "History",
                SuggestionKind::Popular => "Top",
                SuggestionKind::Search => "Search",
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

        let is_typing_forward = cur_len > prev_len;
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
                        self.last_entry_len.set(autofill.chars().count());
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
                new_view.load_html(START_PAGE, None);
            } else {
                new_view.load_uri(&uri);
            }
        } else {
            new_view.load_html(START_PAGE, None);
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
            self.profiles.borrow_mut().set_active(&target_profile_id);
            let uri = to_uri(text);
            self.history.borrow_mut().record_visit(&uri, "");

            if is_new_tab {
                self.new_tab_with_url(&uri);
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

    fn data_modal_open(&self) -> bool {
        self.data_modal.is_visible()
    }

    fn toggle_data_modal(self: &Rc<Self>) {
        if self.data_modal_open() {
            self.hide_data_modal();
        } else {
            self.show_data_modal();
        }
    }

    fn show_data_modal(self: &Rc<Self>) {
        if self.profiles_modal_open() {
            self.hide_profiles_modal();
        }
        if self.palette_open() {
            self.hide_palette();
        }
        self.refresh_data_modal_info();
        let timestamp = glib::DateTime::now_local()
            .ok()
            .and_then(|d| d.format("%Y%m%d_%H%M%S").ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "backup".to_string());
        let default_export = glib::home_dir().join(format!("rail_backup_{}.tar.gz", timestamp));
        self.data_export_entry.set_text(&default_export.to_string_lossy());
        self.data_modal_status_lbl.set_text("");
        self.data_modal.set_visible(true);
        self.data_export_entry.grab_focus();
    }

    fn hide_data_modal(&self) {
        self.data_modal.set_visible(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    fn refresh_data_modal_info(&self) {
        let dir = rail_data_dir();
        let rows = self.rows.borrow();
        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let ws_count = rows.len();
        drop(rows);
        let prof_count = self.profiles.borrow().profiles().len();

        let info_text = format!(
            "Directory: {}\nStatus:    {} open tab(s) in {} workspace(s) | {} profiles (cookies/logins active)",
            dir.display(), total_tabs, ws_count, prof_count
        );
        self.data_modal_info_lbl.set_text(&info_text);
    }

    fn execute_export_action(self: &Rc<Self>) {
        self.save_session_immediate();
        let path_str = self.data_export_entry.text().to_string();
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            self.data_modal_status_lbl.set_markup("<span foreground='#f87171'><b>Error:</b> Please specify an export destination file.</span>");
            return;
        }

        match export_data(&path) {
            Ok(()) => {
                let size_str = std::fs::metadata(&path)
                    .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
                    .unwrap_or_else(|_| "saved".into());
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#4ade80'><b>Export Successful:</b> {} ({})</span>",
                    glib::markup_escape_text(&path.to_string_lossy()), size_str
                ));
            }
            Err(e) => {
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#f87171'><b>Export Failed:</b> {}</span>",
                    glib::markup_escape_text(&e)
                ));
            }
        }
    }

    fn execute_import_action(self: &Rc<Self>) {
        let path_str = self.data_import_entry.text().to_string();
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() || !path.exists() {
            self.data_modal_status_lbl.set_markup("<span foreground='#f87171'><b>Error:</b> Please select a valid .tar.gz backup file.</span>");
            return;
        }

        match import_data(&path) {
            Ok(()) => {
                // Reload profile manager from imported profiles.txt
                *self.profiles.borrow_mut() = ProfileManager::new(rail_data_dir());
                self.rebuild_profiles_modal_list();
                let active_id = self.profiles.borrow().active_profile().id;
                self.select_palette_profile(&active_id);

                // Reload session tabs
                self.restore_session();
                self.refresh_sidebar();
                self.refresh_data_modal_info();

                self.data_modal_status_lbl.set_markup(
                    "<span foreground='#4ade80'><b>Import Successful:</b> All workspaces, tabs, accounts & cookies restored!</span>"
                );
            }
            Err(e) => {
                self.data_modal_status_lbl.set_markup(&format!(
                    "<span foreground='#f87171'><b>Import Failed:</b> {}</span>",
                    glib::markup_escape_text(&e)
                ));
            }
        }
    }

    // ---- Minimap / Roadmap (Ctrl+M) ----

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
        cr.set_source_rgb(0.024, 0.035, 0.075);
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
                cr.set_source_rgb(0.059, 0.090, 0.165);
                let _ = cr.fill_preserve();
                // Subtle border (#1e293b)
                cr.set_source_rgb(0.118, 0.161, 0.231);
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

            // Electric Rail cyan fill (#0284c7)
            cr.set_source_rgba(0.008, 0.518, 0.780, 0.85);
            let _ = cr.fill_preserve();

            // Radiant cyan border (#7dd3fc)
            cr.set_source_rgb(0.490, 0.827, 0.988);
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
            drag_handle.set_tooltip_text(Some("Drag up or down to reorder profiles"));

            row.set_cursor_from_name(Some("grab"));

            if total_profiles > 1 {
                let drag = gtk::GestureDrag::new();
                drag.set_button(gdk::BUTTON_PRIMARY);

                let row_drag = row.clone();
                let handle_drag = drag_handle.clone();
                drag.connect_drag_begin(move |_gesture, _start_x, _start_y| {
                    row_drag.add_css_class("dragging");
                    row_drag.set_cursor_from_name(Some("grabbing"));
                    handle_drag.set_cursor_from_name(Some("grabbing"));
                });

                let rows_vec_update = rows_vec.clone();
                let from_idx = idx;
                drag.connect_drag_update(move |_gesture, _offset_x, offset_y| {
                    let card_height = 54.0f64;
                    let delta = (offset_y / card_height).round() as isize;
                    let target_idx = ((from_idx as isize) + delta).clamp(0, (total_profiles - 1) as isize) as usize;

                    let rows_ref = rows_vec_update.borrow();
                    for (i, r_box) in rows_ref.iter().enumerate() {
                        if i == target_idx && target_idx != from_idx {
                            if delta < 0 {
                                r_box.add_css_class("drop-target-above");
                                r_box.remove_css_class("drop-target-below");
                            } else {
                                r_box.add_css_class("drop-target-below");
                                r_box.remove_css_class("drop-target-above");
                            }
                        } else {
                            r_box.remove_css_class("drop-target-above");
                            r_box.remove_css_class("drop-target-below");
                        }
                    }
                });

                let row_end = row.clone();
                let handle_end = drag_handle.clone();
                let app_end = self.clone();
                let from_idx = idx;
                let rows_vec_end = rows_vec.clone();
                drag.connect_drag_end(move |_gesture, _offset_x, offset_y| {
                    row_end.remove_css_class("dragging");
                    row_end.set_cursor_from_name(Some("grab"));
                    handle_end.set_cursor_from_name(Some("grab"));

                    let rows_ref = rows_vec_end.borrow();
                    for r_box in rows_ref.iter() {
                        r_box.remove_css_class("drop-target-above");
                        r_box.remove_css_class("drop-target-below");
                    }
                    drop(rows_ref);

                    let card_height = 54.0f64;
                    let delta = (offset_y / card_height).round() as isize;
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

                row.add_controller(drag);
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

            let persona = ProfilePersona::for_id(&p.id);
            let sub_text = format!("@{} - {} (100% Isolated)", p.id, persona.summary());
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
            self.new_profile_error_lbl.set_text("Profile already exists");
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
                self.new_profile_error_lbl.set_text(&err);
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

        if self.data_modal_open() {
            if k == gdk::Key::Escape {
                self.hide_data_modal();
                return true;
            }
            if ctrl && !alt && (k == gdk::Key::i || k == gdk::Key::I) {
                self.hide_data_modal();
                return true;
            }
            // All other keystrokes pass directly to data_modal without triggering window shortcuts
            return false;
        }

        if self.map_open() {
            if k == gdk::Key::Escape {
                self.hide_map();
                return true;
            }
            if ctrl && !shift && !alt && (k == gdk::Key::m || k == gdk::Key::M) {
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
            if ctrl && shift && !alt && (k == gdk::Key::i || k == gdk::Key::I) {
                self.hide_devtools();
                return true;
            }
        }

        if !ctrl && !shift && !alt && k == gdk::Key::F12 {
            self.toggle_devtools();
            return true;
        }
        if ctrl && shift && !alt && (k == gdk::Key::i || k == gdk::Key::I) {
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
        let global = ctrl && !shift && !alt && (matches!(k, gdk::Key::o | gdk::Key::t | gdk::Key::T | gdk::Key::q | gdk::Key::p | gdk::Key::m | gdk::Key::i | gdk::Key::I) || keycode == 28);
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
                gdk::Key::p | gdk::Key::P => self.toggle_profiles_modal(),
                gdk::Key::i | gdk::Key::I => self.toggle_data_modal(),
                gdk::Key::t | gdk::Key::T => self.open_new_tab_palette(),
                gdk::Key::n => self.split_vertical(),
                gdk::Key::f => self.toggle_full(),
                gdk::Key::h | gdk::Key::Left => self.focus_column_left(),
                gdk::Key::l | gdk::Key::Right => self.focus_column_right(),
                gdk::Key::j | gdk::Key::Down => self.focus_row_down(),
                gdk::Key::k | gdk::Key::Up => self.focus_row_up(),
                gdk::Key::v | gdk::Key::V => {
                    if self.palette_open() {
                        let clipboard = self.entry.clipboard();
                        let ent = self.entry.clone();
                        clipboard.read_text_async(gio::Cancellable::NONE, move |res| {
                            if let Ok(Some(text)) = res {
                                let text = text.to_string();
                                let (start, end) = ent.selection_bounds().unwrap_or_else(|| {
                                    let pos = ent.position();
                                    (pos, pos)
                                });
                                ent.delete_text(start, end);
                                let mut insert_pos = start;
                                ent.insert_text(&text, &mut insert_pos);
                                ent.set_position(insert_pos);
                            }
                        });
                        return true;
                    }
                    return false;
                }
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
                gdk::Key::q => self.win.close(),
                _ => {
                    if keycode == 28 {
                        self.open_new_tab_palette();
                        return true;
                    }
                    if keycode == 42 {
                        self.show_palette();
                        return true;
                    }
                    if keycode == 55 && self.palette_open() {
                        let clipboard = self.entry.clipboard();
                        let ent = self.entry.clone();
                        clipboard.read_text_async(gio::Cancellable::NONE, move |res| {
                            if let Ok(Some(text)) = res {
                                let text = text.to_string();
                                let (start, end) = ent.selection_bounds().unwrap_or_else(|| {
                                    let pos = ent.position();
                                    (pos, pos)
                                });
                                ent.delete_text(start, end);
                                let mut insert_pos = start;
                                ent.insert_text(&text, &mut insert_pos);
                                ent.set_position(insert_pos);
                            }
                        });
                        return true;
                    }
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
                gdk::Key::i | gdk::Key::I => self.toggle_devtools(),
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

    let profile_mgr = ProfileManager::new(rail_data_dir());
    let history_mgr = HistoryManager::new(rail_data_dir());
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

    let hint = gtk::Label::new(Some("ctrl+h/l tabs  |  ctrl+j/k rows  |  ctrl+shift+v new row"));
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

    let palette_suggestions_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    palette_suggestions_box.add_css_class("palette-suggestions-box");
    palette_suggestions_box.set_visible(false);

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
    palette.append(&palette_suggestions_box);
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

    // ---- data backup & migration modal (Ctrl+I) ----
    let data_modal_title = gtk::Label::new(Some("Data Backup & Migration"));
    data_modal_title.add_css_class("data-modal-title");
    data_modal_title.set_hexpand(true);
    data_modal_title.set_xalign(0.0);

    let data_modal_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    data_modal_close_btn.add_css_class("flat-icon");
    data_modal_close_btn.set_tooltip_text(Some("Close (Esc)"));

    let data_header_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    data_header_row.append(&data_modal_title);
    data_header_row.append(&data_modal_close_btn);

    let data_modal_subtitle = gtk::Label::new(Some("Export or import all workspaces, profiles, logins & cookies  (Ctrl+I)"));
    data_modal_subtitle.add_css_class("data-modal-subtitle");
    data_modal_subtitle.set_xalign(0.0);

    let data_info_lbl = gtk::Label::new(None);
    data_info_lbl.add_css_class("data-info-card");
    data_info_lbl.set_xalign(0.0);
    data_info_lbl.set_margin_top(4);

    // --- Export section ---
    let export_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    export_box.add_css_class("data-section-box");

    let export_title = gtk::Label::new(Some("Export Environment (All Data)"));
    export_title.add_css_class("data-section-title");
    export_title.set_xalign(0.0);

    let export_desc = gtk::Label::new(Some("Pack all your open tabs, workspaces, profiles, accounts & cookies into a single portable .tar.gz archive."));
    export_desc.add_css_class("data-section-desc");
    export_desc.set_xalign(0.0);
    export_desc.set_wrap(true);

    let data_export_entry = gtk::Entry::new();
    data_export_entry.add_css_class("profile-entry-box");
    data_export_entry.set_hexpand(true);

    let btn_export_browse = gtk::Button::with_label("Browse...");
    btn_export_browse.add_css_class("btn-data-action");

    let btn_export_do = gtk::Button::with_label("Export Backup");
    btn_export_do.add_css_class("btn-data-primary");

    let export_actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    export_actions_row.append(&data_export_entry);
    export_actions_row.append(&btn_export_browse);
    export_actions_row.append(&btn_export_do);

    export_box.append(&export_title);
    export_box.append(&export_desc);
    export_box.append(&export_actions_row);

    // --- Import section ---
    let import_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    import_box.add_css_class("data-section-box");

    let import_title = gtk::Label::new(Some("Import Environment"));
    import_title.add_css_class("data-section-title");
    import_title.set_xalign(0.0);

    let import_desc = gtk::Label::new(Some("Restore tabs, workspaces, profiles, and logins from a .tar.gz archive. Replaces current session."));
    import_desc.add_css_class("data-section-desc");
    import_desc.set_xalign(0.0);
    import_desc.set_wrap(true);

    let data_import_entry = gtk::Entry::new();
    data_import_entry.add_css_class("profile-entry-box");
    data_import_entry.set_placeholder_text(Some("Select or enter path to .tar.gz backup..."));
    data_import_entry.set_hexpand(true);

    let btn_import_browse = gtk::Button::with_label("Browse...");
    btn_import_browse.add_css_class("btn-data-action");

    let btn_import_do = gtk::Button::with_label("Import & Restore");
    btn_import_do.add_css_class("btn-data-primary");

    let import_actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    import_actions_row.append(&data_import_entry);
    import_actions_row.append(&btn_import_browse);
    import_actions_row.append(&btn_import_do);

    import_box.append(&import_title);
    import_box.append(&import_desc);
    import_box.append(&import_actions_row);

    let data_modal_status_lbl = gtk::Label::new(None);
    data_modal_status_lbl.add_css_class("data-status-lbl");
    data_modal_status_lbl.set_xalign(0.0);
    data_modal_status_lbl.set_wrap(true);

    let data_modal = gtk::Box::new(gtk::Orientation::Vertical, 8);
    data_modal.add_css_class("data-modal");
    data_modal.set_halign(gtk::Align::Center);
    data_modal.set_valign(gtk::Align::Center);
    data_modal.set_visible(false);

    data_modal.append(&data_header_row);
    data_modal.append(&data_modal_subtitle);
    data_modal.append(&data_info_lbl);
    data_modal.append(&export_box);
    data_modal.append(&import_box);
    data_modal.append(&data_modal_status_lbl);

    // ---- Minimap / Roadmap (Ctrl+M) ----
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

    let map_badge = gtk::Label::new(Some("Ctrl+M"));
    map_badge.add_css_class("minimap-badge");

    let map_hint = gtk::Label::new(Some("Drag header"));
    map_hint.add_css_class("minimap-header-hint");
    map_hint.set_xalign(0.0);

    let map_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    map_close_btn.add_css_class("flat-icon");
    map_close_btn.set_tooltip_text(Some("Close Map (Ctrl+M / Esc)"));
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
    let loc_lbl = gtk::Label::new(Some("recenter"));
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

    // ---- Native Rail Developer Console (F12 / Ctrl+Shift+I) ----
    let devtools_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    devtools_header.add_css_class("devtools-header");

    let devtools_icon = gtk::Image::from_icon_name("utilities-terminal-symbolic");
    devtools_icon.add_css_class("devtools-brand-icon");

    let devtools_title = gtk::Label::new(None);
    devtools_title.set_markup("<span font_family='ui-monospace' font_weight='800'>rail</span><span foreground='#38bdf8'>.devtools</span>");
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
    devtools_detach_btn.set_tooltip_text(Some("Detach to Separate Window / Dock"));

    let devtools_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    devtools_close_btn.add_css_class("flat-icon");
    devtools_close_btn.set_tooltip_text(Some("Close DevTools (Esc / F12)"));

    devtools_header.append(&devtools_icon);
    devtools_header.append(&devtools_title);
    devtools_header.append(&devtools_badge);
    devtools_header.append(&devtools_url_lbl);
    devtools_header.append(&dt_spacer);
    devtools_header.append(&devtools_detach_btn);
    devtools_header.append(&devtools_close_btn);

    let devtools_content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    devtools_content_box.add_css_class("devtools-content-area");
    devtools_content_box.set_vexpand(true);
    devtools_content_box.set_hexpand(true);

    let devtools_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    devtools_panel.add_css_class("devtools-panel");
    devtools_panel.set_halign(gtk::Align::Fill);
    devtools_panel.set_valign(gtk::Align::End);
    devtools_panel.set_height_request(400);
    devtools_panel.set_visible(false);

    devtools_panel.append(&devtools_header);
    devtools_panel.append(&devtools_content_box);

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&shell));
    overlay.add_overlay(&load_bar);
    overlay.add_overlay(&palette);
    overlay.add_overlay(&ov_hint);
    overlay.add_overlay(&profiles_modal);
    overlay.add_overlay(&data_modal);
    overlay.add_overlay(&map_container);
    overlay.add_overlay(&devtools_panel);

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
        palette_for_new_tab: Cell::new(false),
        palette_suggestions_box: palette_suggestions_box.clone(),
        history: RefCell::new(history_mgr),
        current_suggestions: RefCell::new(Vec::new()),
        selected_suggestion_idx: Cell::new(None),
        is_autofilling: Cell::new(false),
        last_entry_len: Cell::new(0),
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
        profiles_modal_scroll: modal_list_scroll.clone(),
        profiles_list_box: profiles_list_box.clone(),
        new_profile_entry: new_profile_entry.clone(),
        new_profile_error_lbl: new_profile_error_lbl.clone(),
        add_prof_btn: add_prof_btn.clone(),
        new_profile_color_idx,
        color_buttons: RefCell::new(color_btns),
        touchpad_mode: Cell::new(false),
        last_saved_session: RefCell::new(String::new()),
        save_session_gen: Cell::new(0),
        data_modal: data_modal.clone(),
        data_modal_info_lbl: data_info_lbl.clone(),
        data_modal_status_lbl: data_modal_status_lbl.clone(),
        data_export_entry: data_export_entry.clone(),
        data_import_entry: data_import_entry.clone(),
        map_container: map_container.clone(),
        map_drawing_area: map_drawing_area.clone(),
        map_location_btn: map_location_btn.clone(),
        map_state: RefCell::new(MinimapState::default()),
        devtools_open: Cell::new(false),
        devtools_panel: devtools_panel.clone(),
        devtools_content_box: devtools_content_box.clone(),
        devtools_url_lbl: devtools_url_lbl.clone(),
        devtools_window: RefCell::new(None),
    });

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
    data_modal_close_btn.connect_clicked(move |_| a.hide_data_modal());

    let a = app.clone();
    btn_export_do.connect_clicked(move |_| a.execute_export_action());

    let a = app.clone();
    btn_import_do.connect_clicked(move |_| a.execute_import_action());

    let a = app.clone();
    let win_weak = win.downgrade();
    btn_export_browse.connect_clicked(move |_| {
        let Some(win) = win_weak.upgrade() else { return };
        let dialog = gtk::FileDialog::builder()
            .title("Save Rail Backup Archive (.tar.gz)")
            .initial_name("rail_backup.tar.gz")
            .build();
        let a = a.clone();
        dialog.save(Some(&win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    a.data_export_entry.set_text(&path.to_string_lossy());
                }
            }
        });
    });

    let a = app.clone();
    let win_weak = win.downgrade();
    btn_import_browse.connect_clicked(move |_| {
        let Some(win) = win_weak.upgrade() else { return };
        let dialog = gtk::FileDialog::builder()
            .title("Select Rail Backup Archive (.tar.gz)")
            .build();
        let a = a.clone();
        dialog.open(Some(&win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    a.data_import_entry.set_text(&path.to_string_lossy());
                }
            }
        });
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

    // DevTools signal connections
    let a_dt_close = app.clone();
    devtools_close_btn.connect_clicked(move |_| a_dt_close.hide_devtools());

    let a_dt_detach = app.clone();
    devtools_detach_btn.connect_clicked(move |_| a_dt_detach.toggle_devtools_undocked());


    let a = app.clone();
    add.connect_clicked(move |_| a.open_new_tab_palette());
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
        if a.profiles_modal_open() {
            a.hide_profiles_modal();
        }
        if a.data_modal_open() {
            a.hide_data_modal();
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
    let held_shortcut: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));

    let a = app.clone();
    let held = held_shortcut.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let alt = state.contains(M::ALT_MASK);
        let super_key = state.contains(M::SUPER_MASK);

        // Autorepeat loop suppressor: block machine-gun repeat when shortcut is held down
        if ctrl || alt || super_key {
            if held.get() == Some(keycode) {
                return glib::Propagation::Stop;
            }
        }

        let handled = a.on_key(key, keycode, state);
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
        return Err(format!("Storage directory does not exist: {}", dir.display()));
    }
    let parent = dir.parent().unwrap_or(&dir);
    let name = dir.file_name().ok_or_else(|| "Invalid directory name".to_string())?;

    let abs_archive = if archive_path.is_absolute() {
        archive_path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?.join(archive_path)
    };

    let status = std::process::Command::new("tar")
        .arg("-czf")
        .arg(&abs_archive)
        .arg("-C")
        .arg(parent)
        .arg(name)
        .status()
        .map_err(|e| format!("Failed to run tar command: {}", e))?;

    if status.success() {
        println!("Successfully exported all Rail data to: {}", abs_archive.display());
        println!("This file contains all your tabs, workspaces, profiles, logins, and cookies.");
        Ok(())
    } else {
        Err(format!("tar command failed with status: {}", status))
    }
}

fn import_data(archive_path: &Path) -> Result<(), String> {
    let abs_archive = if archive_path.is_absolute() {
        archive_path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?.join(archive_path)
    };

    if !abs_archive.exists() {
        return Err(format!("Archive file does not exist: {}", abs_archive.display()));
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
        .map_err(|e| format!("Failed to run tar command: {}", e))?;

    if status.success() {
        println!("Successfully imported all Rail data into: {}", dest_dir.display());
        println!("You can now launch 'rail' and all tabs, workspaces, logins, and accounts will be active.");
        Ok(())
    } else {
        Err(format!("tar command failed with status: {}", status))
    }
}

fn print_data_info() {
    let dir = rail_data_dir();
    println!("Rail Unified Storage Directory: {}", dir.display());
    println!("--------------------------------------------------");
    if !dir.exists() {
        println!("Status: Directory does not exist yet (clean slate)");
        return;
    }

    let session_file = dir.join("session.txt");
    if session_file.exists() {
        if let Ok(c) = std::fs::read_to_string(&session_file) {
            let tabs = c.lines().filter(|l| l.starts_with("tab\t")).count();
            println!("  Session:       {} saved tabs", tabs);
        }
    } else {
        println!("  Session:       No saved session (clean start on next launch)");
    }

    let profiles_file = dir.join("profiles.txt");
    if profiles_file.exists() {
        if let Ok(c) = std::fs::read_to_string(&profiles_file) {
            let count = c.lines().filter(|l| !l.trim().is_empty()).count();
            println!("  Profiles:      {} profiles configured", count);
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
                    println!("    - Profile '{}': accounts/cookies={}, localStorage/IndexedDB={}", name, has_cookies, has_storage);
                }
            }
        }
    }

    println!();
    println!("To share your exact environment with a friend:");
    println!("  Export:   rail --export rail_backup.tar.gz");
    println!("  Import:   rail --import rail_backup.tar.gz");
    println!("  Or copy:  cp -r {} /path/to/destination", dir.display());
}

fn main() -> glib::ExitCode {
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
        if arg == "--help" || arg == "-h" {
            println!("Usage: rail [OPTIONS] [URL]");
            println!();
            println!("Options:");
            println!("  -d, --data-dir <PATH>      Use custom storage directory for 100% portable environment");
            println!("      --export <FILE.tar.gz> Export all tabs, profiles, logins & cookies into archive");
            println!("      --import <FILE.tar.gz> Import all tabs, profiles, logins & cookies from archive");
            println!("      --data-info            Show current storage directory and environment details");
            println!("  -v, --version              Show version");
            println!("  -h, --help                 Show help message");
            println!();
            println!("Environment Variables:");
            println!("  RAIL_DATA_DIR              Override default storage directory (~/.local/share/rail)");
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
                    eprintln!("Error exporting data: {}", e);
                    return glib::ExitCode::FAILURE;
                }
                return glib::ExitCode::SUCCESS;
            } else {
                eprintln!("Error: --export requires an output filename (e.g. rail --export backup.tar.gz)");
                return glib::ExitCode::FAILURE;
            }
        }
        if arg == "--import" {
            if i + 1 < args.len() {
                let inp = PathBuf::from(&args[i + 1]);
                if let Err(e) = import_data(&inp) {
                    eprintln!("Error importing data: {}", e);
                    return glib::ExitCode::FAILURE;
                }
                return glib::ExitCode::SUCCESS;
            } else {
                eprintln!("Error: --import requires an archive filename (e.g. rail --import backup.tar.gz)");
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
}
