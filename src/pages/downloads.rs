//! Download tracking and the rail://downloads page.

use crate::*;

#[derive(Clone, Debug)]
pub(crate) struct DownloadEntry {
    pub(crate) id: u64,
    pub(crate) filename: String,
    pub(crate) url: String,
    pub(crate) path: String,
    pub(crate) status: String, // downloading | finished | failed | cancelled
    pub(crate) progress: f64,
    pub(crate) total: i64,
    pub(crate) received: i64,
}

thread_local! {
    pub(crate) static DOWNLOADS: RefCell<Vec<DownloadEntry>> = RefCell::new(Vec::new());
    pub(crate) static DOWNLOAD_SEQ: Cell<u64> = Cell::new(1);
}

pub(crate) fn downloads_json_path() -> PathBuf {
    rail_data_dir().join("downloads.json")
}

pub(crate) fn load_downloads_from_disk() {
    let Ok(text) = std::fs::read_to_string(downloads_json_path()) else { return; };
    // Simple line format: id\tstatus\tfilename\tpath\turl
    DOWNLOADS.with(|d| {
        let mut v = d.borrow_mut();
        if !v.is_empty() { return; }
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(5, '\t').collect();
            if parts.len() < 5 { continue; }
            let id: u64 = parts[0].parse().unwrap_or(0);
            if id == 0 { continue; }
            v.push(DownloadEntry {
                id,
                status: parts[1].to_string(),
                filename: parts[2].to_string(),
                path: parts[3].to_string(),
                url: parts[4].to_string(),
                progress: if parts[1] == "finished" { 1.0 } else { 0.0 },
                total: 0,
                received: 0,
            });
            DOWNLOAD_SEQ.with(|s| {
                if id >= s.get() { s.set(id + 1); }
            });
        }
    });
}

pub(crate) fn save_downloads_to_disk() {
    DOWNLOADS.with(|d| {
        let mut out = String::new();
        for e in d.borrow().iter() {
            // keep finished/failed history
            if e.status == "downloading" { continue; }
            out.push_str(&format!("{}\t{}\t{}\t{}\t{}\n", e.id, e.status, e.filename, e.path, e.url));
        }
        let _ = std::fs::write(downloads_json_path(), out);
    });
}

