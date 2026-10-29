//! The shortcut editor in Settings > Customize > Shortcuts.

use crate::*;

#[derive(Clone)]
pub(crate) struct ScUi {
    pub(crate) rows: gtk::Box,
    pub(crate) pending: Rc<RefCell<ShortcutConfig>>,
    pub(crate) status: gtk::Label,
    pub(crate) prio: gtk::CheckButton,
    pub(crate) is_syncing: Rc<Cell<bool>>,
    /// One shared key picker for the whole page (not one per row).
    pub(crate) picker_host: gtk::Box,
}

pub(crate) const SC_MODS: &[&str] = &["Alt", "Ctrl", "Alt+Shift", "Ctrl+Shift", "None"];
pub(crate) const SC_KEY_ROWS: &[&str] = &[
    "`1234567890-=",
    "QWERTYUIOP[]",
    "ASDFGHJKL;'",
    "ZXCVBNM,./\\",
];

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

/// Shared on-screen keyboard — one instance, shown relative to the row being edited.
pub(crate) fn sc_open_picker(
    ui: &ScUi,
    anchor: &impl IsA<gtk::Widget>,
    sel: Rc<Cell<u32>>,
    for_id: &'static str,
    on_pick: Rc<dyn Fn(String)>,
) {
    // Drop previous popover children on the host
    while let Some(c) = ui.picker_host.first_child() {
        ui.picker_host.remove(&c);
    }
    let pop = gtk::Popover::new();
    pop.set_parent(&ui.picker_host);
    pop.set_position(gtk::PositionType::Bottom);
    // Point at the button the user clicked
    if let Some(p) = anchor.compute_bounds(&ui.picker_host) {
        // popover relative to host is fine; MenuButton-style is simpler:
    }
    let _ = anchor; // anchor via set_parent of a transient
    let grid = gtk::Grid::new();
    grid.set_row_spacing(3);
    grid.set_column_spacing(3);
    grid.set_margin_top(6);
    grid.set_margin_bottom(6);
    grid.set_margin_start(6);
    grid.set_margin_end(6);
    pop.set_child(Some(&grid));

    let cfg = ui.pending.borrow().clone();
    let mod_idx = sel.get();
    let only_f = mod_idx == 4;
    let weak = pop.downgrade();
    let make = |label: &str, value: &str| -> gtk::Button {
        let b = gtk::Button::with_label(label);
        b.add_css_class("settings-option-item");
        b.set_size_request(if only_f { 48 } else { 32 }, 28);
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
    } else {
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
    }
    pop.popup();
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
        ui.status
            .set_text(tr("Could not write shortcuts-customize.json"));
    }
}

