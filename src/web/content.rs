//! User style sheets and ad & tracker blocking for every web view.

use crate::*;

pub(crate) const DEFAULT_HTML_CSS: &str = include_str!("../html_default.css");

thread_local! {
    pub(crate) static STYLE_MANAGERS: RefCell<Vec<glib::WeakRef<webkit6::UserContentManager>>> =
        RefCell::new(Vec::new());
    /// Compiled ad & tracker rules; `None` until compiled or when blocking is off.
    pub(crate) static ADBLOCK_FILTER: RefCell<Option<webkit6::UserContentFilter>> = const { RefCell::new(None) };
}

/// Ad & tracker blocking is on unless `<data dir>/adblock-off` exists or `RAIL_NO_ADBLOCK=1`.
pub(crate) fn adblock_enabled() -> bool {
    !std::env::var_os("RAIL_NO_ADBLOCK").is_some_and(|v| v != "0")
        && !rail_data_dir().join("adblock-off").exists()
}

/// Compiles the blocklist (plus `<data dir>/blocklist.txt`) once, then applies it to every
/// live web view; views created later pick it up in `create_web_view`.
pub(crate) fn setup_adblock() {
    if !adblock_enabled() {
        return;
    }
    let dir = rail_data_dir();
    let extra = std::fs::read_to_string(dir.join("blocklist.txt")).unwrap_or_default();
    let json = blocklist::rules_json(&extra);
    let store = webkit6::UserContentFilterStore::new(dir.join("filters").to_str().unwrap_or("filters"));
    let keep = store.clone();
    store.save("rail-adblock", &glib::Bytes::from(json.as_bytes()), gio::Cancellable::NONE, move |res| {
        let _keep = keep;
        match res {
            Ok(f) => {
                STYLE_MANAGERS.with(|list| {
                    for ucm in list.borrow().iter().filter_map(|w| w.upgrade()) {
                        ucm.add_filter(&f);
                    }
                });
                ADBLOCK_FILTER.with(|cell| *cell.borrow_mut() = Some(f));
            }
            Err(e) => eprintln!("rail: ad blocking rules failed to compile: {e}"),
        }
    });
}

/// Editable Chrome-like default stylesheet, stored next to the rest of Rail's data.
pub(crate) fn html_css_path() -> PathBuf {
    customize_dir().join("html.css")
}

/// Everything Customize writes lives here, so a Data & Backup export/import carries it along.
pub(crate) fn customize_dir() -> PathBuf {
    rail_data_dir().join(".railassets").join("customize")
}

/// Reads html.css; creates it from the built-in default the first time.
pub(crate) fn load_html_css() -> String {
    let path = html_css_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            // Migrate away from legacy Chrome UA reset dump which forced `color: black;`
            // on buttons/inputs and broke dark mode icons (e.g. YouTube mic and create buttons).
            if text.contains("font: -webkit-small-control")
                || text.contains("/* ===== form controls ===== */")
                || text.contains("Sites that set their own CSS still win; this layer only replaces the defaults.")
            {
                let _ = std::fs::write(&path, DEFAULT_HTML_CSS);
                return DEFAULT_HTML_CSS.to_string();
            }
            text
        }
        Err(_) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            // First run after the move: carry over the old html.css from the data root.
            let legacy = rail_data_dir().join("html.css");
            let initial =
                std::fs::read_to_string(&legacy).unwrap_or_else(|_| DEFAULT_HTML_CSS.to_string());
            let initial = if initial.contains("font: -webkit-small-control")
                || initial.contains("/* ===== form controls ===== */")
                || initial.contains("Sites that set their own CSS still win; this layer only replaces the defaults.")
            {
                DEFAULT_HTML_CSS.to_string()
            } else {
                initial
            };
            let _ = std::fs::write(&path, &initial);
            if legacy.exists() {
                let _ = std::fs::remove_file(&legacy);
            }
            initial
        }
    }
}

pub(crate) fn user_style_sheet(css: &str) -> webkit6::UserStyleSheet {
    webkit6::UserStyleSheet::new(
        css,
        webkit6::UserContentInjectedFrames::AllFrames,
        webkit6::UserStyleLevel::User,
        &[],
        &[],
    )
}

/// Pushes the css to every live web view; returns how many views were updated.
pub(crate) fn apply_html_css(css: &str) -> usize {
    let mut applied = 0;
    STYLE_MANAGERS.with(|list| {
        let mut list = list.borrow_mut();
        list.retain(|w| w.upgrade().is_some());
        for w in list.iter() {
            if let Some(ucm) = w.upgrade() {
                ucm.remove_all_style_sheets();
                if !css.trim().is_empty() {
                    ucm.add_style_sheet(&user_style_sheet(css));
                }
                apply_extensions_to_ucm(&ucm);
                applied += 1;
            }
        }
    });
    applied
}

