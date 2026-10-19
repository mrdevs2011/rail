//! Shortcut definitions, persisted config and key-combo resolution.

use crate::*;

// ---------------------------------------------------------------------
// Customizable keyboard shortcuts (Settings > Customize > Shortcuts)
// ---------------------------------------------------------------------

pub(crate) struct ShortcutDef {
    pub(crate) id: &'static str,
    pub(crate) group: &'static str,
    pub(crate) desc: &'static str,
    pub(crate) defaults: &'static [&'static str],
}

macro_rules! sc {
    ($id:expr, $group:expr, $desc:expr, [$($d:expr),*]) => {
        ShortcutDef { id: $id, group: $group, desc: $desc, defaults: &[$($d),*] }
    };
}

pub(crate) const SC_LOCKED: &[&str] = &["win_left", "win_right", "win_up", "win_down"];

pub(crate) fn sc_is_locked(id: &str) -> bool {
    SC_LOCKED.iter().any(|x| *x == id)
}

pub(crate) const G_TABS: &str = "Tabs & Workspaces";
pub(crate) const G_PANELS: &str = "Panels, Settings & Tools";

pub(crate) const SHORTCUT_DEFS: &[ShortcutDef] = &[
    sc!("new_tab", G_TABS, "Open Omnibar / New tab", ["Ctrl+T"]),
    sc!("focus_url", G_TABS, "Focus URL / Address bar", ["Ctrl+L"]),
    sc!("close_tab", G_TABS, "Close focused tab", ["Ctrl+W"]),
    sc!("reload", G_TABS, "Reload active tab", ["Ctrl+R"]),
    sc!("hard_reload", G_TABS, "Hard reload (bypass cache)", ["Ctrl+Shift+R"]),
    sc!("full_tab", G_TABS, "Toggle Full Tab View", ["Alt+F"]),
    sc!("new_workspace", G_TABS, "New Workspace", ["Alt+Shift+V"]),
    sc!("focus_left", G_TABS, "Focus column Left", ["Alt+H"]),
    sc!("focus_right", G_TABS, "Focus column Right", ["Alt+L"]),
    sc!("focus_down", G_TABS, "Focus window Down", ["Alt+J"]),
    sc!("focus_up", G_TABS, "Focus window Up", ["Alt+K"]),
    sc!("win_left", G_TABS, "Focus column Left (Alt+Arrow)", ["Alt+Left"]),
    sc!("win_right", G_TABS, "Focus column Right (Alt+Arrow)", ["Alt+Right"]),
    sc!("win_up", G_TABS, "Focus workspace Up (Alt+Arrow)", ["Alt+Up"]),
    sc!("win_down", G_TABS, "Focus workspace Down (Alt+Arrow)", ["Alt+Down"]),
    sc!("move_left", G_TABS, "Move tab column Left", ["Alt+Shift+H"]),
    sc!("move_right", G_TABS, "Move tab column Right", ["Alt+Shift+L"]),
    sc!("move_down", G_TABS, "Move window Down", ["Alt+Shift+J"]),
    sc!("move_up", G_TABS, "Move window Up", ["Alt+Shift+K"]),
    sc!("next_tab", G_TABS, "Cycle to next tab", ["Ctrl+Tab"]),
    sc!("prev_tab", G_TABS, "Cycle to previous tab", ["Ctrl+Shift+Tab"]),
    sc!("stack_left", G_TABS, "Stack into left column / unstack left", ["Alt+["]),
    sc!("stack_right", G_TABS, "Stack into right column / unstack right", ["Alt+]"]),
    sc!("tab_1", G_TABS, "Switch to tab 1", ["Ctrl+1"]),
    sc!("tab_2", G_TABS, "Switch to tab 2", ["Ctrl+2"]),
    sc!("tab_3", G_TABS, "Switch to tab 3", ["Ctrl+3"]),
    sc!("tab_4", G_TABS, "Switch to tab 4", ["Ctrl+4"]),
    sc!("tab_5", G_TABS, "Switch to tab 5", ["Ctrl+5"]),
    sc!("tab_6", G_TABS, "Switch to tab 6", ["Ctrl+6"]),
    sc!("tab_7", G_TABS, "Switch to tab 7", ["Ctrl+7"]),
    sc!("tab_8", G_TABS, "Switch to tab 8", ["Ctrl+8"]),
    sc!("tab_9", G_TABS, "Switch to tab 9 (or last tab)", ["Ctrl+9"]),
    sc!("ws_1", G_TABS, "Switch to workspace 1", ["Alt+1"]),
    sc!("ws_2", G_TABS, "Switch to workspace 2", ["Alt+2"]),
    sc!("ws_3", G_TABS, "Switch to workspace 3", ["Alt+3"]),
    sc!("ws_4", G_TABS, "Switch to workspace 4", ["Alt+4"]),
    sc!("ws_5", G_TABS, "Switch to workspace 5", ["Alt+5"]),
    sc!("ws_6", G_TABS, "Switch to workspace 6", ["Alt+6"]),
    sc!("ws_7", G_TABS, "Switch to workspace 7", ["Alt+7"]),
    sc!("ws_8", G_TABS, "Switch to workspace 8", ["Alt+8"]),
    sc!("ws_9", G_TABS, "Switch to workspace 9", ["Alt+9"]),
    sc!("mv_1", G_TABS, "Move tab to workspace 1", ["Alt+Shift+1"]),
    sc!("mv_2", G_TABS, "Move tab to workspace 2", ["Alt+Shift+2"]),
    sc!("mv_3", G_TABS, "Move tab to workspace 3", ["Alt+Shift+3"]),
    sc!("mv_4", G_TABS, "Move tab to workspace 4", ["Alt+Shift+4"]),
    sc!("mv_5", G_TABS, "Move tab to workspace 5", ["Alt+Shift+5"]),
    sc!("mv_6", G_TABS, "Move tab to workspace 6", ["Alt+Shift+6"]),
    sc!("mv_7", G_TABS, "Move tab to workspace 7", ["Alt+Shift+7"]),
    sc!("mv_8", G_TABS, "Move tab to workspace 8", ["Alt+Shift+8"]),
    sc!("mv_9", G_TABS, "Move tab to workspace 9", ["Alt+Shift+9"]),
    sc!("settings", G_PANELS, "Toggle Settings panel", ["Alt+S"]),
    sc!("profiles", G_PANELS, "Profiles / profile switcher", ["Alt+P"]),
    sc!("data", G_PANELS, "Data & Backup", ["Alt+I"]),
    sc!("devtools", G_PANELS, "Toggle DevTools", ["F12"]),
    sc!("devtools_dock", G_PANELS, "Cycle DevTools dock position", ["Alt+Shift+D"]),
    sc!("map", G_PANELS, "Toggle Roadmap / Minimap", ["Alt+M"]),
    sc!("overview", G_PANELS, "Toggle Spatial Overview", ["Alt+O"]),
    sc!("overview_zoom_in", G_PANELS, "Zoom in (page 20-500%, overview scale)", ["Ctrl+plus"]),
    sc!("overview_zoom_out", G_PANELS, "Zoom out (page 20-500%, overview scale)", ["Ctrl+minus"]),
    sc!("zoom_reset", G_PANELS, "Reset zoom (page 100%, overview default)", ["Ctrl+0"]),
    sc!("sidebar", G_PANELS, "Toggle Sidebar", ["Alt+B"]),
    sc!("downloads", G_PANELS, "Open Downloads", ["Ctrl+J"]),
    sc!("fullscreen", G_PANELS, "Toggle Fullscreen", ["F11"]),
    sc!("quit", G_PANELS, "Quit Rail", ["Ctrl+Q"]),
];

