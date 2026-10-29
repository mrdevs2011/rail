//! Overview hint and page load bar.

use crate::*;

pub(crate) struct HintWidgets {
    pub(crate) load_bar: gtk::ProgressBar,
    pub(crate) ov_hint: gtk::Label,
}

pub(crate) fn build_hints() -> HintWidgets {
    // ---- overview hint ----
    let ov_hint = gtk::Label::new(Some(tr(
        "h/l/j/k  move      Enter  open      x  close      Esc  cancel",
    )));
    ov_hint.add_css_class("ov-hint");
    ov_hint.set_halign(gtk::Align::Center);
    ov_hint.set_valign(gtk::Align::End);
    ov_hint.set_margin_bottom(28);
    ov_hint.set_can_target(false);
    ov_hint.set_visible(false);

    let load_bar = gtk::ProgressBar::new();
    load_bar.add_css_class("load-bar");
    load_bar.set_valign(gtk::Align::Start);
    load_bar.set_can_target(false);
    load_bar.set_visible(false);

    HintWidgets { load_bar, ov_hint }
}
