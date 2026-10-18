//! "New version available" card: checks GitHub releases once per launch and can
//! download the newest binary over the installed one.

use crate::i18n::{tr, trf};
use gtk::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use std::cell::Cell;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use sha2::{Sha256, Digest};

const REPO: &str = "mrdevs2011/rail";
/// Asset name for the current OS and architecture.
fn asset_name() -> String {
    use std::env::consts::{OS, ARCH};
    match (OS, ARCH) {
        ("linux", "x86_64") => "rail-linux-x86_64".to_string(),
        ("linux", "aarch64") => "rail-linux-aarch64".to_string(),
        _ => format!("rail-{}-{}", OS, ARCH),
    }
}

pub fn asset_url_for_tag(tag: &str) -> String {
    format!("https://github.com/{}/releases/download/{}/{}", REPO, tag, asset_name())
}

/// The running version. `RAIL_FAKE_VERSION` lets you test the card without a real old build.
pub fn current_version() -> String {
    std::env::var("RAIL_FAKE_VERSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

pub fn parse_ver(s: &str) -> Option<[u32; 3]> {
    let t = s.trim().trim_start_matches(['v', 'V']);
    let core = t.split(['-', '+']).next()?;
    let mut out = [0u32; 3];
    for (i, part) in core.split('.').enumerate() {
        if i >= 3 {
            break;
        }
        out[i] = part.parse().ok()?;
    }
    Some(out)
}

#[derive(Clone)]
pub struct UpdateInfo {
    pub latest_tag: String,
    pub newer_count: usize,
    pub more: bool,
    pub url: String,
}

/// Fetches all tags from the atom feed across all pages and latest update info if any newer.
pub fn fetch_releases() -> Option<(Vec<String>, Option<UpdateInfo>)> {
    let current = parse_ver(&current_version())?;
    let marker = "/releases/tag/";
    let mut seen: Vec<String> = Vec::new();
    let mut after: Option<String> = None;

    // GitHub's Atom feed lists 10 releases per page. Paginate with ?after=<tag>
    // to collect all historical releases (up to 10 pages / 100 releases).
    for _ in 0..10 {
        let feed_url = match &after {
            Some(a) => format!("https://github.com/{}/releases.atom?after={}", REPO, a),
            None => format!("https://github.com/{}/releases.atom", REPO),
        };
        let out = Command::new("curl")
            .args(["-fsSL", "-m", "8", "-A", "rail-browser", &feed_url])
            .stdin(Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() {
            break;
        }
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut rest = text.as_str();
        let mut page_count = 0;
        let mut last_tag = None;
        while let Some(i) = rest.find(marker) {
            rest = &rest[i + marker.len()..];
            let end = rest.find(['"', '<', '\'', ' ']).unwrap_or(rest.len());
            let tag = rest[..end].to_string();
            if !tag.is_empty() {
                if !seen.contains(&tag) {
                    seen.push(tag.clone());
                    page_count += 1;
                }
                last_tag = Some(tag);
            }
        }
        if page_count == 0 {
            break;
        }
        after = last_tag;
    }

    if seen.is_empty() {
        return None;
    }

    // Newest first in the dropdown
    seen.sort_by(|a, b| {
        match (parse_ver(a), parse_ver(b)) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => b.cmp(a),
        }
    });

    let mut count = 0usize;
    let mut best: Option<([u32; 3], String)> = None;
    for tag in &seen {
        let Some(ver) = parse_ver(tag) else { continue; };
        if ver > current {
            count += 1;
            if best.as_ref().map(|(b, _)| ver > *b).unwrap_or(true) {
                best = Some((ver, tag.clone()));
            }
        }
    }
    let more = !seen.is_empty() && count >= seen.len();
    let info = best.map(|(_, tag)| UpdateInfo {
        url: asset_url_for_tag(&tag),
        latest_tag: tag,
        newer_count: count,
        more,
    });
    Some((seen, info))
}

pub fn fetch_update() -> Option<UpdateInfo> {
    // The public releases feed has no API rate limit (the REST API allows only 60 calls/hour per IP).
    let feed_url = format!("https://github.com/{}/releases.atom", REPO);
    let out = Command::new("curl")
        .args(["-fsSL", "-m", "8", "-A", "rail-browser", &feed_url])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let current = parse_ver(&current_version())?;
    let marker = "/releases/tag/";
    let mut seen: Vec<String> = Vec::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find(marker) {
        rest = &rest[i + marker.len()..];
        let end = rest.find(['"', '<', '\'', ' ']).unwrap_or(rest.len());
        let tag = rest[..end].to_string();
        if !seen.contains(&tag) {
            seen.push(tag);
        }
    }
    let mut count = 0usize;
    let mut best: Option<([u32; 3], String)> = None;
    for tag in &seen {
        let Some(ver) = parse_ver(tag) else {
            continue;
        };
        if ver > current {
            count += 1;
            if best.as_ref().map(|(b, _)| ver > *b).unwrap_or(true) {
                best = Some((ver, tag.clone()));
            }
        }
    }
    let (_, tag) = best?;
    // The feed lists only the last 10 releases: if all of them are newer, the real gap is bigger.
    let more = !seen.is_empty() && count >= seen.len();
    Some(UpdateInfo {
        url: asset_url_for_tag(&tag),
        latest_tag: tag,
        newer_count: count,
        more,
    })
}

/// Where the new binary goes: the running executable, or ~/.local/bin/rail for dev builds.
fn install_path() -> PathBuf {
    let exe = std::env::current_exe()
        .ok()
        .map(|p| PathBuf::from(p.to_string_lossy().trim_end_matches(" (deleted)").to_string()));
    match exe {
        Some(p) if !p.to_string_lossy().contains("/target/") => p,
        _ => glib::home_dir().join(".local").join("bin").join("rail"),
    }
}

pub fn download_and_install(url: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dest = install_path();
    let dir = dest.parent().ok_or_else(|| tr("Bad install path").to_string())?.to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| trf("Cannot write to {}: {}", &[&dir.display().to_string(), &e.to_string()]))?;
    let tmp = dir.join(".rail-update.tmp");
    // Capture HTTP code so 404 (no asset for this tag) is not reported as "no internet"
    let out = Command::new("curl")
        .args(["-sSL", "-m", "240", "-w", "\n%{http_code}", "-o"])
        .arg(&tmp)
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| trf("curl failed: {}", &[&e.to_string()]))?;
    let tail = String::from_utf8_lossy(&out.stdout);
    let code = tail.trim().lines().last().unwrap_or("0").parse::<u32>().unwrap_or(0);
    if !out.status.success() || code == 0 {
        let _ = std::fs::remove_file(&tmp);
        return Err(tr("Download failed. Check your connection.").to_string());
    }
    if code == 404 {
        let _ = std::fs::remove_file(&tmp);
        return Err(tr("This release has no Linux binary (asset missing).").to_string());
    }
    if code < 200 || code >= 300 {
        let _ = std::fs::remove_file(&tmp);
        return Err(trf("Download failed (HTTP {}).", &[&code.to_string()]));
    }
    let size = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if size < 200_000 {
        let _ = std::fs::remove_file(&tmp);
        return Err(tr("Downloaded file looks wrong.").to_string());
    }
    // Compute SHA256 of the downloaded file
    let sha256_hex = {
        use std::io::Read;
        let mut file = std::fs::File::open(&tmp).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>()
    };
    // TODO: compare against SHA256SUMS from the release when available.
    // For now we continue with the binary --version check below.
    let _ = sha256_hex; // used in future verification
    let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    let ok = Command::new(&tmp)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).starts_with("rail "))
        .unwrap_or(false);
    if !ok {
        let _ = std::fs::remove_file(&tmp);
        return Err(tr("Downloaded binary does not run on this system.").to_string());
    }
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        trf("Cannot replace {}: {}", &[&dest.display().to_string(), &e.to_string()])
    })
}