#[derive(Clone, Default)]
pub(crate) struct ShortcutConfig {
    pub(crate) overrides: std::collections::HashMap<String, Vec<String>>,
    pub(crate) disabled: std::collections::HashSet<String>,
    pub(crate) site_wins: bool,
}

pub(crate) enum ScResolve {
    Action(String),
    Disabled,
    Unknown,
}

thread_local! {
    pub(crate) static SHORTCUTS: RefCell<ShortcutConfig> = RefCell::new(ShortcutConfig::default());
    pub(crate) static KEY_CTL: RefCell<Option<gtk::EventControllerKey>> = RefCell::new(None);
    /// The one running Rail window state: a second `rail [URL]` launch is forwarded here.
    pub(crate) static MAIN_APP: RefCell<Option<Rc<App>>> = RefCell::new(None);
    /// Re-entrancy guard: Inspector::show() emits bring-to-front, which called show_devtools() again (stack overflow).
    pub(crate) static IN_SHOW_DEVTOOLS: Cell<bool> = Cell::new(false);
    /// The view whose inspector is currently shown in the Rail panel/window (may differ from the focused view after Ctrl+Tab).
    pub(crate) static DT_VIEW: RefCell<Option<WebView>> = RefCell::new(None);
    /// True while Rail itself closes the undocked DevTools window (so close-request must not tear DevTools down).
    pub(crate) static DT_CLOSING_WIN: Cell<bool> = Cell::new(false);
    /// Toolbar slot (0-based, 24px pitch) last clicked in the inspector toolbar; -1 = none.
    pub(crate) static LAST_DT_SLOT: Cell<i32> = Cell::new(-1);
}

