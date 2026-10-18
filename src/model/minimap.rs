//! State of the minimap / roadmap overlay.

pub(crate) struct MinimapState {
    pub(crate) cam_x: f64,
    pub(crate) cam_y: f64,
    pub(crate) zoom: f64,
    pub(crate) target_zoom: f64,
    pub(crate) offset_x: f64,
    pub(crate) offset_y: f64,
    pub(crate) target_offset_x: f64,
    pub(crate) target_offset_y: f64,
    pub(crate) user_panned: bool,
    pub(crate) drag_start_offset_x: f64,
    pub(crate) drag_start_offset_y: f64,
    pub(crate) vel_x: f64,
    pub(crate) vel_y: f64,
    pub(crate) cursor_x: f64,
    pub(crate) cursor_y: f64,
    pub(crate) cursor_w: f64,
    pub(crate) cursor_h: f64,
    pub(crate) cursor_initialized: bool,
    pub(crate) animating: bool,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            target_zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            target_offset_x: 0.0,
            target_offset_y: 0.0,
            user_panned: false,
            drag_start_offset_x: 0.0,
            drag_start_offset_y: 0.0,
            vel_x: 0.0,
            vel_y: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_w: MAP_TAB_W,
            cursor_h: MAP_TAB_H,
            cursor_initialized: false,
            animating: false,
        }
    }
}

pub(crate) const MAP_TAB_W: f64 = 28.0;
pub(crate) const MAP_TAB_W_FULL: f64 = 46.0;
pub(crate) const MAP_TAB_H: f64 = 18.0;
pub(crate) const MAP_GAP_X: f64 = 6.0;
pub(crate) const MAP_GAP_Y: f64 = 6.0;
