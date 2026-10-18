//! Window keyboard shortcuts.

use crate::*;

pub(crate) fn wire_keys(app: &Rc<App>, win: gtk::ApplicationWindow) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    KEY_CTL.with(|k| *k.borrow_mut() = Some(keys.clone()));
    apply_shortcut_priority();
    sc_refresh_bound();
    let held_shortcut: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));

    let a = app.clone();
    let held = held_shortcut.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let alt = state.contains(M::ALT_MASK);
        let super_key = state.contains(M::SUPER_MASK);

        // Omnibar / new-tab input: Esc must close it even if a shortcut map interferes
        if key == gdk::Key::Escape && a.palette_open() {
            a.cancel_palette();
            return glib::Propagation::Stop;
        }

        // Image paste: WebKitGTK gives pages an empty clipboardData for images
        if ctrl && !alt && !state.contains(M::SHIFT_MASK)
            && (key == gdk::Key::v || key == gdk::Key::V || keycode == 55)
            && a.try_paste_image()
        {
            return glib::Propagation::Stop;
        }

        // Autorepeat loop suppressor: block machine-gun repeat when shortcut is held down
        if ctrl || alt || super_key {
            if held.get() == Some(keycode) {
                return glib::Propagation::Stop;
            }
        }

        let combo = shortcut_combo(key, keycode, state);
        let handled = match combo.as_deref().map(sc_resolve) {
            Some(ScResolve::Action(def_combo)) => {
                if Some(def_combo.as_str()) == combo.as_deref() {
                    a.on_key(key, keycode, state)
                } else if let Some((k2, m2)) = sc_synth(&def_combo) {
                    a.on_key(k2, 0, m2)
                } else {
                    false
                }
            }
            Some(ScResolve::Disabled) => false,
            _ => a.on_key(key, keycode, state),
        };
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
}