pub(crate) fn shortcuts_path() -> PathBuf {
    customize_dir().join("shortcuts-customize.json")
}

/// Re-reads everything Customize stores on disk (used after a Data & Backup import).
pub(crate) fn reload_customize_from_disk() {
    SHORTCUTS.with(|c| *c.borrow_mut() = load_shortcuts_config());
    apply_shortcut_priority();
    sc_refresh_bound();
    apply_html_css(&load_html_css());
}

pub(crate) fn load_shortcuts_config() -> ShortcutConfig {
    let mut cfg = ShortcutConfig::default();
    let Ok(text) = std::fs::read_to_string(shortcuts_path()) else {
        return cfg;
    };
    let Some(root) = minijson::parse(&text) else {
        return cfg;
    };
    cfg.site_wins = root.get("priority").and_then(|v| v.as_str()) == Some("site");
    let version = match root.get("version") {
        Some(minijson::Json::Num(n)) => *n,
        _ => 1.0,
    };
    if let Some(minijson::Json::Obj(items)) = root.get("shortcuts") {
        for (id, v) in items {
            let Some(def) = SHORTCUT_DEFS.iter().find(|d| d.id == id.as_str()) else {
                continue;
            };
            if v.get("enabled").and_then(|b| b.as_bool()) == Some(false) {
                cfg.disabled.insert(def.id.to_string());
            }
            if let Some(arr) = v.get("keys").and_then(|k| k.as_arr()) {
                let list: Vec<String> = arr
                    .iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|c| sc_valid_combo(c))
                    .collect();
                cfg.overrides.insert(def.id.to_string(), list);
            }
        }
    }
    if version < 2.0 {
        // v2: standart brauzer shortcutlari. Eski Alt+... defaultlari faylda qotib qolgan bo'lsa, yangi defaultga o'tkazamiz.
        for (id, legacy) in [("new_tab", "Alt+T"), ("focus_url", "Alt+G"), ("close_tab", "Alt+W"), ("reload", "Alt+R"), ("quit", "Alt+Q")] {
            if cfg.overrides.get(id).map(|l| l.len() == 1 && l[0] == legacy).unwrap_or(false) {
                cfg.overrides.remove(id);
            }
        }
    }
    sc_normalize(&mut cfg);
    cfg
}

