//! Address-bar input to URI, and rail:// internal pages.

use crate::*;

pub(crate) const SEARCH_ENGINE_LIST: &[(&str, &str, &str)] = &[
    ("google", "Google", "https://www.google.com/search?q={}"),
    ("bing", "Bing", "https://www.bing.com/search?q={}"),
    ("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q={}"),
    ("yandex", "Yandex", "https://yandex.com/search/?text={}"),
    ("yahoo", "Yahoo!", "https://search.yahoo.com/search?p={}"),
    ("brave", "Brave", "https://search.brave.com/search?q={}"),
    ("baidu", "Baidu", "https://www.baidu.com/s?wd={}"),
    ("ecosia", "Ecosia", "https://www.ecosia.org/search?q={}"),
    ("startpage", "Startpage", "https://www.startpage.com/sp/search?query={}"),
    ("qwant", "Qwant", "https://www.qwant.com/?q={}"),
    ("perplexity", "Perplexity", "https://www.perplexity.ai/search?q={}"),
    ("searxng", "SearXNG", "https://searx.be/search?q={}"),
    ("kagi", "Kagi", "https://kagi.com/search?q={}"),
    ("naver", "Naver", "https://search.naver.com/search.naver?query={}"),
    ("ask", "Ask.com", "https://www.ask.com/web?q={}"),
];

pub(crate) fn to_uri(input: &str, search_engine: &str) -> String {
    let s = input.trim();
    let s = s.strip_prefix("view-source:").unwrap_or(s).trim();
    // Short internal names → rail:// URLs
    match s.to_ascii_lowercase().as_str() {
        "settings" | "rail:settings" => return "rail://settings".into(),
        "downloads" | "rail:downloads" => return "rail://downloads".into(),
        "start" | "newtab" | "rail:start" => return "rail://start/".into(),
        _ => {}
    }
    if s.starts_with("rail:") && !s.contains("://") {
        // rail:settings → rail://settings
        let rest = s.trim_start_matches("rail:").trim_start_matches('/');
        return format!("rail://{rest}");
    }

    // 1. Local filesystem files (file://, /absolute/path, ~/relative/to/home, ./relative)
    if s.starts_with("file://") {
        return s.into();
    }
    if s.starts_with("file:") {
        let path = s.trim_start_matches("file:").trim_start_matches('/');
        return format!("file:///{path}");
    }
    if s.starts_with('/') {
        return format!("file://{s}");
    }
    if s.starts_with("~/") {
        let home = glib::home_dir();
        return format!("file://{}/{}", home.display(), &s[2..]);
    }
    if s.starts_with("./") || s.starts_with("../") {
        if let Ok(cur) = std::env::current_dir() {
            let full = cur.join(s);
            if let Ok(canon) = std::fs::canonicalize(&full) {
                return format!("file://{}", canon.display());
            }
            return format!("file://{}", full.display());
        }
    }

    // Check if `s` exists directly on the filesystem (e.g. "analyze.py", "index.html")
    let direct_path = std::path::Path::new(s);
    if direct_path.exists() {
        if let Ok(canon) = std::fs::canonicalize(direct_path) {
            return format!("file://{}", canon.display());
        }
        return format!("file://{}", direct_path.display());
    }

    // Check if `/{s}` exists on the filesystem (e.g. user typed "home/muhamad/computer-analysis/index.html")
    let abs_candidate = format!("/{s}");
    let abs_path = std::path::Path::new(&abs_candidate);
    if abs_path.exists() {
        if let Ok(canon) = std::fs::canonicalize(abs_path) {
            return format!("file://{}", canon.display());
        }
        return format!("file://{abs_candidate}");
    }

    // Check if relative to user home directory (e.g. "computer-analysis/index.html", "Downloads/file.pdf")
    let home_dir = glib::home_dir();
    let home_candidate = home_dir.join(s);
    if home_candidate.exists() {
        if let Ok(canon) = std::fs::canonicalize(&home_candidate) {
            return format!("file://{}", canon.display());
        }
        return format!("file://{}", home_candidate.display());
    }

    // Common Linux root paths without leading slash (e.g. "home/...", "tmp/...", "etc/...")
    let lower_s = s.to_ascii_lowercase();
    for prefix in &["home/", "etc/", "usr/", "var/", "opt/", "tmp/", "mnt/", "media/", "root/", "dev/"] {
        if lower_s.starts_with(prefix) {
            return format!("file:///{s}");
        }
    }

    // 2. Protocols (http, https, about, data, blob)
    if s.contains("://") || s.starts_with("about:") || s.starts_with("data:") || s.starts_with("blob:") {
        return s.into();
    }

    // 3. Localhost, IP addresses and development ports (e.g. localhost:3000, 127.0.0.1:8080, 192.168.1.1, 0.0.0.0:5000)
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("localhost")
        || lower.starts_with("127.0.0.1")
        || lower.starts_with("0.0.0.0")
        || lower.starts_with("192.168.")
        || lower.starts_with("10.")
        || (lower.starts_with("172.")
            && lower
                .split('.')
                .nth(1)
                .and_then(|n| n.parse::<u8>().ok())
                .map_or(false, |b| (16..=31).contains(&b)))
        || (s.contains(':')
            && !s.contains(' ')
            && s.split(':').nth(1).map_or(false, |p| p.chars().all(|c| c.is_ascii_digit())))
    {
        return format!("http://{s}");
    }

    // 4. Domains (e.g. google.com, github.com/user/repo)
    if !s.contains(' ') {
        let (host_part, _) = if let Some(idx) = s.find('/') {
            (&s[..idx], &s[idx..])
        } else {
            (s, "")
        };

        if is_likely_domain(host_part) {
            return format!("https://{s}");
        }
    }

    let encoded = glib::Uri::escape_string(s, None, false);
    let engine_id = canonical_search_engine(search_engine);
    for &(id, _, template) in SEARCH_ENGINE_LIST {
        if id == engine_id {
            return template.replace("{}", &encoded);
        }
    }
    format!("https://www.google.com/search?q={}", encoded)
}

