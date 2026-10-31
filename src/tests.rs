//! Unit tests for session parsing, URIs, shortcuts and settings.

use super::*;

#[test]
fn test_parse_session_content() {
    let sample = "focus\t1\t0\nsidebar\t1\nwin\t1400\t850\t0\ntab\t0\t0\t1\tdefault\thttps://google.com\ntab\t1\t0\t1\twork\thttps://github.com\n";
    let state = parse_session_content(sample);
    assert_eq!(state.focus, (1, 0));
    assert_eq!(state.sidebar_open, Some(true));
    assert_eq!(state.win_size, Some((1400, 850, false)));
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(
        state.tabs[0],
        SessionTabItem {
            row: 0,
            col: 0,
            full: true,
            profile_id: "default".to_string(),
            url: "https://google.com".to_string(),
            custom_width: None,
            custom_height: None,
            tabbed: false,
        }
    );
    assert_eq!(
        state.tabs[1],
        SessionTabItem {
            row: 1,
            col: 0,
            full: true,
            profile_id: "work".to_string(),
            url: "https://github.com".to_string(),
            custom_width: None,
            custom_height: None,
            tabbed: false,
        }
    );
}

#[test]
fn test_parse_empty_session() {
    let state = parse_session_content("");
    assert_eq!(state.tabs.len(), 0);
    assert_eq!(state.focus, (0, 0));
    assert_eq!(state.sidebar_open, None);
    assert_eq!(state.win_size, None);
}

#[test]
fn test_parse_legacy_session() {
    let sample = "tab\t0\twork\thttps://news.ycombinator.com\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(
        state.tabs[0],
        SessionTabItem {
            row: 0,
            col: 0,
            full: true,
            profile_id: "work".to_string(),
            url: "https://news.ycombinator.com".to_string(),
            custom_width: None,
            custom_height: None,
            tabbed: false,
        }
    );
}

#[test]
fn test_parse_session_with_custom_width() {
    let sample = "tab\t0\t0\t0\tdefault\thttps://claude.ai\t850.5\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].custom_width, Some(850.5));
}

#[test]
fn test_parse_session_with_custom_height_ratio() {
    let sample = "tab\t0\t0\t0\tdefault\thttps://example.test\t\t0.666667\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].custom_width, None);
    assert_eq!(state.tabs[0].custom_height, Some(0.666667));
}

#[test]
fn test_parse_session_with_tabbed_column_flag() {
    let sample = "tab\t0\t0\t1\tdefault\thttps://one.test\t\t0.5\t1\ntab\t0\t1\t1\tdefault\thttps://two.test\t\t0.5\t0\nstack\t0\t1\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 2);
    assert!(state.tabs[0].tabbed);
    assert!(!state.tabs[1].tabbed);
    assert_eq!(state.stacked, vec![(0, 1)]);
}