pub(crate) fn save_shortcuts_config(cfg: &ShortcutConfig) -> bool {
    let mut out = String::from("{\n  \"version\": 2,\n");
    out.push_str(&format!(
        "  \"priority\": \"{}\",\n",
        if cfg.site_wins { "site" } else { "rail" }
    ));
    out.push_str("  \"shortcuts\": {\n");
    let total = SHORTCUT_DEFS.len();
    for (i, def) in SHORTCUT_DEFS.iter().enumerate() {
        let keys: Vec<String> = sc_effective(def, cfg)
            .iter()
            .map(|k| format!("\"{}\"", minijson::escape(k)))
            .collect();
        out.push_str(&format!(
            "    \"{}\": {{ \"description\": \"{}\", \"enabled\": {}, \"keys\": [{}] }}{}\n",
            def.id,
            minijson::escape(def.desc),
            sc_enabled(cfg, def.id),
            keys.join(", "),
            if i + 1 < total { "," } else { "" }
        ));
    }
    out.push_str("  }\n}\n");
    let path = shortcuts_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out).is_ok()
}

pub(crate) fn sc_effective(def: &ShortcutDef, cfg: &ShortcutConfig) -> Vec<String> {
    match cfg.overrides.get(def.id) {
        Some(list) => list.clone(),
        None => def.defaults.iter().map(|s| s.to_string()).collect(),
    }
}

pub(crate) fn sc_enabled(cfg: &ShortcutConfig, id: &str) -> bool {
    sc_is_locked(id) || !cfg.disabled.contains(id)
}

pub(crate) fn sc_normalize(cfg: &mut ShortcutConfig) {
    for id in SC_LOCKED {
        cfg.disabled.remove(*id);
    }
    cfg.disabled
        .retain(|id| SHORTCUT_DEFS.iter().any(|d| d.id == id.as_str()));
    for def in SHORTCUT_DEFS {
        let is_default = cfg
            .overrides
            .get(def.id)
            .map(|l| l.iter().map(|s| s.as_str()).eq(def.defaults.iter().copied()))
            .unwrap_or(false);
        if is_default {
            cfg.overrides.remove(def.id);
        }
    }
    // Locked shortcuts can never end up unbound or remapped.
    for id in SC_LOCKED {
        if cfg.overrides.get(*id).map(|l| l.is_empty()).unwrap_or(false) {
            cfg.overrides.remove(*id);
        }
        // Force defaults for locked ids (read-only).
        cfg.overrides.remove(*id);
    }
}

/// Puts `combo` into slot `slot` of `def`. Takes it away from any other shortcut that
/// used it. Returns a short note about what moved, or why it was refused.
/// Who already owns combo (excluding except_id)? Gray those keys on the picker.
pub(crate) fn sc_combo_owner(cfg: &ShortcutConfig, combo: &str, except_id: &str) -> Option<&'static ShortcutDef> {
    SHORTCUT_DEFS.iter().find(|d| {
        d.id != except_id && sc_effective(d, cfg).iter().any(|c| c == combo)
    })
}

pub(crate) fn sc_set_slot(
    cfg: &mut ShortcutConfig,
    def: &ShortcutDef,
    slot: usize,
    combo: &str,
) -> Result<String, String> {
    if !sc_valid_combo(combo) {
        return Err(tr("Pick Alt or Ctrl with a key (F-keys can be used alone).").to_string());
    }
    // Alt+Arrow focus navigation shortcuts are fully locked (read-only).
    if def.id.starts_with("win_") {
        return Err(tr("This shortcut is locked and cannot be changed.").to_string());
    }
    // Reserved combos of locked shortcuts cannot be stolen.
    for locked in SHORTCUT_DEFS.iter().filter(|d| sc_is_locked(d.id)) {
        if sc_effective(locked, cfg).iter().any(|c| c == combo) {
            return Err(trf("{} is reserved and cannot be reassigned.", &[&combo]));
        }
    }
    for other in SHORTCUT_DEFS {
        if other.id == def.id {
            continue;
        }
        if sc_effective(other, cfg).iter().any(|c| c == combo) {
            return Err(format!(
                "{} is already used by \"{}\". Pick a free key or free that shortcut first.",
                combo,
                tr(other.desc),
            ));
        }
    }
    
    let note = String::new();
    let mut list = sc_effective(def, cfg);
    if slot < list.len() {
        list[slot] = combo.to_string();
    } else {
        list.push(combo.to_string());
    }
    let mut seen: Vec<String> = Vec::new();
    list.retain(|c| {
        if seen.contains(c) {
            false
        } else {
            seen.push(c.clone());
            true
        }
    });
    cfg.overrides.insert(def.id.to_string(), list);
    Ok(note)
}

