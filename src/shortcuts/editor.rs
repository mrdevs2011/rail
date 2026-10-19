//! The shortcut editor in Settings > Customize > Shortcuts.

use crate::*;

#[derive(Clone)]
pub(crate) struct ScUi {
    pub(crate) rows: gtk::Box,
    pub(crate) pending: Rc<RefCell<ShortcutConfig>>,
    pub(crate) status: gtk::Label,
    pub(crate) prio: gtk::CheckButton,
    pub(crate) is_syncing: Rc<Cell<bool>>,
}

pub(crate) const SC_MODS: &[&str] = &["Alt", "Ctrl", "Alt+Shift", "Ctrl+Shift", "None"];
pub(crate) const SC_KEY_ROWS: &[&str] = &["`1234567890-=", "QWERTYUIOP[]", "ASDFGHJKL;'", "ZXCVBNM,./\\"];

pub(crate) fn sc_key_label(v: &str) -> String {
    match v {
        "" => tr("Pick key").to_string(),
        "Left" => "\u{2190}".to_string(),
        "Right" => "\u{2192}".to_string(),
        "Up" => "\u{2191}".to_string(),
        "Down" => "\u{2193}".to_string(),
        other => other.to_string(),
    }
}

/// "Alt+Shift+T" -> (index into SC_MODS, "T")
pub(crate) fn sc_split(combo: &str) -> (u32, String) {
    let mut parts: Vec<&str> = combo.split('+').collect();
    let key = parts.pop().unwrap_or("").to_string();
    let ctrl = parts.contains(&"Ctrl");
    let alt = parts.contains(&"Alt");
    let shift = parts.contains(&"Shift");
    let idx = if ctrl {
        if shift { 3 } else { 1 }
    } else if alt {
        if shift { 2 } else { 0 }
    } else {
        4
    };
    (idx, key)
}

pub(crate) fn sc_join(mod_idx: u32, key: &str) -> String {
    let prefix = match mod_idx {
        0 => "Alt+",
        1 => "Ctrl+",
        2 => "Alt+Shift+",
        3 => "Ctrl+Shift+",
        _ => "",
    };
    format!("{}{}", prefix, key)
}

/// On-screen keyboard: the user taps a key instead of pressing it.
/// What it offers follows the modifier dropdown: "None" only makes sense for F1-F12
/// (a bare letter would steal normal typing), everything else gets the full keyboard.
pub(crate) fn sc_build_picker(
    sel: Rc<Cell<u32>>,
    pending: Rc<RefCell<ShortcutConfig>>,
    for_id: &'static str,
    on_pick: Rc<dyn Fn(String)>,
) -> gtk::Popover {
    let pop = gtk::Popover::new();
    let grid = gtk::Grid::new();
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    grid.set_margin_top(8);
    grid.set_margin_bottom(8);
    grid.set_margin_start(8);
    grid.set_margin_end(8);
    pop.set_child(Some(&grid));

    // Build key grid only when the popover opens — never during sc_build (40× keyboard = freeze).
    {
        let grid = grid.clone();
        let weak = pop.downgrade();
        let pending = pending.clone();
        let sel = sel.clone();
        let on_pick = on_pick.clone();
        pop.connect_show(move |_| {
            while let Some(child) = grid.first_child() {
                grid.remove(&child);
            }
            let cfg = pending.borrow().clone();
            let mod_idx = sel.get();
            let only_f = mod_idx == 4;
            let make = |label: &str, value: &str| -> gtk::Button {
                let b = gtk::Button::with_label(label);
                b.add_css_class("settings-option-item");
                b.set_size_request(if only_f { 52 } else { 34 }, 30);
                let combo = sc_join(mod_idx, value);
                if let Some(owner) = sc_combo_owner(&cfg, &combo, for_id) {
                    b.set_sensitive(false);
                    b.set_opacity(0.35);
                    b.set_tooltip_text(Some(&format!("{} — {}", combo, tr(owner.desc))));
                } else {
                    let v = value.to_string();
                    let cb = on_pick.clone();
                    let w = weak.clone();
                    b.connect_clicked(move |_| {
                        if let Some(p) = w.upgrade() {
                            p.popdown();
                        }
                        cb(v.clone());
                    });
                }
                b
            };
            if only_f {
                for n in 1..=12i32 {
                    let s = format!("F{}", n);
                    grid.attach(&make(&s, &s), (n - 1) % 6, (n - 1) / 6, 1, 1);
                }
                return;
            }
            for (r, chars) in SC_KEY_ROWS.iter().enumerate() {
                for (c, ch) in chars.chars().enumerate() {
                    let s = ch.to_string();
                    grid.attach(&make(&s, &s), c as i32, r as i32, 1, 1);
                }
            }
            for n in 1..=12 {
                let s = format!("F{}", n);
                grid.attach(&make(&s, &s), n - 1, 4, 1, 1);
            }
            let arrows = [
                ("\u{2190}", "Left"),
                ("\u{2191}", "Up"),
                ("\u{2193}", "Down"),
                ("\u{2192}", "Right"),
            ];
            for (i, (l, v)) in arrows.iter().enumerate() {
                grid.attach(&make(l, v), i as i32, 5, 1, 1);
            }
        });
    }
    pop
}

