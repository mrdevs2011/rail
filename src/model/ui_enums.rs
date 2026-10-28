//! Small enums shared by settings and DevTools.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DevToolsDockSide {
    Bottom,
    Left,
    Right,
    Undocked,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Profiles,
    Extensions,
    Data,
    Privacy,
    Customize,
    Update,
    About,
}
