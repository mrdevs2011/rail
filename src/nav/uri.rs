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
    (
        "startpage",
        "Startpage",
        "https://www.startpage.com/sp/search?query={}",
    ),
    ("qwant", "Qwant", "https://www.qwant.com/?q={}"),
    (
        "perplexity",
        "Perplexity",
        "https://www.perplexity.ai/search?q={}",
    ),
    ("searxng", "SearXNG", "https://searx.be/search?q={}"),
    ("kagi", "Kagi", "https://kagi.com/search?q={}"),
    (
        "naver",
        "Naver",
        "https://search.naver.com/search.naver?query={}",
    ),
    ("ask", "Ask.com", "https://www.ask.com/web?q={}"),
];

pub(crate) fn to_uri(input: &str, search_engine: &str) -> String {
    let s = input.trim();
    if s.starts_with("view-source:") {
        let rest = s.trim_start_matches("view-source:").trim();
        return format!("view-source:{}", to_uri(rest, search_engine));
    }
    // Short internal names → rail:// URLs
    match s.to_ascii_lowercase().as_str() {
        "settings" | "rail:settings" => return "rail://settings".into(),
        "history" | "rail:history" | "chrome://history" | "rail://history" => {
            return "rail://settings/data".into();
        }
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
        let p = format!("/{path}");
        if let Ok(uri) = glib::filename_to_uri(&p, None) {
            return uri.to_string();
        }
        return format!("file:///{path}");
    }
    if s.starts_with('/') {
        if let Ok(uri) = glib::filename_to_uri(s, None) {
            return uri.to_string();
        }
        return format!("file://{s}");
    }
    if let Some(rest) = s.strip_prefix("~/") {
        let home = glib::home_dir();
        let full = home.join(rest);
        if let Ok(uri) = glib::filename_to_uri(&full, None) {
            return uri.to_string();
        }
        return format!("file://{}/{}", home.display(), rest);
    }
    if (s.starts_with("./") || s.starts_with("../"))
        && let Ok(cur) = std::env::current_dir()
    {
        let full = cur.join(s);
        let target = std::fs::canonicalize(&full).unwrap_or(full);
        if let Ok(uri) = glib::filename_to_uri(&target, None) {
            return uri.to_string();
        }
        return format!("file://{}", target.display());
    }

    // Check if `s` exists directly on the filesystem (e.g. "analyze.py", "index.html")
    let direct_path = std::path::Path::new(s);
    if direct_path.exists() {
        let target =
            std::fs::canonicalize(direct_path).unwrap_or_else(|_| direct_path.to_path_buf());
        if let Ok(uri) = glib::filename_to_uri(&target, None) {
            return uri.to_string();
        }
        return format!("file://{}", target.display());
    }

    // Check if `/{s}` exists on the filesystem (e.g. user typed "home/muhamad/computer-analysis/index.html")
    let abs_candidate = format!("/{s}");
    let abs_path = std::path::Path::new(&abs_candidate);
    if abs_path.exists() {
        let target = std::fs::canonicalize(abs_path).unwrap_or_else(|_| abs_path.to_path_buf());
        if let Ok(uri) = glib::filename_to_uri(&target, None) {
            return uri.to_string();
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
    for prefix in &[
        "home/", "etc/", "usr/", "var/", "opt/", "tmp/", "mnt/", "media/", "root/", "dev/",
    ] {
        if lower_s.starts_with(prefix) {
            return format!("file:///{s}");
        }
    }

    // Developer-friendly search aliases. These are intentionally checked before
    // generic search handling, while real URLs / local paths above keep priority.
    if let Some((alias, query)) = s.split_once(char::is_whitespace) {
        let query = query.trim();
        if !query.is_empty() {
            let template = match alias.to_ascii_lowercase().as_str() {
                "gh" | "github" => Some("https://github.com/search?q={}"),
                "gl" | "gitlab" => Some("https://gitlab.com/search?search={}"),
                "yt" | "youtube" => Some("https://www.youtube.com/results?search_query={}"),
                "mdn" => Some("https://developer.mozilla.org/en-US/search?q={}"),
                "so" | "stackoverflow" => Some("https://stackoverflow.com/search?q={}"),
                "crates" => Some("https://crates.io/search?q={}"),
                "docs" | "docsrs" => Some("https://docs.rs/releases/search?query={}"),
                "npm" => Some("https://www.npmjs.com/search?q={}"),
                "pypi" | "py" => Some("https://pypi.org/search/?q={}"),
                "docker" => Some("https://hub.docker.com/search?q={}"),
                "k8s" | "kubernetes" => Some("https://kubernetes.io/search/?q={}"),
                "ghcode" => Some("https://github.com/search?type=code&q={}"),
                "reddit" | "r" => Some("https://www.reddit.com/search/?q={}"),
                "wiki" | "w" => Some("https://en.wikipedia.org/w/index.php?search={}"),
                "maps" => Some("https://www.openstreetmap.org/search?query={}"),
                _ => None,
            };
            if let Some(url) = template {
                let encoded = glib::Uri::escape_string(query, None, false);
                return url.replace("{}", &encoded);
            }
        }
    }

    // 2. Protocols (http, https, about, data, blob)
    if s.contains("://")
        || s.starts_with("about:")
        || s.starts_with("data:")
        || s.starts_with("blob:")
    {
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
                .is_some_and(|b| (16..=31).contains(&b)))
        || (s.contains(':')
            && !s.contains(' ')
            && s.split(':')
                .nth(1)
                .is_some_and(|p| p.chars().all(|c| c.is_ascii_digit())))
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
        "html", "htm", "shtml", "xhtml", "php", "asp", "aspx", "jsp", "txt", "md", "pdf", "doc",
        "docx", "xls", "xlsx", "ppt", "pptx", "png", "jpg", "jpeg", "gif", "svg", "webp", "ico",
        "mp3", "mp4", "webm", "wav", "ogg", "flac", "mkv", "avi", "css", "js", "ts", "json", "xml",
        "yaml", "yml", "toml", "zip", "tar", "gz", "bz2", "xz", "7z", "zst", "c", "cpp", "h",
        "hpp", "rs", "py", "go", "java", "rb", "sh", "exe", "bin", "deb", "rpm", "pkg",
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

/// Rail'ning ichki sahifalari (rail://start/ ... va view-source:): WebKit ularni load_uri/reload bilan ocholmaydi.
pub(crate) fn is_internal_uri(u: &str) -> bool {
    u.starts_with("rail://") || u.starts_with("view-source:")
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
        SettingsTab::Data => "rail://settings/data",
        SettingsTab::Privacy => "rail://settings/privacy",
        SettingsTab::Customize => "rail://settings/customize/html/",
        SettingsTab::Update => "rail://settings/updates",
        SettingsTab::About => "rail://settings/about",
    }
}

pub(crate) fn settings_tab_from_uri(uri: &str) -> Option<SettingsTab> {
    let u = uri.trim().trim_end_matches('/');
    if !u.starts_with("rail://settings") {
        return None;
    }
    let rest = u
        .strip_prefix("rail://settings")
        .unwrap_or("")
        .trim_start_matches('/');
    Some(match rest {
        "" | "general" => SettingsTab::General,
        "profiles" | "profile" => SettingsTab::Profiles,
        "extensions" | "extension" => SettingsTab::General, // legacy URL compatibility
        "data" | "backup" | "data/history" | "data/export" | "data/import" | "data/cache" => {
            SettingsTab::Data
        }
        "privacy" | "shield" => SettingsTab::Privacy,
        "customize" | "css" | "customize/html" | "customize/shortcuts" | "shortcuts" => {
            SettingsTab::Customize
        }
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
    if u.starts_with("rail://history") || u.starts_with("chrome://history") {
        return "rail://settings/data".into();
    }
    if u.starts_with("rail://settings/data/") {
        let sub = u
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("history");
        let sub = match sub {
            "export" | "import" | "cache" => sub,
            _ => "history",
        };
        return format!("rail://settings/data/{sub}/");
    }
    if u.starts_with("rail://settings/customize/shortcuts") {
        return "rail://settings/customize/shortcuts/".into();
    }
    if u.starts_with("rail://settings/customize/html") || u.starts_with("rail://settings/css") {
        return "rail://settings/customize/html/".into();
    }
    if u.starts_with("rail://settings") {
        return settings_uri_for_tab(settings_tab_from_uri(u).unwrap_or(SettingsTab::General))
            .into();
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

fn source_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn source_span(class: &str, value: &str) -> String {
    format!("<span class=\"{class}\">{}</span>", source_escape(value))
}

fn highlight_source_line(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len() * 2);
    let mut i = 0;
    while i < chars.len() {
        if i + 3 < chars.len() && chars[i..i + 4] == ['<', '!', '-', '-'] {
            let begin = i;
            i += 4;
            while i + 2 < chars.len() && chars[i..i + 3] != ['-', '-', '>'] {
                i += 1;
            }
            if i + 2 < chars.len() {
                i += 3;
            }
            let token: String = chars[begin..i].iter().collect();
            out.push_str(&source_span("comment", &token));
            continue;
        }
        if chars[i] != '<' {
            let begin = i;
            while i < chars.len() && chars[i] != '<' {
                i += 1;
            }
            let token: String = chars[begin..i].iter().collect();
            out.push_str(&source_escape(&token));
            continue;
        }

        out.push_str(&source_span("punct", "<"));
        i += 1;
        if i < chars.len() && chars[i] == '/' {
            out.push_str(&source_span("punct", "/"));
            i += 1;
        }
        let name_start = i;
        while i < chars.len()
            && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '-' | ':' | '_'))
        {
            i += 1;
        }
        if i > name_start {
            let token: String = chars[name_start..i].iter().collect();
            out.push_str(&source_span("tag", &token));
        }
        while i < chars.len() {
            if chars[i] == '>' {
                out.push_str(&source_span("punct", ">"));
                i += 1;
                break;
            }
            if chars[i].is_whitespace() {
                out.push(chars[i]);
                i += 1;
                continue;
            }
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                let begin = i;
                i += 1;
                while i < chars.len() {
                    let c = chars[i];
                    i += 1;
                    if c == quote {
                        break;
                    }
                }
                let token: String = chars[begin..i].iter().collect();
                out.push_str(&source_span("string", &token));
                continue;
            }
            if chars[i] == '=' {
                out.push_str(&source_span("punct", "="));
                i += 1;
                continue;
            }
            let begin = i;
            while i < chars.len()
                && !chars[i].is_whitespace()
                && !matches!(chars[i], '>' | '=' | '"' | '\'')
            {
                i += 1;
            }
            if i == begin {
                i += 1;
            }
            let token: String = chars[begin..i].iter().collect();
            out.push_str(&source_span("attr", &token));
        }
    }
    out
}

pub(crate) fn format_view_source_html(target_url: &str, raw_code: &str) -> String {
    let mut lines_html = String::with_capacity(raw_code.len() * 3);
    for line in raw_code.lines() {
        lines_html.push_str("<span class=\"line\">");
        lines_html.push_str(&highlight_source_line(line));
        lines_html.push_str("</span>\n");
    }
    format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>view-source:{}</title>
<style>
  html, body {{
    background: #202124;
    color: #e8eaed;
    font-family: 'JetBrains Mono', 'Fira Code', 'DejaVu Sans Mono', monospace;
    font-size: 12px;
    line-height: 1.35;
    margin: 0;
    padding: 0;
    user-select: text;
  }}
  .tag {{ color: #f28b82; }}
  .attr {{ color: #c58af9; }}
  .string {{ color: #81c995; }}
  .comment {{ color: #9aa0a6; font-style: italic; }}
  .punct {{ color: #bdc1c6; }}
  pre {{
    margin: 0;
    counter-reset: line;
  }}
  .line {{
    display: block;
    white-space: pre-wrap;
    word-break: break-all;
  }}
  .line::before {{
    counter-increment: line;
    content: counter(line);
    display: inline-block;
    width: 3.2em;
    margin-right: 0.65em;
    padding: 0 0.5em 0 0;
    color: #80868b;
    text-align: right;
    user-select: none;
  }}
</style>
</head>
<body>
<pre>{lines_html}</pre>
</body>
</html>"#,
        source_escape(target_url)
    )
}

pub(crate) fn load_view_source_page(v: &WebView, target: &str) {
    if target.is_empty() {
        v.load_html(
            &format_view_source_html("about:blank", ""),
            Some("about:blank"),
        );
        return;
    }
    let target_clean = target.to_string();
    let view_weak = v.downgrade();

    if target_clean.starts_with("file://") {
        let path_str = target_clean.trim_start_matches("file://");
        let content = std::fs::read_to_string(path_str)
            .unwrap_or_else(|e| format!("/* Error reading file: {e} */"));
        let html = format_view_source_html(&target_clean, &content);
        v.load_html(&html, Some("about:blank"));
        return;
    }

    // Paint an immediate lightweight state instead of leaving a black about:blank tab
    // while the original response is fetched on a worker thread.
    v.load_html(
        &format!("<!doctype html><meta charset=utf-8><style>body{{margin:0;padding:8px 10px;background:#202124;color:#9aa0a6;font:12px monospace}}</style>Loading source… <span style='color:#5f6368'>{}</span>", source_escape(&target_clean)),
        Some("about:blank"),
    );
    let (tx, rx) = std::sync::mpsc::channel();
    let url_for_thread = target_clean.clone();
    std::thread::spawn(move || {
        let out = std::process::Command::new("curl")
            .args(["-4", "-fsSL", "-L", "-m", "15", "-A", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36", &url_for_thread])
            .output();
        let code = match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
            Ok(o) => format!(
                "/* Error loading source: HTTP status {:?} */\n{}",
                o.status.code(),
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => format!("/* Error fetching source: {e} */"),
        };
        let _ = tx.send(code);
    });

    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        match rx.try_recv() {
            Ok(src) => {
                if let Some(vw) = view_weak.upgrade() {
                    let html = format_view_source_html(&target_clean, &src);
                    vw.load_html(&html, Some("about:blank"));
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        }
    });
}

pub(crate) fn load_internal_page(v: &WebView, uri: &str) {
    if uri.starts_with("rail://downloads") {
        v.load_html(&downloads_page(), Some("rail://downloads"));
    } else if uri.starts_with("rail://settings") {
        v.load_html(
            &settings_shell_page(),
            Some(uri.split('#').next().unwrap_or("rail://settings")),
        );
    } else if uri.starts_with("view-source:") {
        let target = uri.trim_start_matches("view-source:").trim();
        load_view_source_page(v, target);
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

#[cfg(test)]
mod view_source_tests {
    use super::{edit_uri_for_entry, format_view_source_html, highlight_source_line};

    #[test]
    fn view_source_uri_stays_visible_and_editable() {
        let uri = "view-source:https://example.com/page";
        assert_eq!(edit_uri_for_entry(uri), uri);
    }

    #[test]
    fn source_highlighting_escapes_markup_and_colors_html_tokens() {
        let rendered = highlight_source_line(r#"<a href="https://example.com/?a=1&b=2">Link</a>"#);
        assert!(rendered.contains("class=\"tag\""));
        assert!(rendered.contains("class=\"attr\""));
        assert!(rendered.contains("class=\"string\""));
        assert!(rendered.contains("&amp;"));
        assert!(!rendered.contains("<a "));
    }

    #[test]
    fn source_page_uses_compact_chrome_like_gutter_and_colored_tokens() {
        let html = format_view_source_html("https://example.com", "<html>\n<!-- note -->\n</html>");
        assert!(html.contains("padding: 0;"));
        assert!(html.contains("width: 3.2em"));
        assert!(html.contains("class=\"comment\""));
        assert!(html.contains("view-source:https://example.com"));
    }
}
