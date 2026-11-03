//! Window keyboard shortcuts.

use crate::*;

pub(crate) fn wire_keys(app: &Rc<App>, win: gtk::ApplicationWindow) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    KEY_CTL.with(|k| *k.borrow_mut() = Some(keys.clone()));
    apply_shortcut_priority();
    sc_refresh_bound();

    // WebKit consumes Ctrl+V before bubble-phase browser shortcuts run. Handle
    // image paste in a dedicated capture-phase controller; plain text paste still
    // proceeds to WebKit untouched.
    let paste_keys = gtk::EventControllerKey::new();
    paste_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let paste_app = app.clone();
    paste_keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        if state.contains(M::CONTROL_MASK)
            && !state.intersects(M::ALT_MASK | M::SUPER_MASK | M::SHIFT_MASK)
            && (key == gdk::Key::v || key == gdk::Key::V || keycode == 55)
        {
            if paste_app.try_paste_image() {
                return glib::Propagation::Stop;
            }
            if paste_app.try_paste_files() {
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    win.add_controller(paste_keys);

    // Web pages may consume Alt+Arrow before Rail's bubble-phase shortcuts run.
    // Keep spatial focus navigation reliable with a narrow capture-phase handler.
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
            if let Some(action) = action
                && nav_app.run_action(action)
            {
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    win.add_controller(nav_keys);

    let a = app.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        // Omnibar / new-tab input: Esc must close it even if a shortcut map interferes
        if key == gdk::Key::Escape && a.palette_open() {
            a.cancel_palette();
            return glib::Propagation::Stop;
        }

        // Tugma qaysi amalga bog'langan (Settings > Customize > Shortcuts): standart ham, o'zgartirilgan ham
        let resolved = sc_resolve_any(&shortcut_combos(key, keycode, state));

        // Hamma amal ID bo'yicha bajariladi: tugma o'zgartirilgan bo'lsa ham xuddi shu yo'l.
        // `Unknown` — ro'yxatda yo'q tugma (Esc, overview ichidagi oddiy tugmalar, Shift+Enter ...).
        let handled = match resolved {
            ScResolve::Action(id) => a.run_action(id),
            ScResolve::Disabled => false,
            ScResolve::Unknown => a.on_key(key, keycode, state),
        };
        if handled {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    win.add_controller(keys);
}
