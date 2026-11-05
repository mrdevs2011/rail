//! Desktop notifications handler for WebKit Web Notifications and PWAs.
//! Provides native Linux notifications (notify-send / freedesktop notification daemon)
//! that work both while the browser/PWA is active and in background daemon mode.

use std::process::Command;
use webkit6::WebView;
use webkit6::prelude::*;

/// Dispatch native Linux desktop notification using standard freedesktop notify daemon
pub(crate) fn show_desktop_notification(
    app_name: &str,
    title: &str,
    body: &str,
    icon: Option<&str>,
) {
    let mut cmd = Command::new("notify-send");
    cmd.arg("-a").arg(app_name);

    if let Some(i) = icon
        && !i.is_empty()
        && std::path::Path::new(i).exists()
    {
        cmd.arg("-i").arg(i);
    }

    // Keep notification text as positional data even when it starts with '-'.
    cmd.arg("--").arg(title);
    if !body.is_empty() {
        cmd.arg(body);
    }

    // Never block the WebKit signal handler waiting for the desktop notification service.
    let _ = cmd.spawn();
}

/// Attach WebKit show-notification handler to dispatch native desktop notifications
pub(crate) fn attach_notification_handler(
    view: &WebView,
    app_name: Option<String>,
    icon_path: Option<String>,
) {
    view.connect_show_notification(move |_view, notif| {
        let app = app_name
            .clone()
            .unwrap_or_else(|| "Rail Browser".to_string());
        let title = notif
            .title()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| app.clone());
        let body = notif.body().map(|s| s.to_string()).unwrap_or_default();

        show_desktop_notification(&app, &title, &body, icon_path.as_deref());

        let n = notif.clone();
        notif.connect_clicked(move |_| {
            // Web notification clicked callback
            n.close();
        });

        true
    });
}

/// Show a desktop notification whose click opens `url` in Rail. Runs notify-send on a worker
/// thread because waiting for the action blocks until the notification is dismissed.
pub(crate) fn show_clickable_notification(app_name: &str, title: &str, body: &str, url: &str) {
    let (app, title, body, url) = (
        app_name.to_string(),
        title.to_string(),
        body.to_string(),
        url.to_string(),
    );
    std::thread::spawn(move || {
        let mut cmd = Command::new("notify-send");
        cmd.arg("-a")
            .arg(&app)
            .arg("-A")
            .arg("default=Open")
            .arg("--")
            .arg(&title)
            .arg(&body)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let Ok(child) = cmd.spawn() else { return };
        let Ok(out) = child.wait_with_output() else {
            return;
        };
        if String::from_utf8_lossy(&out.stdout).trim() == "default"
            && (url.starts_with("https://") || url.starts_with("http://"))
            && let Ok(exe) = std::env::current_exe()
        {
            let _ = Command::new(exe).arg(&url).spawn();
        }
    });
}
