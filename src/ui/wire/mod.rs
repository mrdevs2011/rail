//! Signal wiring for the main window, one concern per file.

mod minimap;
mod settings;
mod devtools;
mod toast;
mod palette;
mod gestures;
mod keys;
mod close;

pub(crate) use minimap::*;
pub(crate) use settings::*;
pub(crate) use devtools::*;
pub(crate) use toast::*;
pub(crate) use palette::*;
pub(crate) use gestures::*;
pub(crate) use keys::*;
pub(crate) use close::*;
