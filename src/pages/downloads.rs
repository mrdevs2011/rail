//! Download tracking and the interactive rail://downloads page.

use crate::*;

#[derive(Clone, Debug)]
pub(crate) struct DownloadEntry {
    pub(crate) id: u64,
    pub(crate) filename: String,
    pub(crate) url: String,
    pub(crate) path: String,
    pub(crate) mime_type: String,
    pub(crate) status: String, // downloading | finished | failed | cancelled | interrupted
    pub(crate) progress: f64,
    pub(crate) total: i64,
    pub(crate) received: i64,
    pub(crate) started_at: i64,
    pub(crate) completed_at: i64,
}

type DownloadNotice = Rc<dyn Fn(&str, &str)>;

thread_local! {
    pub(crate) static DOWNLOADS: RefCell<Vec<DownloadEntry>> = const { RefCell::new(Vec::new()) };
    pub(crate) static DOWNLOAD_SEQ: Cell<u64> = const { Cell::new(1) };
    static DOWNLOAD_NOTICE: RefCell<Option<DownloadNotice>> = const { RefCell::new(None) };
}

pub(crate) fn install_download_notifications(app: &Rc<App>) {
    // Load prior history before any profile can start a new transfer.
    load_downloads_from_disk();
    let app = Rc::downgrade(app);
    DOWNLOAD_NOTICE.with(|slot| {
        *slot.borrow_mut() = Some(Rc::new(move |title, body| {
            if let Some(app) = app.upgrade() {
                app.show_copied_toast(title, body);
            }
        }));
    });
}

fn notify_download(title: &str, body: &str) {
    let callback = DOWNLOAD_NOTICE.with(|slot| slot.borrow().clone());
    if let Some(callback) = callback {
        callback(title, body);
    }
}

pub(crate) fn downloads_json_path() -> PathBuf {
    rail_data_dir().join("downloads.json")
}

fn encode_field(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn decode_field(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().as_chunks::<2>().0 {
        let hex = std::str::from_utf8(pair).ok()?;
        bytes.push(u8::from_str_radix(hex, 16).ok()?);
    }
    String::from_utf8(bytes).ok()
}

fn parse_download_line(line: &str) -> Option<DownloadEntry> {
    let parts: Vec<&str> = line.split('\t').collect();
    let id = parts.first()?.parse::<u64>().ok()?;
    if id == 0 {
        return None;
    }

    // v2 uses hex-encoded text fields, so tabs/newlines in URLs and paths cannot corrupt history.
    if parts.len() >= 11 {
        let filename = decode_field(parts[7])?;
        let path = decode_field(parts[8])?;
        let url = decode_field(parts[9])?;
        let mime_type = decode_field(parts[10])?;
        return Some(DownloadEntry {
            id,
            status: parts[1].to_string(),
            received: parts[2].parse().unwrap_or(0),
            total: parts[3].parse().unwrap_or(0),
            progress: parts[4].parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0),
            started_at: parts[5].parse().unwrap_or(0),
            completed_at: parts[6].parse().unwrap_or(0),
            filename,
            path,
            url,
            mime_type,
        });
    }

    // Backward compatibility with the original five-column history file.
    if parts.len() >= 5 {
        return Some(DownloadEntry {
            id,
            status: parts[1].to_string(),
            filename: parts[2].to_string(),
            path: parts[3].to_string(),
            url: parts[4].to_string(),
            mime_type: String::new(),
            progress: if parts[1] == "finished" { 1.0 } else { 0.0 },
            total: 0,
            received: 0,
            started_at: 0,
            completed_at: 0,
        });
    }
    None
}

pub(crate) fn load_downloads_from_disk() {
    let Ok(text) = std::fs::read_to_string(downloads_json_path()) else {
        return;
    };
    DOWNLOADS.with(|downloads| {
        let mut list = downloads.borrow_mut();
        if !list.is_empty() {
            return;
        }
        let mut interrupted = false;
        for line in text.lines() {
            let Some(mut entry) = parse_download_line(line) else {
                continue;
            };
            if entry.status == "downloading" {
                entry.status = "interrupted".into();
                entry.completed_at = unix_now();
                interrupted = true;
            }
            DOWNLOAD_SEQ.with(|seq| {
                if entry.id >= seq.get() {
                    seq.set(entry.id + 1);
                }
            });
            list.push(entry);
        }
        drop(list);
        if interrupted {
            save_downloads_to_disk();
        }
    });
}

