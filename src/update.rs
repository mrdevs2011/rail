//! "New version available" card: checks GitHub releases once per launch and can
//! download the newest binary over the installed one.

use gtk::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use std::cell::Cell;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

const REPO: &str = "mrdevs2011/rail";
const ASSET: &str = "rail-linux-x86_64";

/// The running version. `RAIL_FAKE_VERSION` lets you test the card without a real old build.
pub fn current_version() -> String {
    std::env::var("RAIL_FAKE_VERSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn parse_ver(s: &str) -> Option<[u32; 3]> {
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
struct UpdateInfo {
    latest_tag: String,
    newer_count: usize,
    more: bool,
    url: String,
}

fn fetch_update() -> Option<UpdateInfo> {
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
        url: format!("https://github.com/{}/releases/download/{}/{}", REPO, tag, ASSET),
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

fn download_and_install(url: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dest = install_path();
    let dir = dest.parent().ok_or("Bad install path")?.to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot write to {}: {}", dir.display(), e))?;
    let tmp = dir.join(".rail-update.tmp");
    let status = Command::new("curl")
        .args(["-fsSL", "-m", "240", "-o"])
        .arg(&tmp)
        .arg(url)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("curl failed: {}", e))?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err("Download failed. Check your connection.".to_string());
    }
    let size = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if size < 200_000 {
        let _ = std::fs::remove_file(&tmp);
        return Err("Downloaded file looks wrong.".to_string());
    }
    let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    let ok = Command::new(&tmp)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).starts_with("rail "))
        .unwrap_or(false);
    if !ok {
        let _ = std::fs::remove_file(&tmp);
        return Err("Downloaded binary does not run on this system.".to_string());
    }
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("Cannot replace {}: {}", dest.display(), e)
    })
}

fn restart(win: &gtk::ApplicationWindow) {
    let exe = install_path();
    let _ = Command::new("sh")
        .arg("-c")
        .arg("sleep 0.8; exec \"$0\"")
        .arg(&exe)
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

    let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    card.add_css_class("update-card");
    card.set_size_request(270, -1);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let kicker = gtk::Label::new(Some("UPDATE AVAILABLE"));
    kicker.add_css_class("update-kicker");
    kicker.set_xalign(0.0);
    kicker.set_hexpand(true);
    let close = gtk::Button::with_label("\u{00d7}");
    close.add_css_class("update-close");
    close.set_tooltip_text(Some("Dismiss (shows again next launch)"));
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

    let get = gtk::Button::with_label("Update");
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
            b.set_label("Updating...");
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
                        ver.set_text(&format!("{} installed", inf.latest_tag));
                        sub.set_text("Restarting. Your tabs come back.");
                        b.set_label("Restarting...");
                        b.set_sensitive(false);
                        let win = win_done.clone();
                        glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || restart(&win));
                    }
                    Ok(Err(e)) => {
                        err.set_text(&e);
                        err.set_visible(true);
                        b.set_label("Retry");
                    }
                    Err(_) => {
                        err.set_text("Update failed unexpectedly.");
                        err.set_visible(true);
                        b.set_label("Retry");
                    }
                }
            });
        });
    }

    {
        let rev = rev.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(fetch_update).await;
            if let Ok(Some(inf)) = res {
                ver.set_text(&format!("{} is out", inf.latest_tag));
                let n = inf.newer_count;
                sub.set_text(&format!(
                    "You are on v{}, {}{} version{} behind.",
                    current_version(),
                    n,
                    if inf.more { "+" } else { "" },
                    if n == 1 { "" } else { "s" }
                ));
                *info.borrow_mut() = Some(inf);
                rev.set_reveal_child(true);
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
            println!("rail {} is up to date", current_version());
            glib::ExitCode::SUCCESS
        }
        Some(inf) => {
            println!("Updating v{} -> {} ...", current_version(), inf.latest_tag);
            match download_and_install(&inf.url) {
                Ok(()) => {
                    println!("Done: {}", install_path().display());
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
