//! Window gestures: click-away and touchpad / wheel panning of the strip.

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

    // BOTH_AXES only: KINETIC+DISCRETE on a Capture window controller makes page scroll sticky/janky
    // (steals momentum from WebKit). Overview + touchpad slide still work via dx/dy deltas.
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
    let panned = Rc::new(Cell::new(false));
    let panned_c = panned.clone();
    let tracker = Rc::new(RefCell::new(SwipeTracker::new()));
    let tracker_c = tracker.clone();
    let last_ev_us = Rc::new(Cell::new(0i64));
    let last_ev_c = last_ev_us.clone();
    scroll.connect_scroll(move |ctl, dx, dy| {
        // Fast path: normal browsing — do not touch scroll at all (WebKit keeps smooth kinetic scroll)
        if !a.ov.get() && !a.touchpad_mode.get() {
            return glib::Propagation::Proceed;
        }
        if a.settings_modal_open() || a.palette_open() || a.map_open() {
            return glib::Propagation::Proceed;
        }

        let now = glib::monotonic_time();
        a.last_scroll_us.set(now);

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

        // 3. Touchpad slide mode (Ctrl+Shift+T): 1:1 finger tracking, not discrete tab jumps
        if !a.touchpad_mode.get() {
            return glib::Propagation::Proceed;
        }

        // The page can scroll by itself: leave the gesture alone so WebKit scrolls it natively.
        // Touchpad slide only works on pages that have nothing to scroll.
        if a.page_scrollable.get() {
            return glib::Propagation::Proceed;
        }

        // Only clearly horizontal gestures pan the ribbon; everything else (incl. diagonal
        // reading scroll) must reach WebKit or the page feels sticky.
        if dx.abs() > 0.8 && dx.abs() > 2.0 * dy.abs() {
            ls.set(now);
            l_swipe.set(now);
            if !panned_c.get() {
                tracker_c.borrow_mut().reset();
            }
            panned_c.set(true);
            tracker_c.borrow_mut().push(dx, ctl.current_event_time() as f64 / 1000.0);
            last_ev_c.set(now);
            a.pan_canvas_x(dx);
            return glib::Propagation::Stop;
        }

        glib::Propagation::Proceed
    });
    let ax2 = accum_x.clone();
    let ay2 = accum_y.clone();
    let l_swipe2 = last_swipe.clone();
    scroll.connect_decelerate(move |_, _vx, _vy| {
        // The release is handled in scroll_end with our own velocity tracker (niri-style).
        ax2.set(0.0);
        ay2.set(0.0);
        l_swipe2.set(0);
    });
    // `decelerate` never fires without the KINETIC flag, so settle on the nearest column here
    // (only if this gesture actually panned the ribbon).
    let a_end = app.clone();
    let panned_end = panned.clone();
    scroll.connect_scroll_end(move |_| {
        if panned_end.replace(false) && a_end.touchpad_mode.get() && !a_end.ov.get() {
            let idle = (glib::monotonic_time() - last_ev_us.get()) as f64 / 1e6;
            let vel = tracker.borrow_mut().release_velocity(idle);
            tracker.borrow_mut().reset();
            a_end.finish_slide(vel);
        }
    });
    win.add_controller(scroll);
}
