//! DevTools Inspector resource overlay: removes unwanted dock left/right buttons
//! seamlessly using GLib's G_RESOURCE_OVERLAYS facility.

use crate::config::rail_data_dir;
use std::path::Path;
use std::process::Command;

pub(crate) fn ensure_devtools_overlay() {
    let overlay_dir = rail_data_dir().join("inspector_overlay");
    let css_file = overlay_dir.join("Main.css");
    let js_file = overlay_dir.join("Main.js");

    if !css_file.exists() || !js_file.exists() {
        let _ = std::fs::create_dir_all(&overlay_dir);

        let candidate_sos = [
            "/usr/lib/libwebkitgtk-6.0.so.4",
            "/usr/lib64/libwebkitgtk-6.0.so.4",
            "/usr/lib/x86_64-linux-gnu/libwebkitgtk-6.0.so.4",
            "/usr/lib/aarch64-linux-gnu/libwebkitgtk-6.0.so.4",
        ];
        let found_so = candidate_sos.iter().find(|p| Path::new(p).exists());

        if let Some(so) = found_so {
            // 1. Extract and patch Main.css
            if let Ok(output) = Command::new("gresource")
                .args(["extract", so, "/org/webkit/inspector/UserInterface/Main.css"])
                .output()
                && output.status.success()
            {
                let mut css = String::from_utf8_lossy(&output.stdout).to_string();
                css.push_str(
                    "\n\
                    .toolbar .item.dock-left,\n\
                    .toolbar .item.dock-right,\n\
                    .navigation-bar .item.dock-left,\n\
                    .navigation-bar .item.dock-right,\n\
                    .item.dock-left,\n\
                    .item.dock-right,\n\
                    button.dock-left,\n\
                    button.dock-right,\n\
                    [class*=\"dock-left\"],\n\
                    [class*=\"dock-right\"],\n\
                    [aria-label*=\"left\" i][aria-label*=\"dock\" i],\n\
                    [aria-label*=\"right\" i][aria-label*=\"dock\" i],\n\
                    [title*=\"left\" i][title*=\"dock\" i],\n\
                    [title*=\"right\" i][title*=\"dock\" i] {\n\
                        display: none !important;\n\
                        width: 0 !important;\n\
                        min-width: 0 !important;\n\
                        max-width: 0 !important;\n\
                        margin: 0 !important;\n\
                        padding: 0 !important;\n\
                        border: none !important;\n\
                        pointer-events: none !important;\n\
                    }\n",
                );
                let _ = std::fs::write(&css_file, css);
            }

            // 2. Extract and patch Main.js (disable dock left/right buttons from even being created)
            if let Ok(output) = Command::new("gresource")
                .args(["extract", so, "/org/webkit/inspector/UserInterface/Main.js"])
                .output()
                && output.status.success()
            {
                let js = String::from_utf8_lossy(&output.stdout).to_string();
                let patched = js
                    .replace(
                        "function addDockLeftButton(){",
                        "function addDockLeftButton(){return;",
                    )
                    .replace(
                        "function addDockRightButton(){",
                        "function addDockRightButton(){return;",
                    );
                let _ = std::fs::write(&js_file, patched);
            }
        }
    }

    if css_file.exists() {
        let overlay_val = format!(
            "/org/webkit/inspector/UserInterface={}",
            overlay_dir.display()
        );
        let current = std::env::var("G_RESOURCE_OVERLAYS").unwrap_or_default();
        if !current.contains("/org/webkit/inspector/UserInterface") {
            let new_val = if current.is_empty() {
                overlay_val
            } else {
                format!("{}:{}", current, overlay_val)
            };
            unsafe {
                std::env::set_var("G_RESOURCE_OVERLAYS", new_val);
            }
        }
    }
}
