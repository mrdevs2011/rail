//! Web Push for Rail (WebKitGTK ships no Push API).
//!
//! * `push_page.js` is injected into pages and provides `PushManager` / `PushSubscription`
//!   on top of the Mozilla autopush service. Each subscription is stored in
//!   `<data dir>/push/subs/<id>.json` (private key included, mode 0600).
//! * `rail --background` (autostarted at login, kept alive after the browser exits) runs a
//!   hidden WebView with `push_receiver.js`, which keeps one autopush WebSocket per
//!   subscription, decrypts incoming messages (RFC 8291) and asks Rust to show a native
//!   desktop notification. Clicking it opens the site in Rail.

use crate::*;
use std::os::unix::fs::PermissionsExt;

const PAGE_SHIM: &str = include_str!("push_page.js");
const RECEIVER_JS: &str = include_str!("push_receiver.js");

fn subs_dir() -> PathBuf {
    rail_data_dir().join("push").join("subs")
}

fn valid_id(id: &str) -> bool {
    id.len() >= 16 && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn host_of(url: &str) -> String {
    glib::Uri::parse(url, glib::UriFlags::NONE)
        .ok()
        .and_then(|u| u.host().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_default()
}

/// True when at least one push subscription is stored (the daemon must then keep running).
pub(crate) fn has_subscriptions() -> bool {
    std::fs::read_dir(subs_dir())
        .map(|d| {
            d.flatten()
                .any(|e| e.path().extension().is_some_and(|x| x == "json"))
        })
        .unwrap_or(false)
}

/// Register the page-facing bridge on a UserContentManager (call before creating the WebView).
pub(crate) fn install_bridge(ucm: &webkit6::UserContentManager) {
    ucm.register_script_message_handler_with_reply("railPush", None);
    let script = webkit6::UserScript::new(
        PAGE_SHIM,
        webkit6::UserContentInjectedFrames::TopFrame,
        webkit6::UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    ucm.add_script(&script);
}

/// Answer `railPush` requests from the page shim. Needs the owning view so the claimed origin
/// can be checked against the real top-level URL.
pub(crate) fn attach_handler(view: &webkit6::WebView) {
    let Some(ucm) = view.user_content_manager() else {
        return;
    };
    let wv = view.downgrade();
    ucm.connect_script_message_with_reply_received(Some("railPush"), move |_, val, reply| {
        let text = val.to_str().to_string();
        let uri = wv
            .upgrade()
            .and_then(|v| v.uri())
            .map(|u| u.to_string())
            .unwrap_or_default();
        let answer = handle_request(&uri, &text);
        let ctx = webkit6::javascriptcore::Context::new();
        reply.return_value(&webkit6::javascriptcore::Value::new_string(&ctx, Some(&answer)));
        true
    });
}

fn err(msg: &str) -> String {
    format!("{{\"ok\":false,\"error\":\"{}\"}}", minijson::escape(msg))
}

fn handle_request(page_uri: &str, text: &str) -> String {
    let Some(req) = minijson::parse(text) else {
        return err("bad request");
    };
    let op = req.get("op").and_then(|v| v.as_str()).unwrap_or("");
    if op == "log" {
        if std::env::var_os("RAIL_PUSH_DEBUG").is_some() {
            eprintln!("[push-page] {}", req.get("msg").and_then(|v| v.as_str()).unwrap_or(""));
        }
        return "{\"ok\":true}".to_string();
    }
    let origin = req.get("origin").and_then(|v| v.as_str()).unwrap_or("");
    let id = req.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let origin_host = host_of(origin);
    if origin_host.is_empty() || origin_host != host_of(page_uri) || !valid_id(id) {
        return err("origin mismatch");
    }
    let path = subs_dir().join(format!("{id}.json"));
    let pub_path = subs_dir().join(format!("{id}.pub"));
    match op {
        "save" => {
            if !crate::pwa::is_notification_permission_granted(page_uri) {
                return err("notification permission not granted");
            }
            let (Some(full), Some(public)) = (
                req.get("full").and_then(|v| v.as_str()),
                req.get("pub").and_then(|v| v.as_str()),
            ) else {
                return err("missing data");
            };
            let ok = minijson::parse(full)
                .map(|j| {
                    j.get("id").and_then(|v| v.as_str()) == Some(id)
                        && j.get("origin").and_then(|v| v.as_str()) == Some(origin)
                        && j.get("channelID").and_then(|v| v.as_str()).is_some()
                        && j.get("uaid").and_then(|v| v.as_str()).is_some()
                        && j.get("privJwk").is_some()
                })
                .unwrap_or(false)
                && minijson::parse(public).is_some()
                && full.len() < 16 * 1024;
            if !ok {
                return err("invalid subscription");
            }
            if std::fs::create_dir_all(subs_dir()).is_err() {
                return err("cannot create storage");
            }
            if write_private(&pub_path, public).is_err() || write_private(&path, full).is_err() {
                return err("cannot write storage");
            }
            crate::pwa::ensure_background_daemon_running();
            "{\"ok\":true}".to_string()
        }
        "get" => match std::fs::read_to_string(&pub_path) {
            // Only the origin that created the subscription may read it back.
            Ok(public) if stored_origin(&path).as_deref() == Some(origin) => {
                format!("{{\"ok\":true,\"pub\":{public}}}")
            }
            _ => "{\"ok\":true,\"pub\":null}".to_string(),
        },
        "delete" => {
            if stored_origin(&path).as_deref() == Some(origin) || !path.exists() {
                let _ = std::fs::remove_file(&path);
                let _ = std::fs::remove_file(&pub_path);
                "{\"ok\":true}".to_string()
            } else {
                err("not owner")
            }
        }
        _ => err("unknown op"),
    }
}

fn stored_origin(path: &Path) -> Option<String> {
    let json = minijson::parse(&std::fs::read_to_string(path).ok()?)?;
    json.get("origin")?.as_str().map(str::to_string)
}

fn write_private(path: &Path, data: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(tmp, path)
}

/// All stored subscriptions as one JSON array (each file already validated on save).
fn subscriptions_json() -> String {
    let mut parts = Vec::new();
    if let Ok(dir) = std::fs::read_dir(subs_dir()) {
        let mut files: Vec<_> = dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for p in files {
            if let Ok(s) = std::fs::read_to_string(&p)
                && minijson::parse(&s).is_some()
            {
                parts.push(s);
            }
        }
    }
    format!("[{}]", parts.join(","))
}

/// Title/body/url for a decrypted push payload. Handles the common shapes: Notification-API
/// style JSON, FCM-style `{notification:{...}}`, Telegram-style `loc_key`/`loc_args`, and text.
pub(crate) fn describe_payload(origin: &str, payload: Option<&str>) -> (String, String, String) {
    let host = host_of(origin);
    let fallback_title = if host.is_empty() { "Rail".to_string() } else { host };
    let mut url = origin.to_string();
    let Some(text) = payload.map(str::trim).filter(|t| !t.is_empty()) else {
        return (fallback_title, tr("New notification").to_string(), url);
    };
    let Some(json) = minijson::parse(text) else {
        return (fallback_title, text.chars().take(300).collect(), url);
    };
    let pick = |j: &minijson::Json, keys: &[&str]| -> Option<String> {
        keys.iter()
            .find_map(|k| j.get(k).and_then(|v| v.as_str()))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let inner = json.get("notification").filter(|n| n.get("title").is_some() || n.get("body").is_some());
    let src = inner.unwrap_or(&json);
    let mut title = pick(src, &["title", "name", "sender", "from"]);
    let mut body = pick(src, &["body", "message", "text", "description", "msg", "content"]);
    if title.is_none() && body.is_none() {
        // Telegram-style localized payload: loc_args = [sender, text, ...]
        if let Some(args) = json.get("loc_args").and_then(|a| a.as_arr()) {
            let strs: Vec<&str> = args.iter().filter_map(|a| a.as_str()).collect();
            title = strs.first().map(|s| s.to_string());
            body = strs.get(1).map(|s| s.to_string());
        }
    }
    let target = [src, &json]
        .iter()
        .find_map(|j| {
            pick(j, &["url", "link", "click_action", "open_url"]).or_else(|| {
                j.get("data").and_then(|d| pick(d, &["url", "link", "click_action"]))
            })
        })
        .and_then(|u| {
            glib::Uri::parse(origin, glib::UriFlags::NONE)
                .ok()
                .and_then(|base| base.parse_relative(&u, glib::UriFlags::NONE).ok())
                .map(|u| u.to_string())
        });
    if let Some(t) = target
        && host_of(&t) == host_of(origin)
    {
        url = t;
    }
    (
        title.unwrap_or(fallback_title),
        body.unwrap_or_else(|| tr("New notification").to_string()),
        url,
    )
}

fn deliver(id: &str, payload: Option<&str>) {
    let Ok(path) = subs_dir().join(format!("{id}.json")).canonicalize() else {
        return;
    };
    let Some(origin) = stored_origin(&path) else {
        return;
    };
    // Permission may have been revoked after subscribing.
    if !crate::pwa::is_notification_permission_granted(&origin) {
        return;
    }
    let (title, body, url) = describe_payload(&origin, payload);
    crate::notifications::show_clickable_notification(&host_of(&origin), &title, &body, &url);
}

/// Start the receiver inside the background daemon. Keep the returned view alive.
pub(crate) fn start_receiver() -> webkit6::WebView {
    let ucm = webkit6::UserContentManager::new();
    ucm.register_script_message_handler("railPushMsg", None);
    ucm.connect_script_message_received(Some("railPushMsg"), |_, val| {
        let Some(j) = minijson::parse(val.to_str().as_str()) else {
            return;
        };
        if let Some(log) = j.get("log").and_then(|v| v.as_str()) {
            if std::env::var_os("RAIL_PUSH_DEBUG").is_some() {
                eprintln!("[rail-push] {log}");
            }
            return;
        }
        if let Some(id) = j.get("id").and_then(|v| v.as_str()) {
            deliver(id, j.get("payload").and_then(|p| p.as_str()));
        }
    });
    let settings = webkit6::Settings::new();
    settings.set_enable_page_cache(false);
    settings.set_enable_webgl(false);
    settings.set_enable_webaudio(false);
    settings.set_enable_media(false);
    let session = webkit6::NetworkSession::new_ephemeral();
    let view = webkit6::WebView::builder()
        .network_session(&session)
        .settings(&settings)
        .user_content_manager(&ucm)
        .build();
    // localhost is a secure context, which WebCrypto needs.
    let page = format!("<!doctype html><title>rail-push</title><script>{RECEIVER_JS}</script>");
    view.load_html(&page, Some("http://localhost/"));
    // Keep the receiver's connection list in sync with the subscription files.
    let wv = view.downgrade();
    let last = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let ready = std::rc::Rc::new(std::cell::Cell::new(false));
    let ready_c = ready.clone();
    view.connect_load_changed(move |_, ev| {
        if ev == webkit6::LoadEvent::Finished {
            ready_c.set(true);
        }
    });
    // If the web process crashes or is killed under memory pressure, reload so the
    // connections are rebuilt on the next sync tick.
    let ready_t = ready.clone();
    let last_t = last.clone();
    view.connect_web_process_terminated(move |v, _| {
        ready_t.set(false);
        last_t.borrow_mut().clear();
        // reload() would fetch http://localhost/, so load the inline page again.
        v.load_html(&page, Some("http://localhost/"));
    });
    glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
        let Some(v) = wv.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if !ready.get() {
            return glib::ControlFlow::Continue;
        }
        let json = subscriptions_json();
        if *last.borrow() != json {
            *last.borrow_mut() = json.clone();
            v.evaluate_javascript(
                &format!("window.railPushSync({json});"),
                None,
                None,
                None::<&gio::Cancellable>,
                |_| {},
            );
        }
        glib::ControlFlow::Continue
    });
    view
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_notification_style_payload() {
        let (t, b, u) = describe_payload(
            "https://example.com",
            Some(r#"{"title":"Hi","body":"There","url":"/chat/1"}"#),
        );
        assert_eq!((t.as_str(), b.as_str()), ("Hi", "There"));
        assert_eq!(u, "https://example.com/chat/1");
    }

    #[test]
    fn describes_fcm_nested_payload_and_ignores_foreign_urls() {
        let (t, b, u) = describe_payload(
            "https://example.com",
            Some(r#"{"notification":{"title":"A","body":"B"},"url":"https://evil.test/x"}"#),
        );
        assert_eq!((t.as_str(), b.as_str()), ("A", "B"));
        assert_eq!(u, "https://example.com");
    }

    #[test]
    fn describes_loc_args_and_plain_text() {
        let (t, b, _) = describe_payload(
            "https://web.telegram.org",
            Some(r#"{"loc_key":"MESSAGE_TEXT","loc_args":["Ali","salom"]}"#),
        );
        assert_eq!((t.as_str(), b.as_str()), ("Ali", "salom"));
        let (t, b, _) = describe_payload("https://web.telegram.org", Some("plain text"));
        assert_eq!((t.as_str(), b.as_str()), ("web.telegram.org", "plain text"));
        let (_, b, _) = describe_payload("https://x.test", None);
        assert!(!b.is_empty());
    }

    #[test]
    fn rejects_foreign_origin_and_bad_ids() {
        let r = handle_request(
            "https://a.test/",
            r#"{"op":"get","id":"0123456789abcdef0123456789abcdef","origin":"https://b.test"}"#,
        );
        assert!(r.contains("origin mismatch"));
        let r = handle_request("https://a.test/", r#"{"op":"get","id":"../x","origin":"https://a.test"}"#);
        assert!(r.contains("origin mismatch"));
    }
}
