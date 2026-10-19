//! Unit tests for session parsing, URIs, shortcuts and settings.

use super::*;

#[test]
fn test_parse_session_content() {
    let sample = "focus\t1\t0\nsidebar\t1\ntouchpad\t0\nwin\t1400\t850\t0\ntab\t0\t0\t1\tdefault\thttps://google.com\ntab\t1\t0\t1\twork\thttps://github.com\n";
    let state = parse_session_content(sample);
    assert_eq!(state.focus, (1, 0));
    assert_eq!(state.sidebar_open, Some(true));
    assert_eq!(state.touchpad_mode, Some(false));
    assert_eq!(state.win_size, Some((1400, 850, false)));
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.tabs[0], SessionTabItem {
        row: 0,
        col: 0,
        full: true,
        profile_id: "default".to_string(),
        url: "https://google.com".to_string(),
        custom_width: None,
    });
    assert_eq!(state.tabs[1], SessionTabItem {
        row: 1,
        col: 0,
        full: true,
        profile_id: "work".to_string(),
        url: "https://github.com".to_string(),
        custom_width: None,
    });
}

#[test]
fn test_parse_empty_session() {
    let state = parse_session_content("");
    assert_eq!(state.tabs.len(), 0);
    assert_eq!(state.focus, (0, 0));
    assert_eq!(state.sidebar_open, None);
    assert_eq!(state.touchpad_mode, None);
    assert_eq!(state.win_size, None);
}

#[test]
fn test_parse_legacy_session() {
    let sample = "tab\t0\twork\thttps://news.ycombinator.com\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0], SessionTabItem {
        row: 0,
        col: 0,
        full: true,
        profile_id: "work".to_string(),
        url: "https://news.ycombinator.com".to_string(),
        custom_width: None,
    });
}

#[test]
fn test_parse_session_with_custom_width() {
    let sample = "tab\t0\t0\t0\tdefault\thttps://claude.ai\t850.5\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].custom_width, Some(850.5));
}

#[test]
fn test_parse_session_with_search_engine() {
    let sample = "focus\t0\t0\nengine\tduckduckgo\ntab\t0\t0\t1\tdefault\thttps://duckduckgo.com\n";
    let state = parse_session_content(sample);
    assert_eq!(state.search_engine, Some("duckduckgo".to_string()));
    assert_eq!(state.tabs.len(), 1);
}

#[test]
fn test_to_uri_search_engines() {
    assert_eq!(to_uri("rust lang", "google"), "https://www.google.com/search?q=rust%20lang");
    assert_eq!(to_uri("rust lang", "duckduckgo"), "https://duckduckgo.com/?q=rust%20lang");
    assert_eq!(to_uri("rust lang", "brave"), "https://search.brave.com/search?q=rust%20lang");
    assert_eq!(to_uri("rust lang", "bing"), "https://www.bing.com/search?q=rust%20lang");
    assert_eq!(to_uri("rust lang", "ecosia"), "https://www.ecosia.org/search?q=rust%20lang");
    assert_eq!(to_uri("https://github.com", "google"), "https://github.com");
    assert_eq!(to_uri("github.com", "duckduckgo"), "https://github.com");
}

#[test]
fn test_settings_tab_enum() {
    assert_ne!(SettingsTab::General, SettingsTab::Profiles);
    assert_ne!(SettingsTab::Data, SettingsTab::Customize);
    assert_eq!(SettingsTab::General, SettingsTab::General);
}

#[test]
fn test_devtools_dock_side_cycling() {
    let cycle = |side: DevToolsDockSide| match side {
        DevToolsDockSide::Bottom => DevToolsDockSide::Right,
        DevToolsDockSide::Right => DevToolsDockSide::Left,
        DevToolsDockSide::Left => DevToolsDockSide::Undocked,
        DevToolsDockSide::Undocked => DevToolsDockSide::Bottom,
    };
    assert_eq!(cycle(DevToolsDockSide::Bottom), DevToolsDockSide::Right);
    assert_eq!(cycle(DevToolsDockSide::Right), DevToolsDockSide::Left);
    assert_eq!(cycle(DevToolsDockSide::Left), DevToolsDockSide::Undocked);
    assert_eq!(cycle(DevToolsDockSide::Undocked), DevToolsDockSide::Bottom);
}

