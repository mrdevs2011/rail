//! The pages of the Settings tab, one file each.

mod general;
mod customize;
mod profiles;
mod data;
mod privacy;
mod updates;
mod about;

pub(crate) use general::*;
pub(crate) use customize::*;
pub(crate) use profiles::*;
pub(crate) use data::*;
pub(crate) use privacy::*;
pub(crate) use updates::*;
pub(crate) use about::*;
