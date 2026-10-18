//! Widget trees for the main window, built before the App exists.

mod sidebar;
mod strip;
mod palette;
mod hints;
mod minimap;
mod devtools;

pub(crate) use sidebar::*;
pub(crate) use strip::*;
pub(crate) use palette::*;
pub(crate) use hints::*;
pub(crate) use minimap::*;
pub(crate) use devtools::*;
