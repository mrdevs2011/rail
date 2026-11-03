//! Sahifa ichida qidirish (Ctrl+F): kichik panel + WebKit FindController.
//! Panel ochiq bo'lganda F3 / Ctrl+G (Shift — oldingisi) va Shift+Enter natijalar bo'ylab yuradi, Esc yopadi.
//! Barcha tugmalar Settings > Customize > Shortcuts dagi `find`, `find_next`, `find_prev` ga bog'langan.

use crate::*;

const FIND_MAX: u32 = 2000;

pub(crate) struct FindUi {
    bar: gtk::Box,
    entry: gtk::Entry,
    count: gtk::Label,
    total: Cell<u32>,
    idx: Cell<u32>,
    /// Yangi qidiruv boshlandi: keyingi `found-text` da joriy natija 1-chi bo'ladi
    fresh: Cell<bool>,
    view: RefCell<Option<WebView>>,
    hooks: RefCell<Vec<(webkit6::FindController, glib::SignalHandlerId)>>,
}

impl FindUi {
    fn opts() -> u32 {
        (webkit6::FindOptions::CASE_INSENSITIVE | webkit6::FindOptions::WRAP_AROUND).bits()
    }

    fn unhook(&self) {
        let hooks: Vec<_> = self.hooks.borrow_mut().drain(..).collect();
        for (fc, id) in hooks {
            fc.disconnect(id);
        }
    }

    fn show_label(&self) {
        if self.entry.text().is_empty() {
            self.count.set_text("");
            self.bar.remove_css_class("find-none");
            return;
        }
        let total = self.total.get();
        if total == 0 {
            self.count.set_text(tr("No results"));
            self.bar.add_css_class("find-none");
        } else {
            self.bar.remove_css_class("find-none");
            self.count
                .set_text(&format!("{}/{}", self.idx.get().clamp(1, total), total));
        }
    }
}

impl App {
    pub(crate) fn find_is_open(&self) -> bool {
        self.find_ui
            .borrow()
            .as_ref()
            .is_some_and(|u| u.bar.is_visible())
    }

    fn find_build(self: &Rc<Self>) -> Option<Rc<FindUi>> {
        if let Some(u) = self.find_ui.borrow().as_ref() {
            return Some(u.clone());
        }
        let host = self.strip.parent()?.downcast::<gtk::Overlay>().ok()?;

        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        bar.add_css_class("find-bar");
        bar.set_halign(gtk::Align::End);
        bar.set_valign(gtk::Align::Start);
        bar.set_margin_top(14);
        bar.set_margin_end(18);
        bar.set_visible(false);

        let entry = gtk::Entry::new();
        entry.add_css_class("find-entry");
        entry.set_width_chars(24);
        entry.set_placeholder_text(Some(tr("Find in page")));
        let count = gtk::Label::new(None);
        count.add_css_class("find-count");
        count.set_xalign(1.0);

        let prev = gtk::Button::from_icon_name("go-up-symbolic");
        prev.add_css_class("flat-icon");
        prev.set_tooltip_text(Some(tr("Previous match (Shift+Enter)")));
        let next = gtk::Button::from_icon_name("go-down-symbolic");
        next.add_css_class("flat-icon");
        next.set_tooltip_text(Some(tr("Next match (Enter)")));
        let close = gtk::Button::from_icon_name("window-close-symbolic");
        close.add_css_class("flat-icon");
        close.set_tooltip_text(Some(tr("Close (Esc)")));

        bar.append(&entry);
        bar.append(&count);
        bar.append(&prev);
        bar.append(&next);
        bar.append(&close);

        let ui = Rc::new(FindUi {
            bar: bar.clone(),
            entry: entry.clone(),
            count,
            total: Cell::new(0),
            idx: Cell::new(0),
            fresh: Cell::new(false),
            view: RefCell::new(None),
            hooks: RefCell::new(Vec::new()),
        });

        let app = self.clone();
        entry.connect_changed(move |_| app.find_run());
        let app = self.clone();
        entry.connect_activate(move |_| {
            app.find_step(true);
        });
        let app = self.clone();
        prev.connect_clicked(move |_| {
            app.find_step(false);
        });
        let app = self.clone();
        next.connect_clicked(move |_| {
            app.find_step(true);
        });
        let app = self.clone();
        close.connect_clicked(move |_| app.find_close(true));

        host.add_overlay(&bar);
        *self.find_ui.borrow_mut() = Some(ui.clone());
        Some(ui)
    }

