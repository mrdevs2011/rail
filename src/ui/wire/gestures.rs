//! Window gestures: click-away and overview touchpad / wheel panning of the strip.

use crate::*;

pub(crate) fn wire_gestures(app: &Rc<App>, shell: gtk::Box, win: gtk::ApplicationWindow) {
    // Close palette when clicking the page area — never auto-dismiss settings
    // (settings is a real page; shell Capture used to hide it on every click).
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let a = app.clone();
    click.connect_pressed(move |_, _, _, _| {
        // On the welcome page the omnibar is part of the page: never dismiss it by clicking.
        if a.palette_open() && !a.focused_is_welcome() {
            a.hide_palette();
        }
    });
    shell.add_controller(click);

    // Overview pinch-to-zoom (trackpad) + Ctrl+wheel zoom
    {
        let zoom = gtk::GestureZoom::new();
        zoom.set_propagation_phase(gtk::PropagationPhase::Capture);
        let last = Rc::new(Cell::new(1.0f64));
        let last_b = last.clone();
        zoom.connect_begin(move |_, _| {
            last_b.set(1.0);
        });
        let a2 = app.clone();
        zoom.connect_scale_changed(move |_, scale| {
            if !a2.ov.get() || a2.ov_exiting.get() {
                return;
            }
            let prev = last.replace(scale);
            if prev <= 0.01 {
                return;
            }
            let factor = scale / prev;
            if (factor - 1.0).abs() < 0.002 {
                return;
            }
            a2.overview_zoom_by(factor);
        });
        shell.add_controller(zoom);
    }
    {
        // Ctrl+scroll over overview adjusts scale (smooth, same path as Ctrl+/-).
        let scroll_z = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll_z.set_propagation_phase(gtk::PropagationPhase::Capture);
        let a = app.clone();
        scroll_z.connect_scroll(move |_, _dx, dy| {
            if !a.ov.get() || a.ov_exiting.get() {
                return glib::Propagation::Proceed;
            }
            let ctrl = gtk::prelude::WidgetExt::display(&a.win)
                .default_seat()
                .and_then(|s| s.keyboard())
                .map(|k| k.modifier_state().contains(gdk::ModifierType::CONTROL_MASK))
                .unwrap_or(false);
            if !ctrl {
                return glib::Propagation::Proceed;
            }
            if dy < 0.0 {
                a.overview_zoom_by(1.06);
            } else if dy > 0.0 {
                a.overview_zoom_by(1.0 / 1.06);
            }
            glib::Propagation::Stop
        });
        shell.add_controller(scroll_z);
    }

    // Overview-only 2D pan (two-finger swipe between tabs/rows). Normal browsing leaves
    // scroll to WebKit entirely — no Touchpad Slide mode.
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let accum_x = Rc::new(Cell::new(0.0f64));
    let accum_y = Rc::new(Cell::new(0.0f64));
    let last_scroll = Rc::new(Cell::new(0i64));
    let last_swipe = Rc::new(Cell::new(0i64));
    let a = app.clone();
    let ax = accum_x.clone();
    let ay = accum_y.clone();
    let ls = last_scroll.clone();
    let l_swipe = last_swipe.clone();
    scroll.connect_scroll(move |_ctl, dx, dy| {
        if !a.ov.get() {
            return glib::Propagation::Proceed;
        }
        if a.settings_modal_open() || a.palette_open() || a.map_open() {
            return glib::Propagation::Proceed;
        }

        let now = glib::monotonic_time();

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
        glib::Propagation::Proceed
    });
    let ax2 = accum_x.clone();
    let ay2 = accum_y.clone();
    let l_swipe2 = last_swipe.clone();
    scroll.connect_decelerate(move |_, _vx, _vy| {
        ax2.set(0.0);
        ay2.set(0.0);
        l_swipe2.set(0);
    });
    win.add_controller(scroll);
}