#[test]
fn test_parse_session_rejects_invalid_custom_height_ratio() {
    let sample = "tab\t0\t0\t0\tdefault\thttps://example.test\t\t9.0\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs[0].custom_height, None);
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
    assert_eq!(
        to_uri("rust lang", "google"),
        "https://www.google.com/search?q=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "duckduckgo"),
        "https://duckduckgo.com/?q=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "brave"),
        "https://search.brave.com/search?q=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "bing"),
        "https://www.bing.com/search?q=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "ecosia"),
        "https://www.ecosia.org/search?q=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "yandex"),
        "https://yandex.com/search/?text=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "yahoo"),
        "https://search.yahoo.com/search?p=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "perplexity"),
        "https://www.perplexity.ai/search?q=rust%20lang"
    );
    assert_eq!(
        to_uri("rust lang", "kagi"),
        "https://kagi.com/search?q=rust%20lang"
    );
    assert_eq!(
        to_uri("gh rust gtk", "google"),
        "https://github.com/search?q=rust%20gtk"
    );
    assert_eq!(
        to_uri("gl webkit", "google"),
        "https://gitlab.com/search?search=webkit"
    );
    assert_eq!(
        to_uri("mdn AbortController", "google"),
        "https://developer.mozilla.org/en-US/search?q=AbortController"
    );
    assert_eq!(
        to_uri("so gtk4-rs", "google"),
        "https://stackoverflow.com/search?q=gtk4-rs"
    );
    assert_eq!(
        to_uri("crates serde json", "google"),
        "https://crates.io/search?q=serde%20json"
    );
    assert_eq!(
        to_uri("npm vite", "google"),
        "https://www.npmjs.com/search?q=vite"
    );
    assert_eq!(
        to_uri("py pydantic", "google"),
        "https://pypi.org/search/?q=pydantic"
    );
    assert_eq!(
        to_uri("docker postgres", "google"),
        "https://hub.docker.com/search?q=postgres"
    );
    assert_eq!(
        to_uri("k8s ingress", "google"),
        "https://kubernetes.io/search/?q=ingress"
    );
    assert_eq!(
        to_uri("ghcode async rust", "google"),
        "https://github.com/search?type=code&q=async%20rust"
    );
    assert_eq!(to_uri("https://github.com", "google"), "https://github.com");
    assert_eq!(to_uri("github.com", "duckduckgo"), "https://github.com");
    assert_eq!(
        to_uri("/home/user/page.html", "google"),
        "file:///home/user/page.html"
    );
    assert_eq!(
        to_uri("home/muhamad/computer-analysis/index.html", "google"),
        "file:///home/muhamad/computer-analysis/index.html"
    );
    assert_eq!(to_uri("tmp/test.html", "google"), "file:///tmp/test.html");
    assert_eq!(to_uri("etc/hosts", "google"), "file:///etc/hosts");
    assert_eq!(
        to_uri("file:///tmp/test.html", "google"),
        "file:///tmp/test.html"
    );
    assert_eq!(
        to_uri("file:/tmp/test.html", "google"),
        "file:///tmp/test.html"
    );
    assert_eq!(to_uri("localhost:3000", "google"), "http://localhost:3000");
    assert_eq!(to_uri("127.0.0.1:8080", "google"), "http://127.0.0.1:8080");
    assert_eq!(to_uri("192.168.1.1", "google"), "http://192.168.1.1");
}

#[test]
fn test_settings_tab_enum() {
    assert_ne!(SettingsTab::General, SettingsTab::Profiles);
    assert_ne!(SettingsTab::Data, SettingsTab::Customize);
    assert_eq!(SettingsTab::General, SettingsTab::General);
    assert_eq!(
        settings_tab_from_uri("rail://settings/extensions"),
        Some(SettingsTab::General)
    );
    assert_eq!(
        crate::nav::normalize_internal_uri("rail://history"),
        "rail://settings/data"
    );
    assert_eq!(
        crate::nav::to_uri("history", "google"),
        "rail://settings/data"
    );
    assert_eq!(
        crate::nav::to_uri("chrome://history", "google"),
        "rail://settings/data"
    );
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
        smooth_scrolling: false,
        webrtc_enabled: true,
        auto_update: false,
        devtools_dock: "right".to_string(),
        translate_source_lang: "auto".to_string(),
        translate_target_lang: "uz".to_string(),
        sidebar_mode: "fixed".to_string(),
    };
    let mut out = String::from("{\n");
    out.push_str(&format!(
        "  \"search_engine\": \"{}\",\n",
        minijson::escape(&s.search_engine)
    ));
    out.push_str(&format!(
        "  \"smooth_scrolling\": {},\n",
        s.smooth_scrolling
    ));
    out.push_str(&format!("  \"webrtc_enabled\": {}\n", s.webrtc_enabled));
    out.push_str("}\n");
    std::fs::write(&path, &out).unwrap();

    let parsed = minijson::parse(&out).unwrap();
    assert_eq!(
        parsed.get("search_engine").and_then(|v| v.as_str()),
        Some("duckduckgo")
    );
    assert_eq!(
        parsed.get("smooth_scrolling").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        parsed.get("webrtc_enabled").and_then(|v| v.as_bool()),
        Some(true)
    );
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
        let def = SHORTCUT_DEFS
            .iter()
            .find(|d| d.id == id)
            .expect("tab shortcut exists");
        let expected = format!("Ctrl+{}", num);
        assert_eq!(def.defaults[0], expected.as_str());
    }
}

