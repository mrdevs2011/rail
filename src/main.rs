mod strip;

use gtk::glib;
use gtk::{gdk, gio};
use gtk4 as gtk;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use strip::{SIDE_W, ShellLayout, StripLayout, col_width, gap};
use webkit6::prelude::*;
use webkit6::WebView;

const START_PAGE: &str = r#"<!doctype html><meta charset=utf-8><meta name=color-scheme content=dark>
<style>
html,body{height:100%;margin:0}
body{display:grid;place-items:center;background:#16161e;color:#6b7399;font:15px/1.5 system-ui,sans-serif}
.w{text-align:center}
h1{font-weight:600;font-size:60px;letter-spacing:-.03em;margin:0;color:#c0caf5}
h1 b{color:#7aa2f7}
p{margin:4px 0 40px}
.k{display:grid;grid-template-columns:auto auto;gap:12px 32px;text-align:left;justify-content:center}
.k span:nth-child(odd){text-align:right}
kbd{font:12px ui-monospace,monospace;background:#1f2335;border:1px solid #2f334d;border-bottom-width:2px;
    border-radius:6px;padding:2px 7px;color:#a9b1d6;margin-left:3px}
</style>
<div class=w><h1>rail<b>.</b></h1><p>a niri-style tiled browser (horizontal strip & vertical stacks)</p><div class=k>
<span><kbd>Ctrl</kbd><kbd>G</kbd></span><span>go to address</span>
<span><kbd>Ctrl</kbd><kbd>Enter</kbd></span><span>edit current URL</span>
<span><kbd>Ctrl</kbd><kbd>T</kbd></span><span>new column tab</span>
<span><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>T</kbd> / <kbd>Ctrl</kbd><kbd>V</kbd></span><span>split vertically (new window down)</span>
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
.palette { background: #1a1b26; border-radius: 16px; border: 1px solid #2f334d; padding: 16px 20px 12px;
           box-shadow: 0 24px 64px rgba(0,0,0,0.55); min-width: 620px; }
.palette image { color: #7982a9; }
entry.palette-entry, entry.palette-entry:focus-within { background: none; border: none; box-shadow: none; outline: none;
    padding: 0; min-height: 0; font-size: 19px; color: #c0caf5; }
.palette-hint { color: #565f89; font-size: 12px; margin-top: 8px; }

/* ---- overview (the live strip, zoomed out) ---- */
.zoomed .tab { opacity: 0.88; border-radius: 28px; }
.zoomed .tab.focused { opacity: 1; outline: 7px solid #7aa2f7; outline-offset: -5px; }
.ov-hint { background: #1a1b26; color: #8089b3; border: 1px solid #2f334d; border-radius: 10px;
           padding: 8px 16px; font-size: 12px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); }
";

/// How far the strip zooms out in overview (niri uses a similar factor).
const OV_ZOOM: f64 = 0.4;

struct Tab {
    view: WebView,
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

    fn new_tab(self: &Rc<Self>) {
        self.exit_overview();
        let view = WebView::new();
        view.load_html(START_PAGE, None);
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, true); // new column
        self.show_palette();
    }

    fn split_vertical(self: &Rc<Self>) {
        self.exit_overview();
        let view = WebView::new();
        view.load_html(START_PAGE, None);
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, false); // in current column below
        self.show_palette();
    }

    fn add_view(self: &Rc<Self>, view: WebView, new_col: bool) {
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
                    tabs: vec![Tab { view: view.clone() }],
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
                col.tabs.insert(next_r, Tab { view: view.clone() });
                (cur_c, next_r)
            }
        };

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
            let nv = WebView::builder().related_view(v).build();
            app.add_view(nv.clone(), true);
            Some(nv.upcast())
        });
        let app = self.clone();
        view.connect_close(move |v| app.close_view(v));

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

    // ---- address palette -------------------------------------------------

    fn show_palette(&self) {
        let uri = self.focused_view().and_then(|v| v.uri()).filter(|u| !u.starts_with("about:"));
        self.entry.set_text(uri.as_deref().unwrap_or(""));
        self.palette.set_visible(true);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    fn hide_palette(&self) {
        self.palette.set_visible(false);
        if let Some(v) = self.focused_view() {
            v.grab_focus();
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
            if let Some(v) = self.focused_view() {
                let uri = to_uri(text);
                self.pill_label.set_text(&host_of(&uri).unwrap_or(uri.clone()));
                v.load_uri(&uri);
                *self.fresh.borrow_mut() = None;
            }
        }
        self.hide_palette();
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
        let global = m == M::CONTROL_MASK && matches!(k, gdk::Key::o | gdk::Key::t | gdk::Key::q);
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

fn setup_storage() {
    let dir = glib::user_data_dir().join("rail");
    let _ = std::fs::create_dir_all(&dir);
    if let Some(cookies) = webkit6::NetworkSession::default().and_then(|s| s.cookie_manager()) {
        cookies.set_persistent_storage(
            dir.join("cookies.sqlite").to_str().unwrap_or("cookies.sqlite"),
            webkit6::CookiePersistentStorage::Sqlite,
        );
    }
}

fn build(gapp: &gtk::Application) {
    setup_storage();
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
    let prow = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let sicon = gtk::Image::from_icon_name("system-search-symbolic");
    sicon.set_pixel_size(20);
    prow.append(&sicon);
    prow.append(&entry);
    let phint = gtk::Label::new(Some("Enter  go      Esc  cancel"));
    phint.add_css_class("palette-hint");
    phint.set_xalign(0.0);
    let palette = gtk::Box::new(gtk::Orientation::Vertical, 0);
    palette.add_css_class("palette");
    palette.set_halign(gtk::Align::Center);
    palette.set_valign(gtk::Align::Start);
    palette.set_margin_top(110);
    palette.append(&prow);
    palette.append(&phint);
    palette.set_visible(false);

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
    });

    let a = app.clone();
    layout.connect_resized(move || a.reveal_focus());

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
    if let Some(url) = std::env::args().nth(1) {
        app.navigate(&url);
    }
    win.present();
}

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("dev.rail.browser")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build);
    app.run_with_args::<&str>(&[])
}