pub(crate) fn sc_valid_combo(c: &str) -> bool {
    let parts: Vec<&str> = c.split('+').collect();
    let has_mod = parts.iter().any(|p| matches!(*p, "Ctrl" | "Alt" | "Super"));
    let last = parts.last().copied().unwrap_or("");
    let is_fkey = last.len() >= 2
        && last.len() <= 3
        && last.starts_with('F')
        && last[1..].chars().all(|ch| ch.is_ascii_digit());
    has_mod || is_fkey
}

pub(crate) fn is_modifier_key(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::Hyper_L
            | gdk::Key::Hyper_R
            | gdk::Key::Caps_Lock
            | gdk::Key::Num_Lock
            | gdk::Key::ISO_Level3_Shift
    )
}

/// Layout-independent name of a key press, e.g. "Alt+Shift+T" or "F12".
pub(crate) fn shortcut_combo(key: gdk::Key, keycode: u32, state: gdk::ModifierType) -> Option<String> {
    use gdk::ModifierType as M;
    if is_modifier_key(key) {
        return None;
    }
    let mut label: Option<String> = None;
    if let Some(display) = gdk::Display::default() {
        if let Some(entries) = display.map_keycode(keycode) {
            for (kk, k) in entries.iter() {
                if kk.level() != 0 {
                    continue;
                }
                if let Some(c) = k.to_unicode() {
                    if c.is_ascii_graphic() {
                        label = Some(if c == '+' {
                            "plus".to_string()
                        } else {
                            c.to_ascii_uppercase().to_string()
                        });
                        break;
                    }
                }
            }
            if label.is_none() {
                for (kk, k) in entries.iter() {
                    if kk.level() == 0 {
                        if let Some(n) = k.name() {
                            label = Some(n.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }
    let label = match label {
        Some(l) => l,
        None => key.to_lower().name()?.to_string(),
    };
    let mut out = String::new();
    if state.contains(M::CONTROL_MASK) {
        out.push_str("Ctrl+");
    }
    if state.contains(M::ALT_MASK) {
        out.push_str("Alt+");
    }
    if state.contains(M::SHIFT_MASK) {
        out.push_str("Shift+");
    }
    if state.contains(M::SUPER_MASK) {
        out.push_str("Super+");
    }
    out.push_str(&label);
    Some(out)
}

/// Turns a default combo string back into a key event Rail's key handler understands.
pub(crate) fn sc_synth(combo: &str) -> Option<(gdk::Key, gdk::ModifierType)> {
    use gdk::ModifierType as M;
    let mut mods = M::empty();
    let mut name = "";
    for part in combo.split('+') {
        match part {
            "Ctrl" => mods |= M::CONTROL_MASK,
            "Alt" => mods |= M::ALT_MASK,
            "Shift" => mods |= M::SHIFT_MASK,
            "Super" => mods |= M::SUPER_MASK,
            other => name = other,
        }
    }
    let key = match name {
        "[" => gdk::Key::bracketleft,
        "]" => gdk::Key::bracketright,
        _ if name.chars().count() == 1 => gdk::Key::from_name(name.to_lowercase())?,
        _ => gdk::Key::from_name(name)?,
    };
    Some((key, mods))
}

pub(crate) fn sc_resolve(combo: &str) -> ScResolve {
    SHORTCUTS.with(|c| {
        let cfg = c.borrow();
        for def in SHORTCUT_DEFS {
            if sc_effective(def, &cfg).iter().any(|x| x == combo) {
                if !sc_enabled(&cfg, def.id) {
                    return ScResolve::Disabled;
                }
                return ScResolve::Action(def.defaults[0].to_string());
            }
        }
        for def in SHORTCUT_DEFS {
            if def.defaults.contains(&combo) {
                return ScResolve::Disabled;
            }
        }
        ScResolve::Unknown
    })
}