#[test]
fn test_standard_browser_shortcuts() {
    for (id, combo) in [
        ("new_tab", "Ctrl+T"),
        ("focus_url", "Ctrl+L"),
        ("close_tab", "Ctrl+W"),
        ("reload", "Ctrl+R"),
        ("quit", "Ctrl+Q"),
    ] {
        let def = SHORTCUT_DEFS
            .iter()
            .find(|d| d.id == id)
            .expect("shortcut exists");
        assert_eq!(def.defaults[0], combo);
    }
}

#[test]
fn test_parse_session_stack_lines() {
    let sample =
        "tab\t0\t0\t1\tdefault\thttps://a.com\ntab\t0\t1\t1\tdefault\thttps://b.com\nstack\t0\t1\n";
    let state = parse_session_content(sample);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.stacked, vec![(0, 1)]);
    assert!(
        parse_session_content("tab\t0\t0\t1\tdefault\thttps://a.com\n")
            .stacked
            .is_empty()
    );
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
        let def = SHORTCUT_DEFS
            .iter()
            .find(|d| d.id == id)
            .expect("shortcut exists");
        assert_eq!(def.defaults[0], combo);
    }
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    assert!(matches!(
        sc_resolve("Alt+["),
        ScResolve::Action("stack_left")
    ));
    assert!(matches!(
        sc_resolve("Alt+]"),
        ScResolve::Action("stack_right")
    ));
}

/// One action = one shortcut, and one shortcut = one action (or multiple non-conflicting default shortcuts).
#[test]
fn test_no_duplicate_default_shortcuts() {
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for def in SHORTCUT_DEFS {
        assert!(
            !def.defaults.is_empty(),
            "\"{}\" must have at least one default shortcut",
            def.id
        );
        for d in def.defaults {
            if let Some(prev) = seen.insert(*d, def.id) {
                panic!("{} is bound to both \"{}\" and \"{}\"", d, prev, def.id);
            }
        }
    }
}

#[test]
fn test_devtools_shortcuts() {
    let def = SHORTCUT_DEFS
        .iter()
        .find(|d| d.id == "devtools")
        .expect("devtools shortcut exists");
    assert!(def.defaults.contains(&"F12"));
    assert!(def.defaults.contains(&"Ctrl+Shift+I"));
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    assert!(matches!(
        sc_resolve("Ctrl+Shift+I"),
        ScResolve::Action("devtools")
    ));
    assert!(matches!(sc_resolve("F12"), ScResolve::Action("devtools")));
}

/// Customize ro'yxatidagi har bir amalning handleri bor, handlerlar ro'yxatida esa ortiqcha ID yo'q.
#[test]
fn test_every_shortcut_has_a_handler() {
    for def in SHORTCUT_DEFS {
        assert!(
            App::action_known(def.id),
            "\"{}\" Customize ro'yxatida bor, lekin run_action da handleri yo'q",
            def.id
        );
    }
    for id in crate::app::STATIC_ACTIONS {
        assert!(
            SHORTCUT_DEFS.iter().any(|d| d.id == *id),
            "\"{}\" handler bor, lekin Customize ro'yxatida yo'q",
            id
        );
    }
}

/// Har bir standart tugma o'z amaliga olib borishi kerak.
#[test]
fn test_default_combos_resolve_to_own_action() {
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    for def in SHORTCUT_DEFS {
        for d in def.defaults {
            match sc_resolve(d) {
                ScResolve::Action(id) => assert_eq!(id, def.id, "{} -> {}", d, id),
                _ => panic!("{} ({}) hech qaysi amalga ulanmagan", d, def.id),
            }
        }
    }
}

