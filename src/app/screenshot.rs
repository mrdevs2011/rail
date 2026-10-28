//! Zen / Firefox-style page screenshot.
//!
//! Flow (matches Firefox Screenshots / Zen):
//! 1. Dim overlay + instructions.
//! 2. Top-right **Save visible** / **Save full page** capture immediately.
//! 3. Hover highlights page elements; click picks that element.
//! 4. Drag draws a free region; handles resize; drag inside moves.
//! 5. Floating bar under the selection: Cancel · Copy · Download.
//! 6. Esc cancels; C copies; S downloads.

use crate::*;
use std::cell::Cell;

/// Element hover without mutating the page (no outline on DOM — drawn on our overlay).
const HOVER_JS: &str = r#"
(function(){
  function pick(x, y) {
    let el = document.elementFromPoint(x, y);
    if (!el || el === document.documentElement || el === document.body) return null;
    let curr = el;
    while (curr && curr.parentElement && curr.parentElement !== document.body && curr.parentElement !== document.documentElement) {
      const r = curr.getBoundingClientRect();
      if (r.width >= 16 && r.height >= 14) break;
      curr = curr.parentElement;
    }
    const r = (curr || el).getBoundingClientRect();
    return { x: r.left, y: r.top, w: r.width, h: r.height };
  }
  window.__railSs = { pick };
})();
"#;

const CLEANUP_JS: &str = r#"
(function(){
  if (!window.__railSs) return;
  delete window.__railSs;
})();
"#;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Picking,
    Region,
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn normalized(self) -> Self {
        let (mut x, mut y, mut w, mut h) = (self.x, self.y, self.w, self.h);
        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }
        Self { x, y, w, h }
    }
    fn contains(self, px: f64, py: f64) -> bool {
        let r = self.normalized();
        px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h
    }
}

struct State {
    mode: Cell<Mode>,
    region: Cell<Option<Rect>>,
    drag_origin: Cell<Option<(f64, f64)>>,
    /// When moving an existing region: pointer offset from region top-left.
    move_offset: Cell<Option<(f64, f64)>>,
    resize_edge: Cell<Option<&'static str>>,
    hover_el: Cell<Option<Rect>>,
}

