//! The rail://start welcome page.

use crate::*;

pub(crate) const START_PAGE: &str = include_str!("../start_page.html");

thread_local! {
    /// Sidebar widgets from the last `refresh_sidebar()`: workspace headers and tab rows.
    /// Lets overview navigation flip the active highlight without rebuilding the whole list.
    pub(crate) static SIDEBAR_ITEMS: RefCell<(Vec<(usize, gtk::Box)>, Vec<(usize, usize, gtk::Box)>)> =
        RefCell::new((Vec::new(), Vec::new()));
}

/// Boshlang'ich sahifa: til, sarlavha, tavsif va qidiruv placeholder joriy tilga o'giriladi.
pub(crate) fn start_page() -> String {
    START_PAGE
        .replacen("<html lang=\"en\">", &format!("<html lang=\"{}\">", i18n::current().code()), 1)
        .replacen("fast pages many workspaces with RAIL", tr("fast pages many workspaces with RAIL"), 1)
}
