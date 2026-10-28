//! The browser window state; behaviour lives in the submodules.

use crate::*;

mod core;
mod scroll;
mod tabs;
mod devtools;
mod dialogs;
mod signals;
mod lifecycle;
mod focus;
mod moving;
mod session;
mod sidebar;
mod load_bar;
mod welcome;
mod palette;
mod settings_modal;
mod data_actions;
mod minimap;
mod profiles_modal;
mod overview;
mod keys;
mod translate;
mod ctxmenu;
mod screenshot;

pub(crate) struct App {
    pub(crate) win: gtk::ApplicationWindow,
    pub(crate) shell: gtk::Box,
    pub(crate) shell_layout: ShellLayout,
    pub(crate) side: gtk::Box,
    /// Square Rail logo in sidebar header (fixed mode only).
    pub(crate) brand_logo: gtk::Revealer,
    pub(crate) strip: gtk::Box,
    pub(crate) layout: StripLayout,
    pub(crate) list: gtk::ListBox,
    pub(crate) pill_label: gtk::Label,
    pub(crate) pill_engine_icon: gtk::Image,
    pub(crate) load_bar: gtk::ProgressBar,
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
    /// Overview is active: the strip is zoomed out and tabs ignore the pointer.
    pub(crate) ov: Cell<bool>,
    /// Zoom-in from overview is running; CSS/ov stay until the camera lands.
    pub(crate) ov_exiting: Cell<bool>,
    /// Last overview scale (Ctrl+/- / pinch); reused next time overview opens.
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
    /// Last focused tab index per workspace row (restored when moving up/down between rows).
    pub(crate) row_last_col: RefCell<std::collections::HashMap<usize, usize>>,
    pub(crate) target_x: Cell<f64>,
    pub(crate) target_y: Cell<f64>,
    pub(crate) animating: Cell<bool>,
    /// Touchpad slide: at most one strip allocate per frame (avoids jank)
    pub(crate) pan_alloc_pending: Cell<bool>,
    /// Niri-style springs for the camera; `None` while at rest.
    pub(crate) cam_spring_x: Cell<Option<CamSpring>>,
    pub(crate) cam_spring_y: Cell<Option<CamSpring>>,
    /// Release velocity (px/s) of a touchpad flick, consumed when the next x spring starts.
    pub(crate) cam_v0_x: Cell<f64>,
    /// Animatsiya paytida freeze-kadrga almashtirilgan tablar; animatsiya tugagach jonliga qaytariladi.
    pub(crate) anim_frozen: RefCell<Vec<(gtk::Picture, WebView)>>,
    pub(crate) side_open: Cell<bool>,
    /// true = edge-hover auto mode; false = fixed (Claude-style always visible when open).
    pub(crate) sidebar_auto: Cell<bool>,
    pub(crate) side_gen: Cell<u32>,
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
    /// Active screenshot overlay (Zen-style); removed on capture/cancel.
    pub(crate) ss_overlay: RefCell<Option<gtk::Overlay>>,
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
    pub(crate) devtools_dock_side: Cell<DevToolsDockSide>,
    pub(crate) devtools_panel: gtk::Box,
    pub(crate) devtools_content_box: gtk::Box,
    pub(crate) devtools_resize_handle: gtk::Box,
    pub(crate) devtools_url_lbl: gtk::Label,
    pub(crate) devtools_window: RefCell<Option<gtk::Window>>,
    // Floating Settings Panel (Alt+Shift+S / Alt+P / Alt+I)
    pub(crate) settings_modal: gtk::Box,
    pub(crate) settings_body: gtk::Box,
    pub(crate) settings_sidebar: gtk::Box,
    pub(crate) settings_sidebar_scroll: gtk::ScrolledWindow,
    pub(crate) settings_stack: gtk::Stack,
    pub(crate) settings_tab_buttons: RefCell<Vec<(SettingsTab, gtk::Button)>>,
    pub(crate) current_settings_tab: Cell<SettingsTab>,
    pub(crate) css_buffer: gtk::TextBuffer,
    pub(crate) search_engine: RefCell<String>,
}
