//! Zen-style page screenshot: element hover outline, click-to-scope, resizable
//! region, visible / full-document capture via WebKit snapshot.

use crate::*;
use std::cell::Cell;

const HIGHLIGHT_JS: &str = r#"
(function(){
  if (window.__railSs) return;
  const st = { el: null, last: null };
  function outline(el) {
    if (st.el && st.el !== el) {
      st.el.style.outline = st.last || '';
    }
    st.el = el;
    if (el) {
      st.last = el.style.outline;
      el.style.outline = '2px solid #3b82f6';
      el.style.outlineOffset = '1px';
    }
  }
  function clear() {
    if (st.el) { st.el.style.outline = st.last || ''; st.el = null; st.last = null; }
  }
  function onMove(e) {
    const el = document.elementFromPoint(e.clientX, e.clientY);
    if (!el || el === document.documentElement || el === document.body) { outline(null); return; }
    outline(el);
  }
  function onClick(e) {
    e.preventDefault(); e.stopPropagation();
    const el = document.elementFromPoint(e.clientX, e.clientY);
    if (!el) return;
    const r = el.getBoundingClientRect();
    window.__railSsPick = {
      x: r.left, y: r.top, w: r.width, h: r.height,
      tag: el.tagName || ''
    };
  }
  window.__railSs = { clear, onMove, onClick };
  document.addEventListener('mousemove', onMove, true);
  document.addEventListener('click', onClick, true);
})();
"#;

const CLEANUP_JS: &str = r#"
(function(){
  if (!window.__railSs) return;
  window.__railSs.clear();
  document.removeEventListener('mousemove', window.__railSs.onMove, true);
  document.removeEventListener('click', window.__railSs.onClick, true);
  delete window.__railSs; delete window.__railSsPick;
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
        if w < 0.0 { x += w; w = -w; }
        if h < 0.0 { y += h; h = -h; }
        Self { x, y, w, h }
    }
    fn contains(self, px: f64, py: f64) -> bool {
        let r = self.normalized();
        px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h
    }
}

struct State {
    mode: Cell<Mode>,
    full_page: Cell<bool>,
    region: Cell<Option<Rect>>,
    drag_origin: Cell<Option<(f64, f64)>>,
    resize_edge: Cell<Option<&'static str>>,
    hover_el: Cell<Option<Rect>>,
}

impl App {
    pub(crate) fn start_screenshot_mode(self: &Rc<Self>, view: &WebView) {
        // Parent the overlay over the webview's allocation inside the window.
        let overlay = gtk::Overlay::new();
        overlay.add_css_class("ss-overlay");
        overlay.set_can_target(true);

        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_cursor_from_name(Some("crosshair"));

        let state = Rc::new(State {
            mode: Cell::new(Mode::Picking),
            full_page: Cell::new(false),
            region: Cell::new(None),
            drag_origin: Cell::new(None),
            resize_edge: Cell::new(None),
            hover_el: Cell::new(None),
        });

        // Top bar: Save visible / Save full page
        let topbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        topbar.add_css_class("ss-topbar");
        topbar.set_halign(gtk::Align::End);
        topbar.set_valign(gtk::Align::Start);
        topbar.set_margin_top(12);
        topbar.set_margin_end(12);

        let btn_vis = gtk::Button::with_label(tr("Save visible"));
        let btn_full = gtk::Button::with_label(tr("Save full page"));
        btn_vis.add_css_class("ss-active");
        topbar.append(&btn_vis);
        topbar.append(&btn_full);

        // Hint label (center)
        let hint = gtk::Label::new(Some(tr(
            "Drag or click on the page to select a region. Press ESC to cancel.",
        )));
        hint.add_css_class("ss-hint");
        hint.set_halign(gtk::Align::Center);
        hint.set_valign(gtk::Align::Center);

        // Size label
        let size_lbl = gtk::Label::new(None);
        size_lbl.add_css_class("ss-size");
        size_lbl.set_halign(gtk::Align::Center);
        size_lbl.set_valign(gtk::Align::Start);
        size_lbl.set_margin_top(56);
        size_lbl.set_visible(false);

        // Floating toolbar near selection
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        toolbar.add_css_class("ss-toolbar");
        toolbar.set_halign(gtk::Align::Center);
        toolbar.set_valign(gtk::Align::End);
        toolbar.set_margin_bottom(24);
        toolbar.set_visible(false);

        let btn_close = gtk::Button::with_label("✕");
        let btn_copy = gtk::Button::with_label(tr("Copy"));
        let btn_dl = gtk::Button::with_label(tr("Download"));
        toolbar.append(&btn_close);
        toolbar.append(&btn_copy);
        toolbar.append(&btn_dl);

        overlay.set_child(Some(&area));
        overlay.add_overlay(&topbar);
        overlay.add_overlay(&hint);
        overlay.add_overlay(&size_lbl);
        overlay.add_overlay(&toolbar);

        // Inject hover highlighter
        view.evaluate_javascript(HIGHLIGHT_JS, None, None, None::<&gio::Cancellable>, |_| {});

        // Draw dim + selection rect
        let st_d = state.clone();
        area.set_draw_func(move |_, cr, w, h| {
            let ww = w as f64;
            let hh = h as f64;
            // Dim everything
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.45);
            let _ = cr.paint();

            if let Some(r) = st_d.region.get().map(|r| r.normalized()) {
                // Clear hole
                cr.set_operator(gtk::cairo::Operator::Clear);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.fill();
                cr.set_operator(gtk::cairo::Operator::Over);

                // Border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.95);
                cr.set_line_width(1.5);
                cr.set_dash(&[6.0, 4.0], 0.0);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.stroke();
                cr.set_dash(&[], 0.0);

                // Corner handles
                let hs = 8.0;
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
                // Live element outline while picking
                cr.set_source_rgba(0.23, 0.51, 0.96, 0.35);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.fill();
                cr.set_source_rgba(0.23, 0.51, 0.96, 0.95);
                cr.set_line_width(2.0);
                cr.rectangle(r.x, r.y, r.w, r.h);
                let _ = cr.stroke();
            }

            let _ = (ww, hh);
        });

