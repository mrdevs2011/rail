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
    let Ok(text) = std::fs::read_to_string(downloads_json_path()) else {
        return;
    };
    // Simple line format: id\tstatus\tfilename\tpath\turl
    DOWNLOADS.with(|d| {
        let mut v = d.borrow_mut();
        if !v.is_empty() {
            return;
        }
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(5, '\t').collect();
            if parts.len() < 5 {
                continue;
            }
            let id: u64 = parts[0].parse().unwrap_or(0);
            if id == 0 {
                continue;
            }
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
                if id >= s.get() {
                    s.set(id + 1);
                }
            });
        }
    });
}

pub(crate) fn save_downloads_to_disk() {
    DOWNLOADS.with(|d| {
        let mut out = String::new();
        for e in d.borrow().iter() {
            // keep finished/failed history
            if e.status == "downloading" {
                continue;
            }
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                e.id, e.status, e.filename, e.path, e.url
            ));
        }
        let _ = std::fs::write(downloads_json_path(), out);
    });
}

pub(crate) fn downloads_page() -> String {
    load_downloads_from_disk();
    let rows = DOWNLOADS.with(|d| {
        let list = d.borrow();
        if list.is_empty() {
            return format!("<div class=\"empty\">{}</div>", tr("No downloads yet"));
        }
        let mut html = String::new();
        for e in list.iter().rev() {
            let pct = (e.progress * 100.0).round() as i32;
            let size = if e.total > 0 {
                format!(
                    "{} / {} · {}%",
                    format_bytes(e.received),
                    format_bytes(e.total),
                    pct
                )
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
  <div class="dl-header">
    <div class="dl-name">{fname}</div>
    <div class="dl-meta">{size} · <span class="{cls}">{status}</span></div>
  </div>
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
    margin: 0;
    padding: clamp(16px, 4vw, 36px) clamp(14px, 4vw, 32px);
    background: #000000;
    color: #e8e8e8;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', ui-monospace, monospace;
    display: flex;
    justify-content: center;
  }}
  .container {{
    width: 100%;
    max-width: 860px;
  }}
  h1 {{
    font-size: clamp(18px, 4vw, 24px);
    font-weight: 700;
    margin: 0 0 6px;
    letter-spacing: -0.02em;
    color: #fafafa;
  }}
  .sub {{
    color: #737373;
    font-size: clamp(10.5px, 2.5vw, 12px);
    margin-bottom: 22px;
    word-break: break-word;
  }}
  .empty {{
    color: #666;
    padding: 60px 16px;
    text-align: center;
    font-size: 13px;
    border: 1px dashed #1c1c1c;
    border-radius: 4px;
  }}
  .dl-row {{
    border: 1px solid #1a1a1a;
    background: #0e0e0e;
    border-radius: 4px;
    padding: clamp(10px, 3vw, 14px) clamp(12px, 3vw, 16px);
    margin-bottom: 10px;
    transition: border-color 120ms ease;
  }}
  .dl-row:hover {{
    border-color: #2c2c2c;
  }}
  .dl-header {{
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: baseline;
    gap: 6px;
  }}
  .dl-name {{
    font-size: clamp(12.5px, 2.8vw, 13.5px);
    font-weight: 600;
    color: #f4f4f4;
    word-break: break-word;
  }}
  .dl-meta {{
    font-size: clamp(10.5px, 2.4vw, 11.5px);
    color: #888;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
  }}
  .dl-meta .ok {{ color: #4ade80; font-weight: 600; }}
  .dl-meta .err {{ color: #f87171; font-weight: 600; }}
  .dl-meta .run {{ color: #38bdf8; font-weight: 600; }}
  .dl-bar {{
    height: 4px;
    background: #181818;
    border-radius: 2px;
    margin-top: 10px;
    overflow: hidden;
  }}
  .dl-fill {{
    height: 100%;
    background: #38bdf8;
    border-radius: 2px;
    transition: width 150ms ease;
  }}
  .dl-url {{
    font-size: clamp(9.5px, 2.2vw, 10.5px);
    color: #525252;
    margin-top: 6px;
    word-break: break-all;
  }}
</style>
</head>
<body>
  <div class="container">
    <h1>{title}</h1>
    <div class="sub">rail://downloads · {hint}</div>
    {rows}
  </div>
</body>
</html>"#,
        lang = i18n::current().code(),
        title = tr("Downloads"),
        hint = tr("Chrome-style download history for this profile session"),
        rows = rows,
    )
}

pub(crate) fn format_bytes(n: i64) -> String {
    if n < 1024 {
        return format!("{} B", n);
    }
    if n < 1024 * 1024 {
        return format!("{:.1} KB", n as f64 / 1024.0);
    }
    if n < 1024 * 1024 * 1024 {
        return format!("{:.1} MB", n as f64 / (1024.0 * 1024.0));
    }
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
        if s.contains(&key) {
            true
        } else {
            s.insert(key);
            false
        }
    });
    if already {
        return;
    }
    attach_download_handlers(session);
}

pub(crate) fn attach_download_handlers(session: &webkit6::NetworkSession) {
    session.connect_download_started(|_sess, download| {
        let dest_dir = glib::user_special_dir(glib::UserDirectory::Downloads)
            .unwrap_or_else(|| glib::home_dir().join("Downloads"));
        let _ = std::fs::create_dir_all(&dest_dir);

        let uri = download
            .request()
            .and_then(|r| r.uri())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let suggested = download
            .destination()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                // from URI path
                uri.rsplit('/')
                    .next()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "download".to_string());
        // strip query
        let suggested = suggested
            .split('?')
            .next()
            .unwrap_or("download")
            .to_string();
        let mut final_name = suggested.clone();
        let mut path = dest_dir.join(&final_name);
        if path.exists() {
            let stem = std::path::Path::new(&suggested)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("download");
            let ext = std::path::Path::new(&suggested)
                .extension()
                .and_then(|s| s.to_str())
                .map(|e| format!(".{e}"))
                .unwrap_or_default();
            let mut counter = 1;
            loop {
                let candidate_name = format!("{stem} ({counter}){ext}");
                let candidate = dest_dir.join(&candidate_name);
                if !candidate.exists() {
                    final_name = candidate_name;
                    path = candidate;
                    break;
                }
                counter += 1;
            }
        }
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
                filename: final_name,
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
            let total = if prog > 0.01 {
                (received as f64 / prog) as i64
            } else {
                0
            };
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
