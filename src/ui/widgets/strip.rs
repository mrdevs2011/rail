//! The tab strip and the shell that holds sidebar + strip.

use crate::*;

pub(crate) struct StripWidgets {
    pub(crate) layout: StripLayout,
    pub(crate) shell: gtk::Box,
    pub(crate) shell_layout: ShellLayout,
    pub(crate) strip: gtk::Box,
    pub(crate) strip_host: gtk::Overlay,
}

pub(crate) fn build_strip(side: gtk::Box) -> StripWidgets {
    // ---- strip + shell ----
    let layout = StripLayout::new();
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    strip.set_layout_manager(Some(layout.clone()));
    strip.set_overflow(gtk::Overflow::Hidden);
    strip.set_margin_top(8);
    strip.set_margin_bottom(8);
    strip.set_margin_end(8);
    strip.set_margin_start(6);

    // Settings lives in the strip (tab square), like a normal page — not window fullscreen
    let strip_host = gtk::Overlay::new();
    strip_host.set_hexpand(true);
    strip_host.set_vexpand(true);
    strip_host.set_child(Some(&strip));

    let shell_layout = ShellLayout::new();
    shell_layout.set_side(SIDE_W as f64);
    let shell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    shell.set_layout_manager(Some(shell_layout.clone()));
    shell.set_overflow(gtk::Overflow::Hidden);
    shell.set_hexpand(true);
    shell.set_vexpand(true);
    shell.append(&side);
    shell.append(&strip_host);

    StripWidgets { layout, shell, shell_layout, strip, strip_host }
}