        // Pointer handlers
        let click = gtk::GestureClick::new();
        click.set_button(1);
        let st_c = state.clone();
        let area_c = area.clone();
        let view_c = view.clone();
        let hint_c = hint.clone();
        let toolbar_c = toolbar.clone();
        let size_c = size_lbl.clone();
        click.connect_pressed(move |_, _, x, y| {
            if st_c.mode.get() == Mode::Picking {
                // Prefer element pick from JS
                let st2 = st_c.clone();
                let area2 = area_c.clone();
                let hint2 = hint_c.clone();
                let tb2 = toolbar_c.clone();
                let sz2 = size_c.clone();
                let ox = x;
                let oy = y;
                view_c.evaluate_javascript(
                    "window.__railSsPick ? JSON.stringify(window.__railSsPick) : null",
                    None,
                    None,
                    None::<&gio::Cancellable>,
                    move |res| {
                        let mut used_el = false;
                        if let Ok(val) = res {
                            let s = val.to_string();
                            if s != "null" && s.len() > 2 {
                                // crude parse: {"x":..,"y":..,"w":..,"h":..}
                                if let Some(r) = parse_js_rect(&s) {
                                    st2.region.set(Some(r));
                                    used_el = true;
                                }
                            }
                        }
                        if !used_el {
                            st2.drag_origin.set(Some((ox, oy)));
                            st2.region.set(Some(Rect { x: ox, y: oy, w: 0.0, h: 0.0 }));
                        }
                        st2.mode.set(Mode::Region);
                        hint2.set_visible(false);
                        tb2.set_visible(true);
                        if let Some(r) = st2.region.get().map(|r| r.normalized()) {
                            sz2.set_text(&format!("{} × {}", r.w.round() as i32, r.h.round() as i32));
                            sz2.set_visible(true);
                        }
                        area2.queue_draw();
                    },
                );
            } else if let Some(r) = st_c.region.get().map(|r| r.normalized()) {
                // Check handle hit for resize
                let edge = hit_handle(r, x, y);
                st_c.resize_edge.set(edge);
                if edge.is_none() && !r.contains(x, y) {
                    // restart drag outside
                    st_c.drag_origin.set(Some((x, y)));
                    st_c.region.set(Some(Rect { x, y, w: 0.0, h: 0.0 }));
                }
            }
        });
        let st_r = state.clone();
        let area_r = area.clone();
        click.connect_released(move |_, _, _, _| {
            st_r.drag_origin.set(None);
            st_r.resize_edge.set(None);
            if let Some(r) = st_r.region.get().map(|r| r.normalized()) {
                if r.w < 4.0 || r.h < 4.0 {
                    st_r.region.set(None);
                    st_r.mode.set(Mode::Picking);
                } else {
                    st_r.region.set(Some(r));
                }
            }
            area_r.queue_draw();
        });
        area.add_controller(click);