pub(crate) fn save_css_editor(buffer: &gtk::TextBuffer, btn: &gtk::Button) {
    let (start, end) = buffer.bounds();
    let text = buffer.text(&start, &end, false).to_string();
    let path = html_css_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::write(&path, &text) {
        Ok(_) => {
            apply_html_css(&text);
            btn.set_label(tr("Saved"));
        }
        Err(_) => btn.set_label(tr("Save failed")),
    }
    let b = btn.clone();
    glib::timeout_add_local_once(Duration::from_millis(1500), move || b.set_label(tr("Save")));
}

/// Injects high-performance media optimizations across web views:
/// - Avoids heavy CPU-bound AV1 software decoding; prioritizes hardware-accelerated VP9/H.264
pub(crate) fn inject_web_optimizations(ucm: &webkit6::UserContentManager) {
    let script_code = r#"
(function() {
  if (window.__railWebOptimized) return;
  window.__railWebOptimized = true;

  var isYT = location.hostname.indexOf('youtube.com') !== -1;
  if (isYT) {
    // 1. Avoid heavy AV1 software decoding; prefer hardware-friendly VP9 / AVC1 (H.264).
    if (window.MediaSource && MediaSource.isTypeSupported) {
      var origIsTypeSupported = MediaSource.isTypeSupported.bind(MediaSource);
      MediaSource.isTypeSupported = function(type) {
        if (typeof type === 'string' && type.indexOf('av01') !== -1) {
          return false;
        }
        return origIsTypeSupported(type);
      };
    }
  }

  // 2. Custom context menu & preventDefault detection.
  // When a site provides its own context menu or suppresses the browser menu (e.g. YouTube,
  // Google Docs, Telegram, Figma, SpaceMR), notify the host so Rail's menu doesn't collide.
  function notifyContextPrevented() {
    try {
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railContextMenu) {
        window.webkit.messageHandlers.railContextMenu.postMessage("prevented");
      }
    } catch(_) {}
  }

  // Capture phase: intercept e.preventDefault on contextmenu
  window.addEventListener('contextmenu', function(e) {
    if (e.defaultPrevented) {
      notifyContextPrevented();
      return;
    }
    var orig = e.preventDefault;
    e.preventDefault = function() {
      notifyContextPrevented();
      return orig.apply(this, arguments);
    };
  }, true);

  // Bubble phase on window: catches cases where handlers return false or set defaultPrevented
  window.addEventListener('contextmenu', function(e) {
    if (e.defaultPrevented) {
      notifyContextPrevented();
      return;
    }
    var target = e.target;
    if (target && target.closest) {
      if (target.closest('.html5-video-player, ytd-player, #movie_player, .monaco-editor, [data-context-menu], [role="menu"]')) {
        notifyContextPrevented();
      }
    }
  }, false);

  window.addEventListener('pointerdown', function(e) {
    if (e.button === 2) {
      var orig = e.preventDefault;
      e.preventDefault = function() {
        notifyContextPrevented();
        return orig.apply(this, arguments);
      };
    }
  }, true);

  // 3. HTML5 Video inline playback & play reliability:
  // Modern web feeds (SpaceMR, Telegram Web, Instagram, Shorts) require 'playsinline'
  // and responsive click-to-play handling.
  function ensureVideoPlayback(root) {
    try {
      var vids = (root || document).querySelectorAll('video');
      for (var i = 0; i < vids.length; i++) {
        var v = vids[i];
        if (!v.hasAttribute('playsinline')) {
          v.setAttribute('playsinline', '');
          v.setAttribute('webkit-playsinline', '');
        }
        if (!v.__railHooked) {
          v.__railHooked = true;
          v.addEventListener('click', function() {
            if (this.paused) {
              this.play().catch(function() {});
            }
          });
        }
      }
    } catch(_) {}
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function() { ensureVideoPlayback(); });
  } else {
    ensureVideoPlayback();
  }
  window.addEventListener('load', function() { ensureVideoPlayback(); });
  if (window.MutationObserver && document.documentElement) {
    try {
      new MutationObserver(function(muts) {
        for (var i = 0; i < muts.length; i++) {
          if (muts[i].addedNodes && muts[i].addedNodes.length > 0) {
            ensureVideoPlayback(muts[i].target);
          }
        }
      }).observe(document.documentElement, { childList: true, subtree: true });
    } catch(_) {}
  }
})();
"#;

    let script = webkit6::UserScript::new(
        script_code,
        webkit6::UserContentInjectedFrames::AllFrames,
        webkit6::UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    ucm.add_script(&script);
}
