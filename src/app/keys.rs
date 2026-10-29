//! Keyboard handling.
//!
//! Ikki qism:
//!  * `run_action(id)` — Settings > Customize > Shortcuts ro'yxatidagi har bir amal ID bo'yicha.
//!    Tugma default bo'lsa ham, foydalanuvchi o'zgartirgan bo'lsa ham shu yerga keladi
//!    (tugma → ID moslashtirishni `sc_resolve` qiladi), shuning uchun hech narsa bitta tugmaga bog'lanmagan.
//!  * `on_key` — ro'yxatdan tashqari qat'iy tugmalar: Esc, overview ichidagi oddiy tugmalar,
//!    kengaytma buyruqlari, qidiruv paneli (Shift+Enter).

use crate::*;

/// Customize ro'yxatidagi raqamsiz amallar. Raqamlilari: tab_1..9, ws_1..9, mv_1..9.
/// Yangi `sc!` qo'shilsa, bu yerga va `run_action` ga ham qo'shiladi (test shuni tekshiradi).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const STATIC_ACTIONS: &[&str] = &[
    "new_tab", "focus_url", "close_tab", "reload", "hard_reload", "full_tab", "new_workspace",
    "focus_left", "focus_right", "focus_down", "focus_up",
    "win_left", "win_right", "win_up", "win_down",
    "move_left", "move_right", "move_down", "move_up",
    "next_tab", "prev_tab", "stack_left", "stack_right",
    "settings", "profiles", "data", "devtools", "devtools_dock", "map", "overview",
    "overview_zoom_in", "overview_zoom_out", "zoom_reset",
    "sidebar", "downloads", "fullscreen", "quit",
    "reopen_tab", "history", "open_file", "view_source",
    "find", "find_next", "find_prev", "extension_mode",
];

impl App {
    // ---- zoom ------------------------------------------------------------