impl App {
    pub(crate) fn start_screenshot_mode(self: &Rc<Self>, view: &WebView) {
        // One session at a time: remove any leftover overlay.
        self.end_screenshot_overlay();

        let overlay = gtk::Overlay::new();
        overlay.add_css_class("ss-overlay");
        overlay.set_can_target(true);
        overlay.set_focusable(true);

        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_cursor_from_name(Some("crosshair"));

        let state = Rc::new(State {
            mode: Cell::new(Mode::Picking),
            region: Cell::new(None),
            drag_origin: Cell::new(None),
            move_offset: Cell::new(None),
            resize_edge: Cell::new(None),
            hover_el: Cell::new(None),
        });

        // --- Top-right: Save visible / Save full page (immediate capture, Zen style) ---
        let topbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        topbar.add_css_class("ss-topbar");
        topbar.set_halign(gtk::Align::End);
        topbar.set_valign(gtk::Align::Start);
        topbar.set_margin_top(12);
        topbar.set_margin_end(12);

        let btn_vis = gtk::Button::with_label(tr("Save visible"));
        let btn_full = gtk::Button::with_label(tr("Save full page"));
        btn_vis.add_css_class("ss-top-btn");
        btn_full.add_css_class("ss-top-btn");
        topbar.append(&btn_vis);
        topbar.append(&btn_full);

        // Hint (center) — only while picking
        let hint = gtk::Label::new(Some(tr(
            "Drag or click on the page to select a region. Press ESC to cancel.",
        )));
        hint.add_css_class("ss-hint");
        hint.set_halign(gtk::Align::Center);
        hint.set_valign(gtk::Align::Center);

        // Size badge — placed over the selection via margins in motion handler
        let size_lbl = gtk::Label::new(None);
        size_lbl.add_css_class("ss-size");
        size_lbl.set_halign(gtk::Align::Start);
        size_lbl.set_valign(gtk::Align::Start);
        size_lbl.set_visible(false);

        // Floating toolbar under selection: Cancel · Copy · Download (styled like design)
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        toolbar.add_css_class("ss-toolbar");
        toolbar.set_halign(gtk::Align::Start);
        toolbar.set_valign(gtk::Align::Start);
        toolbar.set_visible(false);

        let btn_close = gtk::Button::from_icon_name("window-close-symbolic");
        btn_close.add_css_class("ss-btn-close");
        btn_close.set_tooltip_text(Some(tr("Cancel")));

        let btn_copy = gtk::Button::new();
        btn_copy.add_css_class("ss-btn-copy");
        let copy_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let copy_icon = gtk::Image::from_icon_name("edit-copy-symbolic");
        let copy_lbl = gtk::Label::new(Some(tr("Copy")));
        copy_box.append(&copy_icon);
        copy_box.append(&copy_lbl);
        btn_copy.set_child(Some(&copy_box));

        let btn_dl = gtk::Button::new();
        btn_dl.add_css_class("ss-primary");
        btn_dl.add_css_class("ss-btn-dl");
        let dl_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let dl_icon = gtk::Image::from_icon_name("document-save-symbolic");
        let dl_lbl = gtk::Label::new(Some(tr("Download")));
        dl_box.append(&dl_icon);
        dl_box.append(&dl_lbl);
        btn_dl.set_child(Some(&dl_box));

        toolbar.append(&btn_close);
        toolbar.append(&btn_copy);
        toolbar.append(&btn_dl);

        overlay.set_child(Some(&area));
        overlay.add_overlay(&topbar);
        overlay.add_overlay(&hint);
        overlay.add_overlay(&size_lbl);
        overlay.add_overlay(&toolbar);

        view.evaluate_javascript(HOVER_JS, None, None, None::<&gio::Cancellable>, |_| {});

        // --- Draw: dim + hole + border + handles / hover highlight ---
        let st_d = state.clone();
        area.set_draw_func(move |_, cr, w, h| {
            let ww = w as f64;
            let hh = h as f64;
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.55);
            let _ = cr.paint();

            if let Some(r) = st_d.region.get().map(|r| r.normalized()) {
                // Clear the selection hole so the page shows through
                cr.set_operator(gtk::cairo::Operator::Clear);
                cr.rectangle(r.x, r.y, r.w.max(1.0), r.h.max(1.0));
                let _ = cr.fill();
                cr.set_operator(gtk::cairo::Operator::Over);

                // White solid border (Firefox style)
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.stroke();

                // Corner + edge handles
                let hs = 7.0;
                cr.set_source_rgb(1.0, 1.0, 1.0);
                for (cx, cy) in [
                    (r.x, r.y),
                    (r.x + r.w, r.y),
                    (r.x, r.y + r.h),
                    (r.x + r.w, r.y + r.h),
                    (r.x + r.w / 2.0, r.y),
                    (r.x + r.w / 2.0, r.y + r.h),
                    (r.x, r.y + r.h / 2.0),
                    (r.x + r.w, r.y + r.h / 2.0),
                ] {
                    cr.rectangle(cx - hs / 2.0, cy - hs / 2.0, hs, hs);
                    let _ = cr.fill();
                }
            } else if let Some(r) = st_d.hover_el.get() {
                // Soft blue highlight like Firefox element picker
                cr.set_source_rgba(0.15, 0.55, 0.95, 0.22);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.fill();
                cr.set_source_rgba(0.15, 0.55, 0.95, 0.95);
                cr.set_line_width(2.0);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.stroke();
            }
            let _ = (ww, hh);
        });

        let position_chrome = {
            let size_lbl = size_lbl.clone();
            let toolbar = toolbar.clone();
            let area = area.clone();
            move |r: Rect| {
                let r = r.normalized();
                size_lbl.set_text(&format!("{} × {}", r.w.round() as i32, r.h.round() as i32));
                size_lbl.set_visible(true);
                // Size just above the selection
                size_lbl.set_margin_start(r.x.max(0.0) as i32);
                size_lbl.set_margin_top((r.y - 24.0).max(4.0) as i32);

                let aw = area.width() as f64;
                let ah = area.height() as f64;
                let tw = 220.0;
                let th = 42.0;
                let mut tx = r.x + r.w - tw;
                if tx < 4.0 {
                    tx = r.x.max(4.0);
                }
                let mut ty = r.y + r.h + 8.0;
                if ty + th > ah - 4.0 {
                    ty = (r.y - th - 8.0).max(4.0);
                }
                tx = tx.clamp(4.0, (aw - tw - 4.0).max(4.0));
                toolbar.set_margin_start(tx as i32);
                toolbar.set_margin_top(ty as i32);
            }
        };