fn is_likely_domain(host: &str) -> bool {
    let host = host.split(':').next().unwrap_or(host).trim();
    if !host.contains('.') || host.starts_with('.') || host.ends_with('.') {
        return false;
    }
    let tld = host.rsplit('.').next().unwrap_or("");
    if tld.is_empty() {
        return false;
    }
    const NON_DOMAIN_EXTS: &[&str] = &[
        "html", "htm", "shtml", "xhtml", "php", "asp", "aspx", "jsp",
        "txt", "md", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
        "png", "jpg", "jpeg", "gif", "svg", "webp", "ico",
        "mp3", "mp4", "webm", "wav", "ogg", "flac", "mkv", "avi",
        "css", "js", "ts", "json", "xml", "yaml", "yml", "toml",
        "zip", "tar", "gz", "bz2", "xz", "7z", "zst",
        "c", "cpp", "h", "hpp", "rs", "py", "go", "java", "rb", "sh",
        "exe", "bin", "deb", "rpm", "pkg",
    ];
    let lower_tld = tld.to_ascii_lowercase();
    if NON_DOMAIN_EXTS.contains(&lower_tld.as_str()) {
        return false;
    }
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        let parts: Vec<&str> = host.split('.').collect();
        return parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok());
    }
    if lower_tld.starts_with("xn--") {
        return true;
    }
    lower_tld.len() >= 2 && lower_tld.chars().all(|c| c.is_ascii_alphabetic())
}

pub(crate) fn host_of(uri: &str) -> Option<String> {
    let u = glib::Uri::parse(uri, glib::UriFlags::NONE).ok()?;
    let h = u.host()?;
    Some(h.trim_start_matches("www.").to_string())
}

/// Rail'ning ichki sahifalari (rail://start/ ...): WebKit ularni load_uri/reload bilan ocholmaydi.
pub(crate) fn is_internal_uri(u: &str) -> bool {
    u.starts_with("rail://")
}