        let motion = gtk::EventControllerMotion::new();
        let st_m = state.clone();
        let area_m = area.clone();
        let size_m = size_lbl.clone();
        let view_m = view.clone();
        motion.connect_motion(move |_, x, y| {
            match st_m.mode.get() {
                Mode::Picking => {
                    // Poll JS hover rect occasionally via elementFromPoint on our side is hard;
                    // instead ask for last outlined element's rect.
                    let st3 = st_m.clone();
                    let a3 = area_m.clone();
                    view_m.evaluate_javascript(
                        "(function(){var e=window.__railSs&&window.__railSs.el;if(!e)return null;var r=e.getBoundingClientRect();return JSON.stringify({x:r.left,y:r.top,w:r.width,h:r.height});})()",
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
                    if let Some((ox, oy)) = st_m.drag_origin.get() {
                        st_m.region.set(Some(Rect {
                            x: ox,
                            y: oy,
                            w: x - ox,
                            h: y - oy,
                        }));
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
                                // Auto-scroll when dragging bottom past viewport
                                let vh = area_m.height() as f64;
                                if y > vh - 24.0 {
                                    view_m.evaluate_javascript(
                                        "window.scrollBy(0, 40)",
                                        None,
                                        None,
                                        None::<&gio::Cancellable>,
                                        |_| {},
                                    );
                                } else if y < 24.0 {
                                    view_m.evaluate_javascript(
                                        "window.scrollBy(0, -40)",
                                        None,
                                        None,
                                        None::<&gio::Cancellable>,
                                        |_| {},
                                    );
                                }
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
                    if let Some(r) = st_m.region.get().map(|r| r.normalized()) {
                        size_m.set_text(&format!("{} × {}", r.w.round() as i32, r.h.round() as i32));
                        size_m.set_visible(true);
                    }
                    area_m.queue_draw();
                }
            }
        });
        area.add_controller(motion);

        // ESC to cancel
        let key = gtk::EventControllerKey::new();
        let app_k = self.clone();
        let view_k = view.clone();
        let overlay_k = overlay.downgrade();
        key.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                cleanup_screenshot(&view_k);
                if let Some(o) = overlay_k.upgrade() {
                    if let Some(parent) = o.parent() {
                        if let Some(boxp) = parent.downcast_ref::<gtk::Box>() {
                            boxp.remove(&o);
                        } else if let Some(ov) = parent.downcast_ref::<gtk::Overlay>() {
                            ov.remove_overlay(&o);
                        }
                    }
                }
                let _ = &app_k;
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        overlay.add_controller(key);

        // Top bar toggles
        let st_v = state.clone();
        let bv = btn_vis.clone();
        let bf = btn_full.clone();
        btn_vis.connect_clicked(move |_| {
            st_v.full_page.set(false);
            bv.add_css_class("ss-active");
            bf.remove_css_class("ss-active");
        });
        let st_f = state.clone();
        let bv2 = btn_vis.clone();
        let bf2 = btn_full.clone();
        btn_full.connect_clicked(move |_| {
            st_f.full_page.set(true);
            bf2.add_css_class("ss-active");
            bv2.remove_css_class("ss-active");
        });

        // Close
        let view_cl = view.clone();
        let overlay_cl = overlay.downgrade();
        btn_close.connect_clicked(move |_| {
            cleanup_screenshot(&view_cl);
            if let Some(o) = overlay_cl.upgrade() {
                if let Some(parent) = o.parent() {
                    if let Some(boxp) = parent.downcast_ref::<gtk::Box>() {
                        boxp.remove(&o);
                    } else if let Some(ov) = parent.downcast_ref::<gtk::Overlay>() {
                        ov.remove_overlay(&o);
                    }
                }
            }
        });

        // Capture helpers
        let capture = {
            let app = self.clone();
            let view = view.clone();
            let state = state.clone();
            let overlay = overlay.downgrade();
            move |to_clipboard: bool| {
                let region = if state.full_page.get() {
                    webkit6::SnapshotRegion::FullDocument
                } else {
                    webkit6::SnapshotRegion::Visible
                };
                let opts = webkit6::SnapshotOptions::empty();
                let app2 = app.clone();
                let st2 = state.clone();
                let ov2 = overlay.clone();
                let view2 = view.clone();
                view.snapshot(region, opts, None::<&gio::Cancellable>, move |res| {
                    match res {
                        Ok(tex) => {
                            let final_tex = if !st2.full_page.get() {
                                if let Some(r) = st2.region.get().map(|r| r.normalized()) {
                                    crop_texture(&tex, r).unwrap_or(tex)
                                } else {
                                    tex
                                }
                            } else {
                                tex
                            };
                            if to_clipboard {
                                app2.win.clipboard().set_texture(&final_tex);
                                eprintln!("[rail] screenshot copied");
                            } else {
                                save_texture_png(&final_tex);
                                eprintln!("[rail] screenshot saved");
                            }
                            cleanup_screenshot(&view2);
                            if let Some(o) = ov2.upgrade() {
                                if let Some(parent) = o.parent() {
                                    if let Some(boxp) = parent.downcast_ref::<gtk::Box>() {
                                        boxp.remove(&o);
                                    } else if let Some(ov) = parent.downcast_ref::<gtk::Overlay>() {
                                        ov.remove_overlay(&o);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[rail] screenshot failed: {e}");
                        }
                    }
                });
            }
        };

        let cap_c = capture.clone();
        btn_copy.connect_clicked(move |_| cap_c(true));
        let cap_d = capture.clone();
        btn_dl.connect_clicked(move |_| cap_d(false));

        // Mount overlay on top of the focused tab's view parent if possible,
        // otherwise as a window-level overlay child of shell.
        // Prefer putting it as a sibling covering the webview.
        let parent = view.parent();
        if let Some(p) = parent {
            if let Some(ov) = p.downcast_ref::<gtk::Overlay>() {
                ov.add_overlay(&overlay);
            } else if let Some(boxp) = p.downcast_ref::<gtk::Box>() {
                // Wrap is invasive; fall back to window overlay via a full-window DrawingArea approach:
                // put overlay as last child of shell with expand.
                let _ = boxp;
                self.shell.append(&overlay);
                // Position via CSS/size — make it cover the strip area roughly by expanding.
                overlay.set_hexpand(true);
                overlay.set_vexpand(true);
            } else {
                self.shell.append(&overlay);
                overlay.set_hexpand(true);
                overlay.set_vexpand(true);
            }
        } else {
            self.shell.append(&overlay);
            overlay.set_hexpand(true);
            overlay.set_vexpand(true);
        }

        // Grab focus so ESC works
        overlay.grab_focus();
    }
}

fn cleanup_screenshot(view: &WebView) {
    view.evaluate_javascript(CLEANUP_JS, None, None, None::<&gio::Cancellable>, |_| {});
}

fn parse_js_rect(s: &str) -> Option<Rect> {
    // Accept both raw JSON and quoted JS string values.
    let s = s.trim().trim_matches('"');
    let s = s.replace("\\\"", "\"");
    let get = |key: &str| -> Option<f64> {
        let pat = format!("\"{}\":", key);
        let idx = s.find(&pat)?;
        let rest = &s[idx + pat.len()..];
        let num: String = rest
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' )
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
    if on_x && on_y { return Some("nw"); }
    if on_xr && on_y { return Some("ne"); }
    if on_x && on_yb { return Some("sw"); }
    if on_xr && on_yb { return Some("se"); }
    if mid_x && on_y { return Some("n"); }
    if mid_x && on_yb { return Some("s"); }
    if on_x && mid_y { return Some("w"); }
    if on_xr && mid_y { return Some("e"); }
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
    // Download full pixels then copy sub-rect via MemoryTexture.
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
    Some(gdk::MemoryTexture::new(
        iw,
        ih,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        (iw * 4) as usize,
    ).upcast())
}

fn save_texture_png(tex: &gdk::Texture) {
    // Rail data dir: ~/.local/share/rail/screenshots/ (or $RAIL_DATA_DIR/screenshots)
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
    // Prefer local wall-clock via `date`; fallback to unique epoch-based name.
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