pub(crate) fn downloads_page() -> String {
    load_downloads_from_disk();
    let rows = DOWNLOADS.with(|d| {
        let list = d.borrow();
        if list.is_empty() {
            return format!(
                "<div class=\"empty\">{}</div>",
                tr("No downloads yet")
            );
        }
        let mut html = String::new();
        for e in list.iter().rev() {
            let pct = (e.progress * 100.0).round() as i32;
            let size = if e.total > 0 {
                format!("{} / {} · {}%", format_bytes(e.received), format_bytes(e.total), pct)
            } else if e.received > 0 {
                format_bytes(e.received)
            } else {
                e.status.clone()
            };
            let status_cls = match e.status.as_str() {
                "finished" => "ok",
                "failed" | "cancelled" => "err",
                _ => "run",
            };
            html.push_str(&format!(
                r#"<div class="dl-row">
  <div class="dl-name">{fname}</div>
  <div class="dl-meta">{size} · <span class="{cls}">{status}</span></div>
  <div class="dl-bar"><div class="dl-fill" style="width:{pct}%"></div></div>
  <div class="dl-url">{url}</div>
</div>"#,
                fname = glib::markup_escape_text(&e.filename),
                size = glib::markup_escape_text(&size),
                cls = status_cls,
                status = glib::markup_escape_text(&e.status),
                pct = pct,
                url = glib::markup_escape_text(&e.url),
            ));
        }
        html
    });
    format!(
r#"<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; padding: 32px 28px 48px;
    background: #000000; color: #e8e8e8;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', ui-monospace, monospace;
  }}
  h1 {{ font-size: 22px; font-weight: 700; margin: 0 0 6px; letter-spacing: -0.02em; }}
  .sub {{ color: #737373; font-size: 12px; margin-bottom: 22px; }}
  .empty {{ color: #666; padding: 40px 0; text-align: center; font-size: 13px; }}
  .dl-row {{
    border: 1px solid #1a1a1a; background: #111; border-radius: 0;
    padding: 12px 14px; margin-bottom: 10px;
  }}
  .dl-name {{ font-size: 13px; font-weight: 600; color: #f4f4f4; }}
  .dl-meta {{ font-size: 11px; color: #888; margin-top: 4px; }}
  .dl-meta .ok {{ color: #4ade80; }}
  .dl-meta .err {{ color: #f87171; }}
  .dl-meta .run {{ color: #93b4ff; }}
  .dl-bar {{ height: 3px; background: #1a1a1a; border-radius: 2px; margin-top: 8px; overflow: hidden; }}
  .dl-fill {{ height: 100%; background: #2f6bff; }}
  .dl-url {{ font-size: 10px; color: #555; margin-top: 6px; word-break: break-all; }}
</style>
</head>
<body>
  <h1>{title}</h1>
  <div class="sub">rail://downloads · {hint}</div>
  {rows}
</body>
</html>"#,
        lang = i18n::current().code(),
        title = tr("Downloads"),
        hint = tr("Chrome-style download history for this profile session"),
        rows = rows,
    )
}

pub(crate) fn format_bytes(n: i64) -> String {
    if n < 1024 { return format!("{} B", n); }
    if n < 1024 * 1024 { return format!("{:.1} KB", n as f64 / 1024.0); }
    if n < 1024 * 1024 * 1024 { return format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)); }
    format!("{:.2} GB", n as f64 / (1024.0 * 1024.0 * 1024.0))
}

pub(crate) fn attach_download_handlers_once(session: &webkit6::NetworkSession) {
    use std::collections::HashSet;
    thread_local! {
        static HOOKED: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    }
    let key = session.as_ptr() as usize;
    let already = HOOKED.with(|h| {
        let mut s = h.borrow_mut();
        if s.contains(&key) { true } else { s.insert(key); false }
    });
    if already { return; }
    attach_download_handlers(session);
}

pub(crate) fn attach_download_handlers(session: &webkit6::NetworkSession) {
    session.connect_download_started(|_sess, download| {
        let dest_dir = glib::user_special_dir(glib::UserDirectory::Downloads)
            .unwrap_or_else(|| glib::home_dir().join("Downloads"));
        let _ = std::fs::create_dir_all(&dest_dir);

        let uri = download.request()
            .and_then(|r| r.uri())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let suggested = download.destination()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                // from URI path
                uri.rsplit('/').next().map(|s| s.to_string()).filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "download".to_string());
        // strip query
        let suggested = suggested.split('?').next().unwrap_or("download").to_string();
        let path = dest_dir.join(&suggested);
        let path_s = path.to_string_lossy().to_string();
        download.set_destination(&path_s);

        let id = DOWNLOAD_SEQ.with(|s| {
            let id = s.get();
            s.set(id + 1);
            id
        });
        DOWNLOADS.with(|d| {
            d.borrow_mut().push(DownloadEntry {
                id,
                filename: suggested.clone(),
                url: uri.clone(),
                path: path_s.clone(),
                status: "downloading".into(),
                progress: 0.0,
                total: 0,
                received: 0,
            });
        });

        let id_c = id;
        download.connect_received_data(move |dl, data_length| {
            let received = dl.received_data_length() as i64;
            // estimated_progress is 0..1
            let prog = dl.estimated_progress();
            let total = if prog > 0.01 { (received as f64 / prog) as i64 } else { 0 };
            DOWNLOADS.with(|d| {
                if let Some(e) = d.borrow_mut().iter_mut().find(|e| e.id == id_c) {
                    e.received = received;
                    e.total = total;
                    e.progress = prog;
                    e.status = "downloading".into();
                }
            });
            let _ = data_length;
        });

        let id_f = id;
        download.connect_finished(move |_dl| {
            DOWNLOADS.with(|d| {
                if let Some(e) = d.borrow_mut().iter_mut().find(|e| e.id == id_f) {
                    e.status = "finished".into();
                    e.progress = 1.0;
                }
            });
            save_downloads_to_disk();
        });

        let id_e = id;
        download.connect_failed(move |_dl, _err| {
            DOWNLOADS.with(|d| {
                if let Some(e) = d.borrow_mut().iter_mut().find(|e| e.id == id_e) {
                    e.status = "failed".into();
                }
            });
            save_downloads_to_disk();
        });
    });
}
