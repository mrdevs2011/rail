//! Keyboard handling.

use crate::*;

impl App {
    // ---- keys ------------------------------------------------------------

    pub(crate) fn on_key(self: &Rc<Self>, key: gdk::Key, keycode: u32, state: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let shift = state.contains(M::SHIFT_MASK);
        let alt = state.contains(M::ALT_MASK);
        let k = key.to_lower();

        // Overview is a read-only bird's-eye view: it comes first, before settings / devtools /
        // page handling, so arrows (and h/j/k/l, Enter, x, Esc) work WITHOUT Alt and never
        // reach any tab (including the settings tab). Ctrl/Alt/Super combos fall through.
        if self.overview_open() {
            if k == gdk::Key::Escape {
                self.cancel_overview();
                return true;
            }
            // Ctrl+/- (and keypad): scale the overview grid, not the page.
            if ctrl && !alt && !shift {
                if matches!(k, gdk::Key::plus | gdk::Key::KP_Add) {
                    self.overview_zoom_by(1.12);
                    return true;
                }
                if matches!(k, gdk::Key::minus | gdk::Key::KP_Subtract) {
                    self.overview_zoom_by(1.0 / 1.12);
                    return true;
                }
            }
            if !ctrl && !alt && !state.contains(M::SUPER_MASK) {
                self.ov_key(k, M::empty());
                return true;
            }
        }

        if self.settings_modal_open() {
            // Settings is a real tab (rail://settings) — Esc never closes it.
            // Ctrl+W / other browser shortcuts fall through below.
            if k == gdk::Key::Escape {
                return true; // swallow; do not hide
            }
            if alt && !shift && !ctrl && (k == gdk::Key::p || k == gdk::Key::P) {
                self.open_settings_tab(SettingsTab::Profiles);
                return true;
            }
            if alt && !shift && !ctrl && (k == gdk::Key::i || k == gdk::Key::I) {
                self.open_settings_tab(SettingsTab::Data);
                return true;
            }
            // Ctrl / Alt browser shortcuts: continue to global handlers (Ctrl+W closes tab)
            if ctrl || (alt && !shift) {
                // fall through
            } else {
                // Typing goes to settings form fields
                return false;
            }
        }

        if self.map_open() {
            if k == gdk::Key::Escape {
                self.hide_map();
                return true;
            }
            if alt && !shift && !ctrl && (k == gdk::Key::m || k == gdk::Key::M) {
                self.hide_map();
                return true;
            }
        }

        if self.devtools_open() {
            if k == gdk::Key::Escape {
                self.hide_devtools();
                return true;
            }
            if !ctrl && !shift && !alt && k == gdk::Key::F12 {
                self.hide_devtools();
                return true;
            }
        }

        if alt && shift && !ctrl && (k == gdk::Key::d || k == gdk::Key::D) {
            self.cycle_devtools_dock();
            return true;
        }

        if !ctrl && !shift && !alt && k == gdk::Key::F12 {
            self.toggle_devtools();
            return true;
        }

        if k == gdk::Key::Escape {
            if self.palette_open() {
                self.cancel_palette();
                return true;
            }
            if self.ov.get() {
                self.cancel_overview();
                return true;
            }
            return false;
        }
        if self.overview_open() && self.ov_key(k, state & gtk::accelerator_get_default_mod_mask()) {
            return true;
        }

        // Ctrl+Shift+R — hard refresh (bypass cache), Chrome/Brave bilan bir xil
        if ctrl && shift && !alt && (k == gdk::Key::r || keycode == 27) {
            if let Some(v) = self.focused_view() {
                hard_reload_or_start(&v);
            }
            return true;
        }

        // Ctrl+J — Downloads (Chrome parity)
        if ctrl && !shift && !alt && (k == gdk::Key::j || keycode == 44) {
            self.open_downloads();
            return true;
        }

        // Ctrl+Tab cycling and Ctrl+1..9 tab switching
        if ctrl && !shift && !alt {
            // Standart brauzer shortcutlari: Ctrl+T / Ctrl+L / Ctrl+W / Ctrl+R / Ctrl+Q
            // (keycode — boshqa klaviatura tartibida ham ishlashi uchun)
            if k == gdk::Key::t || keycode == 28 {
                self.open_new_tab_palette();
                return true;
            }
            if k == gdk::Key::l || keycode == 46 {
                self.show_palette();
                return true;
            }
            if k == gdk::Key::w || keycode == 25 {
                if !self.palette_open() {
                    if let Some(v) = self.focused_view() {
                        self.close_view(&v);
                    }
                }
                return true;
            }
            if k == gdk::Key::r || keycode == 27 {
                if let Some(v) = self.focused_view() {
                    reload_or_start(&v);
                }
                return true;
            }
            if k == gdk::Key::q || keycode == 24 {
                self.win.close();
                return true;
            }
            if k == gdk::Key::Tab || k == gdk::Key::ISO_Left_Tab || keycode == 23 {
                self.cycle_tab(1);
                return true;
            }
            // Tab switching within active workspace (Ctrl+1..8 to tabs 1..8, Ctrl+9 to last tab)
            if keycode == 10 || keycode == 87 || k == gdk::Key::_1 || k == gdk::Key::KP_1 || key.to_unicode() == Some('1') {
                self.focus_tab_index(0);
                return true;
            }
            if keycode == 11 || keycode == 88 || k == gdk::Key::_2 || k == gdk::Key::KP_2 || key.to_unicode() == Some('2') {
                self.focus_tab_index(1);
                return true;
            }
            if keycode == 12 || keycode == 89 || k == gdk::Key::_3 || k == gdk::Key::KP_3 || key.to_unicode() == Some('3') {
                self.focus_tab_index(2);
                return true;
            }
            if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                self.focus_tab_index(3);
                return true;
            }
            if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                self.focus_tab_index(4);
                return true;
            }
            if keycode == 15 || keycode == 92 || k == gdk::Key::_6 || k == gdk::Key::KP_6 || key.to_unicode() == Some('6') {
                self.focus_tab_index(5);
                return true;
            }
            if keycode == 16 || keycode == 93 || k == gdk::Key::_7 || k == gdk::Key::KP_7 || key.to_unicode() == Some('7') {
                self.focus_tab_index(6);
                return true;
            }
            if keycode == 17 || keycode == 94 || k == gdk::Key::_8 || k == gdk::Key::KP_8 || key.to_unicode() == Some('8') {
                self.focus_tab_index(7);
                return true;
            }
            if keycode == 18 || keycode == 95 || k == gdk::Key::_9 || k == gdk::Key::KP_9 || key.to_unicode() == Some('9') {
                self.focus_last_tab();
                return true;
            }
        }
        if ctrl && shift && !alt && (k == gdk::Key::ISO_Left_Tab || k == gdk::Key::Tab || keycode == 23) {
            self.cycle_tab(-1);
            return true;
        }
        // Ctrl+Shift+T: toggle Touchpad Slide (keycode 28 = physical T on any layout)
        if ctrl && shift && !alt && (k == gdk::Key::t || keycode == 28) {
            self.toggle_touchpad_mode();
            return true;
        }

        if !ctrl && !shift && !alt && k == gdk::Key::F11 {
            self.win.set_fullscreened(!self.win.is_fullscreen());
            return true;
        }

        // Alt + Arrow: navigate tabs (columns) and workspaces (rows) — locked shortcuts
        if alt && !shift && !ctrl {
            match k {
                gdk::Key::Left | gdk::Key::KP_Left => {
                    self.focus_column_left();
                    return true;
                }
                gdk::Key::Right | gdk::Key::KP_Right => {
                    self.focus_column_right();
                    return true;
                }
                gdk::Key::Up | gdk::Key::KP_Up => {
                    self.focus_row_up();
                    return true;
                }
                gdk::Key::Down | gdk::Key::KP_Down => {
                    self.focus_row_down();
                    return true;
                }
                gdk::Key::p | gdk::Key::P => {
                    if self.focused_is_welcome() && !self.settings_modal_open() {
                        self.toggle_welcome_profiles();
                    } else if self.palette_open() {
                        if self.profile_popover.is_visible() {
                            self.profile_popover.popdown();
                        } else {
                            self.profile_popover.popup();
                        }
                    } else {
                        self.toggle_profiles_modal();
                    }
                }
                gdk::Key::i | gdk::Key::I => self.toggle_data_modal(),
                gdk::Key::f => self.toggle_full(),
                gdk::Key::h => self.focus_column_left(),
                gdk::Key::l => self.focus_column_right(),
                gdk::Key::j => self.focus_row_down(),
                gdk::Key::k => self.focus_row_up(),
                gdk::Key::b => self.toggle_sidebar(),
                gdk::Key::o => self.toggle_overview(),
                gdk::Key::bracketleft => self.consume_or_expel(-1),
                gdk::Key::bracketright => self.consume_or_expel(1),
                gdk::Key::m => self.toggle_map(),
                gdk::Key::s | gdk::Key::S => {
                    self.toggle_settings_modal();
                    return true;
                }
                _ => {
                    if keycode == 39 {
                        self.toggle_settings_modal();
                        return true;
                    }
                    // Alt+[ / Alt+] (physical keys, so they work on any keyboard layout)
                    if keycode == 34 {
                        self.consume_or_expel(-1);
                        return true;
                    }
                    if keycode == 35 {
                        self.consume_or_expel(1);
                        return true;
                    }
                    // Vertical workspace selection (Alt+1..9)
                    if keycode == 10 || keycode == 87 || k == gdk::Key::_1 || k == gdk::Key::KP_1 || key.to_unicode() == Some('1') {
                        self.focus_row(0);
                        return true;
                    }
                    if keycode == 11 || keycode == 88 || k == gdk::Key::_2 || k == gdk::Key::KP_2 || key.to_unicode() == Some('2') {
                        self.focus_row(1);
                        return true;
                    }
                    if keycode == 12 || keycode == 89 || k == gdk::Key::_3 || k == gdk::Key::KP_3 || key.to_unicode() == Some('3') {
                        self.focus_row(2);
                        return true;
                    }
                    if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                        self.focus_row(3);
                        return true;
                    }
                    if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                        self.focus_row(4);
                        return true;
                    }
                    if keycode == 15 || keycode == 92 || k == gdk::Key::_6 || k == gdk::Key::KP_6 || key.to_unicode() == Some('6') {
                        self.focus_row(5);
                        return true;
                    }
                    if keycode == 16 || keycode == 93 || k == gdk::Key::_7 || k == gdk::Key::KP_7 || key.to_unicode() == Some('7') {
                        self.focus_row(6);
                        return true;
                    }
                    if keycode == 17 || keycode == 94 || k == gdk::Key::_8 || k == gdk::Key::KP_8 || key.to_unicode() == Some('8') {
                        self.focus_row(7);
                        return true;
                    }
                    if keycode == 18 || keycode == 95 || k == gdk::Key::_9 || k == gdk::Key::KP_9 || key.to_unicode() == Some('9') {
                        self.focus_row(8);
                        return true;
                    }
                    return false;
                }
            }
            return true;
        }

        if alt && shift && !ctrl {
            match k {
                gdk::Key::t | gdk::Key::T => self.toggle_touchpad_mode(),
                gdk::Key::v | gdk::Key::V => self.split_vertical(),
                gdk::Key::h => self.move_column(-1),
                gdk::Key::l => self.move_column(1),
                gdk::Key::j => self.move_window_vertical(1),
                gdk::Key::k => self.move_window_vertical(-1),
                _ => {
                    // Alt+Shift+1 -> move to workspace/row 1
                    if keycode == 10 || keycode == 87
                        || k == gdk::Key::_1 || k == gdk::Key::exclam || k == gdk::Key::KP_1
                        || key == gdk::Key::_1 || key == gdk::Key::exclam || key == gdk::Key::KP_1
                        || key.to_unicode() == Some('1') || key.to_unicode() == Some('!')
                    {
                        self.move_tab_to_row(0);
                        return true;
                    }
                    // Alt+Shift+2 -> move to workspace/row 2
                    if keycode == 11 || keycode == 88
                        || k == gdk::Key::_2 || k == gdk::Key::at || k == gdk::Key::quotedbl || k == gdk::Key::KP_2
                        || key == gdk::Key::_2 || key == gdk::Key::at || key == gdk::Key::quotedbl || key == gdk::Key::KP_2
                        || key.to_unicode() == Some('2') || key.to_unicode() == Some('@') || key.to_unicode() == Some('"')
                    {
                        self.move_tab_to_row(1);
                        return true;
                    }
                    // Alt+Shift+3 -> move to workspace/row 3
                    if keycode == 12 || keycode == 89
                        || k == gdk::Key::_3 || k == gdk::Key::numbersign || k == gdk::Key::KP_3
                        || key == gdk::Key::_3 || key == gdk::Key::numbersign || key == gdk::Key::KP_3
                        || key.to_unicode() == Some('3') || key.to_unicode() == Some('#')
                    {
                        self.move_tab_to_row(2);
                        return true;
                    }
                    // Alt+Shift+4 -> move to workspace/row 4
                    if keycode == 13 || keycode == 90 || k == gdk::Key::_4 || k == gdk::Key::KP_4 || key.to_unicode() == Some('4') {
                        self.move_tab_to_row(3);
                        return true;
                    }
                    // Alt+Shift+5 -> move to workspace/row 5
                    if keycode == 14 || keycode == 91 || k == gdk::Key::_5 || k == gdk::Key::KP_5 || key.to_unicode() == Some('5') {
                        self.move_tab_to_row(4);
                        return true;
                    }
                    // Alt+Shift+6 -> move to workspace/row 6
                    if keycode == 15 || keycode == 92 || k == gdk::Key::_6 || k == gdk::Key::asciicircum || k == gdk::Key::KP_6 || key.to_unicode() == Some('6') || key.to_unicode() == Some('^') {
                        self.move_tab_to_row(5);
                        return true;
                    }
                    // Alt+Shift+7 -> move to workspace/row 7
                    if keycode == 16 || keycode == 93 || k == gdk::Key::_7 || k == gdk::Key::ampersand || k == gdk::Key::KP_7 || key.to_unicode() == Some('7') || key.to_unicode() == Some('&') {
                        self.move_tab_to_row(6);
                        return true;
                    }
                    // Alt+Shift+8 -> move to workspace/row 8
                    if keycode == 17 || keycode == 94 || k == gdk::Key::_8 || k == gdk::Key::asterisk || k == gdk::Key::KP_8 || key.to_unicode() == Some('8') || key.to_unicode() == Some('*') {
                        self.move_tab_to_row(7);
                        return true;
                    }
                    // Alt+Shift+9 -> move to workspace/row 9
                    if keycode == 18 || keycode == 95 || k == gdk::Key::_9 || k == gdk::Key::parenleft || k == gdk::Key::KP_9 || key.to_unicode() == Some('9') || key.to_unicode() == Some('(') {
                        self.move_tab_to_row(8);
                        return true;
                    }
                    return false;
                }
            }
            return true;
        }
        false
    }
}
