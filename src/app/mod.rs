//! The browser window state; behaviour lives in the submodules.

use crate::*;

mod core;
pub(crate) mod devtools;
mod scroll;
mod tabs;
pub(crate) use devtools::*;
#[cfg(test)]
pub(crate) use tabs::is_external_uri_scheme;
mod data_actions;
mod dialogs;
mod focus;
mod keys;
mod lifecycle;
mod load_bar;
mod minimap;
mod moving;
mod overview;
mod palette;
mod profiles_modal;
mod session;
mod settings_modal;
mod sidebar;
mod signals;
mod welcome;
mod window_layout;
#[cfg(test)]
pub(crate) use keys::STATIC_ACTIONS;
mod ctxmenu;
mod find;
mod screenshot;
mod translate;
mod zoom;

pub(crate) struct App {
    pub(crate) win: gtk::ApplicationWindow,
    pub(crate) shell: gtk::Box,
    pub(crate) shell_layout: ShellLayout,
    pub(crate) side: gtk::Box,
    /// Compact sidebar launcher shown when the Niri column is narrower than the dock.
    pub(crate) responsive_sidebar_toggle: gtk::Button,
    pub(crate) responsive_layout: Cell<bool>,
    /// Square Rail logo in sidebar header (fixed mode only).
    pub(crate) brand_logo: gtk::Revealer,
    pub(crate) strip: gtk::Box,
    pub(crate) layout: StripLayout,
    pub(crate) list: gtk::ListBox,
    pub(crate) pill_label: gtk::Label,
    pub(crate) pill_engine_icon: gtk::Image,
    pub(crate) pill_pwa_btn: gtk::Button,
    pub(crate) load_bar: gtk::ProgressBar,
    /// Browser-style bottom-left URL preview for the currently hovered link.
    pub(crate) hovered_link_label: gtk::Label,
    pub(crate) palette: gtk::Box,
    pub(crate) palette_engine_icon: gtk::Image,
    pub(crate) entry: gtk::Entry,
    pub(crate) palette_for_new_tab: Cell<bool>,
    /// True while the palette is pinned under the brand on a welcome (rail://start) tab.
    pub(crate) welcome_palette: Cell<bool>,
    pub(crate) palette_suggestions_box: gtk::Box,
    pub(crate) history: RefCell<HistoryManager>,
    pub(crate) current_suggestions: RefCell<Vec<HistorySuggestion>>,
    pub(crate) selected_suggestion_idx: Cell<Option<usize>>,
    pub(crate) is_autofilling: Cell<bool>,
    pub(crate) last_entry_len: Cell<usize>,
    /// Oxirgi bosilgan tugma o'chirish (Backspace/Delete/Ctrl+X...) bo'lganmi: shunda inline to'ldirish ishlamaydi
    pub(crate) delete_key: Cell<bool>,
    pub(crate) ov_hint: gtk::Label,
    /// Small, non-interactive zoom percentage indicator anchored at the window's top-right.
    pub(crate) zoom_indicator: gtk::Revealer,
    pub(crate) zoom_indicator_label: gtk::Label,
    pub(crate) zoom_indicator_timer: RefCell<Option<glib::SourceId>>,
    /// Sahifa ichida qidirish paneli (Ctrl+F); birinchi ochilganda yaratiladi.
    pub(crate) find_ui: RefCell<Option<Rc<find::FindUi>>>,
    /// Overview is active: the strip is zoomed out and tabs ignore the pointer.
    pub(crate) ov: Cell<bool>,
    /// Zoom-in from overview is running; CSS/ov stay until the camera lands.
    pub(crate) ov_exiting: Cell<bool>,
    /// Last overview scale (pinch); reused next time overview opens.
    pub(crate) ov_user_zoom: Cell<f64>,
    /// Tab coordinates (row, col) that were focused when overview opened; Esc returns here.
    pub(crate) ov_return: Cell<(usize, usize)>,
    /// Scroll position before overview, restored when zooming back in.
    pub(crate) ov_saved_x: Cell<f64>,
    pub(crate) ov_saved_y: Cell<f64>,
    pub(crate) target_zoom: Cell<f64>,
    pub(crate) rows: RefCell<Vec<Row>>,
    pub(crate) focus_row: Cell<usize>,
    pub(crate) focus_col: Cell<usize>,
    /// Workspace visited immediately before the current one (Niri focus-workspace-previous).
    pub(crate) previous_workspace: Cell<Option<usize>>,
    /// Last focused tab index per workspace row (restored when moving up/down between rows).
    pub(crate) row_last_col: RefCell<std::collections::HashMap<usize, usize>>,
    pub(crate) target_x: Cell<f64>,
    pub(crate) target_y: Cell<f64>,
    pub(crate) animating: Cell<bool>,
    /// Niri-style springs for the camera; `None` while at rest.
    pub(crate) cam_spring_x: Cell<Option<CamSpring>>,
    pub(crate) cam_spring_y: Cell<Option<CamSpring>>,
    /// Release velocity (px/s) of a touchpad flick, consumed when the next x spring starts.
    /// Animatsiya paytida freeze-kadrga almashtirilgan tablar; animatsiya tugagach jonliga qaytariladi.
    pub(crate) anim_frozen: RefCell<Vec<(gtk::Picture, WebView)>>,
    pub(crate) side_open: Cell<bool>,
    /// true = edge-hover auto mode; false = fixed (Claude-style always visible when open).
    pub(crate) sidebar_auto: Cell<bool>,
    /// When true (e.g. a modal dialog is open), sidebar auto-close is locked/prevented.
    pub(crate) sidebar_locked: Cell<bool>,
    pub(crate) side_gen: Cell<u32>,
    pub(crate) side_open_timer: RefCell<Option<glib::SourceId>>,
    pub(crate) side_close_timer: RefCell<Option<glib::SourceId>>,
    /// A just-created tab that has never navigated; Esc discards it.
    pub(crate) fresh: RefCell<Option<WebView>>,
    // Profiles
    pub(crate) profiles: RefCell<ProfileManager>,
    pub(crate) selected_palette_profile: RefCell<String>,
    pub(crate) profile_btn: gtk::Button,
    pub(crate) profile_btn_lbl: gtk::Label,
    pub(crate) profile_popover: gtk::Popover,
    pub(crate) profiles_modal_scroll: gtk::ScrolledWindow,
    pub(crate) profiles_list_box: gtk::ListBox,
    pub(crate) new_profile_entry: gtk::Entry,
    pub(crate) new_profile_error_lbl: gtk::Label,
    pub(crate) add_prof_btn: gtk::Button,
    pub(crate) new_profile_color_idx: Cell<usize>,
    pub(crate) color_buttons: RefCell<Vec<gtk::Button>>,
    pub(crate) last_saved_session: RefCell<String>,
    pub(crate) save_session_gen: Cell<u64>,
    pub(crate) data_modal_info_lbl: gtk::Label,
    pub(crate) data_modal_status_lbl: gtk::Label,
    pub(crate) data_export_entry: gtk::Entry,
    pub(crate) data_import_entry: gtk::Entry,
    pub(crate) sync_url_result_entry: gtk::Entry,
    pub(crate) sync_restore_entry: gtk::Entry,
    // Minimap / Roadmap (Alt+M)
    pub(crate) map_container: gtk::Box,
    pub(crate) map_drawing_area: gtk::DrawingArea,
    pub(crate) map_location_btn: gtk::Button,
    pub(crate) map_state: RefCell<MinimapState>,
    // DevTools Panel (F12 / Alt+Shift+I / Alt+Shift+D)
    pub(crate) devtools_open: Cell<bool>,
    pub(crate) default_devtools_height: Cell<i32>,
    pub(crate) default_devtools_width: Cell<i32>,
    pub(crate) devtools_dock_side: Cell<DevToolsDockSide>,
    pub(crate) devtools_panel: gtk::Box,
    pub(crate) devtools_window: RefCell<Option<gtk::Window>>,
    // Floating Settings Panel (Alt+Shift+S / Alt+P / Alt+I)
    pub(crate) qr_modal: RefCell<Option<gtk::Box>>,
    pub(crate) settings_modal: gtk::Box,
    pub(crate) settings_body: gtk::Box,
    pub(crate) settings_sidebar: gtk::Box,
    pub(crate) settings_sidebar_scroll: gtk::ScrolledWindow,
    pub(crate) settings_stack: gtk::Stack,
    pub(crate) settings_tab_buttons: RefCell<Vec<(SettingsTab, gtk::Button)>>,
    pub(crate) current_settings_tab: Cell<SettingsTab>,
    pub(crate) css_buffer: gtk::TextBuffer,
    pub(crate) search_engine: RefCell<String>,
    pub(crate) recently_closed: RefCell<Vec<(String, String)>>,
}