pub fn restart(win: &gtk::ApplicationWindow) {
    let exe = install_path();
    // Rail is single-instance: wait until THIS process has exited, otherwise the new one
    // would just hand over to us and quit.
    let _ = Command::new("sh")
        .arg("-c")
        .arg("i=0; while kill -0 \"$1\" 2>/dev/null && [ $i -lt 50 ]; do i=$((i+1)); sleep 0.2; done; exec \"$0\"")
        .arg(&exe)
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    win.close();
}

/// Builds the (initially hidden) card. It shows itself if a newer release exists.
pub fn build_card(win: &gtk::ApplicationWindow) -> gtk::Revealer {
    let rev = gtk::Revealer::new();
    rev.set_transition_type(gtk::RevealerTransitionType::SlideUp);
    rev.set_halign(gtk::Align::End);
    rev.set_valign(gtk::Align::End);
    rev.set_margin_end(16);
    rev.set_margin_bottom(16);
    rev.set_reveal_child(false);
    // Keep pointer events on the card so the page underneath is not clicked
    rev.set_can_target(true);

    let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    card.add_css_class("update-card");
    card.set_size_request(270, -1);
    card.set_can_target(true);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let kicker = gtk::Label::new(Some(tr("UPDATE AVAILABLE")));
    kicker.add_css_class("update-kicker");
    kicker.set_xalign(0.0);
    kicker.set_hexpand(true);
    let close = gtk::Button::with_label("\u{00d7}");
    close.add_css_class("update-close");
    close.set_tooltip_text(Some(tr("Dismiss (shows again next launch)")));
    top.append(&kicker);
    top.append(&close);

    let ver = gtk::Label::new(None);
    ver.add_css_class("update-ver");
    ver.set_xalign(0.0);
    let sub = gtk::Label::new(None);
    sub.add_css_class("update-sub");
    sub.set_xalign(0.0);
    sub.set_wrap(true);
    let err = gtk::Label::new(None);
    err.add_css_class("update-err");
    err.set_xalign(0.0);
    err.set_wrap(true);
    err.set_visible(false);

    let get = gtk::Button::with_label(tr("Update"));
    get.add_css_class("update-get");
    get.set_halign(gtk::Align::Start);
    get.set_margin_top(6);

    card.append(&top);
    card.append(&ver);
    card.append(&sub);
    card.append(&err);
    card.append(&get);
    rev.set_child(Some(&card));

    {
        let rev = rev.clone();
        close.connect_clicked(move |_| rev.set_reveal_child(false));
    }

    let info: Rc<std::cell::RefCell<Option<UpdateInfo>>> = Rc::new(std::cell::RefCell::new(None));
    let installed = Rc::new(Cell::new(false));

    {
        let info = info.clone();
        let installed = installed.clone();
        let win = win.clone();
        let sub2 = sub.clone();
        let ver2 = ver.clone();
        let err2 = err.clone();
        get.connect_clicked(move |b| {
            if installed.get() {
                restart(&win);
                return;
            }
            let Some(inf) = info.borrow().clone() else {
                return;
            };
            b.set_sensitive(false);
            b.set_label(tr("Updating..."));
            err2.set_visible(false);
            let b = b.clone();
            let installed = installed.clone();
            let sub = sub2.clone();
            let ver = ver2.clone();
            let err = err2.clone();
            let win_done = win.clone();
            glib::spawn_future_local(async move {
                let url = inf.url.clone();
                let res = gio::spawn_blocking(move || download_and_install(&url)).await;
                b.set_sensitive(true);
                match res {
                    Ok(Ok(())) => {
                        installed.set(true);
                        ver.set_text(&trf("{} installed", &[&inf.latest_tag]));
                        sub.set_text(tr("Restarting. Your tabs come back."));
                        b.set_label(tr("Restarting..."));
                        b.set_sensitive(false);
                        let win = win_done.clone();
                        glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || restart(&win));
                    }
                    Ok(Err(e)) => {
                        err.set_text(&e);
                        err.set_visible(true);
                        b.set_label(tr("Retry"));
                    }
                    Err(_) => {
                        err.set_text(tr("Update failed unexpectedly."));
                        err.set_visible(true);
                        b.set_label(tr("Retry"));
                    }
                }
            });
        });
    }

    {
        let rev = rev.clone();
        let win = win.clone();
        let get = get.clone();
        let info = info.clone();
        let installed = installed.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(fetch_update).await;
            if let Ok(Some(inf)) = res {
                ver.set_text(&trf("{} is out", &[&inf.latest_tag]));
                let n = inf.newer_count;
                let behind = format!("{}{}", n, if inf.more { "+" } else { "" });
                sub.set_text(&trf(
                    if n == 1 { "You are on v{}, {} version behind." } else { "You are on v{}, {} versions behind." },
                    &[&current_version(), &behind],
                ));
                let url = inf.url.clone();
                let tag = inf.latest_tag.clone();
                *info.borrow_mut() = Some(inf);
                rev.set_reveal_child(true);

                // Auto-update: download + install + restart on every launch when enabled
                if crate::load_app_settings().auto_update {
                    get.set_sensitive(false);
                    get.set_label(tr("Updating..."));
                    sub.set_text(tr("Downloading update..."));
                    err.set_visible(false);
                    let res = gio::spawn_blocking(move || download_and_install(&url)).await;
                    match res {
                        Ok(Ok(())) => {
                            installed.set(true);
                            ver.set_text(&trf("{} installed", &[&tag]));
                            sub.set_text(tr("Restarting. Your tabs come back."));
                            get.set_label(tr("Restarting..."));
                            let win = win.clone();
                            glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || restart(&win));
                        }
                        Ok(Err(e)) => {
                            err.set_text(&e);
                            err.set_visible(true);
                            get.set_sensitive(true);
                            get.set_label(tr("Retry"));
                        }
                        Err(_) => {
                            err.set_text(tr("Update failed unexpectedly."));
                            err.set_visible(true);
                            get.set_sensitive(true);
                            get.set_label(tr("Retry"));
                        }
                    }
                }
            }
        });
    }
    rev
}

