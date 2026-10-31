//! Signal wiring for the main window, one concern per file.

mod close;
mod devtools;
mod gestures;
mod keys;
mod minimap;
mod palette;
mod settings;
mod sidebar_header;

pub(crate) use close::*;
pub(crate) use devtools::*;
pub(crate) use gestures::*;
pub(crate) use keys::*;
pub(crate) use minimap::*;
pub(crate) use palette::*;
pub(crate) use settings::*;
pub(crate) use sidebar_header::*;