thread_local! {
    static SC_BUILD_QUEUED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn sc_refresh(ui: &ScUi) {
    if SC_BUILD_QUEUED.with(|q| q.replace(true)) {
        return;
    }
    let ui = ui.clone();
    glib::idle_add_local_once(move || {
        SC_BUILD_QUEUED.with(|q| q.set(false));
        if ui.is_syncing.get() {
            return; // drop — next user action can refresh
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
    const TABS: &[&str] = &[
        "tab_1", "tab_2", "tab_3", "tab_4", "tab_5", "tab_6", "tab_7", "tab_8", "tab_9",
    ];
    const WS: &[&str] = &[
        "ws_1", "ws_2", "ws_3", "ws_4", "ws_5", "ws_6", "ws_7", "ws_8", "ws_9",
    ];
    const MV: &[&str] = &[
        "mv_1", "mv_2", "mv_3", "mv_4", "mv_5", "mv_6", "mv_7", "mv_8", "mv_9",
    ];
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
    if ui.is_syncing.get() && ui.rows.first_child().is_some() {
        // Already mid-build; ignore re-entry.
        return;
    }
    ui.is_syncing.set(true);
    let container = &ui.rows;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // Build ALL rows in one pass but with LIGHT widgets only (no DropDown, no prebuilt popovers).
    // DropDown::from_strings + Popover ×40 was the freeze/RAM spike.
    let cfg = ui.pending.borrow().clone();
    let mut last_group = "";
    let mut i = 0;
    while i < SHORTCUT_DEFS.len() {
        let def = &SHORTCUT_DEFS[i];
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

        if let Some((title, ids)) = sc_series_for(def.id) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.add_css_class("shortcut-row");
            row.set_margin_top(2);
            row.set_margin_bottom(2);
            let d = gtk::Label::new(Some(tr(title)));
            d.add_css_class("shortcut-desc");
            d.set_xalign(0.0);
            d.set_hexpand(true);
            row.append(&d);
            let last_id = ids.last().copied().unwrap_or(def.id);
            let last_def = SHORTCUT_DEFS
                .iter()
                .find(|d| d.id == last_id)
                .unwrap_or(def);
            let last_combo = sc_effective(last_def, &cfg)
                .first()
                .cloned()
                .unwrap_or_default();
            let kbd = gtk::Label::new(Some(&last_combo));
            kbd.add_css_class("shortcut-kbd");
            kbd.set_tooltip_text(Some(tr(
                "Numbered series — change individually via defaults or reset",
            )));
            row.append(&kbd);
            container.append(&row);
            i += ids.len();
            continue;
        }

        let enabled = sc_enabled(&cfg, def.id);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("shortcut-row");
        row.set_margin_top(2);
        row.set_margin_bottom(2);

        let d = gtk::Label::new(Some(tr(def.desc)));
        d.add_css_class("shortcut-desc");
        d.set_xalign(0.0);
        d.set_hexpand(true);
        d.set_wrap(true);
        d.set_opacity(if enabled { 1.0 } else { 0.45 });
        row.append(&d);

        if sc_is_locked(def.id) {
            let lock = gtk::Image::from_icon_name("changes-prevent-symbolic");
            lock.set_opacity(0.7);
            lock.set_tooltip_text(Some(tr(
                "Locked (read-only): system navigation, cannot be edited.",
            )));
            row.append(&lock);
            d.set_opacity(0.55);
            let combo = sc_effective(def, &cfg).first().cloned().unwrap_or_default();
            let kbd = gtk::Label::new(Some(&combo));
            kbd.add_css_class("shortcut-kbd");
            kbd.set_opacity(0.5);
            row.append(&kbd);
            container.append(&row);
            i += 1;
            continue;
        }

        let sw = gtk::Switch::new();
        sw.set_active(enabled);
        sw.set_valign(gtk::Align::Center);
        {
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
                    let all_on = SHORTCUT_DEFS.iter().all(|d| sc_enabled(&c, d.id));
                    c.site_wins = !all_on;
                }
                let all_on = {
                    let c = ui2.pending.borrow();
                    SHORTCUT_DEFS.iter().all(|d| sc_enabled(&c, d.id))
                };
                ui2.is_syncing.set(true);
                ui2.prio.set_active(all_on);
                ui2.is_syncing.set(false);
                sc_commit(&ui2);
                // Update opacity without full rebuild
                if let Some(parent) = s.parent() {
                    if let Some(boxp) = parent.downcast_ref::<gtk::Box>() {
                        if let Some(lab) = boxp
                            .first_child()
                            .and_then(|w| w.downcast::<gtk::Label>().ok())
                        {
                            lab.set_opacity(if is_on { 1.0 } else { 0.45 });
                        }
                    }
                }
            });
        }
        row.append(&sw);

        let eff = sc_effective(def, &cfg);
        let combo = eff.first().cloned().unwrap_or_default();
        let (mod_idx, key) = if combo.is_empty() {
            (0u32, String::new())
        } else {
            sc_split(&combo)
        };
        let sel = Rc::new(Cell::new(mod_idx));

        // Modifier as a small toggle button (cycles) — far cheaper than DropDown
        let mod_btn = gtk::Button::with_label(SC_MODS[mod_idx as usize]);
        mod_btn.add_css_class("settings-option-item");
        mod_btn.set_sensitive(enabled);
        {
            let ui3 = ui.clone();
            let sel = sel.clone();
            let key = key.clone();
            let mod_btn2 = mod_btn.clone();
            mod_btn.connect_clicked(move |_| {
                if ui3.is_syncing.get() {
                    return;
                }
                let n = ((sel.get() + 1) % SC_MODS.len() as u32);
                sel.set(n);
                mod_btn2.set_label(SC_MODS[n as usize]);
                let is_f = key.len() >= 2
                    && key.starts_with('F')
                    && key[1..].chars().all(|c| c.is_ascii_digit());
                if n == 4 && !is_f {
                    ui3.status.set_text(tr("No modifier: pick an F-key"));
                } else if !key.is_empty() {
                    sc_apply_slot(&ui3, def, 0, sc_join(n, &key));
                }
            });
        }
        row.append(&mod_btn);

        // Key button opens the ONE shared picker
        let kb = gtk::Button::with_label(&sc_key_label(&key));
        kb.add_css_class("settings-option-item");
        kb.set_size_request(72, -1);
        kb.set_sensitive(enabled);
        {
            let ui4 = ui.clone();
            let sel = sel.clone();
            let kb2 = kb.clone();
            kb.connect_clicked(move |b| {
                if ui4.is_syncing.get() {
                    return;
                }
                let ui5 = ui4.clone();
                let sel2 = sel.clone();
                let kb3 = kb2.clone();
                sc_open_picker(
                    &ui4,
                    b,
                    sel.clone(),
                    def.id,
                    Rc::new(move |v: String| {
                        kb3.set_label(&sc_key_label(&v));
                        sc_apply_slot(&ui5, def, 0, sc_join(sel2.get(), &v));
                    }),
                );
            });
        }
        row.append(&kb);

        if cfg.overrides.contains_key(def.id) || cfg.disabled.contains(def.id) {
            let reset = gtk::Button::from_icon_name("edit-undo-symbolic");
            reset.add_css_class("flat");
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
        }

        container.append(&row);
        i += 1;
    }

    // Yield to the main loop before accepting more input-driven rebuilds.
    let ui_done = ui.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
        ui_done.is_syncing.set(false);
    });
}