        // --- Pointer: press ---
        let click = gtk::GestureClick::new();
        click.set_button(1);
        let st_c = state.clone();
        let area_c = area.clone();
        let view_c = view.clone();
        let hint_c = hint.clone();
        let pos_c = position_chrome.clone();
        let tb_c = toolbar.clone();
        click.connect_pressed(move |_, _, x, y| {
            tb_c.set_visible(false);
            match st_c.mode.get() {
                Mode::Picking => {
                    let st2 = st_c.clone();
                    let area2 = area_c.clone();
                    let hint2 = hint_c.clone();
                    let pos2 = pos_c.clone();
                    let tb2 = tb_c.clone();
                    let ox = x;
                    let oy = y;
                    let js = format!(
                        "window.__railSs && window.__railSs.pick({}, {}) ? JSON.stringify(window.__railSs.pick({}, {})) : null",
                        ox, oy, ox, oy
                    );
                    view_c.evaluate_javascript(
                        &js,
                        None,
                        None,
                        None::<&gio::Cancellable>,
                        move |res| {
                            let mut used_el = false;
                            if let Ok(val) = res {
                                let s = val.to_string();
                                if s != "null" && s.len() > 2 {
                                    if let Some(r) = parse_js_rect(&s) {
                                        if r.w >= 4.0 && r.h >= 4.0 {
                                            st2.region.set(Some(r));
                                            used_el = true;
                                        }
                                    }
                                }
                            }
                            if used_el {
                                st2.mode.set(Mode::Region);
                                hint2.set_visible(false);
                                if let Some(r) = st2.region.get() {
                                    pos2(r);
                                    tb2.set_visible(true);
                                }
                            } else {
                                st2.drag_origin.set(Some((ox, oy)));
                                st2.region.set(Some(Rect {
                                    x: ox,
                                    y: oy,
                                    w: 0.0,
                                    h: 0.0,
                                }));
                                st2.mode.set(Mode::Region);
                                hint2.set_visible(false);
                                tb2.set_visible(false);
                            }
                            area2.queue_draw();
                        },
                    );
                    view_c.evaluate_javascript(
                        "if(window.__railSs) window.__railSs.clicked=null",
                        None,
                        None,
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                }
                Mode::Region => {
                    if let Some(r) = st_c.region.get().map(|r| r.normalized()) {
                        if let Some(edge) = hit_handle(r, x, y) {
                            st_c.resize_edge.set(Some(edge));
                            st_c.move_offset.set(None);
                            st_c.drag_origin.set(None);
                        } else if r.contains(x, y) {
                            // Move whole region
                            st_c.move_offset.set(Some((x - r.x, y - r.y)));
                            st_c.resize_edge.set(None);
                            st_c.drag_origin.set(None);
                        } else {
                            // New drag outside
                            st_c.drag_origin.set(Some((x, y)));
                            st_c.region.set(Some(Rect {
                                x,
                                y,
                                w: 0.0,
                                h: 0.0,
                            }));
                            st_c.resize_edge.set(None);
                            st_c.move_offset.set(None);
                        }
                    }
                }
            }
        });

        let st_r = state.clone();
        let area_r = area.clone();
        let pos_r = position_chrome.clone();
        let tb_r = toolbar.clone();
        let hint_r = hint.clone();
        click.connect_released(move |_, _, _, _| {
            st_r.drag_origin.set(None);
            st_r.resize_edge.set(None);
            st_r.move_offset.set(None);
            if let Some(r) = st_r.region.get().map(|r| r.normalized()) {
                if r.w < 10.0 || r.h < 10.0 {
                    st_r.region.set(None);
                    st_r.mode.set(Mode::Picking);
                    tb_r.set_visible(false);
                    hint_r.set_visible(true);
                } else {
                    st_r.region.set(Some(r));
                    pos_r(r);
                    tb_r.set_visible(true);
                    hint_r.set_visible(false);
                }
            } else {
                tb_r.set_visible(false);
            }
            area_r.queue_draw();
        });
        area.add_controller(click);

        // --- Motion ---
        let motion = gtk::EventControllerMotion::new();
        let st_m = state.clone();
        let area_m = area.clone();
        let view_m = view.clone();
        let pos_m = position_chrome.clone();
        let tb_m = toolbar.clone();
        motion.connect_motion(move |_, x, y| {
            match st_m.mode.get() {
                Mode::Picking => {
                    let st3 = st_m.clone();
                    let a3 = area_m.clone();
                    let js = format!(
                        "window.__railSs && window.__railSs.pick({}, {}) ? JSON.stringify(window.__railSs.pick({}, {})) : null",
                        x, y, x, y
                    );
                    view_m.evaluate_javascript(
                        &js,
                        None,
                        None,
                        None::<&gio::Cancellable>,
                        move |res| {
                            if let Ok(val) = res {
                                let s = val.to_string();
                                if s != "null" && s.len() > 2 {
                                    if let Some(r) = parse_js_rect(&s) {
                                        st3.hover_el.set(Some(r));
                                        a3.queue_draw();
                                        return;
                                    }
                                }
                            }
                            st3.hover_el.set(None);
                            a3.queue_draw();
                        },
                    );
                }
                Mode::Region => {
                    let is_active_drag = st_m.drag_origin.get().is_some()
                        || st_m.resize_edge.get().is_some()
                        || st_m.move_offset.get().is_some();
                    if is_active_drag {
                        tb_m.set_visible(false);
                    }
                    let vh = area_m.height() as f64;
                    let vw = area_m.width() as f64;
                    // Edge auto-scroll while resizing/drawing
                    let edge_zone = 28.0;
                    if y > vh - edge_zone {
                        view_m.evaluate_javascript(
                            "window.scrollBy(0, 48)",
                            None,
                            None,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    } else if y < edge_zone {
                        view_m.evaluate_javascript(
                            "window.scrollBy(0, -48)",
                            None,
                            None,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    }
                    if x > vw - edge_zone {
                        view_m.evaluate_javascript(
                            "window.scrollBy(48, 0)",
                            None,
                            None,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    } else if x < edge_zone {
                        view_m.evaluate_javascript(
                            "window.scrollBy(-48, 0)",
                            None,
                            None,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    }

                    if let Some((ox, oy)) = st_m.drag_origin.get() {
                        st_m.region.set(Some(Rect {
                            x: ox,
                            y: oy,
                            w: x - ox,
                            h: y - oy,
                        }));
                    } else if let (Some((off_x, off_y)), Some(r0)) =
                        (st_m.move_offset.get(), st_m.region.get().map(|r| r.normalized()))
                    {
                        let mut r = r0;
                        r.x = (x - off_x).clamp(0.0, (vw - r.w).max(0.0));
                        r.y = (y - off_y).clamp(0.0, (vh - r.h).max(0.0));
                        st_m.region.set(Some(r));
                    } else if let (Some(edge), Some(mut r)) =
                        (st_m.resize_edge.get(), st_m.region.get().map(|r| r.normalized()))
                    {
                        match edge {
                            "nw" => {
                                r.w += r.x - x;
                                r.h += r.y - y;
                                r.x = x;
                                r.y = y;
                            }
                            "ne" => {
                                r.w = x - r.x;
                                r.h += r.y - y;
                                r.y = y;
                            }
                            "sw" => {
                                r.w += r.x - x;
                                r.x = x;
                                r.h = y - r.y;
                            }
                            "se" => {
                                r.w = x - r.x;
                                r.h = y - r.y;
                            }
                            "n" => {
                                r.h += r.y - y;
                                r.y = y;
                            }
                            "s" => {
                                r.h = y - r.y;
                            }
                            "w" => {
                                r.w += r.x - x;
                                r.x = x;
                            }
                            "e" => {
                                r.w = x - r.x;
                            }
                            _ => {}
                        }
                        st_m.region.set(Some(r));
                    }
                    if let Some(r) = st_m.region.get() {
                        pos_m(r);
                    }
                    area_m.queue_draw();
                }
            }
        });
        area.add_controller(motion);

        // --- Scroll on page while picking / selecting ---
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let view_scroll = view.clone();
        let area_scroll = area.clone();
        let st_scroll = state.clone();
        scroll.connect_scroll(move |_, _, dy| {
            let px = dy * 70.0;
            let js = format!("window.scrollBy({{ top: {}, behavior: 'instant' }})", px);
            let st_sc = st_scroll.clone();
            let a_sc = area_scroll.clone();
            view_scroll.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, move |_| {
                if st_sc.mode.get() == Mode::Picking {
                    st_sc.hover_el.set(None);
                    a_sc.queue_draw();
                }
            });
            glib::Propagation::Stop
        });
        area.add_controller(scroll);

        // --- Capture helpers ---
        let dismiss = {
            let app = self.clone();
            let view = view.clone();
            move || {
                cleanup_screenshot(&view);
                app.end_screenshot_overlay();
            }
        };

        let capture_region = {
            let app = self.clone();
            let view = view.clone();
            let state = state.clone();
            let dismiss = dismiss.clone();
            move |to_clipboard: bool| {
                let region = webkit6::SnapshotRegion::Visible;
                let opts = webkit6::SnapshotOptions::empty();
                let app2 = app.clone();
                let st2 = state.clone();
                let view2 = view.clone();
                let dismiss2 = dismiss.clone();
                let vw = view.width().max(1) as f64;
                let vh = view.height().max(1) as f64;
                view.snapshot(region, opts, None::<&gio::Cancellable>, move |res| {
                    match res {
                        Ok(tex) => {
                            let final_tex = if let Some(r) = st2.region.get().map(|r| r.normalized()) {
                                // Scale crop to texture pixels (HiDPI)
                                let sx = tex.width() as f64 / vw;
                                let sy = tex.height() as f64 / vh;
                                let scaled = Rect {
                                    x: r.x * sx,
                                    y: r.y * sy,
                                    w: r.w * sx,
                                    h: r.h * sy,
                                };
                                crop_texture(&tex, scaled).unwrap_or(tex)
                            } else {
                                tex
                            };
                            finish_capture(&app2, &final_tex, to_clipboard);
                            dismiss2();
                            let _ = &view2;
                        }
                        Err(e) => eprintln!("[rail] screenshot failed: {e}"),
                    }
                });
            }
        };

        let capture_preset = {
            let app = self.clone();
            let view = view.clone();
            let dismiss = dismiss.clone();
            move |full: bool, to_clipboard: bool| {
                let region = if full {
                    webkit6::SnapshotRegion::FullDocument
                } else {
                    webkit6::SnapshotRegion::Visible
                };
                let opts = webkit6::SnapshotOptions::empty();
                let app2 = app.clone();
                let dismiss2 = dismiss.clone();
                view.snapshot(region, opts, None::<&gio::Cancellable>, move |res| {
                    match res {
                        Ok(tex) => {
                            finish_capture(&app2, &tex, to_clipboard);
                            dismiss2();
                        }
                        Err(e) => eprintln!("[rail] screenshot failed: {e}"),
                    }
                });
            }
        };

        // Top bar: immediate capture → Download (Firefox downloads by default from these buttons;
        // we also support clipboard via the region toolbar).
        {
            let cap = capture_preset.clone();
            btn_vis.connect_clicked(move |_| cap(false, false));
        }
        {
            let cap = capture_preset.clone();
            btn_full.connect_clicked(move |_| cap(true, false));
        }

        {
            let d = dismiss.clone();
            btn_close.connect_clicked(move |_| d());
        }
        {
            let cap = capture_region.clone();
            btn_copy.connect_clicked(move |_| cap(true));
        }
        {
            let cap = capture_region.clone();
            btn_dl.connect_clicked(move |_| cap(false));
        }

        // Keyboard
        let key = gtk::EventControllerKey::new();
        let dismiss_k = dismiss.clone();
        let cap_k = capture_region.clone();
        let st_k = state.clone();
        key.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                dismiss_k();
                return glib::Propagation::Stop;
            }
            if st_k.mode.get() == Mode::Region && st_k.region.get().is_some() {
                if k == gdk::Key::c || k == gdk::Key::C {
                    cap_k(true);
                    return glib::Propagation::Stop;
                }
                if k == gdk::Key::s || k == gdk::Key::S {
                    cap_k(false);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        overlay.add_controller(key);

        // Mount over the webview's Overlay parent (tab widget)
        let mut attached = false;
        let mut cur = view.parent();
        while let Some(w) = cur {
            if let Some(ov) = w.downcast_ref::<gtk::Overlay>() {
                ov.add_overlay(&overlay);
                attached = true;
                break;
            }
            cur = w.parent();
        }
        if !attached {
            self.shell.append(&overlay);
            overlay.set_hexpand(true);
            overlay.set_vexpand(true);
        }

        // Stash so we can remove it later
        *self.ss_overlay.borrow_mut() = Some(overlay.clone());
        overlay.grab_focus();
    }

    pub(crate) fn end_screenshot_overlay(&self) {
        if let Some(o) = self.ss_overlay.borrow_mut().take() {
            if let Some(parent) = o.parent() {
                if let Some(boxp) = parent.downcast_ref::<gtk::Box>() {
                    boxp.remove(&o);
                } else if let Some(ov) = parent.downcast_ref::<gtk::Overlay>() {
                    ov.remove_overlay(&o);
                }
            }
        }
    }
}

fn finish_capture(app: &App, tex: &gdk::Texture, to_clipboard: bool) {
    if to_clipboard {
        app.win.clipboard().set_texture(tex);
        eprintln!("[rail] screenshot copied to clipboard");
    } else {
        save_texture_png(tex);
    }
}

fn cleanup_screenshot(view: &WebView) {
    view.evaluate_javascript(CLEANUP_JS, None, None, None::<&gio::Cancellable>, |_| {});
}

fn parse_js_rect(s: &str) -> Option<Rect> {
    let s = s.trim().trim_matches('"');
    let s = s.replace("\\\"", "\"");
    let get = |key: &str| -> Option<f64> {
        let pat = format!("\"{}\":", key);
        let idx = s.find(&pat)?;
        let rest = &s[idx + pat.len()..];
        let num: String = rest
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        num.parse().ok()
    };
    Some(Rect {
        x: get("x")?,
        y: get("y")?,
        w: get("w")?,
        h: get("h")?,
    })
}

fn hit_handle(r: Rect, x: f64, y: f64) -> Option<&'static str> {
    let t = 10.0;
    let near = |a: f64, b: f64| (a - b).abs() < t;
    let on_x = near(x, r.x);
    let on_xr = near(x, r.x + r.w);
    let on_y = near(y, r.y);
    let on_yb = near(y, r.y + r.h);
    let mid_x = near(x, r.x + r.w / 2.0);
    let mid_y = near(y, r.y + r.h / 2.0);
    if on_x && on_y {
        return Some("nw");
    }
    if on_xr && on_y {
        return Some("ne");
    }
    if on_x && on_yb {
        return Some("sw");
    }
    if on_xr && on_yb {
        return Some("se");
    }
    if mid_x && on_y {
        return Some("n");
    }
    if mid_x && on_yb {
        return Some("s");
    }
    if on_x && mid_y {
        return Some("w");
    }
    if on_xr && mid_y {
        return Some("e");
    }
    None
}

fn crop_texture(tex: &gdk::Texture, r: Rect) -> Option<gdk::Texture> {
    let tw = tex.width() as f64;
    let th = tex.height() as f64;
    let x = r.x.clamp(0.0, tw - 1.0);
    let y = r.y.clamp(0.0, th - 1.0);
    let w = r.w.clamp(1.0, tw - x);
    let h = r.h.clamp(1.0, th - y);
    let ix = x as i32;
    let iy = y as i32;
    let iw = w as i32;
    let ih = h as i32;
    if iw <= 0 || ih <= 0 {
        return None;
    }
    let stride = tw as i32 * 4;
    let mut buf = vec![0u8; (tw as usize) * (th as usize) * 4];
    tex.download(&mut buf, stride as usize);
    let mut out = vec![0u8; (iw as usize) * (ih as usize) * 4];
    for row in 0..ih as usize {
        let src_off = ((iy as usize + row) * stride as usize) + (ix as usize * 4);
        let dst_off = row * (iw as usize) * 4;
        out[dst_off..dst_off + iw as usize * 4]
            .copy_from_slice(&buf[src_off..src_off + iw as usize * 4]);
    }
    let bytes = glib::Bytes::from_owned(out);
    Some(
        gdk::MemoryTexture::new(iw, ih, gdk::MemoryFormat::R8g8b8a8, &bytes, (iw * 4) as usize)
            .upcast(),
    )
}

fn save_texture_png(tex: &gdk::Texture) {
    let dir = rail_data_dir().join("screenshots");
    let _ = std::fs::create_dir_all(&dir);
    let ts = chrono_like_timestamp();
    let path = dir.join(format!("Screenshot {ts}.png"));
    if let Err(e) = tex.save_to_png(&path) {
        eprintln!("[rail] save png failed: {e}");
    } else {
        eprintln!("[rail] screenshot saved → {}", path.display());
    }
}

fn chrono_like_timestamp() -> String {
    if let Ok(out) = std::process::Command::new("date")
        .args(["+%Y-%m-%d %H-%M-%S"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