    /// Ctrl zoom keys: `Some(1)` in, `Some(-1)` out, `Some(0)` reset.
    /// `=` counts as plus (US layout: Ctrl+= without Shift); keypad +/- work too.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn zoom_dir(k: gdk::Key, keycode: u32, shift: bool) -> Option<i32> {
        if matches!(k, gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add) {
            return Some(1);
        }
        if !shift && matches!(k, gdk::Key::minus | gdk::Key::KP_Subtract) {
            return Some(-1);
        }
        // keycode 19 = physical 0 (digit row), like Ctrl+1..9 above
        if matches!(k, gdk::Key::_0 | gdk::Key::KP_0) || keycode == 19 {
            return Some(0);
        }
        None
    }

    /// Zoom the focused tab: `dir` > 0 in, < 0 out, 0 reset to 100%.
    /// Always base the next step on WebKit's live zoom level so pinch gestures and keyboard
    /// zoom stay in sync. Opposite directions are never dropped by a shared debounce timer.
    pub(crate) fn page_zoom(self: &Rc<Self>, dir: i32) {
        let Some(v) = self.focused_view() else { return };
        let cur = v.zoom_level();
        let z = if dir == 0 { 1.0 } else { next_page_zoom(cur, dir) };
        v.set_zoom_level(z);
    }

    // ---- actions ---------------------------------------------------------

    /// `tab_3` → Some(3) (faqat 1..=9).
    fn action_num(id: &str, prefix: &str) -> Option<usize> {
        id.strip_prefix(prefix)?.parse::<usize>().ok().filter(|n| (1..=9).contains(n))
    }

    /// Customize ro'yxatidagi `id` uchun handler bormi?
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn action_known(id: &str) -> bool {
        STATIC_ACTIONS.contains(&id) || ["tab_", "ws_", "mv_"].iter().any(|p| Self::action_num(id, p).is_some())
    }

    /// Shortcut amalini bajaradi. `false` — tugma ishlatilmadi va sahifaga o'tishi kerak.
    pub(crate) fn run_action(self: &Rc<Self>, id: &str) -> bool {
        if let Some(n) = Self::action_num(id, "tab_") {
            if n == 9 {
                self.focus_last_tab();
            } else {
                self.focus_tab_index(n - 1);
            }
            return true;
        }
        if let Some(n) = Self::action_num(id, "ws_") {
            self.focus_row(n - 1);
            return true;
        }
        if let Some(n) = Self::action_num(id, "mv_") {
            self.move_tab_to_row(n - 1);
            return true;
        }
        match id {
            // ---- tabs ----
            "new_tab" => self.open_new_tab_palette(),
            "focus_url" => self.show_palette(),
            "close_tab" => {
                if !self.palette_open() {
                    if let Some(v) = self.focused_view() {
                        self.close_view(&v);
                    }
                }
            }
            "reload" => {
                if let Some(v) = self.focused_view() {
                    reload_or_start(&v);
                }
            }
            "hard_reload" => {
                if let Some(v) = self.focused_view() {
                    hard_reload_or_start(&v);
                }
            }
            "reopen_tab" => self.reopen_closed_tab(),
            "next_tab" => self.cycle_tab(1),
            "prev_tab" => self.cycle_tab(-1),
            // ---- layout (niri-style) ----
            "full_tab" => self.toggle_full(),
            "new_workspace" => self.split_vertical(),
            "focus_left" | "win_left" => self.focus_column_left(),
            "focus_right" | "win_right" => self.focus_column_right(),
            "focus_up" | "win_up" => self.focus_row_up(),
            "focus_down" | "win_down" => self.focus_row_down(),
            "move_left" => self.move_column(-1),
            "move_right" => self.move_column(1),
            "move_up" => self.move_window_vertical(-1),
            "move_down" => self.move_window_vertical(1),
            "stack_left" => self.consume_or_expel(-1),
            "stack_right" => self.consume_or_expel(1),
            // ---- panels ----
            "settings" => self.toggle_settings_modal(),
            "profiles" => {
                if self.settings_modal_open() {
                    self.open_settings_tab(SettingsTab::Profiles);
                } else if self.focused_is_welcome() {
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
            "data" => {
                if self.settings_modal_open() {
                    self.open_settings_tab(SettingsTab::Data);
                } else {
                    self.toggle_data_modal();
                }
            }
            "history" => self.open_history(),
            "downloads" => self.open_downloads(),
            "open_file" => self.open_file_dialog(),
            "view_source" => self.view_source_of_focused(),
            "devtools" => {
                if self.devtools_open() {
                    self.hide_devtools();
                } else {
                    self.toggle_devtools();
                }
            }
            "devtools_dock" => self.cycle_devtools_dock(),
            "map" => {
                if self.map_open() {
                    self.hide_map();
                } else {
                    self.toggle_map();
                }
            }
            "overview" => self.toggle_overview(),
            "sidebar" => self.toggle_sidebar(),
            "fullscreen" => self.win.set_fullscreened(!self.win.is_fullscreen()),
            "quit" => self.win.close(),
            // ---- zoom: overview ichida grid, aks holda sahifa ----
            "overview_zoom_in" => {
                if self.overview_open() {
                    self.overview_zoom_by(1.06);
                } else {
                    self.page_zoom(1);
                }
            }
            "overview_zoom_out" => {
                if self.overview_open() {
                    self.overview_zoom_by(1.0 / 1.06);
                } else {
                    self.page_zoom(-1);
                }
            }
            "zoom_reset" => {
                if self.overview_open() {
                    self.overview_zoom_by(OV_ZOOM / self.target_zoom.get());
                } else {
                    self.page_zoom(0);
                }
            }
            "extension_mode" => self.toggle_extension_mode(),
            // ---- find in page ----
            "find" => return self.find_open(),
            "find_next" => return self.find_step(true),
            "find_prev" => return self.find_step(false),
            _ => return false,
        }
        true
    }

    // ---- fixed keys ------------------------------------------------------

    /// Customize ro'yxatiga kirmaydigan qat'iy tugmalar: Esc, overview ichidagi oddiy tugmalar,
    /// kengaytma buyruqlari, Shift+Enter (qidiruvda oldingi natija).
    pub(crate) fn on_key(self: &Rc<Self>, key: gdk::Key, _keycode: u32, state: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
        let ctrl = state.contains(M::CONTROL_MASK);
        let shift = state.contains(M::SHIFT_MASK);
        let alt = state.contains(M::ALT_MASK);
        let k = key.to_lower();

        // Overview is a read-only bird's-eye view: plain keys (arrows, h/j/k/l, Enter, x, Esc)
        // work WITHOUT Alt and never reach any tab. Ctrl/Alt/Super combos are shortcuts (run_action).
        if self.overview_open() {
            if k == gdk::Key::Escape {
                self.cancel_overview();
                return true;
            }
            if !ctrl && !alt && !state.contains(M::SUPER_MASK) {
                self.ov_key(k, M::empty());
                return true;
            }
        }

        // Extension commands (e.g. Quick Open Ctrl+Comma, Crystal Text Alt+F)
        if self.try_extension_command(key, state) {
            return true;
        }

        // Find bar: Esc yopadi, Shift+Enter — oldingi natija
        if self.find_is_open() && !self.settings_modal_open() {
            if k == gdk::Key::Escape {
                self.find_close(true);
                return true;
            }
            let enter = matches!(k, gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter);
            if enter && shift && !ctrl && !alt {
                self.find_step(false);
                return true;
            }
        }

        if self.settings_modal_open() {
            // Settings is a real tab (rail://settings) — Esc never closes it.
            if k == gdk::Key::Escape {
                return true; // swallow; do not hide
            }
            // Typing goes to settings form fields
            return false;
        }

        if self.map_open() && k == gdk::Key::Escape {
            self.hide_map();
            return true;
        }

        if self.devtools_open() && k == gdk::Key::Escape {
            self.hide_devtools();
            return true;
        }

        if k == gdk::Key::Escape {
            if self.qr_modal_open() {
                self.hide_qr_modal();
                return true;
            }
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
        false
    }
}
