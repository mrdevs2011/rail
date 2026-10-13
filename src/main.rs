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
body{display:grid;place-items:center;background:#16161e;color:#6b7399;font:14px/1.5 system-ui,-apple-system,sans-serif;padding:16px;overflow-y:auto;overflow-x:hidden}
.w{text-align:center;max-width:540px;width:100%;margin:auto;box-sizing:border-box}
h1{font-weight:600;font-size:clamp(30px, 7vw, 56px);letter-spacing:-.03em;margin:0;color:#c0caf5}
h1 b{color:#7aa2f7}
p{margin:4px 0 24px;font-size:clamp(12px, 2.5vw, 14px);color:#7982a9}
.k{display:grid;grid-template-columns:auto 1fr;gap:8px 16px;text-align:left;justify-content:center;align-items:center;font-size:13px}
.k span:nth-child(odd){text-align:right;white-space:nowrap}
.k span:nth-child(even){text-align:left;color:#a9b1d6}
kbd{font:11px ui-monospace,monospace;background:#1f2335;border:1px solid #2f334d;border-bottom-width:2px;
    border-radius:6px;padding:2px 6px;color:#c0caf5;margin-left:3px;white-space:nowrap}
@media (max-width: 440px) {
  body{padding:8px}
  .k{font-size:11px;gap:6px 10px}
  kbd{padding:1px 4px;font-size:10px}
}
</style>
<div class=w><h1>rail<b>.</b></h1><p>a niri-style tiled browser (horizontal strip, vertical stacks & isolated profiles)</p><div class=k>
<span><kbd>Ctrl</kbd><kbd>G</kbd></span><span>go to address</span>
<span><kbd>Ctrl</kbd><kbd>Enter</kbd></span><span>edit current URL</span>
<span><kbd>Ctrl</kbd><kbd>T</kbd></span><span>new column tab</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>T</kbd> / <kbd>Ctrl</kbd><kbd>V</kbd></span><span>split vertically (new window down)</span>
<span><kbd>Ctrl</kbd><kbd>P</kbd> / <kbd>Shift</kbd><kbd>P</kbd></span><span>manage profiles</span>
<span><kbd>Ctrl</kbd><kbd>H</kbd> <kbd>L</kbd></span><span>focus left / right column</span>
<span><kbd>Ctrl</kbd><kbd>J</kbd> <kbd>K</kbd></span><span>focus down / up window</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>H</kbd> <kbd>L</kbd></span><span>move column left / right</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>J</kbd> <kbd>K</kbd></span><span>move window down / up</span>
<span><kbd>Ctrl</kbd><kbd>F</kbd></span><span>full / half width</span>
<span><kbd>Ctrl</kbd><kbd>O</kbd></span><span>overview</span>
<span><kbd>Ctrl</kbd><kbd>B</kbd></span><span>sidebar</span>
</div></div>"#;

const CSS: &str = "
window { background: #0f1016; border-radius: 0; box-shadow: none; }
label { color: #a9b1d6; }

/* ---- columns & tabs ---- */
.rail-col { background: transparent; }
.tab { border-radius: 12px; border: 2px solid #1a1b26; }
.tab.focused { border-color: #7aa2f7; }

/* ---- sidebar ---- */
.brand { font-weight: 700; font-size: 17px; letter-spacing: -0.02em; color: #c0caf5; }
.brand-dot { color: #7aa2f7; }
button.flat-icon { background: transparent; border: none; box-shadow: none; min-width: 28px; min-height: 28px;
                   padding: 0; border-radius: 8px; color: #7982a9; }
button.flat-icon:hover { background: #1f2335; color: #c0caf5; }
button.pill { background: #1a1b26; border: 1px solid #272a40; border-radius: 10px; padding: 8px 12px;
              box-shadow: none; color: #a9b1d6; }
button.pill:hover { background: #1f2335; border-color: #3b4261; }
button.pill label { color: #c0caf5; }
.tablist, .tablist row { background: transparent; }
.tablist row { padding: 0; margin: 1px 0; border-radius: 10px; }
.tablist row:hover { background: #151620; }
.tablist row:selected { background: #24283b; }
.tabrow { padding: 8px 8px 8px 10px; }
.tabrow label { color: #8089b3; font-size: 13px; }
row:selected .tabrow label { color: #c0caf5; }
.tabrow image { color: #7982a9; }
.tab-close { opacity: 0; min-width: 22px; min-height: 22px; padding: 0; border-radius: 6px;
             background: transparent; border: none; box-shadow: none; color: #7982a9; transition: opacity 120ms; }
.tab-close:hover { background: #3b4261; color: #c0caf5; }
row:hover .tab-close, row:selected .tab-close { opacity: 1; }
.hint { color: #3f4561; font-size: 11px; }

.fav, .tabrow label.fav, row:selected .tabrow label.fav { border-radius: 5px; font-size: 10px; font-weight: 700; color: #0f1016; }
.fav-0 { background: #7aa2f7; } .fav-1 { background: #bb9af7; } .fav-2 { background: #9ece6a; }
.fav-3 { background: #e0af68; } .fav-4 { background: #f7768e; } .fav-5 { background: #7dcfff; }
.fav-6 { background: #ff9e64; } .fav-7 { background: #73daca; }

/* ---- load progress ---- */
progressbar.load-bar { min-height: 3px; margin: 0 8px; }
progressbar.load-bar trough { min-height: 3px; background: transparent; border: none; }
progressbar.load-bar progress { min-height: 3px; background: #7aa2f7; border-radius: 3px;
                                box-shadow: 0 0 10px rgba(122,162,247,0.7); }

/* ---- address palette ---- */
.palette { background: #1a1b26; border-radius: 16px; border: 1px solid #2f334d; padding: 14px 18px 12px;
           box-shadow: 0 24px 64px rgba(0,0,0,0.55); }
.palette image { color: #7982a9; }
entry.palette-entry, entry.palette-entry:focus-within { background: none; border: none; box-shadow: none; outline: none;
    padding: 0; min-height: 0; font-size: 16px; color: #c0caf5; }
.palette-hint { color: #565f89; font-size: 11px; margin-top: 6px; }

/* ---- overview (the live strip, zoomed out) ---- */
.zoomed .tab { opacity: 0.88; border-radius: 28px; }
.zoomed .tab.focused { opacity: 1; outline: 7px solid #7aa2f7; outline-offset: -5px; }
.ov-hint { background: #1a1b26; color: #8089b3; border: 1px solid #2f334d; border-radius: 10px;
           padding: 8px 16px; font-size: 12px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); }

/* ---- profile colors (Tokyo Night) ---- */
.prof-bg-0 { background-color: #7aa2f7; color: #0f1016; }
.prof-bg-1 { background-color: #bb9af7; color: #0f1016; }
.prof-bg-2 { background-color: #9ece6a; color: #0f1016; }
.prof-bg-3 { background-color: #e0af68; color: #0f1016; }
.prof-bg-4 { background-color: #f7768e; color: #0f1016; }
.prof-bg-5 { background-color: #7dcfff; color: #0f1016; }
.prof-bg-6 { background-color: #ff9e64; color: #0f1016; }
.prof-bg-7 { background-color: #73daca; color: #0f1016; }

/* ---- profile circle button in URL palette ---- */
button.profile-circle-btn {
    min-width: 32px;
    min-height: 32px;
    padding: 0;
    border-radius: 9999px;
    border: 2px solid #2f334d;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    transition: transform 120ms, border-color 120ms;
}
button.profile-circle-btn:hover {
    transform: scale(1.1);
    border-color: #c0caf5;
}
.profile-circle-label {
    font-weight: 800;
    font-size: 13px;
    color: #0f1016;
}

/* ---- profile popover dropdown ---- */
.profile-popover {
    background: #1a1b26;
    border: 1px solid #2f334d;
    border-radius: 14px;
    padding: 6px;
    box-shadow: 0 16px 48px rgba(0,0,0,0.65);
}
.profile-popover-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #565f89;
    padding: 4px 8px 2px;
}
button.profile-item-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    border-radius: 10px;
    padding: 6px 10px;
    transition: background 100ms;
}
button.profile-item-btn:hover {
    background: #24283b;
}
button.profile-item-btn.selected {
    background: #1f2335;
}
.profile-item-dot {
    min-width: 24px;
    max-width: 24px;
    min-height: 24px;
    max-height: 24px;
    border-radius: 9999px;
    font-size: 11px;
    font-weight: 800;
    color: #16161e;
    padding: 0;
}
.profile-item-name {
    font-size: 13px;
    font-weight: 600;
    color: #c0caf5;
}
.profile-item-check {
    font-size: 13px;
    font-weight: 700;
    color: #7aa2f7;
}

/* ---- profiles manager modal (Ctrl+P / Ctrl+Shift+P) ---- */
.profiles-modal {
    background: #16161e;
    border: 1px solid #2f334d;
    border-radius: 20px;
    padding: 24px 28px;
    box-shadow: 0 32px 80px rgba(0,0,0,0.85);
    min-width: 440px;
    max-width: 480px;
    margin-left: 16px;
    margin-right: 16px;
}
.profiles-modal-title {
    font-size: 19px;
    font-weight: 700;
    color: #c0caf5;
    letter-spacing: -0.02em;
}
.profiles-modal-subtitle {
    font-size: 12px;
    color: #565f89;
    margin-top: 2px;
}
.profile-card {
    background: #1a1b26;
    border: 1px solid #24283b;
    border-radius: 12px;
    padding: 10px 14px;
    margin-bottom: 8px;
    transition: all 120ms ease;
}
.profile-card:hover {
    border-color: #3b4261;
}
.profile-card.active-profile {
    border-color: #7aa2f7;
    background: #1d2033;
    box-shadow: 0 0 16px rgba(122, 162, 247, 0.12);
}
.profile-avatar-circle {
    min-width: 32px;
    max-width: 32px;
    min-height: 32px;
    max-height: 32px;
    border-radius: 9999px;
    font-size: 13px;
    font-weight: 800;
    color: #16161e;
    margin: 0;
    padding: 0;
}
.profile-card-name {
    font-size: 14px;
    font-weight: 600;
    color: #c0caf5;
}
.profile-card-sub {
    font-size: 11px;
    color: #565f89;
    font-family: ui-monospace, monospace;
}
.profile-badge-active {
    font-size: 11px;
    font-weight: 600;
    border-radius: 9999px;
    padding: 3px 10px;
    background: rgba(122, 162, 247, 0.14);
    color: #7aa2f7;
    border: 1px solid rgba(122, 162, 247, 0.35);
}
button.btn-make-active {
    background: #24283b;
    color: #a9b1d6;
    border: 1px solid #3b4261;
    border-radius: 8px;
    padding: 4px 12px;
    font-size: 12px;
    font-weight: 600;
    transition: all 120ms;
}
button.btn-make-active:hover {
    background: #7aa2f7;
    color: #16161e;
    border-color: #7aa2f7;
}
button.btn-del-profile {
    background: transparent;
    border: none;
    border-radius: 8px;
    padding: 4px 6px;
    color: #565f89;
    transition: all 120ms;
}
button.btn-del-profile:hover {
    background: rgba(247, 118, 142, 0.15);
    color: #f7768e;
}
.profile-section-label {
    font-size: 11px;
    font-weight: 700;
    color: #565f89;
    letter-spacing: 0.05em;
    margin-top: 4px;
}
.profile-entry-box {
    background: #13141c;
    border: 1px solid #2f334d;
    border-radius: 10px;
    padding: 8px 12px;
    color: #c0caf5;
    font-size: 13px;
    transition: all 120ms;
}
.profile-entry-box:focus {
    border-color: #7aa2f7;
    box-shadow: 0 0 0 1px #7aa2f7;
}
.profile-entry-box.error {
    border-color: #f7768e;
    box-shadow: 0 0 0 1px #f7768e;
}
.profile-error-label {
    font-size: 11px;
    font-weight: 600;
    color: #f7768e;
    margin-top: 2px;
}
button.btn-create-profile {
    background: #7aa2f7;
    color: #16161e;
    font-weight: 700;
    font-size: 12px;
    border: none;
    border-radius: 8px;
    padding: 6px 16px;
    transition: all 120ms;
}
button.btn-create-profile:hover {
    background: #89b4fa;
}
.profile-badge-mini {
    font-size: 9px;
    font-weight: 700;
    min-width: 15px;
    min-height: 15px;
    border-radius: 9999px;
    margin-right: 6px;
    color: #0f1016;
}
button.color-choice {
    min-width: 24px;
    max-width: 24px;
    min-height: 24px;
    max-height: 24px;
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
    box-shadow: 0 0 0 2px #16161e;
    transform: scale(1.15);
}
";

/// How far the strip zooms out in overview (niri uses a similar factor).
const OV_ZOOM: f64 = 0.4;

struct Tab {
    view: WebView,
    profile_id: String,
}

struct Column {
    container: gtk::Box,
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
    /// Tab coordinates that were focused when overview opened; Esc returns here.
    ov_return: Cell<(usize, usize)>,
    /// Scroll position before the overview, restored when zooming back in.
    ov_saved: Cell<f64>,
    target_zoom: Cell<f64>,
    cols: RefCell<Vec<Column>>,
    focus_col: Cell<usize>,
    focus_row: Cell<usize>,
    target: Cell<f64>,
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
    new_profile_color_idx: Cell<usize>,
    color_buttons: RefCell<Vec<gtk::Button>>,
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

impl App {
    fn focused_view(&self) -> Option<WebView> {
        let cols = self.cols.borrow();
        cols.get(self.focus_col.get())
            .and_then(|c| c.tabs.get(self.focus_row.get()))
            .map(|t| t.view.clone())
    }

    fn palette_open(&self) -> bool {
        gtk::prelude::WidgetExt::is_visible(&self.palette)
    }

    fn overview_open(&self) -> bool {
        self.ov.get()
    }

    // ---- strip scrolling -------------------------------------------------

    /// Normal mode: scroll the minimum needed to show the focused column.
    /// Overview: center the focused column in the zoomed-out strip.
    fn reveal_focus(self: &Rc<Self>) {
        let vw = self.strip.width() as f64;
        if vw <= 0.0 {
            return;
        }
        let visible = vw / self.target_zoom.get(); // strip width that fits on screen
        let g = gap(self.target_zoom.get());
        let cols = self.cols.borrow();
        let mut x = 0.0;
        let mut span = None;
        for (i, col) in cols.iter().enumerate() {
            let w = col_width(col.container.has_css_class("full"), vw);
            if i == self.focus_col.get() {
                span = Some((x, x + w));
            }
            x += w + g;
        }
        let total = x - g;
        let Some((l, r)) = span else { return };
        let max = (total - visible).max(0.0);
        let t = if self.ov.get() {
            if total <= visible {
                -(visible - total) / 2.0 // everything fits: center it all
            } else {
                ((l + r) / 2.0 - visible / 2.0).clamp(0.0, max)
            }
        } else {
            let mut t = self.target.get().clamp(0.0, max);
            if l < t {
                t = l;
            } else if r > t + visible {
                t = r - visible;
            }
            t.clamp(0.0, max)
        };
        self.target.set(t);
        drop(cols);
        self.animate();
    }

    /// Ease both the scroll offset and the zoom toward their targets.
    fn animate(self: &Rc<Self>) {
        if self.animating.get() {
            return;
        }
        let near = |a: f64, b: f64, eps: f64| (a - b).abs() < eps;
        if near(self.target.get(), self.layout.offset(), 0.5) && near(self.target_zoom.get(), self.layout.zoom(), 0.002) {
            self.layout.set_offset(self.target.get());
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
            let w = (strip.width() as f64).max(1.0);
            let (oc, ot) = (app.layout.offset(), app.target.get());
            let (zc, zt) = (app.layout.zoom(), app.target_zoom.get());
            let (cc, ct) = (oc + w / (2.0 * zc), ot + w / (2.0 * zt));
            let mut cn = cc + (ct - cc) * k;
            let mut znext = zc + (zt - zc) * k;
            if near(ct, cn, 0.5) {
                cn = ct;
            }
            if near(zt, znext, 0.002) {
                znext = zt;
            }
            let next = if cn == ct && znext == zt { ot } else { cn - w / (2.0 * znext) };
            let tgt = ot;
            app.layout.set_offset(next);
            app.layout.set_zoom(znext);
            strip.queue_allocate();
            if next == tgt && znext == zt {
                app.animating.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    // ---- tabs & columns --------------------------------------------------

    fn focused_tab_profile(&self) -> Option<String> {
        let cur_c = self.focus_col.get();
        let cur_r = self.focus_row.get();
        let cols = self.cols.borrow();
        cols.get(cur_c).and_then(|c| c.tabs.get(cur_r)).map(|t| t.profile_id.clone())
    }

    fn find_view_profile(&self, view: &WebView) -> Option<String> {
        let cols = self.cols.borrow();
        for col in cols.iter() {
            for tab in col.tabs.iter() {
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
        self.add_view(view, prof_id.clone(), true); // new column
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
        self.add_view(view, prof_id.clone(), false); // in current column below
        self.select_palette_profile(&prof_id);
        self.show_palette();
    }

    fn setup_view_signals(self: &Rc<Self>, view: &WebView) {
        let app = self.clone();
        view.connect_title_notify(move |_| app.refresh_sidebar());
        let app = self.clone();
        view.connect_uri_notify(move |_| app.refresh_sidebar());
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
            app.add_view(nv.clone(), parent_prof, true);
            Some(nv.upcast())
        });
        let app = self.clone();
        view.connect_close(move |v| app.close_view(v));
    }

    fn add_view(self: &Rc<Self>, view: WebView, profile_id: String, new_col: bool) {
        view.add_css_class("tab");
        view.set_overflow(gtk::Overflow::Hidden);
        view.set_can_target(!self.ov.get());
        view.set_vexpand(true);
        view.set_hexpand(true);

        let (target_c, target_r) = {
            let mut cols = self.cols.borrow_mut();
            if cols.is_empty() || new_col {
                let box_widget = gtk::Box::new(gtk::Orientation::Vertical, 6);
                box_widget.add_css_class("rail-col");
                box_widget.set_vexpand(true);
                box_widget.set_hexpand(true);
                box_widget.set_homogeneous(true);
                box_widget.append(&view);

                let c_idx = if cols.is_empty() {
                    self.strip.prepend(&box_widget);
                    0
                } else {
                    let cur = self.focus_col.get();
                    let prev = &cols[cur].container;
                    self.strip.insert_child_after(&box_widget, Some(prev));
                    cur + 1
                };

                cols.insert(c_idx, Column {
                    container: box_widget,
                    tabs: vec![Tab { view: view.clone(), profile_id }],
                });
                (c_idx, 0)
            } else {
                let cur_c = self.focus_col.get();
                let cur_r = self.focus_row.get();
                let col = &mut cols[cur_c];
                let next_r = (cur_r + 1).min(col.tabs.len());
                if next_r == 0 {
                    col.container.prepend(&view);
                } else {
                    let prev_tab = &col.tabs[next_r - 1].view;
                    col.container.insert_child_after(&view, Some(prev_tab));
                }
                col.tabs.insert(next_r, Tab { view: view.clone(), profile_id });
                (cur_c, next_r)
            }
        };

        self.setup_view_signals(&view);
        self.focus_coords(target_c, target_r);
    }

    fn find_view_coords(&self, view: &WebView) -> Option<(usize, usize)> {
        let cols = self.cols.borrow();
        for (c_idx, col) in cols.iter().enumerate() {
            for (r_idx, tab) in col.tabs.iter().enumerate() {
                if &tab.view == view {
                    return Some((c_idx, r_idx));
                }
            }
        }
        None
    }

    fn close_view(self: &Rc<Self>, view: &WebView) {
        let Some((c_idx, r_idx)) = self.find_view_coords(view) else { return };
        if self.fresh.borrow().as_ref() == Some(view) {
            *self.fresh.borrow_mut() = None;
        }

        let mut cols = self.cols.borrow_mut();
        cols[c_idx].container.remove(view);
        cols[c_idx].tabs.remove(r_idx);

        let col_empty = cols[c_idx].tabs.is_empty();
        if col_empty {
            self.strip.remove(&cols[c_idx].container);
            cols.remove(c_idx);
        }

        let total_cols = cols.len();
        drop(cols);

        if total_cols == 0 {
            self.new_tab();
        } else {
            let next_c = c_idx.min(total_cols - 1);
            let cols = self.cols.borrow();
            let next_r = if col_empty {
                0
            } else {
                r_idx.saturating_sub(1).min(cols[next_c].tabs.len().saturating_sub(1))
            };
            drop(cols);
            self.focus_coords(next_c, next_r);
        }
    }

    fn focus_coords(self: &Rc<Self>, c_idx: usize, r_idx: usize) {
        let cols = self.cols.borrow();
        if c_idx >= cols.len() {
            return;
        }
        let r_clamped = r_idx.min(cols[c_idx].tabs.len().saturating_sub(1));
        self.focus_col.set(c_idx);
        self.focus_row.set(r_clamped);

        for (ci, col) in cols.iter().enumerate() {
            for (ri, tab) in col.tabs.iter().enumerate() {
                if ci == c_idx && ri == r_clamped {
                    tab.view.add_css_class("focused");
                } else {
                    tab.view.remove_css_class("focused");
                }
            }
        }

        if let Some(tab) = cols[c_idx].tabs.get(r_clamped) {
            if !tab.view.is_loading() {
                self.load_bar.set_visible(false);
            }
        }
        drop(cols);

        self.update_load_bar();
        self.reveal_focus();
        self.refresh_sidebar();

        if !self.palette_open() && !self.ov.get() {
            if let Some(v) = self.focused_view() {
                v.grab_focus();
            }
        }
    }

    fn focus_column_left(self: &Rc<Self>) {
        let cur_c = self.focus_col.get();
        if cur_c > 0 {
            self.focus_coords(cur_c - 1, self.focus_row.get());
        }
    }

    fn focus_column_right(self: &Rc<Self>) {
        let cur_c = self.focus_col.get();
        let cols = self.cols.borrow();
        if cur_c + 1 < cols.len() {
            drop(cols);
            self.focus_coords(cur_c + 1, self.focus_row.get());
        }
    }

    fn focus_window_up(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        if cur_r > 0 {
            self.focus_coords(self.focus_col.get(), cur_r - 1);
        }
    }

    fn focus_window_down(self: &Rc<Self>) {
        let cur_c = self.focus_col.get();
        let cur_r = self.focus_row.get();
        let cols = self.cols.borrow();
        if let Some(col) = cols.get(cur_c) {
            if cur_r + 1 < col.tabs.len() {
                drop(cols);
                self.focus_coords(cur_c, cur_r + 1);
            }
        }
    }

    fn move_column(self: &Rc<Self>, dir: isize) {
        let cur = self.focus_col.get();
        let mut cols = self.cols.borrow_mut();
        let Some(target) = cur.checked_add_signed(dir).filter(|&t| t < cols.len()) else { return };
        cols.swap(cur, target);
        let mut prev: Option<&gtk::Widget> = None;
        for c in cols.iter() {
            self.strip.reorder_child_after(&c.container, prev);
            prev = Some(c.container.as_ref());
        }
        drop(cols);
        self.focus_col.set(target);
        self.strip.queue_allocate();
        self.reveal_focus();
        self.refresh_sidebar();
    }

    fn move_window(self: &Rc<Self>, dir: isize) {
        let cur_c = self.focus_col.get();
        let cur_r = self.focus_row.get();
        let mut cols = self.cols.borrow_mut();
        let Some(col) = cols.get_mut(cur_c) else { return };
        let Some(target_r) = cur_r.checked_add_signed(dir).filter(|&t| t < col.tabs.len()) else { return };
        col.tabs.swap(cur_r, target_r);
        let mut prev: Option<&gtk::Widget> = None;
        for t in col.tabs.iter() {
            col.container.reorder_child_after(&t.view, prev);
            prev = Some(t.view.as_ref());
        }
        drop(cols);
        self.focus_row.set(target_r);
        self.focus_coords(cur_c, target_r);
    }

    fn toggle_full(self: &Rc<Self>) {
        let cur_c = self.focus_col.get();
        let cols = self.cols.borrow();
        let Some(col) = cols.get(cur_c) else { return };
        if col.container.has_css_class("full") {
            col.container.remove_css_class("full");
        } else {
            col.container.add_css_class("full");
        }
        drop(cols);
        self.strip.queue_allocate();
        self.reveal_focus();
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
        badge.upcast()
    }

    fn refresh_sidebar(self: &Rc<Self>) {
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        let cols = self.cols.borrow();
        let cur_c = self.focus_col.get();
        let cur_r = self.focus_row.get();
        let mut row_idx = 0;
        let mut selected_row_idx = None;

        for (ci, col) in cols.iter().enumerate() {
            for (ri, t) in col.tabs.iter().enumerate() {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                row.add_css_class("tabrow");
                row.append(&Self::tab_icon(t, 16));

                let mgr = self.profiles.borrow();
                if mgr.profiles().len() > 1 {
                    if let Some(p) = mgr.get_profile(&t.profile_id) {
                        let p_dot = gtk::Label::new(Some(&p.initial_letter()));
                        p_dot.add_css_class("profile-badge-mini");
                        p_dot.add_css_class(&format!("prof-bg-{}", p.color_idx));
                        p_dot.set_valign(gtk::Align::Center);
                        p_dot.set_tooltip_text(Some(&format!("Profile: {}", p.name)));
                        row.append(&p_dot);
                    }
                }
                drop(mgr);

                let title_prefix = if col.tabs.len() > 1 {
                    format!("[{}:{}] ", ci + 1, ri + 1)
                } else {
                    format!("[{}] ", ci + 1)
                };
                let full_title = format!("{}{}", title_prefix, Self::tab_label(t));
                let label = gtk::Label::new(Some(&full_title));
                label.set_xalign(0.0);
                label.set_hexpand(true);
                label.set_width_chars(1);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                row.append(&label);

                let close = gtk::Button::from_icon_name("window-close-symbolic");
                close.add_css_class("tab-close");
                close.set_valign(gtk::Align::Center);
                let (app, v) = (self.clone(), t.view.clone());
                close.connect_clicked(move |_| app.close_view(&v));
                row.append(&close);

                self.list.append(&row);
                if ci == cur_c && ri == cur_r {
                    selected_row_idx = Some(row_idx);
                }
                row_idx += 1;
            }
        }
        drop(cols);

        if let Some(s_idx) = selected_row_idx {
            if let Some(row) = self.list.row_at_index(s_idx as i32) {
                self.list.select_row(Some(&row));
            }
        }

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

        let mgr = self.profiles.borrow();
        let selected_id = self.selected_palette_profile.borrow().clone();

        for p in mgr.profiles() {
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
            let total_tabs: usize = self.cols.borrow().iter().map(|c| c.tabs.len()).sum();
            if v == f && total_tabs > 1 {
                self.close_view(&v);
            }
        }
    }

    fn navigate(self: &Rc<Self>, text: &str) {
        if !text.trim().is_empty() {
            let target_profile_id = self.selected_palette_profile.borrow().clone();
            let cur_c = self.focus_col.get();
            let cur_r = self.focus_row.get();
            let uri = to_uri(text);

            let needs_view_recreation = {
                let cols = self.cols.borrow();
                cols.get(cur_c)
                    .and_then(|c| c.tabs.get(cur_r))
                    .map(|t| t.profile_id != target_profile_id)
                    .unwrap_or(false)
            };

            if needs_view_recreation {
                let session = self.profiles.borrow_mut().get_or_create_session(&target_profile_id);
                let new_view = WebView::builder().network_session(&session).build();
                new_view.add_css_class("tab");
                new_view.set_overflow(gtk::Overflow::Hidden);
                new_view.set_can_target(!self.ov.get());
                new_view.set_vexpand(true);
                new_view.set_hexpand(true);
                self.setup_view_signals(&new_view);

                let mut cols = self.cols.borrow_mut();
                if let Some(col) = cols.get_mut(cur_c) {
                    if let Some(tab) = col.tabs.get_mut(cur_r) {
                        col.container.insert_child_after(&new_view, Some(&tab.view));
                        col.container.remove(&tab.view);
                        tab.view = new_view.clone();
                        tab.profile_id = target_profile_id;
                    }
                }
                drop(cols);

                self.pill_label.set_text(&host_of(&uri).unwrap_or(uri.clone()));
                new_view.load_uri(&uri);
                *self.fresh.borrow_mut() = None;
                self.refresh_sidebar();
                self.focus_coords(cur_c, cur_r);
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
        self.new_profile_error_lbl.set_visible(false);
        self.new_profile_entry.remove_css_class("error");
        self.select_new_profile_color(0);
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

        let mgr = self.profiles.borrow();
        let profiles = mgr.profiles().to_vec();
        let active_id = mgr.active_id().to_string();
        let can_delete = profiles.len() > 1;

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
                    app.rebuild_profiles_modal_list();
                    app.rebuild_profile_popover();
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
                    app.rebuild_profiles_modal_list();
                    app.rebuild_profile_popover();
                    app.refresh_sidebar();
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

    fn create_new_profile_action(self: &Rc<Self>) {
        let name = self.new_profile_entry.text().to_string();
        let clean = name.trim();
        if clean.is_empty() {
            self.new_profile_error_lbl.set_text("⚠ Profile name cannot be empty");
            self.new_profile_error_lbl.set_visible(true);
            self.new_profile_entry.add_css_class("error");
            return;
        }

        let color_idx = self.new_profile_color_idx.get();
        match self.profiles.borrow_mut().create_profile(clean, color_idx) {
            Ok(_) => {
                self.new_profile_error_lbl.set_visible(false);
                self.new_profile_entry.remove_css_class("error");
                self.new_profile_entry.set_text("");
                self.rebuild_profiles_modal_list();
                self.rebuild_profile_popover();
                self.refresh_sidebar();
            }
            Err(err) => {
                self.new_profile_error_lbl.set_text(&format!("⚠ {}", err));
                self.new_profile_error_lbl.set_visible(true);
                self.new_profile_entry.add_css_class("error");
            }
        }
    }

    // ---- overview (niri-style: zoomed out) --------------------------------

    fn toggle_overview(self: &Rc<Self>) {
        if self.ov.get() {
            self.exit_overview();
        } else {
            self.enter_overview();
        }
    }

    fn enter_overview(self: &Rc<Self>) {
        if self.ov.get() || self.cols.borrow().is_empty() {
            return;
        }
        self.palette.set_visible(false);
        self.ov.set(true);
        self.ov_return.set((self.focus_col.get(), self.focus_row.get()));
        self.ov_saved.set(self.target.get());
        self.strip.add_css_class("zoomed");
        self.ov_hint.set_visible(true);
        for col in self.cols.borrow().iter() {
            for t in col.tabs.iter() {
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
        for col in self.cols.borrow().iter() {
            for t in col.tabs.iter() {
                t.view.set_can_target(true);
            }
        }
        self.target_zoom.set(1.0);
        self.target.set(self.ov_saved.get());
        self.reveal_focus();
        if let Some(v) = self.focused_view() {
            v.grab_focus();
        }
    }

    fn cancel_overview(self: &Rc<Self>) {
        let (c, r) = self.ov_return.get();
        self.focus_coords(c, r);
        self.exit_overview();
    }

    fn ov_click(self: &Rc<Self>, x: f64) {
        let vw = self.strip.width() as f64;
        let (z, off) = (self.layout.zoom(), self.layout.offset());
        let g = gap(z);
        let mut hit = None;
        let mut cx = 0.0;
        let cols = self.cols.borrow();
        for (i, col) in cols.iter().enumerate() {
            let w = col_width(col.container.has_css_class("full"), vw);
            let left = (cx - off) * z;
            if x >= left && x <= left + w * z {
                hit = Some(i);
                break;
            }
            cx += w + g;
        }
        drop(cols);
        if let Some(i) = hit {
            self.focus_coords(i, 0);
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
                    self.focus_window_down();
                    return true;
                }
                gdk::Key::k | gdk::Key::Up => {
                    self.focus_window_up();
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

    fn on_key(self: &Rc<Self>, key: gdk::Key, state: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
        let m = state & gtk::accelerator_get_default_mod_mask();
        let k = key.to_lower();

        if k == gdk::Key::Escape {
            if self.profiles_modal_open() {
                self.hide_profiles_modal();
                return true;
            }
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
        let global = m == M::CONTROL_MASK && matches!(k, gdk::Key::o | gdk::Key::t | gdk::Key::q | gdk::Key::p);
        if self.overview_open() && !global && self.ov_key(k, m) {
            return true;
        }
        if m.is_empty() && k == gdk::Key::F11 {
            self.win.set_fullscreened(!self.win.is_fullscreen());
            return true;
        }
        if m == M::ALT_MASK {
            let v = self.focused_view();
            match k {
                gdk::Key::Left => v.map(|v| v.go_back()),
                gdk::Key::Right => v.map(|v| v.go_forward()),
                _ => return false,
            };
            return true;
        }

        if m == M::CONTROL_MASK {
            match k {
                gdk::Key::p => self.toggle_profiles_modal(),
                gdk::Key::t => self.new_tab(),
                gdk::Key::v => self.split_vertical(),
                gdk::Key::f => self.toggle_full(),
                // Horizontal navigation (columns)
                gdk::Key::h | gdk::Key::Left => self.focus_column_left(),
                gdk::Key::l | gdk::Key::Right => self.focus_column_right(),
                // Vertical navigation (windows in column)
                gdk::Key::j | gdk::Key::Down => self.focus_window_down(),
                gdk::Key::k | gdk::Key::Up => self.focus_window_up(),
                gdk::Key::w => {
                    if let Some(v) = self.focused_view() {
                        self.close_view(&v);
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
                _ => return false,
            }
            return true;
        }
        if m == M::CONTROL_MASK | M::SHIFT_MASK {
            match k {
                gdk::Key::p | gdk::Key::P => self.toggle_profiles_modal(),
                gdk::Key::t => self.split_vertical(),
                gdk::Key::h | gdk::Key::Left => self.move_column(-1),
                gdk::Key::l | gdk::Key::Right => self.move_column(1),
                gdk::Key::j | gdk::Key::Down => self.move_window(1),
                gdk::Key::k | gdk::Key::Up => self.move_window(-1),
                _ => return false,
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
    brand.set_markup("<span weight='bold' size='large' foreground='#c0caf5'>rail</span><span weight='bold' size='large' foreground='#7aa2f7'>.</span>");
    brand.set_hexpand(true);
    brand.set_xalign(0.0);
    let add = gtk::Button::from_icon_name("list-add-symbolic");
    add.add_css_class("flat-icon");
    add.set_tooltip_text(Some("New column tab  (Ctrl+T)"));
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    head.set_margin_start(8);
    head.append(&brand);
    head.append(&add);

    let pill_label = gtk::Label::new(Some("Search or enter address"));
    pill_label.set_xalign(0.0);
    pill_label.set_hexpand(true);
    pill_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    pill_label.set_width_chars(1);
    let pill_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    pill_inner.append(&gtk::Image::from_icon_name("system-search-symbolic"));
    pill_inner.append(&pill_label);
    let pill = gtk::Button::new();
    pill.add_css_class("pill");
    pill.set_child(Some(&pill_inner));
    pill.set_tooltip_text(Some("Go to address  (Ctrl+G)"));

    let list = gtk::ListBox::new();
    list.add_css_class("tablist");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_vexpand(true);
    list_scroll.set_child(Some(&list));

    let hint = gtk::Label::new(Some("ctrl+h/l cols  ·  ctrl+j/k rows  ·  ctrl+v split"));
    hint.add_css_class("hint");
    hint.set_xalign(0.0);
    hint.set_margin_start(8);

    let side = gtk::Box::new(gtk::Orientation::Vertical, 10);
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
        ov_saved: Cell::new(0.0),
        target_zoom: Cell::new(1.0),
        cols: RefCell::new(Vec::new()),
        focus_col: Cell::new(0),
        focus_row: Cell::new(0),
        target: Cell::new(0.0),
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
        new_profile_color_idx,
        color_buttons: RefCell::new(color_btns),
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
    new_profile_entry.connect_changed(move |entry| {
        let text = entry.text().to_string();
        let clean = text.trim();
        if clean.is_empty() {
            a.new_profile_error_lbl.set_visible(false);
            a.new_profile_entry.remove_css_class("error");
        } else if a.profiles.borrow().profile_exists(clean) {
            a.new_profile_error_lbl.set_text("⚠ Profile already exists");
            a.new_profile_error_lbl.set_visible(true);
            a.new_profile_entry.add_css_class("error");
        } else {
            a.new_profile_error_lbl.set_visible(false);
            a.new_profile_entry.remove_css_class("error");
        }
    });

    let a = app.clone();
    layout.connect_resized(move || {
        a.reveal_focus();
        if a.palette_open() {
            a.update_palette_size();
        }
    });

    let a = app.clone();
    list.connect_row_activated(move |_, row| {
        let idx = row.index() as usize;
        let mut counted = 0;
        let cols = a.cols.borrow();
        for (ci, col) in cols.iter().enumerate() {
            if idx < counted + col.tabs.len() {
                let ri = idx - counted;
                drop(cols);
                a.focus_coords(ci, ri);
                return;
            }
            counted += col.tabs.len();
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
    pick.connect_released(move |_, _, x, _| {
        if a.ov.get() {
            a.ov_click(x);
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

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let a = app.clone();
    keys.connect_key_pressed(move |_, key, _, state| {
        if a.on_key(key, state) { glib::Propagation::Stop } else { glib::Propagation::Proceed }
    });
    win.add_controller(keys);

    app.new_tab();
    if let Some(url) = std::env::args().nth(1).filter(|u| !u.starts_with('-')) {
        app.navigate(&url);
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
