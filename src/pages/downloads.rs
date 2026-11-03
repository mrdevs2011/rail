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
    pub(crate) timestamp: i64,
}

thread_local! {
    pub(crate) static DOWNLOADS: RefCell<Vec<DownloadEntry>> = const { RefCell::new(Vec::new()) };
    pub(crate) static DOWNLOAD_SEQ: Cell<u64> = const { Cell::new(1) };
}

pub(crate) fn downloads_json_path() -> PathBuf {
    rail_data_dir().join("downloads.json")
}

pub(crate) fn load_downloads_from_disk() {
    let Some(text) = crate::storage::read_with_auto_healing(downloads_json_path(), |t| !t.trim().is_empty()) else {
        return;
    };
    DOWNLOADS.with(|d| {
        let mut v = d.borrow_mut();
        if !v.is_empty() {
            return;
        }
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(6, '\t').collect();
            if parts.len() < 5 {
                continue;
            }
            let id: u64 = parts[0].parse().unwrap_or(0);
            if id == 0 {
                continue;
            }
            let timestamp: i64 = if parts.len() >= 6 {
                parts[5].parse().unwrap_or(0)
            } else {
                0
            };
            v.push(DownloadEntry {
                id,
                status: parts[1].to_string(),
                filename: parts[2].to_string(),
                path: parts[3].to_string(),
                url: parts[4].to_string(),
                progress: if parts[1] == "finished" { 1.0 } else { 0.0 },
                total: 0,
                received: 0,
                timestamp,
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
            // keep finished/failed/cancelled history
            if e.status == "downloading" {
                continue;
            }
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                e.id, e.status, e.filename, e.path, e.url, e.timestamp
            ));
        }
        let _ = crate::storage::atomic_write_with_backup(downloads_json_path(), &out);
    });
}

pub(crate) fn mark_download_cancelled(id: u64) {
    DOWNLOADS.with(|d| {
        if let Some(e) = d.borrow_mut().iter_mut().find(|e| e.id == id) {
            e.status = "cancelled".to_string();
        }
    });
    save_downloads_to_disk();
}

pub(crate) fn remove_download(id: u64) {
    DOWNLOADS.with(|d| {
        d.borrow_mut().retain(|e| e.id != id);
    });
    save_downloads_to_disk();
}

pub(crate) fn clear_all_downloads() {
    DOWNLOADS.with(|d| {
        d.borrow_mut().clear();
    });
    let _ = crate::storage::atomic_write_with_backup(downloads_json_path(), "");
}

pub(crate) fn get_download_path(id: u64) -> Option<String> {
    DOWNLOADS.with(|d| {
        d.borrow().iter().find(|e| e.id == id).map(|e| e.path.clone())
    })
}

pub(crate) fn format_timestamp(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    glib::DateTime::from_unix_local(ts)
        .and_then(|d| d.format("%Y-%m-%d  %H:%M"))
        .map(|s| s.to_string())
        .unwrap_or_default()
}

pub(crate) fn mime_to_extension(mime: &str) -> Option<&'static str> {
    let clean = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match clean.as_str() {
        "text/html" | "application/xhtml+xml" => Some("html"),
        "text/plain" => Some("txt"),
        "text/css" => Some("css"),
        "text/javascript" | "application/javascript" | "application/x-javascript" => Some("js"),
        "text/markdown" | "text/x-markdown" => Some("md"),
        "text/csv" => Some("csv"),
        "application/json" => Some("json"),
        "application/xml" | "text/xml" => Some("xml"),
        "application/pdf" => Some("pdf"),
        "application/zip" => Some("zip"),
        "application/gzip" | "application/x-gzip" => Some("gz"),
        "application/x-tar" => Some("tar"),
        "application/x-bzip2" => Some("bz2"),
        "application/x-xz" => Some("xz"),
        "application/x-7z-compressed" => Some("7z"),
        "application/x-rar-compressed" => Some("rar"),
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/avif" => Some("avif"),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        "audio/mpeg" => Some("mp3"),
        "audio/ogg" => Some("ogg"),
        "audio/wav" => Some("wav"),
        "audio/flac" => Some("flac"),
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        "video/quicktime" => Some("mov"),
        "video/x-matroska" => Some("mkv"),
        "application/epub+zip" => Some("epub"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some("docx"),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some("xlsx"),
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Some("pptx"),
        "application/msword" => Some("doc"),
        "application/vnd.ms-excel" => Some("xls"),
        _ => None,
    }
}