/// Foydalanuvchi tugmani o'zgartirsa: yangi tugma ishlaydi, eskisi sahifaga qaytadi, o'chirilgani ham.
#[test]
fn test_niri_layout_shortcuts_resolve_from_gdk_key_names() {
    use gdk::ModifierType as M;
    SHORTCUTS.with(|config| *config.borrow_mut() = ShortcutConfig::default());
    let resolve = |key, modifiers| {
        let combos = shortcut_combos(key, 0, modifiers);
        sc_resolve_any(&combos)
    };
    assert!(matches!(
        resolve(gdk::Key::Page_Up, M::ALT_MASK),
        ScResolve::Action("focus_workspace_up")
    ));
    assert!(matches!(
        resolve(gdk::Key::Page_Down, M::ALT_MASK),
        ScResolve::Action("focus_workspace_down")
    ));
    assert!(matches!(
        resolve(gdk::Key::comma, M::ALT_MASK),
        ScResolve::Action("consume_into_left")
    ));
}

#[test]
fn test_customized_shortcut_follows_action_not_key() {
    let mut cfg = ShortcutConfig::default();
    cfg.overrides
        .insert("find".to_string(), vec!["Alt+G".to_string()]);
    cfg.overrides
        .insert("full_tab".to_string(), vec!["Ctrl+Shift+F".to_string()]);
    cfg.disabled.insert("fullscreen".to_string());
    SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
    assert!(matches!(sc_resolve("Alt+G"), ScResolve::Action("find")));
    assert!(matches!(sc_resolve("Ctrl+F"), ScResolve::Disabled));
    assert!(matches!(
        sc_resolve("Ctrl+Shift+F"),
        ScResolve::Action("full_tab")
    ));
    assert!(matches!(sc_resolve("Alt+F"), ScResolve::Disabled));
    assert!(matches!(sc_resolve("F11"), ScResolve::Disabled));
    // ro'yxatda umuman yo'q tugma qat'iy tugmalarga (Esc, overview) qoladi
    assert!(matches!(sc_resolve("Escape"), ScResolve::Unknown));
    assert!(matches!(sc_resolve("Shift+Return"), ScResolve::Unknown));
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
}

#[test]
fn test_combo_spelling_is_canonical() {
    assert_eq!(sc_canon("Ctrl+minus"), sc_canon("Ctrl+-"));
    assert_eq!(sc_canon("Ctrl+equal"), sc_canon("Ctrl+="));
    assert_eq!(sc_canon("Shift+Ctrl+t"), sc_canon("Ctrl+Shift+T"));
    assert_eq!(sc_canon("Ctrl++"), sc_canon("Ctrl+plus"));
    assert_eq!(sc_canon("Ctrl+KP_Add"), sc_canon("Ctrl+plus"));
    assert_ne!(sc_canon("Ctrl+T"), sc_canon("Ctrl+Shift+T"));
}

#[test]
fn test_find_reload_zoom_defaults() {
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    for (combo, id) in [
        ("Ctrl+F", "find"),
        ("F3", "find_next"),
        ("Ctrl+G", "find_next"),
        ("Shift+F3", "find_prev"),
        ("Ctrl+Shift+G", "find_prev"),
        ("F5", "reload"),
        ("Ctrl+R", "reload"),
        ("Ctrl+F5", "hard_reload"),
        ("Shift+F5", "hard_reload"),
        ("Ctrl+Shift+R", "hard_reload"),
        ("Ctrl+=", "overview_zoom_in"),
        ("Ctrl+plus", "overview_zoom_in"),
        ("Ctrl+-", "overview_zoom_out"),
        ("Ctrl+0", "zoom_reset"),
        ("Ctrl+Shift+T", "reopen_tab"),
        ("Ctrl+H", "history"),
        ("Alt+F", "full_tab"),
        ("F11", "fullscreen"),
    ] {
        match sc_resolve(combo) {
            ScResolve::Action(got) => assert_eq!(got, id, "{}", combo),
            _ => panic!("{} -> {} ga ulanmagan", combo, id),
        }
    }
}

