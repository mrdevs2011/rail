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
    // Overview zoom uses a frame-rate-independent hold loop rather than relying on the
    // desktop's keyboard-repeat delay (which may emit only one shortcut event).
    let zoom_repeat_key: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));
    let zoom_repeat_source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let a = app.clone();
    let held = held_shortcut.clone();
    let zoom_key = zoom_repeat_key.clone();
    let zoom_source = zoom_repeat_source.clone();
    keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let alt = state.contains(M::ALT_MASK);
        let super_key = state.contains(M::SUPER_MASK);

        // Ignore OS key-repeat events while our smooth overview zoom loop owns this key.
        if zoom_key.get() == Some(keycode) {
            return glib::Propagation::Stop;
        }

        // Omnibar / new-tab input: Esc must close it even if a shortcut map interferes
        if key == gdk::Key::Escape && a.palette_open() {
            a.cancel_palette();
            return glib::Propagation::Stop;
        }

        // Tugma qaysi amalga bog'langan (Settings > Customize > Shortcuts): standart ham, o'zgartirilgan ham
        let resolved = sc_resolve_any(&shortcut_combos(key, keycode, state));

        // Extension shortcut mode toggle (see App::toggle_extension_mode). Extension rejimida ham ishlashi kerak.
        if let ScResolve::Action("extension_mode") = resolved {
            if held.get() == Some(keycode) {
                return glib::Propagation::Stop;
            }
            a.toggle_extension_mode();
            held.set(Some(keycode));
            return glib::Propagation::Stop;
        }

        // Image paste: WebKitGTK gives pages an empty clipboardData for images
        if ctrl && !alt && !state.contains(M::SHIFT_MASK)
            && (key == gdk::Key::v || key == gdk::Key::V || keycode == 55)
            && a.try_paste_image()
        {
            return glib::Propagation::Stop;
        }

        // File paste: copied files (text/uri-list) are replayed into the page as File objects
        if ctrl && !alt && !state.contains(M::SHIFT_MASK)
            && (key == gdk::Key::v || key == gdk::Key::V || keycode == 55)
            && a.try_paste_files()
        {
            return glib::Propagation::Stop;
        }

        // Extension mode: Rail's own shortcuts are off, only extension commands are handled.
        // Everything else goes straight to the page.
        if crate::extensions::ext_keys_on() {
            if (ctrl || alt || super_key) && held.get() == Some(keycode) {
                return glib::Propagation::Stop;
            }
            if a.try_extension_command(key, state) {
                if ctrl || alt || super_key {
                    held.set(Some(keycode));
                }
                return glib::Propagation::Stop;
            }
            return glib::Propagation::Proceed;
        }

        // Autorepeat loop suppressor: block machine-gun repeat when shortcut is held down
        if ctrl || alt || super_key {
            if held.get() == Some(keycode) {
                return glib::Propagation::Stop;
            }
        }

        // Overview zoom hold direction must be captured before dispatch consumes the action.
        let overview_repeat_dir = if a.overview_open() {
            match &resolved {
                ScResolve::Action("overview_zoom_in") => Some(1.0f64),
                ScResolve::Action("overview_zoom_out") => Some(-1.0f64),
                _ => None,
            }
        } else {
            None
        };

        // Hamma amal ID bo'yicha bajariladi: tugma o'zgartirilgan bo'lsa ham xuddi shu yo'l.
        // `Unknown` — ro'yxatda yo'q tugma (Esc, overview ichidagi oddiy tugmalar, Shift+Enter ...).
        let handled = match resolved {
            ScResolve::Action(id) => a.run_action(id),
            ScResolve::Disabled => false,
            ScResolve::Unknown => a.on_key(key, keycode, state),
        };
        // Zoom va qidiruvda tugmani bosib turish qadam-baqadam ishlashi kerak (o'z tezlik chegarasi bor)
        let repeatable = matches!(
            resolved,
            ScResolve::Action("overview_zoom_in" | "overview_zoom_out" | "find_next" | "find_prev")
        );
        if handled {
            if (ctrl || alt || super_key) && !repeatable {
                held.set(Some(keycode));
            }
            if let Some(dir) = overview_repeat_dir {
                zoom_key.set(Some(keycode));
                let a_zoom = a.clone();
                let key_zoom = zoom_key.clone();
                let source = glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                    if key_zoom.get() != Some(keycode) {
                        return glib::ControlFlow::Break;
                    }
                    if a_zoom.overview_open() && !a_zoom.ov_exiting.get() {
                        let factor = if dir > 0.0 { 1.012 } else { 1.0 / 1.012 };
                        a_zoom.overview_zoom_by(factor);
                    }
                    glib::ControlFlow::Continue
                });
                *zoom_source.borrow_mut() = Some(source);
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    // Tugma qo'yib yuborilishi har doim ko'rinishi kerak: asosiy controller Bubble fazada, WebKit
    // release'ni "yutib" yuborsa `held` qotib qolib, keyingi bosish yo'qolib qolardi. Shuning uchun
    // release'ni alohida Capture controller kuzatadi.
    let release = gtk::EventControllerKey::new();
    release.set_propagation_phase(gtk::PropagationPhase::Capture);
    let held_rel = held_shortcut.clone();
    let zoom_key_rel = zoom_repeat_key.clone();
    let zoom_source_rel = zoom_repeat_source.clone();
    release.connect_key_released(move |_, _, keycode, _| {
        if held_rel.get() == Some(keycode) {
            held_rel.set(None);
        }
        if zoom_key_rel.get() == Some(keycode) {
            zoom_key_rel.set(None);
            if let Some(source) = zoom_source_rel.borrow_mut().take() {
                source.remove();
            }
        }
    });
    win.add_controller(release);

    let held_focus = held_shortcut.clone();
    let zoom_key_focus = zoom_repeat_key.clone();
    let zoom_source_focus = zoom_repeat_source.clone();
    win.connect_has_focus_notify(move |_| {
        held_focus.set(None);
        zoom_key_focus.set(None);
        if let Some(source) = zoom_source_focus.borrow_mut().take() {
            source.remove();
        }
    });
    win.add_controller(keys);
}