pub(crate) fn resolve_download_filename(suggested: &str, uri: &str, mime: Option<&str>) -> String {
    let mut candidate = suggested.trim().to_string();
    if candidate.is_empty() || candidate == "download" || candidate == "blob" {
        let uri_path = uri
            .split('?')
            .next()
            .unwrap_or("")
            .split('#')
            .next()
            .unwrap_or("");
        if let Some(last) = uri_path.rsplit('/').next() {
            let last_decoded = glib::Uri::unescape_string(last, None)
                .map(|g| g.to_string())
                .unwrap_or_else(|| last.to_string());
            if !last_decoded.trim().is_empty() {
                candidate = last_decoded.trim().to_string();
            }
        }
    }
    if candidate.is_empty() {
        candidate = "download".to_string();
    }

    let mut sanitized: String = candidate
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c == '\0' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    sanitized = sanitized.trim().trim_start_matches('.').to_string();
    if sanitized.is_empty() {
        sanitized = "download".to_string();
    }

    let has_valid_ext = std::path::Path::new(&sanitized)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            !e.is_empty()
                && e.len() <= 8
                && (e.chars().all(|c| c.is_ascii_alphabetic())
                    || e.eq_ignore_ascii_case("7z")
                    || e.eq_ignore_ascii_case("mp3")
                    || e.eq_ignore_ascii_case("mp4"))
        })
        .unwrap_or(false);

    if !has_valid_ext {
        if let Some(ext) = mime.and_then(mime_to_extension) {
            sanitized = format!("{}.{}", sanitized, ext);
        } else {
            let uri_path = uri
                .split('?')
                .next()
                .unwrap_or("")
                .split('#')
                .next()
                .unwrap_or("");
            if let Some(ext) = std::path::Path::new(uri_path).extension().and_then(|e| e.to_str())
                && !ext.is_empty()
                && ext.len() <= 6
                && ext.chars().all(|c| c.is_ascii_alphabetic())
            {
                sanitized = format!("{}.{}", sanitized, ext);
            }
        }
    }

    sanitized
}

pub(crate) fn resolve_download_destination(
    suggested: &str,
    uri: &str,
    mime: Option<&str>,
) -> (String, String) {
    let dest_dir = glib::user_special_dir(glib::UserDirectory::Downloads)
        .unwrap_or_else(|| glib::home_dir().join("Downloads"));
    let _ = std::fs::create_dir_all(&dest_dir);

    let base_filename = resolve_download_filename(suggested, uri, mime);
    let mut final_filename = base_filename.clone();
    let mut dest_path = dest_dir.join(&final_filename);

    if dest_path.exists() {
        let p = std::path::Path::new(&base_filename);
        let stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("download");
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let mut counter = 1;
        loop {
            let candidate_name = format!("{} ({}){}", stem, counter, ext);
            let candidate_path = dest_dir.join(&candidate_name);
            if !candidate_path.exists() {
                final_filename = candidate_name;
                dest_path = candidate_path;
                break;
            }
            counter += 1;
        }
    }

    (final_filename, dest_path.to_string_lossy().to_string())
}