pub(crate) fn sc_commit(ui: &ScUi) {
    let cfg = {
        let mut p = ui.pending.borrow_mut();
        sc_normalize(&mut p);
        p.clone()
    };
    if save_shortcuts_config(&cfg) {
        SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
        apply_shortcut_priority();
        sc_refresh_bound();
    } else {
        ui.status.set_text(tr("Could not write shortcuts-customize.json"));
    }
}

/// Rebuilds the rows after the current event finishes, keeping the scroll position.
thread_local! {
    static SC_BUILD_QUEUED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn sc_refresh(ui: &ScUi) {
    // Coalesce many apply/toggle events into one rebuild (prevents RAM explosion).
    if SC_BUILD_QUEUED.with(|q| q.replace(true)) {
        return;
    }
    let ui = ui.clone();
    glib::idle_add_local_once(move || {
        SC_BUILD_QUEUED.with(|q| q.set(false));
        if ui.is_syncing.get() {
            // Still building — try again next idle.
            sc_refresh(&ui);
            return;
        }
        let adj = ui
            .rows
            .ancestor(gtk::ScrolledWindow::static_type())
            .and_then(|w| w.downcast::<gtk::ScrolledWindow>().ok())
            .map(|sw| sw.vadjustment());
        let pos = adj.as_ref().map(|a| a.value());
        sc_build(&ui);
        if let (Some(adj), Some(pos)) = (adj, pos) {
            glib::idle_add_local_once(move || adj.set_value(pos));
        }
    });
}

pub(crate) fn sc_apply_slot(ui: &ScUi, def: &'static ShortcutDef, slot: usize, combo: String) {
    let res = sc_set_slot(&mut ui.pending.borrow_mut(), def, slot, &combo);
    match res {
        Ok(note) => {
            sc_commit(ui);
            ui.status.set_text(&note);
        }
        Err(e) => ui.status.set_text(&e),
    }
    sc_refresh(ui);
}


/// Numbered series (Ctrl+1..9 etc.) collapsed into one stacked card.
pub(crate) fn sc_series_for(id: &str) -> Option<(&'static str, &'static [&'static str])> {
    const TABS: &[&str] = &["tab_1", "tab_2", "tab_3", "tab_4", "tab_5", "tab_6", "tab_7", "tab_8", "tab_9"];
    const WS: &[&str] = &["ws_1", "ws_2", "ws_3", "ws_4", "ws_5", "ws_6", "ws_7", "ws_8", "ws_9"];
    const MV: &[&str] = &["mv_1", "mv_2", "mv_3", "mv_4", "mv_5", "mv_6", "mv_7", "mv_8", "mv_9"];
    match id {
        "tab_1" => Some(("Switch to tab 1–9", TABS)),
        "ws_1" => Some(("Switch to workspace 1–9", WS)),
        "mv_1" => Some(("Move tab to workspace 1–9", MV)),
        _ => None,
    }
}

pub(crate) fn sc_series_member(id: &str) -> bool {
    id.starts_with("tab_") || id.starts_with("ws_") || id.starts_with("mv_")
}

pub(crate) fn sc_build(ui: &ScUi) {
    // Prevent Switch::set_active during rebuild from firing handlers that mutate config
    ui.is_syncing.set(true);
    let container = &ui.rows;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let cfg = ui.pending.borrow().clone();
    let mut last_group = "";
    let mut i = 0;
    while i < SHORTCUT_DEFS.len() {
        let def = &SHORTCUT_DEFS[i];
        // Skip non-head members of a numbered series (rendered via stack card)
        if sc_series_member(def.id) && sc_series_for(def.id).is_none() {
            i += 1;
            continue;
        }
        if def.group != last_group {
            last_group = def.group;
            let h = gtk::Label::new(Some(tr(def.group)));
            h.add_css_class("settings-card-title");
            h.set_xalign(0.0);
            h.set_margin_top(10);
            h.set_margin_bottom(4);
            container.append(&h);
        }

        // Stacked card for Ctrl+1..9 / Alt+1..9 series
        if let Some((title, ids)) = sc_series_for(def.id) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.add_css_class("shortcut-row");
            row.set_margin_top(3);
            row.set_margin_bottom(3);
            let d = gtk::Label::new(Some(tr(title)));
            d.add_css_class("shortcut-desc");
            d.set_xalign(0.0);
            d.set_hexpand(true);
            d.set_valign(gtk::Align::Start);
            row.append(&d);

            let stack = gtk::Box::new(gtk::Orientation::Vertical, 0);
            stack.add_css_class("shortcut-stack");
            stack.set_halign(gtk::Align::End);

            // Top card shows the *last* binding (e.g. Ctrl+9)
            let last_id = ids.last().copied().unwrap_or(def.id);
            let last_def = SHORTCUT_DEFS.iter().find(|d| d.id == last_id).unwrap_or(def);
            let last_combo = sc_effective(last_def, &cfg).first().cloned().unwrap_or_default();
            let top = gtk::Button::with_label(&last_combo);
            top.add_css_class("shortcut-stack-card");
            top.add_css_class("shortcut-stack-layers");
            top.set_tooltip_text(Some(tr("Click to show all bindings")));

            let expand = gtk::Box::new(gtk::Orientation::Vertical, 2);
            expand.add_css_class("shortcut-stack-expand");
            expand.set_visible(false);
            for sid in ids.iter() {
                if let Some(sd) = SHORTCUT_DEFS.iter().find(|d| d.id == *sid) {
                    let c = sc_effective(sd, &cfg).first().cloned().unwrap_or_default();
                    let item = gtk::Label::new(Some(&format!("{}  ·  {}", c, tr(sd.desc))));
                    item.add_css_class("shortcut-stack-item");
                    item.add_css_class("shortcut-kbd");
                    item.set_xalign(1.0);
                    expand.append(&item);
                }
            }
            let expand_c = expand.clone();
            top.connect_clicked(move |_| {
                expand_c.set_visible(!expand_c.is_visible());
            });
            stack.append(&top);
            stack.append(&expand);
            row.append(&stack);
            container.append(&row);
            i += ids.len();
            continue;
        }

        let enabled = sc_enabled(&cfg, def.id);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("shortcut-row");
        row.set_margin_top(3);
        row.set_margin_bottom(3);

        let d = gtk::Label::new(Some(tr(def.desc)));
        d.add_css_class("shortcut-desc");
        d.set_xalign(0.0);
        d.set_hexpand(true);
        d.set_wrap(true);
        d.set_valign(gtk::Align::Center);
        d.set_opacity(if enabled { 1.0 } else { 0.45 });
        row.append(&d);

        if sc_is_locked(def.id) {
            // Only Alt+Arrow focus nav stays locked (no per-row enable switch)
            let lock = gtk::Image::from_icon_name("changes-prevent-symbolic");
            lock.set_valign(gtk::Align::Center);
            lock.set_opacity(0.7);
            lock.set_tooltip_text(Some(tr("Locked (read-only): system navigation, cannot be edited.")));
            row.append(&lock);
            d.set_opacity(0.55);
        } else {
            let sw = gtk::Switch::new();
            sw.set_active(enabled);
            sw.set_valign(gtk::Align::Center);
            sw.set_tooltip_text(Some(tr("Rail shortcuts win over website shortcuts")));
            let ui2 = ui.clone();
            sw.connect_active_notify(move |s| {
                if ui2.is_syncing.get() {
                    return;
                }
                let is_on = s.is_active();
                {
                    let mut c = ui2.pending.borrow_mut();
                    if is_on {
                        c.disabled.remove(def.id);
                    } else {
                        c.disabled.insert(def.id.to_string());
                    }
                }
                let all_on = {
                    let c = ui2.pending.borrow();
                    SHORTCUT_DEFS.iter().all(|d| sc_enabled(&c, d.id))
                };
                ui2.pending.borrow_mut().site_wins = !all_on;

                ui2.is_syncing.set(true);
                ui2.prio.set_active(all_on);
                ui2.is_syncing.set(false);

                sc_commit(&ui2);
                sc_refresh(&ui2);
            });
            row.append(&sw);
        }

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        line.set_sensitive(enabled);
        line.set_valign(gtk::Align::Center);
        let eff = sc_effective(def, &cfg);
        let combo = eff.first().cloned().unwrap_or_default();

        // Alt+Arrow locked shortcuts: gray read-only badge, no editor
        if def.id.starts_with("win_") {
            let kbd = gtk::Label::new(Some(&combo));
            kbd.add_css_class("shortcut-kbd");
            kbd.set_opacity(0.5);
            kbd.set_tooltip_text(Some(tr("Locked — display only, not editable")));
            line.append(&kbd);
            row.append(&line);
            let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spacer.set_size_request(32, -1);
            row.append(&spacer);
            container.append(&row);
            continue;
        }

        let (mod_idx, key) = if combo.is_empty() {
            (0u32, String::new())
        } else {
            sc_split(&combo)
        };
        let sel = Rc::new(Cell::new(mod_idx));

        let dd = gtk::DropDown::from_strings(SC_MODS);
        dd.set_selected(mod_idx);
        {
            let ui3 = ui.clone();
            let sel = sel.clone();
            let key = key.clone();
            dd.connect_selected_notify(move |d| {
                // Building rows sets selected on every dropdown — must not commit/rebuild or the UI freezes.
                if ui3.is_syncing.get() {
                    return;
                }
                sel.set(d.selected());
                let is_f = key.len() >= 2 && key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit());
                if sel.get() == 4 && !is_f {
                    ui3.status.set_text(tr("No modifier: tap the key button and pick F1-F12"));
                } else if !key.is_empty() {
                    sc_apply_slot(&ui3, def, 0, sc_join(sel.get(), &key));
                }
            });
        }
        line.append(&dd);

        let kb = gtk::MenuButton::new();
        kb.set_label(&sc_key_label(&key));
        kb.set_size_request(80, -1);
        {
            // Popover shell only — key grid is built on first show (see sc_build_picker).
            let ui4 = ui.clone();
            let sel = sel.clone();
            let pop = sc_build_picker(
                sel.clone(),
                ui4.pending.clone(),
                def.id,
                Rc::new({
                    let ui4 = ui4.clone();
                    let sel = sel.clone();
                    move |v: String| {
                        sc_apply_slot(&ui4, def, 0, sc_join(sel.get(), &v));
                    }
                }),
            );
            kb.set_popover(Some(&pop));
        }
        line.append(&kb);
        row.append(&line);

        if cfg.overrides.contains_key(def.id) || cfg.disabled.contains(def.id) {
            let reset = gtk::Button::from_icon_name("edit-undo-symbolic");
            reset.add_css_class("flat");
            reset.set_valign(gtk::Align::Center);
            reset.set_tooltip_text(Some(tr("Reset to default")));
            let ui6 = ui.clone();
            reset.connect_clicked(move |_| {
                {
                    let mut c = ui6.pending.borrow_mut();
                    c.overrides.remove(def.id);
                    c.disabled.remove(def.id);
                }
                sc_commit(&ui6);
                sc_refresh(&ui6);
            });
            row.append(&reset);
        } else {
            let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spacer.set_size_request(32, -1);
            row.append(&spacer);
        }
        container.append(&row);
        i += 1;
    }
    // Keep is_syncing true until after GTK drains queued notify signals from DropDown::set_selected.
    let ui_done = ui.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(80), move || {
        ui_done.is_syncing.set(false);
    });
}
