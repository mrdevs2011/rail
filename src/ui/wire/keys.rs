//! Window keyboard shortcuts and smooth, press-and-hold zoom.

use crate::*;

pub(crate) fn wire_keys(app: &Rc<App>, win: gtk::ApplicationWindow) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    KEY_CTL.with(|k| *k.borrow_mut() = Some(keys.clone()));
    apply_shortcut_priority();
    sc_refresh_bound();

    // A held key is represented by its hardware keycode. A frame-clock callback checks
    // that state instead of depending on OS key repeat or a cancellable timeout source.
    let zoom_repeat_key: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));

    // WebKit consumes Ctrl+V before browser shortcuts run. Handle image paste in capture
    // phase; plain text paste still proceeds to the focused WebView unchanged.
    let paste_keys = gtk::EventControllerKey::new();
    paste_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let paste_app = app.clone();
    paste_keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        if state.contains(M::CONTROL_MASK)
            && !state.intersects(M::ALT_MASK | M::SUPER_MASK | M::SHIFT_MASK)
            && (key == gdk::Key::v || key == gdk::Key::V || keycode == 55)
            && (paste_app.try_paste_image() || paste_app.try_paste_files())
        {
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    win.add_controller(paste_keys);

    // Web pages may consume Alt+Arrow. Keep Rail's spatial focus navigation reliable.
    let nav_keys = gtk::EventControllerKey::new();
    nav_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let nav_app = app.clone();
    nav_keys.connect_key_pressed(move |_, key, _, state| {
        use gdk::ModifierType as M;
        if state.contains(M::ALT_MASK)
            && !state.intersects(M::CONTROL_MASK | M::SUPER_MASK | M::SHIFT_MASK)
        {
            let action = match key {
                gdk::Key::Left => Some("win_left"),
                gdk::Key::Right => Some("win_right"),
                gdk::Key::Up => Some("win_up"),
                gdk::Key::Down => Some("win_down"),
                _ => None,
            };
            if let Some(action) = action {
                if nav_app.run_action(action) {
                    return glib::Propagation::Stop;
                }
            }
        }
        glib::Propagation::Proceed
    });
    win.add_controller(nav_keys);

    let app_key = app.clone();
    let held_key = zoom_repeat_key.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        // Repeated key-down events are ignored while the frame clock owns this key.
        if held_key.get() == Some(keycode) {
            return glib::Propagation::Stop;
        }

        // Omnibar / new-tab input: Esc must close it even if a shortcut map interferes.
        if key == gdk::Key::Escape && app_key.palette_open() {
            app_key.cancel_palette();
            return glib::Propagation::Stop;
        }

        // Resolve the same customizable shortcut IDs used by Settings > Customize.
        let resolved = sc_resolve_any(&shortcut_combos(key, keycode, state));
        let zoom_direction = match &resolved {
            ScResolve::Action("overview_zoom_in") => Some(1),
            ScResolve::Action("overview_zoom_out") => Some(-1),
            _ => None,
        };

        if let Some(direction) = zoom_direction {
            let was_overview = app_key.overview_open();
            held_key.set(Some(keycode));

            // A tap always means exactly one step. Continuous movement begins only after
            // the explicit hold delay, so a short press can never run to either zoom limit.
            app_key.zoom_step(direction);

            let weak_app = Rc::downgrade(&app_key);
            let held = held_key.clone();
            let started_at = glib::monotonic_time();
            let last_frame = Cell::new(None::<i64>);
            app_key.strip.add_tick_callback(move |_, clock| {
                if held.get() != Some(keycode) {
                    return glib::ControlFlow::Break;
                }
                let Some(app) = weak_app.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                // A mode change while the key is held ends this gesture; it never leaks
                // from overview into a page (or vice versa).
                if app.overview_open() != was_overview || (was_overview && app.ov_exiting.get()) {
                    held.set(None);
                    return glib::ControlFlow::Break;
                }

                let now = clock.frame_time();
                if now - started_at < ZOOM_HOLD_DELAY.as_micros() as i64 {
                    return glib::ControlFlow::Continue;
                }
                let dt = last_frame.replace(Some(now)).map_or(0.0, |last| {
                    ((now - last) as f64 / 1_000_000.0).clamp(0.0, 0.05)
                });
                if dt > 0.0 {
                    app.zoom_by_factor(zoom_hold_factor(direction, dt));
                }
                glib::ControlFlow::Continue
            });
            return glib::Propagation::Stop;
        }

        // All remaining actions use the central shortcut dispatcher. Unknown keys are
        // allowed to reach WebKit, including ordinary editing and site-specific shortcuts.
        let handled = match resolved {
            ScResolve::Action(id) => app_key.run_action(id),
            ScResolve::Disabled => false,
            ScResolve::Unknown => app_key.on_key(key, keycode, state),
        };
        if handled {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    // Capture release in the same phase/controller as key-down. No timer handle is kept:
    // the frame callback sees None on its next frame and exits without a source-ID race.
    let held_release = zoom_repeat_key.clone();
    keys.connect_key_released(move |_, _, keycode, _| {
        if held_release.get() == Some(keycode) {
            held_release.set(None);
        }
    });

    let held_focus = zoom_repeat_key.clone();
    win.connect_has_focus_notify(move |_| held_focus.set(None));
    win.add_controller(keys);
}