/// Tugma hodisasi: Shift+= (US) zoom-in, keypad + / 0 / strelkalar ham o'z amaliga tushadi.
#[test]
fn test_key_events_resolve_through_all_spellings() {
    use gdk::ModifierType as M;
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    let r = |k: gdk::Key, code: u32, m: M| sc_resolve_any(&shortcut_combos(k, code, m));
    assert!(matches!(
        r(gdk::Key::plus, 21, M::CONTROL_MASK | M::SHIFT_MASK),
        ScResolve::Action("overview_zoom_in")
    ));
    assert!(matches!(
        r(gdk::Key::equal, 21, M::CONTROL_MASK),
        ScResolve::Action("overview_zoom_in")
    ));
    assert!(matches!(
        r(gdk::Key::KP_Add, 86, M::CONTROL_MASK),
        ScResolve::Action("overview_zoom_in")
    ));
    assert!(matches!(
        r(gdk::Key::KP_Subtract, 82, M::CONTROL_MASK),
        ScResolve::Action("overview_zoom_out")
    ));
    assert!(matches!(
        r(gdk::Key::KP_0, 90, M::CONTROL_MASK),
        ScResolve::Action("zoom_reset")
    ));
    assert!(matches!(
        r(gdk::Key::KP_3, 89, M::CONTROL_MASK),
        ScResolve::Action("tab_3")
    ));
    assert!(matches!(
        r(gdk::Key::KP_Left, 83, M::ALT_MASK),
        ScResolve::Action("win_left")
    ));
    assert!(matches!(
        r(gdk::Key::ISO_Left_Tab, 23, M::CONTROL_MASK | M::SHIFT_MASK),
        ScResolve::Action("prev_tab")
    ));
    // oddiy harf / Esc ro'yxatga kirmaydi
    assert!(matches!(r(gdk::Key::h, 43, M::empty()), ScResolve::Unknown));
    assert!(matches!(
        r(gdk::Key::Escape, 9, M::empty()),
        ScResolve::Unknown
    ));
}

#[test]
fn test_ui_shortcut_labels_follow_config() {
    // Standart: matn o'zgarmaydi
    SHORTCUTS.with(|c| *c.borrow_mut() = ShortcutConfig::default());
    assert_eq!(sc_display("focus_url"), "Ctrl+L");
    assert_eq!(
        sc_apply("+ New Workspace (Alt+Shift+V)", &["new_workspace"]),
        "+ New Workspace (Alt+Shift+V)"
    );

    // Foydalanuvchi o'zgartirgan: matn joriy qiymatni ko'rsatadi (tarjima qilingan matnda ham)
    let mut cfg = ShortcutConfig::default();
    cfg.overrides.insert(
        "new_workspace".to_string(),
        vec!["Ctrl+Shift+N".to_string()],
    );
    // ikki shortcut o'rin almashsa ham to'qnashmaydi
    cfg.overrides
        .insert("new_tab".to_string(), vec!["Ctrl+L".to_string()]);
    cfg.overrides
        .insert("focus_url".to_string(), vec!["Ctrl+K".to_string()]);
    SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
    assert_eq!(sc_display("new_workspace"), "Ctrl+Shift+N");
    assert_eq!(
        sc_apply("+ Yangi ish maydoni (Alt+Shift+V)", &["new_workspace"]),
        "+ Yangi ish maydoni (Ctrl+Shift+N)"
    );
    assert_eq!(
        sc_apply("Omnibar (Ctrl+T / Ctrl+L)", &["new_tab", "focus_url"]),
        "Omnibar (Ctrl+L / Ctrl+K)"
    );

    // O'chirilgan: qavs va ajratgich chiroyli olib tashlanadi
    let mut cfg = ShortcutConfig::default();
    cfg.disabled.insert("devtools".to_string());
    cfg.disabled.insert("profiles".to_string());
    SHORTCUTS.with(|c| *c.borrow_mut() = cfg);
    assert_eq!(sc_display("devtools"), "");
    assert_eq!(
        sc_apply("Close DevTools (Esc / F12)", &["devtools"]),
        "Close DevTools (Esc)"
    );
    assert_eq!(
        sc_apply("Manage Profiles in Settings (Alt+P)", &["profiles"]),
        "Manage Profiles in Settings"
    );
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
    // Immediately reversing direction must return to the previous zoom step.
    assert_eq!(next_page_zoom(next_page_zoom(1.0, -1), 1), 1.0);
    assert_eq!(next_page_zoom(next_page_zoom(1.0, 1), -1), 1.0);
}

