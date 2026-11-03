//! Address palette: pill, entry typing, keys, activation and tab picking.

use crate::*;

pub(crate) fn wire_palette(app: &Rc<App>, entry: gtk::Entry, pill: gtk::Button) {
    let a = app.clone();
    pill.connect_clicked(move |_| a.show_palette());

    let a = app.clone();
    entry.connect_changed(move |_| {
        a.handle_entry_changed();
    });

    let entry_keys = gtk::EventControllerKey::new();
    entry_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let a = app.clone();
    let ent = entry.clone();
    entry_keys.connect_key_pressed(move |_, key, keycode, state| {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let shift = state.contains(M::SHIFT_MASK);
        let alt = state.contains(M::ALT_MASK);
        let k = key.to_lower();

        a.delete_key.set(
            matches!(key, gdk::Key::BackSpace | gdk::Key::Delete)
                || (ctrl
                    && !alt
                    && (k == gdk::Key::x
                        || k == gdk::Key::w
                        || k == gdk::Key::u
                        || k == gdk::Key::k
                        || k == gdk::Key::h)),
        );

        if ctrl && !shift && !alt && (k == gdk::Key::v || k == gdk::Key::V || keycode == 55) {
            let clipboard = ent.clipboard();
            let entry_clone = ent.clone();
            clipboard.read_text_async(gio::Cancellable::NONE, move |res| {
                if let Ok(Some(text)) = res {
                    let text = text.to_string();
                    let (start, end) = entry_clone.selection_bounds().unwrap_or_else(|| {
                        let pos = entry_clone.position();
                        (pos, pos)
                    });
                    entry_clone.delete_text(start, end);
                    let mut insert_pos = start;
                    entry_clone.insert_text(&text, &mut insert_pos);
                    entry_clone.set_position(insert_pos);
                }
            });
            return glib::Propagation::Stop;
        }

        match key {
            gdk::Key::Escape => {
                // New-tab / omnibar input: Esc dismisses the palette
                if a.palette_open() {
                    a.cancel_palette();
                    return glib::Propagation::Stop;
                }
            }
            gdk::Key::Down => {
                if a.move_suggestion_selection(1) {
                    return glib::Propagation::Stop;
                }
            }
            gdk::Key::Up => {
                if a.move_suggestion_selection(-1) {
                    return glib::Propagation::Stop;
                }
            }
            gdk::Key::Tab if a.accept_autofill_or_next_suggestion() => {
                return glib::Propagation::Stop;
            }
            _ => {}
        }
        glib::Propagation::Proceed
    });
    entry.add_controller(entry_keys);

    let a = app.clone();
    entry.connect_activate(move |e| {
        if let Some(idx) = a.selected_suggestion_idx.get() {
            let sugs = a.current_suggestions.borrow();
            if let Some(sug) = sugs.get(idx) {
                let target = sug.target_url.clone();
                drop(sugs);
                a.navigate(&target);
                return;
            }
        }
        a.navigate(&e.text());
    });

    let pick = gtk::GestureClick::new();
    let a = app.clone();
    pick.connect_released(move |_, _, x, y| {
        if a.ov.get() {
            a.ov_click(x, y);
        }
    });
    app.strip.add_controller(pick);
}