pub(crate) fn downloads_page() -> String {
    load_downloads_from_disk();
    let mut has_active = false;
    let (rows, is_empty) = DOWNLOADS.with(|d| {
        let list = d.borrow();
        if list.is_empty() {
            return (format!("<div class=\"empty\">{}</div>", tr("No downloads yet")), true);
        }
        let mut html = String::new();
        for e in list.iter().rev() {
            if e.status == "downloading" {
                has_active = true;
            }
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
            let date_str = format_timestamp(e.timestamp);
            let date_badge = if !date_str.is_empty() {
                format!("<span class=\"dl-date\">{}</span> · ", glib::markup_escape_text(&date_str))
            } else {
                String::new()
            };

            let origin = if !e.url.is_empty() {
                crate::history::extract_clean_domain(&e.url)
            } else {
                String::new()
            };
            let origin_disp = if !origin.is_empty() {
                origin
            } else {
                e.url.clone()
            };

            html.push_str(&format!(
                r##"<div class="dl-row">
  <div class="dl-header">
    <div class="dl-name-row">
      <a href="#" class="dl-name" onclick="openFile({id}); return false;" title="{open_file_hint}">{fname}</a>
      <button class="dl-remove-btn" onclick="removeDl({id}); return false;" title="{remove_hint}">✕</button>
    </div>
    <div class="dl-meta">{date_badge}<span class="dl-size">{size}</span> · <span class="{cls}">{status}</span></div>
  </div>
  <div class="dl-bar"><div class="dl-fill" style="width:{pct}%"></div></div>
  <div class="dl-footer">
    <a href="{url}" class="dl-url" target="_blank" rel="noopener">{origin_disp}</a>
    <div class="dl-actions">
      <a href="#" class="dl-action-link" onclick="openFolder({id}); return false;">{show_in_folder_hint}</a>
    </div>
  </div>
</div>"##,
                id = e.id,
                open_file_hint = glib::markup_escape_text(tr("Open File")),
                remove_hint = glib::markup_escape_text(tr("Remove from history")),
                fname = glib::markup_escape_text(&e.filename),
                date_badge = date_badge,
                size = glib::markup_escape_text(&size),
                cls = status_cls,
                status = glib::markup_escape_text(&e.status),
                pct = pct,
                url = glib::markup_escape_text(&e.url),
                origin_disp = glib::markup_escape_text(&origin_disp),
                show_in_folder_hint = glib::markup_escape_text(tr("Show in Folder")),
            ));
        }
        (html, false)
    });

    let clear_all_btn = if !is_empty {
        format!(
            r##"<button class="dl-clear-all-btn" onclick="clearAll()">{label}</button>"##,
            label = glib::markup_escape_text(tr("Clear All"))
        )
    } else {
        String::new()
    };

    let auto_reload = if has_active {
        "<script>setTimeout(function(){ location.reload(); }, 1000);</script>"
    } else {
        ""
    };

    format!(
        r##"<!doctype html>
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
  .dl-header-top {{
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 22px;
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
    word-break: break-word;
  }}
  .dl-clear-all-btn {{
    background: #141414;
    border: 1px solid #27272a;
    color: #a1a1aa;
    border-radius: 4px;
    padding: 6px 12px;
    font-size: 11.5px;
    font-family: inherit;
    font-weight: 600;
    cursor: pointer;
    transition: all 120ms ease;
    white-space: nowrap;
  }}
  .dl-clear-all-btn:hover {{
    background: #27272a;
    border-color: #ef4444;
    color: #ef4444;
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
  .dl-name-row {{
    display: flex;
    align-items: center;
    gap: 8px;
    flex-grow: 1;
  }}
  .dl-name {{
    font-size: clamp(12.5px, 2.8vw, 13.5px);
    font-weight: 600;
    color: #f4f4f4;
    text-decoration: none;
    word-break: break-word;
    cursor: pointer;
  }}
  .dl-name:hover {{
    color: #38bdf8;
    text-decoration: underline;
  }}
  .dl-remove-btn {{
    opacity: 0;
    transition: opacity 120ms ease;
    background: transparent;
    border: 1px solid #27272a;
    color: #71717a;
    border-radius: 3px;
    cursor: pointer;
    padding: 1px 6px;
    font-size: 11px;
    line-height: 1.2;
  }}
  .dl-row:hover .dl-remove-btn {{
    opacity: 1;
  }}
  .dl-remove-btn:hover {{
    background: #271515;
    border-color: #ef4444;
    color: #ef4444;
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
  .dl-date {{ color: #71717a; }}
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
  .dl-footer {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }}
  .dl-url {{
    font-size: clamp(9.5px, 2.2vw, 10.5px);
    color: #525252;
    text-decoration: none;
    word-break: break-all;
  }}
  .dl-url:hover {{
    color: #a1a1aa;
    text-decoration: underline;
  }}
  .dl-action-link {{
    font-size: 11px;
    color: #71717a;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
  }}
  .dl-action-link:hover {{
    color: #ffffff;
    text-decoration: underline;
  }}
</style>
<script>
  function postDl(msg) {{
    try {{
      window.webkit.messageHandlers.railDownloads.postMessage(msg);
    }} catch (e) {{}}
  }}
  function clearAll() {{ postDl('clear_all'); }}
  function removeDl(id) {{ postDl('remove:' + id); }}
  function openFile(id) {{ postDl('open_file:' + id); }}
  function openFolder(id) {{ postDl('open_folder:' + id); }}
</script>
</head>
<body>
  <div class="container">
    <div class="dl-header-top">
      <div>
        <h1>{title}</h1>
        <div class="sub">rail://downloads · {hint}</div>
      </div>
      {clear_all_btn}
    </div>
    {rows}
  </div>
  {auto_reload}
</body>
</html>"##,
        lang = i18n::current().code(),
        title = tr("Downloads"),
        hint = tr("Chrome-style download history for this profile session"),
        clear_all_btn = clear_all_btn,
        rows = rows,
        auto_reload = auto_reload,
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
        let dl_c = download.clone();
        download.connect_decide_destination(move |dl, suggested| {
            let uri = dl
                .request()
                .and_then(|r| r.uri())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let mime = dl
                .response()
                .and_then(|r| r.mime_type())
                .map(|s| s.to_string());

            let (final_name, path_s) =
                resolve_download_destination(suggested, &uri, mime.as_deref());
            dl.set_destination(&path_s);

            let id = DOWNLOAD_SEQ.with(|s| {
                let id = s.get();
                s.set(id + 1);
                id
            });
            let now_ts = glib::real_time() / 1_000_000;
            DOWNLOADS.with(|d| {
                d.borrow_mut().push(DownloadEntry {
                    id,
                    filename: final_name.clone(),
                    url: uri.clone(),
                    path: path_s.clone(),
                    status: "downloading".into(),
                    progress: 0.0,
                    total: 0,
                    received: 0,
                    timestamp: now_ts,
                });
            });

            if let Some(app) = crate::MAIN_APP.with(|m| m.borrow().clone()) {
                app.show_download_toast(id, &final_name, dl);
            }

            let id_c = id;
            dl.connect_received_data(move |dl_recv, _length| {
                let received = dl_recv.received_data_length() as i64;
                let prog = dl_recv.estimated_progress();
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
            });

            let id_f = id;
            dl.connect_finished(move |_dl_fin| {
                DOWNLOADS.with(|d| {
                    if let Some(e) = d.borrow_mut().iter_mut().find(|e| e.id == id_f) {
                        e.status = "finished".into();
                        e.progress = 1.0;
                    }
                });
                save_downloads_to_disk();
            });

            let id_e = id;
            dl.connect_failed(move |_dl_fail, _err| {
                DOWNLOADS.with(|d| {
                    if let Some(e) = d.borrow_mut().iter_mut().find(|e| e.id == id_e) {
                        e.status = "failed".into();
                    }
                });
                save_downloads_to_disk();
            });

            true
        });
        let _ = dl_c;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_download_filename_html() {
        let name = resolve_download_filename("", "https://news.ycombinator.com/item?id=123", Some("text/html; charset=utf-8"));
        assert_eq!(name, "item.html");
    }

    #[test]
    fn test_resolve_download_filename_with_query_and_no_ext() {
        let name = resolve_download_filename("export", "https://example.com/export?format=html", Some("text/html"));
        assert_eq!(name, "export.html");
    }

    #[test]
    fn test_resolve_download_filename_preserves_existing_ext() {
        let name = resolve_download_filename("archive.tar.gz", "https://example.com/archive.tar.gz", Some("application/gzip"));
        assert_eq!(name, "archive.tar.gz");

        let pdf = resolve_download_filename("paper.pdf", "https://example.com/paper.pdf", Some("application/pdf"));
        assert_eq!(pdf, "paper.pdf");
    }

    #[test]
    fn test_resolve_download_filename_sanitizes_bad_characters() {
        let name = resolve_download_filename("../../evil/test\0file.pdf", "https://example.com/evil", Some("application/pdf"));
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
        assert!(!name.contains('\0'));
        assert!(name.ends_with(".pdf"));
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "");
        let formatted = format_timestamp(1700000000);
        assert!(!formatted.is_empty());
        assert!(formatted.contains(':'));
    }

    #[test]
    fn test_mime_to_extension_mappings() {
        assert_eq!(mime_to_extension("text/html"), Some("html"));
        assert_eq!(mime_to_extension("application/pdf"), Some("pdf"));
        assert_eq!(mime_to_extension("image/png"), Some("png"));
        assert_eq!(mime_to_extension("application/zip"), Some("zip"));
    }
}