pub(crate) fn save_downloads_to_disk() {
    let mut output = String::new();
    DOWNLOADS.with(|downloads| {
        for entry in downloads.borrow().iter() {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                entry.id,
                entry.status,
                entry.received,
                entry.total,
                entry.progress.clamp(0.0, 1.0),
                entry.started_at,
                entry.completed_at,
                encode_field(&entry.filename),
                encode_field(&entry.path),
                encode_field(&entry.url),
                encode_field(&entry.mime_type),
            ));
        }
    });

    let path = downloads_json_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, output).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

fn timestamp_label(timestamp: i64) -> String {
    if timestamp <= 0 {
        return tr("Unknown").to_string();
    }
    glib::DateTime::from_unix_local(timestamp)
        .ok()
        .and_then(|date| date.format("%Y-%m-%d %H:%M").ok())
        .map(|formatted| formatted.to_string())
        .unwrap_or_else(|| timestamp.to_string())
}

fn json_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            ch if ch <= '\u{1f}' => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn downloads_snapshot_json() -> String {
    load_downloads_from_disk();
    DOWNLOADS.with(|downloads| {
        let mut items: Vec<DownloadEntry> = downloads.borrow().iter().cloned().collect();
        items.sort_by_key(|entry| (entry.started_at, entry.id));
        items.reverse();
        let mut out = String::from("[");
        for (index, entry) in items.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let type_label = std::path::Path::new(&entry.filename)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.to_uppercase())
                .filter(|extension| !extension.is_empty())
                .unwrap_or_else(|| {
                    if entry.mime_type.is_empty() {
                        tr("File").to_string()
                    } else {
                        entry.mime_type.clone()
                    }
                });
            let status_label = match entry.status.as_str() {
                "downloading" => tr("Downloading"),
                "finished" => tr("Completed"),
                "cancelled" => tr("Cancelled"),
                "interrupted" => tr("Interrupted"),
                _ => tr("Failed"),
            };
            let size_label = if entry.total > 0 {
                format!("{} / {}", format_bytes(entry.received), format_bytes(entry.total))
            } else {
                format_bytes(entry.received)
            };
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "{{\"id\":{},\"filename\":{},\"url\":{},\"path\":{},\"exists\":{},\"mime\":{},\"type\":{},\"status\":{},\"status_label\":{},\"progress\":{:.4},\"percent\":{},\"received\":{},\"total\":{},\"size\":{},\"started\":{},\"completed\":{}}}",
                    entry.id,
                    json_quote(&entry.filename),
                    json_quote(&entry.url),
                    json_quote(&entry.path),
                    Path::new(&entry.path).is_file(),
                    json_quote(&entry.mime_type),
                    json_quote(&type_label),
                    json_quote(&entry.status),
                    json_quote(status_label),
                    entry.progress.clamp(0.0, 1.0),
                    (entry.progress.clamp(0.0, 1.0) * 100.0).round() as i32,
                    entry.received.max(0),
                    entry.total.max(0),
                    json_quote(&size_label),
                    json_quote(&timestamp_label(entry.started_at)),
                    json_quote(&timestamp_label(if entry.completed_at > 0 { entry.completed_at } else { entry.started_at })),
                ),
            );
        }
        out.push(']');
        out
    })
}

fn filename_from_uri(uri: &str) -> String {
    uri.split(['?', '#'])
        .next()
        .unwrap_or(uri)
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("download")
        .to_string()
}