#[test]
fn test_external_uri_protocols_are_routed_outside_webkit() {
    for scheme in [
        "mailto", "tel", "sms", "sip", "sips", "webcal", "xmpp", "facetime", "tg", "telegram",
        "magnet", "slack", "discord", "zoommtg", "zoomus", "skype", "callto", "signal", "viber",
        "whatsapp", "msteams", "ms-teams", "steam", "spotify",
    ] {
        assert!(
            is_external_uri_scheme(scheme),
            "missing external scheme: {scheme}"
        );
    }
    for scheme in ["http", "https", "file", "blob", "data", "javascript"] {
        assert!(
            !is_external_uri_scheme(scheme),
            "web scheme intercepted: {scheme}"
        );
    }
}

#[test]
fn test_roomy_tab_cap_scales_with_physical_memory() {
    assert_eq!(roomy_max_live_tabs(Some(4 * 1024)), 6);
    assert_eq!(roomy_max_live_tabs(Some(8 * 1024)), 8);
    assert_eq!(roomy_max_live_tabs(Some(16 * 1024)), 16);
    assert_eq!(roomy_max_live_tabs(Some(32 * 1024)), 20);
    assert_eq!(roomy_max_live_tabs(None), 8);
}

#[test]
fn test_zoom_math_bounds_and_hold_rate_are_stable() {
    assert_eq!(scale_zoom(1.0, 10.0, 0.2, 5.0), 5.0);
    assert_eq!(scale_zoom(1.0, 0.01, 0.2, 5.0), 0.2);
    assert_eq!(scale_zoom(f64::NAN, 1.1, 0.2, 5.0), 1.1);
    assert_eq!(scale_zoom(2.0, f64::NAN, 0.2, 5.0), 2.0);
    assert_eq!(next_page_zoom(f64::NAN, 1), 1.1);
    assert_eq!(overview_scale(f64::NAN), OV_ZOOM);

    let one_frame = zoom_hold_factor(1, 1.0 / 60.0);
    let capped_frame = zoom_hold_factor(1, 1.0);
    assert!((one_frame - (ZOOM_HOLD_RATE / 60.0).exp()).abs() < 1e-12);
    assert!((capped_frame - (ZOOM_HOLD_RATE * 0.05).exp()).abs() < 1e-12);
    assert_eq!(zoom_hold_factor(-1, 0.0), 1.0);
    assert_eq!(zoom_hold_factor(0, 0.05), 1.0);
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

#[test]
fn test_translate_host_escapes_hyphens() {
    assert_eq!(
        translate_host("www.example.com"),
        "www-example-com.translate.goog"
    );
    assert_eq!(
        translate_host("my-site.co.uk"),
        "my--site-co-uk.translate.goog"
    );
}

#[test]
fn test_translate_url_building() {
    assert_eq!(
        translate_url("https://en.wikipedia.org/wiki/Rust?x=1#top", "auto", "uz").as_deref(),
        Some(
            "https://en-wikipedia-org.translate.goog/wiki/Rust?x=1&_x_tr_sl=auto&_x_tr_tl=uz&_x_tr_hl=uz#top"
        )
    );
    assert_eq!(
        translate_url("http://example.com", "en", "ru").as_deref(),
        Some("https://example-com.translate.goog/?_x_tr_sl=en&_x_tr_tl=ru&_x_tr_hl=ru")
    );
    // old _x_tr_* params are replaced, not duplicated
    assert_eq!(
        translate_url("https://a.com/p?_x_tr_sl=de&q=1", "fr", "uz").as_deref(),
        Some("https://a-com.translate.goog/p?q=1&_x_tr_sl=fr&_x_tr_tl=uz&_x_tr_hl=uz")
    );
    // unsafe language codes fall back to defaults
    assert_eq!(
        translate_url("https://a.com/", "x&y=1", "").as_deref(),
        Some("https://a-com.translate.goog/?_x_tr_sl=auto&_x_tr_tl=uz&_x_tr_hl=uz")
    );
}

#[test]
fn test_translate_url_rejects_untranslatable() {
    for u in [
        "rail://start/",
        "about:blank",
        "file:///tmp/a.html",
        "http://localhost:8080/",
        "http://localhost/",
        "http://192.168.1.1/",
        "http://[::1]/",
        "https://a.com:8443/",
        "https://a-com.translate.goog/",
    ] {
        assert!(translate_url(u, "auto", "uz").is_none(), "{u}");
    }
}

#[test]
fn test_untranslate_round_trip() {
    for u in [
        "https://en.wikipedia.org/wiki/Rust?x=1#top",
        "https://my-site.co.uk/a/b",
        "https://example.com/",
    ] {
        let t = translate_url(u, "auto", "uz").unwrap();
        assert!(t.contains(".translate.goog/"));
        let back = untranslate_url(&t).unwrap();
        assert_eq!(back, u.replace("?x=1#top", "?x=1#top"));
    }
    assert!(untranslate_url("https://example.com/").is_none());
}

#[test]
fn test_normalize_detected_lang() {
    assert_eq!(normalize_detected_lang("en-US"), "en");
    assert_eq!(normalize_detected_lang("en_US"), "en");
    assert_eq!(normalize_detected_lang("RU"), "ru");
    assert_eq!(normalize_detected_lang(""), "auto");
    assert_eq!(normalize_detected_lang("  "), "auto");
    assert_eq!(normalize_detected_lang("x"), "auto");
    assert_eq!(normalize_detected_lang("zh-TW"), "zh-TW");
    assert_eq!(normalize_detected_lang("zh"), "zh-CN");
    assert_eq!(normalize_detected_lang("nb-NO"), "no");
}

#[test]
fn test_supported_languages_list() {
    assert_eq!(SUPPORTED_LANGUAGES[0].0, "auto");
    assert_eq!(SUPPORTED_LANGUAGES[1].0, "uz");
    let mut seen = std::collections::HashSet::new();
    for (code, name) in SUPPORTED_LANGUAGES {
        assert!(seen.insert(*code), "dublikat: {code}");
        assert_eq!(clean_lang_code(code, "?"), *code, "{code}");
        assert!(!name.is_empty());
    }
    assert_eq!(SUPPORTED_LANGUAGES.len(), 145);
}

#[test]
fn test_translate_url_zh_maps_to_zh_cn() {
    assert_eq!(
        translate_url("https://a.com/", "auto", "zh").as_deref(),
        Some("https://a-com.translate.goog/?_x_tr_sl=auto&_x_tr_tl=zh-CN&_x_tr_hl=zh-CN")
    );
}

#[test]
fn test_every_language_has_flag_png() {
    for (code, _) in SUPPORTED_LANGUAGES {
        match *code {
            "auto" | "eo" => assert!(lang_flag_bytes(code).is_none(), "{code}"),
            _ => {
                let b = lang_flag_bytes(code).unwrap_or_else(|| panic!("bayroq yo'q: {code}"));
                assert_eq!(&b[..4], b"\x89PNG", "{code}");
            }
        }
    }
}

#[test]
fn test_pwa_slugify_and_cli_app() {
    assert_eq!(crate::pwa::slugify("YouTube"), "youtube");
    assert_eq!(crate::pwa::slugify("Telegram Web"), "telegram-web");
    assert_eq!(crate::pwa::slugify("SpaceMR — App!"), "spacemr-app");

    let args1 = vec!["rail".into(), "--app=https://youtube.com".into()];
    assert_eq!(
        crate::parse_cli_app(&args1),
        Some("https://youtube.com".into())
    );

    let args2 = vec![
        "rail".into(),
        "--app".into(),
        "https://spacemr.vercel.app".into(),
    ];
    assert_eq!(
        crate::parse_cli_app(&args2),
        Some("https://spacemr.vercel.app".into())
    );

    let args3 = vec!["rail".into(), "https://google.com".into()];
    assert_eq!(crate::parse_cli_app(&args3), None);
}