#[test]
fn test_tab_width_pair_clamping() {
    let min_w = 280.0;
    let max_w = 1200.0;
    let initial0 = 500.0;
    let initial1 = 500.0;
    let total_pair = initial0 + initial1;

    let clamp_pair = |req_w0: f64| {
        let clamped_w0 = req_w0.clamp(min_w, (total_pair - min_w).min(max_w));
        let new_w1 = (total_pair - clamped_w0).max(min_w);
        (clamped_w0, new_w1)
    };

    // Test normal resize
    let (w0, w1) = clamp_pair(600.0);
    assert_eq!(w0, 600.0);
    assert_eq!(w1, 400.0);
    assert_eq!(w0 + w1, 1000.0);

    // Test below min_w constraint
    let (w0, w1) = clamp_pair(100.0);
    assert_eq!(w0, 280.0);
    assert_eq!(w1, 720.0);
    assert_eq!(w0 + w1, 1000.0);

    // Test above maximum allowed by neighbour's min_w
    let (w0, w1) = clamp_pair(950.0);
    assert_eq!(w0, 720.0);
    assert_eq!(w1, 280.0);
    assert_eq!(w0 + w1, 1000.0);
}

#[test]
fn test_app_settings_roundtrip() {
    let dir = std::env::temp_dir().join(format!("rail_test_settings_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("settings.json");
    let s = AppSettings {
        search_engine: "duckduckgo".to_string(),
        touchpad_mode: true,
        smooth_scrolling: false,
        webrtc_enabled: true,
        auto_update: false,
        devtools_dock: "right".to_string(),
    };
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"search_engine\": \"{}\",\n", minijson::escape(&s.search_engine)));
    out.push_str(&format!("  \"touchpad_navigation\": {},\n", s.touchpad_mode));
    out.push_str(&format!("  \"smooth_scrolling\": {},\n", s.smooth_scrolling));
    out.push_str(&format!("  \"webrtc_enabled\": {}\n", s.webrtc_enabled));
    out.push_str("}\n");
    std::fs::write(&path, &out).unwrap();

    let parsed = minijson::parse(&out).unwrap();
    assert_eq!(parsed.get("search_engine").and_then(|v| v.as_str()), Some("duckduckgo"));
    assert_eq!(parsed.get("touchpad_navigation").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(parsed.get("smooth_scrolling").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(parsed.get("webrtc_enabled").and_then(|v| v.as_bool()), Some(true));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_shortcut_master_and_individual_priority() {
    let mut cfg = ShortcutConfig::default();
    // Initially all enabled
    assert!(SHORTCUT_DEFS.iter().all(|d| sc_enabled(&cfg, d.id)));

    // User disables one shortcut (e.g. "data" Alt+I)
    cfg.disabled.insert("data".to_string());
    assert!(!sc_enabled(&cfg, "data"));
    assert!(!SHORTCUT_DEFS.iter().all(|d| sc_enabled(&cfg, d.id)));

    // Master checkbox turned ON -> forces all shortcuts enabled
    cfg.disabled.clear();
    cfg.site_wins = false;
    assert!(SHORTCUT_DEFS.iter().all(|d| sc_enabled(&cfg, d.id)));

    // Master checkbox turned OFF -> forces all shortcuts disabled except locked settings
    for def in SHORTCUT_DEFS {
        if !sc_is_locked(def.id) {
            cfg.disabled.insert(def.id.to_string());
        }
    }
    cfg.site_wins = true;
    assert!(!sc_enabled(&cfg, "data"));
    assert!(!sc_enabled(&cfg, "ws_1"));
    // settings is not in SC_LOCKED, so it is disabled with the rest
    assert!(!sc_enabled(&cfg, "settings"));
}

#[test]
fn test_ctrl_tab_shortcuts_definitions() {
    for num in 1..=9 {
        let id = format!("tab_{}", num);
        let def = SHORTCUT_DEFS.iter().find(|d| d.id == id).expect("tab shortcut exists");
        let expected = format!("Ctrl+{}", num);
        assert_eq!(def.defaults[0], expected.as_str());
    }
}

#[test]
fn test_standard_browser_shortcuts() {
    for (id, combo) in [("new_tab", "Ctrl+T"), ("focus_url", "Ctrl+L"), ("close_tab", "Ctrl+W"), ("reload", "Ctrl+R"), ("quit", "Ctrl+Q")] {
        let def = SHORTCUT_DEFS.iter().find(|d| d.id == id).expect("shortcut exists");
        assert_eq!(def.defaults[0], combo);
    }
}

#[test]
fn test_parse_session_stack_lines() {
    let sample = "tab\t0\t0\t1\tdefault\thttps://a.com\ntab\t0\t1\t1\tdefault\thttps://b.com\nstack\t0\t1\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.stacked, vec![(0, 1)]);
    assert!(parse_session_content("tab\t0\t0\t1\tdefault\thttps://a.com\n").stacked.is_empty());
}

#[test]
fn test_internal_uris_are_not_loadable() {
    assert!(is_internal_uri("rail://start/"));
    assert!(!is_internal_uri("https://example.com"));
    assert!(!is_internal_uri(""));
}

#[test]
fn test_stack_shortcuts() {
    for (id, combo) in [("stack_left", "Alt+["), ("stack_right", "Alt+]")] {
        let def = SHORTCUT_DEFS.iter().find(|d| d.id == id).expect("shortcut exists");
        assert_eq!(def.defaults[0], combo);
    }
    assert_eq!(sc_synth("Alt+[").map(|(k, _)| k), Some(gdk::Key::bracketleft));
    assert_eq!(sc_synth("Alt+]").map(|(k, _)| k), Some(gdk::Key::bracketright));
}

/// One action = one shortcut, and one shortcut = one action.
#[test]
fn test_no_duplicate_default_shortcuts() {
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for def in SHORTCUT_DEFS {
        assert_eq!(def.defaults.len(), 1, "\"{}\" must have exactly one default shortcut", def.id);
        if let Some(prev) = seen.insert(def.defaults[0], def.id) {
            panic!("{} is bound to both \"{}\" and \"{}\"", def.defaults[0], prev, def.id);
        }
    }
}

#[test]
fn test_ui_shortcut_labels_follow_config() {
    // Standart: matn o'zgarmaydi
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    assert_eq!(sc_display("focus_url"), "Ctrl+L");
    assert_eq!(sc_apply("+ New Workspace (Alt+Shift+V)", &["new_workspace"]), "+ New Workspace (Alt+Shift+V)");

    // Foydalanuvchi o'zgartirgan: matn joriy qiymatni ko'rsatadi (tarjima qilingan matnda ham)
    let mut cfg = ShortcutConfig::default();
    cfg.overrides.insert("new_workspace".to_string(), vec!["Ctrl+Shift+N".to_string()]);
    // ikki shortcut o'rin almashsa ham to'qnashmaydi
    cfg.overrides.insert("new_tab".to_string(), vec!["Ctrl+L".to_string()]);
    cfg.overrides.insert("focus_url".to_string(), vec!["Ctrl+K".to_string()]);
    SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
    assert_eq!(sc_display("new_workspace"), "Ctrl+Shift+N");
    assert_eq!(sc_apply("+ Yangi ish maydoni (Alt+Shift+V)", &["new_workspace"]), "+ Yangi ish maydoni (Ctrl+Shift+N)");
    assert_eq!(sc_apply("Omnibar (Ctrl+T / Ctrl+L)", &["new_tab", "focus_url"]), "Omnibar (Ctrl+L / Ctrl+K)");

    // O'chirilgan: qavs va ajratgich chiroyli olib tashlanadi
    let mut cfg = ShortcutConfig::default();
    cfg.disabled.insert("devtools".to_string());
    cfg.disabled.insert("profiles".to_string());
    SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
    assert_eq!(sc_display("devtools"), "");
    assert_eq!(sc_apply("Close DevTools (Esc / F12)", &["devtools"]), "Close DevTools (Esc)");
    assert_eq!(sc_apply("Manage Profiles in Settings (Alt+P)", &["profiles"]), "Manage Profiles in Settings");
}

#[test]
fn test_page_zoom_steps_cover_20_to_500() {
    assert_eq!(next_page_zoom(1.0, 1), 1.10);
    assert_eq!(next_page_zoom(1.0, -1), 0.90);
    assert_eq!(next_page_zoom(0.20, -1), 0.20);
    assert_eq!(next_page_zoom(5.0, 1), 5.0);
    let mut z = 1.0;
    for _ in 0..40 {
        z = next_page_zoom(z, 1);
    }
    assert_eq!(z, 5.0);
    for _ in 0..40 {
        z = next_page_zoom(z, -1);
    }
    assert_eq!(z, 0.20);
    // off-step value snaps to the neighbouring step
    assert_eq!(next_page_zoom(1.05, 1), 1.10);
    assert_eq!(next_page_zoom(1.05, -1), 1.0);
}

#[test]
fn test_zoom_keys_plus_works_without_shift() {
    // Ctrl+= (US layout, no Shift) must zoom in, same as Ctrl+Shift++
    assert_eq!(App::zoom_dir(gdk::Key::equal, 21, false), Some(1));
    assert_eq!(App::zoom_dir(gdk::Key::plus, 21, true), Some(1));
    assert_eq!(App::zoom_dir(gdk::Key::KP_Add, 86, false), Some(1));
    assert_eq!(App::zoom_dir(gdk::Key::minus, 20, false), Some(-1));
    assert_eq!(App::zoom_dir(gdk::Key::KP_Subtract, 82, false), Some(-1));
    assert_eq!(App::zoom_dir(gdk::Key::_0, 19, false), Some(0));
    assert_eq!(App::zoom_dir(gdk::Key::a, 38, false), None);
}