fn filename_with_mime_extension(filename: &str, mime_type: &str) -> String {
    if Path::new(filename).extension().is_some() {
        return filename.to_string();
    }
    let mime = mime_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let extension = match mime.as_str() {
        "application/pdf" => "pdf",
        "application/zip" => "zip",
        "application/gzip" | "application/x-gzip" => "gz",
        "application/x-7z-compressed" => "7z",
        "application/x-rar-compressed" | "application/vnd.rar" => "rar",
        "application/json" => "json",
        "application/xml" | "text/xml" => "xml",
        "text/html" => "html",
        "text/plain" => "txt",
        "text/css" => "css",
        "text/javascript" | "application/javascript" => "js",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/avif" => "avif",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "audio/wav" | "audio/x-wav" => "wav",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "application/msword" => "doc",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.ms-powerpoint" => "ppt",
        _ => return filename.to_string(),
    };
    format!("{filename}.{extension}")
}

fn sanitize_filename(name: &str) -> String {
    let normalized = name.replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or("");
    let cleaned: String = leaf
        .chars()
        .filter(|ch| !ch.is_control() && !matches!(ch, '/' | '\\' | ':'))
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "download".into()
    } else {
        cleaned.to_string()
    }
}

fn choose_destination(dir: &std::path::Path, filename: &str, current_id: u64) -> PathBuf {
    let base = sanitize_filename(filename);
    let candidate = dir.join(&base);
    let in_use = |path: &std::path::Path| {
        path.exists()
            || DOWNLOADS.with(|downloads| {
                downloads.borrow().iter().any(|entry| {
                    entry.id != current_id
                        && entry.status == "downloading"
                        && Path::new(&entry.path) == path
                })
            })
    };
    if !in_use(&candidate) {
        return candidate;
    }

    let source = Path::new(&base);
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    let extension = source.extension().and_then(|s| s.to_str());
    for counter in 1u64.. {
        let name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem} ({counter}).{ext}"),
            _ => format!("{stem} ({counter})"),
        };
        let candidate = dir.join(name);
        if !in_use(&candidate) {
            return candidate;
        }
    }
    dir.join("download")
}

fn update_download(id: u64, update: impl FnOnce(&mut DownloadEntry)) {
    DOWNLOADS.with(|downloads| {
        if let Some(entry) = downloads
            .borrow_mut()
            .iter_mut()
            .find(|entry| entry.id == id)
        {
            update(entry);
        }
    });
}