pub(crate) fn settings_shell_page() -> String {
    // Same bg as .settings-modal so overview / tab-switch never flash a different page
    format!(
        r#"<!doctype html><html lang="{lang}"><head><meta charset="utf-8">
<title>Settings</title>
<style>html,body{{margin:0;background:#000000;height:100%}}</style>
</head><body></body></html>"#,
        lang = i18n::current().code()
    )
}

pub(crate) fn settings_uri_for_tab(tab: SettingsTab) -> &'static str {
    match tab {
        SettingsTab::General => "rail://settings",
        SettingsTab::Profiles => "rail://settings/profiles",
        SettingsTab::Extensions => "rail://settings/extensions",
        SettingsTab::Data => "rail://settings/data",
        SettingsTab::Privacy => "rail://settings/privacy",
        SettingsTab::Customize => "rail://settings/customize",
        SettingsTab::Update => "rail://settings/updates",
        SettingsTab::About => "rail://settings/about",
    }
}

pub(crate) fn settings_tab_from_uri(uri: &str) -> Option<SettingsTab> {
    let u = uri.trim().trim_end_matches('/');
    if !u.starts_with("rail://settings") {
        return None;
    }
    let rest = u.strip_prefix("rail://settings").unwrap_or("").trim_start_matches('/');
    Some(match rest {
        "" | "general" => SettingsTab::General,
        "profiles" | "profile" => SettingsTab::Profiles,
        "extensions" | "extension" => SettingsTab::Extensions,
        "data" | "backup" => SettingsTab::Data,
        "privacy" | "shield" => SettingsTab::Privacy,
        "customize" | "css" | "shortcuts" => SettingsTab::Customize,
        "updates" | "update" => SettingsTab::Update,
        "about" => SettingsTab::About,
        _ => SettingsTab::General,
    })
}

/// Canonical internal URI for display / load base.
pub(crate) fn normalize_internal_uri(uri: &str) -> String {
    let u = uri.trim();
    if u.starts_with("rail://downloads") {
        return "rail://downloads".into();
    }
    if u.starts_with("rail://settings") {
        return settings_uri_for_tab(settings_tab_from_uri(u).unwrap_or(SettingsTab::General)).into();
    }
    if u.starts_with("rail://start") || u == "rail://" || u == "rail:" {
        return "rail://start/".into();
    }
    u.into()
}

/// What the omnibar / pill shows for a tab URI.
pub(crate) fn display_uri_for_pill(uri: &str) -> String {
    if uri.is_empty() || uri.starts_with("about:") {
        return tr("Search or enter address").into();
    }
    // Welcome / start page: never put rail://start into the omnibar — keep input empty
    if uri.starts_with("rail://start") || uri == "rail://" || uri == "rail:" {
        return tr("Search or enter address").into();
    }
    if is_internal_uri(uri) {
        return normalize_internal_uri(uri);
    }
    host_of(uri).unwrap_or_else(|| uri.to_string())
}

/// What Ctrl+L puts into the address entry: the FULL url (pill shows only the host).
pub(crate) fn edit_uri_for_entry(uri: &str) -> String {
    if uri.is_empty() || uri.starts_with("about:") || is_internal_uri(uri) {
        return display_uri_for_pill(uri);
    }
    uri.to_string()
}

pub(crate) fn load_internal_page(v: &WebView, uri: &str) {
    if uri.starts_with("rail://downloads") {
        v.load_html(&downloads_page(), Some("rail://downloads"));
    } else if uri.starts_with("rail://settings") {
        v.load_html(&settings_shell_page(), Some(uri.split('#').next().unwrap_or("rail://settings")));
    } else {
        v.load_html(&start_page(), Some("rail://start/"));
    }
}

/// Start / downloads: reload() "The URL can't be shown" beradi — HTML dan qayta yuklaymiz.
pub(crate) fn reload_or_start(v: &WebView) {
    let uri = v.uri().map(|s| s.to_string()).unwrap_or_default();
    if is_internal_uri(&uri) {
        load_internal_page(v, &uri);
    } else {
        v.reload();
    }
}

/// Hard refresh (Ctrl+Shift+R): bypass HTTP cache like Chrome / Brave.
pub(crate) fn hard_reload_or_start(v: &WebView) {
    let uri = v.uri().map(|s| s.to_string()).unwrap_or_default();
    if is_internal_uri(&uri) {
        load_internal_page(v, &uri);
    } else {
        v.reload_bypass_cache();
    }
}