const ICON_SVG: &str = include_str!("../packaging/rail.svg");

/// Keeps the launcher icon current: after an update the new icon lands by itself.
pub fn ensure_icon() {
    let dir = glib::home_dir().join(".local/share/icons/hicolor/scalable/apps");
    let path = dir.join("rail.svg");
    if std::fs::read_to_string(&path).map(|c| c == ICON_SVG).unwrap_or(false) {
        return;
    }
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(&path, ICON_SVG);
    }
}

/// `rail --update`: same download as the card, for terminals and scripts.
pub fn cli_update() -> glib::ExitCode {
    match fetch_update() {
        None => {
            println!("{}", trf("rail {} is up to date", &[&current_version()]));
            glib::ExitCode::SUCCESS
        }
        Some(inf) => {
            println!("{}", trf("Updating v{} -> {} ...", &[&current_version(), &inf.latest_tag]));
            match download_and_install(&inf.url) {
                Ok(()) => {
                    println!("{}", trf("Done: {}", &[&install_path().display().to_string()]));
                    glib::ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e);
                    glib::ExitCode::FAILURE
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ver() {
        assert_eq!(parse_ver("v0.5.4"), Some([0, 5, 4]));
        assert_eq!(parse_ver("0.1.0"), Some([0, 1, 0]));
        assert_eq!(parse_ver("v1.2.3-beta"), Some([1, 2, 3]));
    }

    #[test]
    fn test_fetch_releases_gets_more_than_ten() {
        if let Some((tags, _)) = fetch_releases() {
            assert!(tags.len() > 10, "Should fetch all historical releases beyond 10, got {}", tags.len());
            assert!(tags.contains(&"v0.1.0".to_string()));
            assert!(tags.contains(&"v0.5.4".to_string()));
        }
    }
}