pub(crate) fn downloads_page() -> String {
    let labels = format!(
        "{{\"clearConfirm\":{},\"empty\":{},\"emptyHint\":{},\"open\":{},\"folder\":{},\"delete\":{},\"source\":{},\"savedTo\":{},\"type\":{},\"started\":{},\"completed\":{},\"size\":{},\"items\":{},\"unknown\":{}}}",
        json_quote(tr(
            "Remove all download entries from this list? The files will stay on disk."
        )),
        json_quote(tr("No downloads yet")),
        json_quote(tr("When you download a file, it will appear here.")),
        json_quote(tr("Open file")),
        json_quote(tr("Show in folder")),
        json_quote(tr("Delete from list")),
        json_quote(tr("Source")),
        json_quote(tr("Saved to")),
        json_quote(tr("File type")),
        json_quote(tr("Started")),
        json_quote(tr("Last updated")),
        json_quote(tr("Size")),
        json_quote(tr("downloads")),
        json_quote(tr("Unknown")),
    );
    let template = r##"<!doctype html>
<html lang="__LANG__">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__TITLE__</title>
<style>
  * { box-sizing: border-box; }
  :root { color-scheme: dark; font-family: Inter, 'Cantarell', 'Noto Sans', system-ui, sans-serif; }
  body { margin: 0; padding: clamp(18px, 4vw, 38px); background: #000; color: #f4f4f4; }
  .container { width: 100%; max-width: 920px; margin: 0 auto; }
  .top { display: flex; justify-content: space-between; align-items: flex-start; gap: 20px; margin-bottom: 26px; }
  h1 { margin: 0; font-size: clamp(23px, 4vw, 30px); line-height: 1.1; letter-spacing: -.045em; font-weight: 760; }
  .sub { color: #777; font-size: 12px; margin-top: 8px; line-height: 1.5; }
  .top-actions { display: flex; align-items: center; gap: 12px; flex-shrink: 0; }
  .count { color: #777; font-size: 11px; white-space: nowrap; }
  button { font: inherit; cursor: pointer; }
  .clear { border: 1px solid #343434; color: #eee; background: #111; padding: 9px 13px; border-radius: 8px; font-size: 12px; font-weight: 650; transition: background .15s,border-color .15s; }
  .clear:hover { background: #1b1b1b; border-color: #666; }
  .clear:disabled { opacity: .35; cursor: default; }
  #download-list { display: flex; flex-direction: column; gap: 10px; }
  .empty { border: 1px dashed #292929; border-radius: 12px; padding: 64px 18px; text-align: center; color: #777; background: #080808; }
  .empty strong { display: block; color: #d7d7d7; font-size: 14px; margin-bottom: 7px; }
  .empty span { font-size: 12px; }
  .dl-card { position: relative; border: 1px solid #202020; border-radius: 11px; background: #0b0b0b; padding: 15px 16px 14px; transition: border-color .15s, background .15s; }
  .dl-card:hover, .dl-card:focus-within { border-color: #444; background: #0e0e0e; }
  .dl-head { display: flex; align-items: flex-start; gap: 12px; min-width: 0; }
  .file-icon { flex: 0 0 42px; width: 42px; height: 46px; display: grid; place-items: center; border: 1px solid #303030; border-radius: 8px; color: #aaa; background: #141414; font: 700 9px ui-monospace,monospace; overflow: hidden; }
  .file-icon svg { width: 21px; height: 21px; stroke: currentColor; }
  .dl-main { flex: 1; min-width: 0; }
  .dl-title-row { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; }
  .dl-name { font-size: 14px; line-height: 1.45; font-weight: 680; color: #f1f1f1; overflow-wrap: anywhere; }
  .dl-type { border: 1px solid #303030; border-radius: 5px; padding: 2px 5px; color: #999; font: 9px ui-monospace,monospace; white-space: nowrap; }
  .dl-status { display: inline-flex; align-items: center; gap: 6px; margin-top: 6px; color: #aaa; font-size: 11px; }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: #888; }
  .dl-status.finished .dot { background: #d8d8d8; }
  .dl-status.failed .dot, .dl-status.cancelled .dot, .dl-status.interrupted .dot { background: #666; }
  .dl-actions { display: flex; align-items: center; gap: 6px; opacity: 0; pointer-events: none; transition: opacity .15s; }
  .dl-card:hover .dl-actions, .dl-card:focus-within .dl-actions { opacity: 1; pointer-events: auto; }
  .action { display: inline-flex; align-items: center; gap: 6px; border: 1px solid #303030; border-radius: 7px; background: #151515; color: #ccc; padding: 7px 9px; font-size: 11px; white-space: nowrap; }
  .action:hover { color: white; background: #252525; border-color: #555; }
  .action:disabled { opacity: .35; cursor: default; }
  .action.delete:hover { color: #fff; border-color: #777; background: #292929; }
  .action svg { width: 13px; height: 13px; stroke: currentColor; fill: none; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .dl-progress { height: 3px; background: #202020; border-radius: 4px; overflow: hidden; margin: 13px 0 11px 54px; }
  .dl-progress[hidden] { display: none; }
  .dl-fill { width: 0; height: 100%; background: #bbb; border-radius: inherit; transition: width .2s ease; }
  .dl-details { margin-left: 54px; display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 9px 24px; }
  .detail { min-width: 0; }
  .detail-label { color: #656565; text-transform: uppercase; letter-spacing: .07em; font: 9px ui-monospace,monospace; margin-bottom: 4px; }
  .detail-value { color: #aaa; font-size: 11px; line-height: 1.45; overflow-wrap: anywhere; }
  .detail-value a { color: inherit; text-decoration: none; }
  .detail-value a:hover { color: #fff; text-decoration: underline; }
  .dl-size { color: #a8a8a8; margin-top: 6px; font: 10px ui-monospace,monospace; }
  .footnote { color: #626262; font-size: 10px; margin-top: 20px; }
  @media (hover:none), (max-width: 680px) { .dl-actions { opacity: 1; pointer-events: auto; flex-wrap: wrap; } .dl-details { grid-template-columns: 1fr; } }
  @media (max-width: 520px) { body { padding: 18px 12px; } .top { flex-direction: column; } .top-actions { width: 100%; justify-content: space-between; } .dl-card { padding: 12px; } .dl-progress,.dl-details { margin-left: 0; } .dl-actions { margin-left: 54px; } }
  button:focus-visible { outline: 2px solid #aaa; outline-offset: 2px; }
</style>
</head>
<body>
<div class="container">
  <header class="top">
    <div><h1>__TITLE__</h1><div class="sub">rail://downloads · __HINT__</div></div>
    <div class="top-actions"><span class="count" id="count"></span><button class="clear" id="clear-all" type="button">__CLEAR__</button></div>
  </header>
  <main id="download-list" aria-live="polite"></main>
  <div class="footnote">__CLEAR_HINT__</div>
</div>
<script>
(function () {
  'use strict';
  const L = __LABELS__;
  const list = document.getElementById('download-list');
  const count = document.getElementById('count');
  const clearButton = document.getElementById('clear-all');
  const post = (message) => { try { window.webkit.messageHandlers.railDownloads.postMessage(message); } catch (_) {} };
  const node = (tag, className, text) => { const n = document.createElement(tag); if (className) n.className = className; if (text != null) n.textContent = text; return n; };
  const iconFor = (name) => {
    const ext = String(name || '').split('.').pop().toLowerCase();
    if (!ext || ext === name.toLowerCase()) return 'FILE';
    return ext.slice(0, 6).toUpperCase();
  };
  function action(label, title, kind, id, svg) {
    const b = node('button', 'action ' + kind, label); b.type = 'button'; b.title = title; b.setAttribute('aria-label', title);
    b.insertAdjacentHTML('afterbegin', svg);
    b.addEventListener('click', () => post(kind + '\t' + id));
    return b;
  }
  function detail(parent, label, value) {
    const wrap = node('div', 'detail'); wrap.appendChild(node('div', 'detail-label', label));
    const val = node('div', 'detail-value'); val.textContent = value || L.unknown;
    wrap.appendChild(val); parent.appendChild(wrap);
  }
  function render(items) {
    if (!Array.isArray(items)) return;
    list.replaceChildren();
    count.textContent = items.length + ' ' + L.items;
    clearButton.disabled = items.length === 0;
    if (!items.length) {
      const empty = node('div', 'empty'); empty.appendChild(node('strong', '', L.empty)); empty.appendChild(node('span', '', L.emptyHint)); list.appendChild(empty); return;
    }
    const openSvg = '<svg viewBox="0 0 24 24"><path d="M14 3h7v7M10 14 21 3"/><path d="M19 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h6"/></svg>';
    const folderSvg = '<svg viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M3 10h18"/></svg>';
    const deleteSvg = '<svg viewBox="0 0 24 24"><path d="M3 6h18M8 6V4h8v2m3 0-1 14H6L5 6m4 4v6m6-6v6"/></svg>';
    items.forEach((item) => {
      const card = node('article', 'dl-card'); card.dataset.id = item.id;
      const head = node('div', 'dl-head');
      head.appendChild(node('div', 'file-icon', iconFor(item.filename)));
      const main = node('div', 'dl-main');
      const titleRow = node('div', 'dl-title-row'); titleRow.appendChild(node('div', 'dl-name', item.filename || 'download')); titleRow.appendChild(node('span', 'dl-type', item.type || 'FILE'));
      main.appendChild(titleRow);
      const status = node('div', 'dl-status ' + item.status); status.appendChild(node('span', 'dot')); status.appendChild(document.createTextNode(item.status_label || item.status));
      main.appendChild(status);
      main.appendChild(node('div', 'dl-size', item.size + (item.total > 0 ? ' · ' + item.percent + '%' : '')));
      head.appendChild(main);
      const actions = node('div', 'dl-actions');
      const openButton = action(L.open, L.open, 'open', item.id, openSvg);
      const folderButton = action(L.folder, L.folder, 'folder', item.id, folderSvg);
      openButton.disabled = !item.exists;
      folderButton.disabled = !item.path;
      actions.appendChild(openButton); actions.appendChild(folderButton);
      actions.appendChild(action(L.delete, L.delete, 'delete', item.id, deleteSvg));
      head.appendChild(actions); card.appendChild(head);
      const progress = node('div', 'dl-progress'); if (item.status !== 'downloading') progress.hidden = true;
      const fill = node('div', 'dl-fill'); fill.style.width = Math.max(0, Math.min(100, item.percent)) + '%'; progress.appendChild(fill); card.appendChild(progress);
      const details = node('div', 'dl-details');
      detail(details, L.type, item.mime || item.type);
      detail(details, L.size, item.size);
      detail(details, L.started, item.started);
      detail(details, L.source, item.url);
      detail(details, L.savedTo, item.path);
      detail(details, L.completed, item.completed);
      card.appendChild(details); list.appendChild(card);
    });
  }
  window.__railDownloadsUpdate = render;
  clearButton.addEventListener('click', () => { if (window.confirm(L.clearConfirm)) post('clear'); });
  post('snapshot');
  window.setInterval(() => post('snapshot'), 900);
})();
</script>
</body>
</html>"##;
    template
        .replace("__LANG__", i18n::current().code())
        .replace("__TITLE__", &glib::markup_escape_text(tr("Downloads")))
        .replace(
            "__HINT__",
            &glib::markup_escape_text(tr("Your downloaded files and their status")),
        )
        .replace("__CLEAR__", &glib::markup_escape_text(tr("Clear all")))
        .replace(
            "__CLEAR_HINT__",
            &glib::markup_escape_text(tr(
                "Downloaded files are kept on disk when you clear this list.",
            )),
        )
        .replace("__LABELS__", &labels)
}

pub(crate) fn format_bytes(n: i64) -> String {
    let n = n.max(0);
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

fn send_download_snapshot(view: &WebView) {
    if !view
        .uri()
        .is_some_and(|uri| uri.starts_with("rail://downloads"))
    {
        return;
    }
    let js = format!(
        "if(window.__railDownloadsUpdate)window.__railDownloadsUpdate({});",
        downloads_snapshot_json()
    );
    view.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
}

impl App {
    pub(crate) fn handle_downloads_message(&self, view: &WebView, message: &str) {
        // The bridge exists in all tabs, so accept commands only from Rail's own Downloads page.
        if !view
            .uri()
            .is_some_and(|uri| uri.starts_with("rail://downloads"))
        {
            return;
        }
        let (action, argument) = message.split_once('\t').unwrap_or((message, ""));
        match action {
            "snapshot" => send_download_snapshot(view),
            "delete" => {
                if let Ok(id) = argument.parse::<u64>() {
                    DOWNLOADS
                        .with(|downloads| downloads.borrow_mut().retain(|entry| entry.id != id));
                    save_downloads_to_disk();
                    send_download_snapshot(view);
                }
            }
            "clear" => {
                // Keep active transfers visible and trackable after history is cleared.
                DOWNLOADS.with(|downloads| {
                    downloads
                        .borrow_mut()
                        .retain(|entry| entry.status == "downloading");
                });
                save_downloads_to_disk();
                send_download_snapshot(view);
            }
            "open" | "folder" => {
                if let Ok(id) = argument.parse::<u64>() {
                    let path = DOWNLOADS.with(|downloads| {
                        downloads
                            .borrow()
                            .iter()
                            .find(|entry| entry.id == id)
                            .map(|entry| entry.path.clone())
                    });
                    if let Some(path) = path {
                        let target = if action == "folder" {
                            Path::new(&path)
                                .parent()
                                .map(Path::to_path_buf)
                                .unwrap_or_else(|| PathBuf::from(&path))
                        } else {
                            PathBuf::from(path)
                        };
                        let uri = gio::File::for_path(target).uri().to_string();
                        if gio::AppInfo::launch_default_for_uri(
                            &uri,
                            None::<&gio::AppLaunchContext>,
                        )
                        .is_err()
                        {
                            self.show_copied_toast(tr("Could not open location"), &uri);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn attach_download_handlers_once(session: &webkit6::NetworkSession) {
    use std::collections::HashSet;
    thread_local! {
        static HOOKED: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    }
    let key = session.as_ptr() as usize;
    let already = HOOKED.with(|hooked| {
        let mut sessions = hooked.borrow_mut();
        if sessions.contains(&key) {
            true
        } else {
            sessions.insert(key);
            false
        }
    });
    if !already {
        attach_download_handlers(session);
    }
}

pub(crate) fn attach_download_handlers(session: &webkit6::NetworkSession) {
    session.connect_download_started(|_session, download| {
        let dest_dir = glib::user_special_dir(glib::UserDirectory::Downloads)
            .unwrap_or_else(|| glib::home_dir().join("Downloads"));
        let _ = std::fs::create_dir_all(&dest_dir);

        let uri = download
            .request()
            .and_then(|request| request.uri())
            .map(|uri| uri.to_string())
            .unwrap_or_default();
        let response = download.response();
        let mime_type = response
            .as_ref()
            .and_then(|response| response.mime_type())
            .map(|mime| mime.to_string())
            .unwrap_or_default();
        let total = response
            .as_ref()
            .map(|response| response.content_length().min(i64::MAX as u64) as i64)
            .unwrap_or(0);
        let initial_name = response
            .as_ref()
            .and_then(|response| response.suggested_filename())
            .map(|name| name.to_string())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| filename_from_uri(&uri));

        let id = DOWNLOAD_SEQ.with(|seq| {
            let id = seq.get();
            seq.set(id + 1);
            id
        });
        DOWNLOADS.with(|downloads| {
            downloads.borrow_mut().push(DownloadEntry {
                id,
                filename: sanitize_filename(&initial_name),
                url: uri,
                path: String::new(),
                mime_type,
                status: "downloading".into(),
                progress: 0.0,
                total,
                received: 0,
                started_at: unix_now(),
                completed_at: 0,
            });
        });
        save_downloads_to_disk();

        // WebKit's decide-destination argument is the server/URI-derived suggested filename.
        // Do not use download.destination() here: before this signal it may be a temporary UUID.
        let id_dest = id;
        let dir_for_dest = dest_dir.clone();
        let uri_for_dest = download
            .request()
            .and_then(|request| request.uri())
            .map(|uri| uri.to_string())
            .unwrap_or_default();
        download.connect_decide_destination(move |dl, suggested_filename| {
            let suggested = if suggested_filename.trim().is_empty() {
                filename_from_uri(&uri_for_dest)
            } else {
                suggested_filename.to_string()
            };
            let response = dl.response();
            let mime_type = response
                .as_ref()
                .and_then(|response| response.mime_type())
                .map(|mime| mime.to_string())
                .unwrap_or_default();
            let filename = sanitize_filename(&filename_with_mime_extension(&suggested, &mime_type));
            let path = choose_destination(&dir_for_dest, &filename, id_dest);
            let path_string = path.to_string_lossy().to_string();
            let destination_uri = gio::File::for_path(&path).uri().to_string();
            dl.set_destination(&destination_uri);
            let total = response
                .as_ref()
                .map(|response| response.content_length().min(i64::MAX as u64) as i64)
                .unwrap_or(0);
            update_download(id_dest, |entry| {
                entry.filename = filename.clone();
                entry.path = path_string;
                entry.mime_type = mime_type;
                if total > 0 {
                    entry.total = total;
                }
            });
            save_downloads_to_disk();
            notify_download(tr("Download started"), &filename);
            true
        });

        let id_created = id;
        download.connect_created_destination(move |_dl, destination| {
            let actual_path = gio::File::for_uri(destination)
                .path()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| destination.to_string());
            update_download(id_created, |entry| entry.path = actual_path);
            save_downloads_to_disk();
        });

        let id_data = id;
        download.connect_received_data(move |dl, _data_length| {
            let received = dl.received_data_length().min(i64::MAX as u64) as i64;
            let progress = dl.estimated_progress().clamp(0.0, 1.0);
            let response_total = dl
                .response()
                .map(|response| response.content_length().min(i64::MAX as u64) as i64)
                .unwrap_or(0);
            update_download(id_data, |entry| {
                entry.received = received;
                if response_total > 0 {
                    entry.total = response_total;
                } else if progress > 0.01 {
                    entry.total = (received as f64 / progress).min(i64::MAX as f64) as i64;
                }
                entry.progress = progress;
                entry.status = "downloading".into();
            });
        });

        let id_done = id;
        download.connect_finished(move |dl| {
            let received = dl.received_data_length().min(i64::MAX as u64) as i64;
            let filename = DOWNLOADS.with(|downloads| {
                let mut list = downloads.borrow_mut();
                let Some(entry) = list.iter_mut().find(|entry| entry.id == id_done) else {
                    return String::new();
                };
                entry.status = "finished".into();
                entry.progress = 1.0;
                entry.received = received;
                if entry.total <= 0 {
                    entry.total = received;
                }
                entry.completed_at = unix_now();
                entry.filename.clone()
            });
            save_downloads_to_disk();
            if !filename.is_empty() {
                notify_download(tr("Download complete"), &filename);
            }
        });

        let id_failed = id;
        download.connect_failed(move |_dl, error| {
            let message = error.message().to_lowercase();
            let status = if message.contains("cancel") {
                "cancelled"
            } else {
                "failed"
            };
            let filename = DOWNLOADS.with(|downloads| {
                let mut list = downloads.borrow_mut();
                let Some(entry) = list.iter_mut().find(|entry| entry.id == id_failed) else {
                    return String::new();
                };
                entry.status = status.into();
                entry.completed_at = unix_now();
                entry.filename.clone()
            });
            save_downloads_to_disk();
            if !filename.is_empty() {
                notify_download(tr("Download failed"), &filename);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{
        decode_field, encode_field, filename_from_uri, filename_with_mime_extension, format_bytes,
        json_quote, parse_download_line, sanitize_filename,
    };

    #[test]
    fn filename_sanitization_keeps_extension_and_blocks_paths() {
        assert_eq!(sanitize_filename("../report.pdf"), "report.pdf");
        assert_eq!(sanitize_filename("C:\\temp\\photo.png"), "photo.png");
        assert_eq!(sanitize_filename("   "), "download");
        assert_eq!(
            filename_with_mime_extension("7da7d0c8", "application/pdf"),
            "7da7d0c8.pdf"
        );
        assert_eq!(
            filename_with_mime_extension("report.pdf", "application/pdf"),
            "report.pdf"
        );
        assert_eq!(
            filename_with_mime_extension("opaque-id", "application/octet-stream"),
            "opaque-id"
        );
    }

    #[test]
    fn uri_fallback_uses_last_path_component_without_query() {
        assert_eq!(
            filename_from_uri("https://example.test/files/archive.zip?token=1"),
            "archive.zip"
        );
        assert_eq!(
            filename_from_uri("https://example.test/download"),
            "download"
        );
    }

    #[test]
    fn history_fields_round_trip_tabs_newlines_and_unicode() {
        let value = "file\tname\nўзбекистон.zip";
        assert_eq!(decode_field(&encode_field(value)).as_deref(), Some(value));
    }

    #[test]
    fn legacy_download_history_is_still_readable() {
        let entry = parse_download_line("7\tfinished\tarchive.zip\t/home/user/Downloads/archive.zip\thttps://example.test/archive.zip").unwrap();
        assert_eq!(entry.id, 7);
        assert_eq!(entry.filename, "archive.zip");
        assert_eq!(entry.progress, 1.0);
    }

    #[test]
    fn json_and_bytes_formatting_are_safe() {
        assert_eq!(json_quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
        assert_eq!(json_quote("</script>"), "\"\\u003c/script\\u003e\"");
        assert_eq!(format_bytes(-1), "0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
    }

    #[test]
    fn version_two_download_history_restores_full_metadata() {
        let line = format!(
            "9\tfinished\t1200\t1200\t1.0\t1700000000\t1700000010\t{}\t{}\t{}\t{}",
            encode_field("archive.zip"),
            encode_field("/home/user/Downloads/archive.zip"),
            encode_field("https://example.test/archive.zip"),
            encode_field("application/zip"),
        );
        let entry = parse_download_line(&line).unwrap();
        assert_eq!(entry.filename, "archive.zip");
        assert_eq!(entry.mime_type, "application/zip");
        assert_eq!(entry.received, 1200);
        assert_eq!(entry.started_at, 1_700_000_000);
        assert_eq!(entry.completed_at, 1_700_000_010);
    }
}
