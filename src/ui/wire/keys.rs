//! Window keyboard shortcuts and responsive page zoom.

use crate::*;

pub(crate) fn wire_keys(app: &Rc<App>, win: gtk::ApplicationWindow) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    KEY_CTL.with(|k| *k.borrow_mut() = Some(keys.clone()));
    apply_shortcut_priority();
    sc_refresh_bound();

    // Image/file paste is handled only when the clipboard actually contains those types;
    // ordinary text paste is left untouched for the focused website.
    let paste_keys = gtk::EventControllerKey::new();
    paste_keys.set_propagation_phase(gtk::PropagationPhase::Bubble);
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

    let app_key = app.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        // Omnibar / new-tab input: Esc must close it even if a shortcut map interferes.
        if key == gdk::Key::Escape && app_key.palette_open() {
            app_key.cancel_palette();
            return glib::Propagation::Stop;
        }

        // Resolve the same customizable shortcut IDs used by Settings > Customize.
        let resolved = sc_resolve_any(&shortcut_combos(key, keycode, state));

        // Overview is navigation-only: browser zoom shortcuts must not scale its grid.
        // Swallow the bindings here so they cannot fall through to a page behind it.
        if app_key.overview_open()
            && matches!(
                &resolved,
                ScResolve::Action("overview_zoom_in")
                    | ScResolve::Action("overview_zoom_out")
                    | ScResolve::Action("zoom_reset")
            )
        {
            return glib::Propagation::Stop;
        }

        let zoom_direction = match &resolved {
            ScResolve::Action("overview_zoom_in") => Some(1),
            ScResolve::Action("overview_zoom_out") => Some(-1),
            _ => None,
        };

        if let Some(direction) = zoom_direction {
            // Apply every key-down, including the desktop's key-repeat events. This makes
            // holding Ctrl +/- zoom the page continuously instead of feeling unresponsive.
            app_key.zoom_step(direction);
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

    win.add_controller(keys);
}