    /// Hozirgi tab'ning FindController signallariga ulanadi (eskisidan uziladi).
    fn find_bind(&self, ui: &Rc<FindUi>, v: &WebView) {
        ui.unhook();
        *ui.view.borrow_mut() = Some(v.clone());
        let Some(fc) = v.find_controller() else {
            return;
        };
        let u = ui.clone();
        let h1 = fc.connect_found_text(move |_, _| {
            // This signal only confirms a found match; never overwrite the total here.
            // The separate counted-matches signal supplies the complete result count.
            if u.fresh.replace(false) {
                u.idx.set(1);
            }
            u.show_label();
        });
        let u = ui.clone();
        let h2 = fc.connect_counted_matches(move |_, n| {
            // `found-text` reports the currently found text; it is not a reliable
            // total for the page. WebKit's asynchronous `counted-matches` signal is
            // the authoritative total used by the find bar and next/previous wrapping.
            u.total.set(n);
            if n == 0 {
                u.idx.set(0);
            } else if u.idx.get() == 0 || u.fresh.get() {
                u.idx.set(1);
            } else {
                u.idx.set(u.idx.get().min(n));
            }
            u.show_label();
        });
        let u = ui.clone();
        let h3 = fc.connect_failed_to_find_text(move |_| {
            u.total.set(0);
            u.idx.set(0);
            u.fresh.set(false);
            u.show_label();
        });
        let mut hooks = ui.hooks.borrow_mut();
        hooks.push((fc.clone(), h1));
        hooks.push((fc.clone(), h2));
        hooks.push((fc, h3));
    }

    /// Ctrl+F. `false` — hozir ochib bo'lmaydi (omnibar, sozlamalar, overview): tugma sahifaga o'tadi.
    pub(crate) fn find_open(self: &Rc<Self>) -> bool {
        if self.palette_open() || self.settings_modal_open() || self.overview_open() {
            return false;
        }
        let Some(v) = self.focused_view() else {
            return false;
        };
        let Some(ui) = self.find_build() else {
            return false;
        };
        let same = ui.view.borrow().as_ref() == Some(&v);
        if !same {
            if let Some(old) = ui.view.borrow().clone()
                && let Some(fc) = old.find_controller()
            {
                fc.search_finish();
            }
            self.find_bind(&ui, &v);
        }
        ui.bar.set_visible(true);
        ui.entry.grab_focus();
        ui.entry.select_region(0, -1);
        if !same && !ui.entry.text().is_empty() {
            self.find_run();
        }
        true
    }

    /// Panelni yopadi, belgilashlarni olib tashlaydi. `refocus` — fokus sahifaga qaytsin.
    pub(crate) fn find_close(self: &Rc<Self>, refocus: bool) {
        let Some(ui) = self.find_ui.borrow().clone() else {
            return;
        };
        ui.bar.set_visible(false);
        let view = ui.view.borrow_mut().take();
        ui.unhook();
        ui.total.set(0);
        ui.idx.set(0);
        if let Some(v) = view {
            if let Some(fc) = v.find_controller() {
                fc.search_finish();
            }
            if refocus {
                v.grab_focus();
            }
        }
    }

    fn find_run(self: &Rc<Self>) {
        let Some(ui) = self.find_ui.borrow().clone() else {
            return;
        };
        let Some(v) = ui.view.borrow().clone() else {
            return;
        };
        let Some(fc) = v.find_controller() else {
            return;
        };
        let text = ui.entry.text().to_string();
        ui.total.set(0);
        ui.idx.set(0);
        if text.is_empty() {
            fc.search_finish();
            ui.show_label();
            return;
        }
        ui.fresh.set(true);
        ui.count.set_text("");
        ui.bar.remove_css_class("find-none");
        fc.search(&text, FindUi::opts(), FIND_MAX);
        fc.count_matches(&text, FindUi::opts(), FIND_MAX);
    }

    /// Keyingi / oldingi natija. `false` — panel yopiq (tugma sahifaga o'tadi).
    pub(crate) fn find_step(self: &Rc<Self>, forward: bool) -> bool {
        if !self.find_is_open() {
            return false;
        }
        let Some(ui) = self.find_ui.borrow().clone() else {
            return false;
        };
        let Some(v) = ui.view.borrow().clone() else {
            return false;
        };
        let Some(fc) = v.find_controller() else {
            return false;
        };
        if ui.entry.text().is_empty() {
            return true;
        }
        let total = ui.total.get();
        if total == 0 {
            self.find_run();
            return true;
        }
        let cur = ui.idx.get().max(1);
        if forward {
            fc.search_next();
            ui.idx.set(cur % total + 1);
        } else {
            fc.search_previous();
            ui.idx.set((cur + total - 2) % total + 1);
        }
        ui.show_label();
        true
    }

    /// Fokus boshqa tabga o'tsa, panel eski tabga bog'langan qolmasin.
    pub(crate) fn find_sync_focus(self: &Rc<Self>) {
        if !self.find_is_open() {
            return;
        }
        let bound = self
            .find_ui
            .borrow()
            .as_ref()
            .and_then(|u| u.view.borrow().clone());
        if self.focused_view() != bound {
            self.find_close(false);
        }
    }
}
