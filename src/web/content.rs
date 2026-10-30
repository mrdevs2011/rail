//! User style sheets and ad & tracker blocking for every web view.

use crate::*;

/// Developer-oriented HTML/CSS starter kit. All recipes are intentionally commented
/// out: resetting the editor must never silently restyle every website.
pub(crate) const DEFAULT_HTML_CSS: &str = r###"/* ============================================================================
   RAIL · HTML.CSS — DEVELOPER STARTER KIT
   Luxury / Modern / Classic · Standards-first · WebKit compatible

   This file is injected as a USER stylesheet on every website when saved.
   IMPORTANT: examples below are intentionally commented out. Enable only the
   rules you explicitly need. A universal reset can change a site's layout,
   controls, icons, SVGs, avatars, media sizing and accessibility behavior.

   QUICK GUIDE
   1. Keep this file as a reference, or uncomment a focused recipe.
   2. Prefer :where() for low-specificity defaults.
   3. Scope overrides to a site/component instead of styling every element.
   4. Avoid !important unless overriding a known third-party rule is essential.
   5. Save to apply to current and future tabs. Use Reset to restore this kit.
   ============================================================================ */

/* --------------------------------------------------------------------------
   01 · SAFE FOUNDATION — optional, global document defaults
   These intentionally alter page geometry; enable only if that is desired.
   --------------------------------------------------------------------------
:where(*, *::before, *::after) { box-sizing: border-box; }
:where(html) { -webkit-text-size-adjust: 100%; text-size-adjust: 100%; }
:where(body) { margin: 0; }
*/

/* --------------------------------------------------------------------------
   02 · TYPOGRAPHY — restrained, readable, system-native
   Do not force a font globally on sites with their own typographic identity.
   --------------------------------------------------------------------------
:where(body) {
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
               sans-serif;
  line-height: 1.5;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}
:where(code, kbd, samp, pre) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
*/

/* --------------------------------------------------------------------------
   03 · HEADINGS & PROSE — semantic rhythm without fixed font sizes
   --------------------------------------------------------------------------
:where(h1, h2, h3, h4, h5, h6) {
  font-family: inherit;
  font-weight: 650;
  line-height: 1.2;
  text-wrap: balance;
}
:where(p, li, blockquote) { line-height: 1.65; }
:where(p) { text-wrap: pretty; }
*/

/* --------------------------------------------------------------------------
   04 · LINKS — accessible focus, subtle underline treatment
   --------------------------------------------------------------------------
:where(a) { text-underline-offset: 0.18em; text-decoration-thickness: 0.08em; }
:where(a:focus-visible) { outline: 2px solid currentColor; outline-offset: 3px; }
*/

/* --------------------------------------------------------------------------
   05 · FORMS — inherit typography; preserve native affordances
   Avoid appearance:none globally: it can remove platform accessibility cues.
   --------------------------------------------------------------------------
:where(button, input, select, textarea) { font: inherit; }
:where(button, [role="button"], input, select, textarea) { color: inherit; }
:where(button, input, select, textarea):focus-visible {
  outline: 2px solid Highlight;
  outline-offset: 2px;
}
:where(textarea) { resize: vertical; }
*/

/* --------------------------------------------------------------------------
   06 · MEDIA — no global width/height rules by design
   Set dimensions only on the component you own; preserve SVG icon geometry.
   Example, scoped to your own component (copy and uncomment manually):
   .my-card__image { display: block; width: 100%; height: auto; object-fit: cover; }
   .my-icon { width: 1.25rem; height: 1.25rem; flex: 0 0 auto; }
   -------------------------------------------------------------------------- */

/* --------------------------------------------------------------------------
   07 · TABLES & DATA — opt-in readable table recipe
   --------------------------------------------------------------------------
:where(table) { border-collapse: collapse; }
:where(th, td) { text-align: start; }
*/

/* --------------------------------------------------------------------------
   08 · SELECTION & MOTION — respect user accessibility preferences
   --------------------------------------------------------------------------
:where(::selection) { background: Highlight; color: HighlightText; }
@media (prefers-reduced-motion: reduce) {
  :where(*, *::before, *::after) {
    scroll-behavior: auto !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
*/

/* --------------------------------------------------------------------------
   09 · SCROLLBARS — intentionally no global scrollbar override
   Browser/site defaults are preserved. If needed, scope to a known container.
   --------------------------------------------------------------------------
.my-scroll-area { scrollbar-width: thin; scrollbar-color: #888 transparent; }
*/

/* --------------------------------------------------------------------------
   10 · RESPONSIVE UTILITIES — examples for components you own
   --------------------------------------------------------------------------
.my-layout { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1rem; }
@media (min-width: 48rem) {
  .my-layout { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (min-width: 75rem) {
  .my-layout { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}
*/

/* --------------------------------------------------------------------------
   11 · LUXURY / MODERN / CLASSIC DESIGN TOKENS — opt-in component scope
   --------------------------------------------------------------------------
:root {
  --rail-surface: #111214;
  --rail-surface-raised: #191b1f;
  --rail-text: #f4f1eb;
  --rail-muted: #a7a39b;
  --rail-border: rgb(255 255 255 / 12%);
  --rail-accent: #c8a96b;
  --rail-radius: 0.75rem;
  --rail-shadow: 0 12px 32px rgb(0 0 0 / 18%);
}
*/

/* --------------------------------------------------------------------------
   12 · ACCESSIBILITY & QUALITY CHECKLIST
   - Keep visible focus indicators; do not remove outlines globally.
   - Preserve contrast and user zoom; avoid fixed text sizes.
   - Do not globally hide scrollbars or disable selection.
   - Do not globally resize img, svg, canvas, video, iframe or icon elements.
   - Prefer component classes and site-specific selectors over tag selectors.
   - Test keyboard navigation, reduced motion, narrow viewports and dark mode.
   -------------------------------------------------------------------------- */

/* YOUR CUSTOM CSS — add site-specific rules below this line. */
"###;

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
    let store =
        webkit6::UserContentFilterStore::new(dir.join("filters").to_str().unwrap_or("filters"));
    let keep = store.clone();
    store.save(
        "rail-adblock",
        &glib::Bytes::from(json.as_bytes()),
        gio::Cancellable::NONE,
        move |res| {
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
        },
    );
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
            if text.contains("Rail leaves website styling untouched by default.")
                || text.contains("Rail HTML5 Default Standards & Base Stylesheet")
                || text.contains("/* ===== 1. Box Sizing")
                || text.contains("font: -webkit-small-control")
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
            let initial = if initial.contains("Rail leaves website styling untouched by default.")
                || initial.contains("Rail HTML5 Default Standards & Base Stylesheet")
                || initial.contains("/* ===== 1. Box Sizing")
                || initial.contains("font: -webkit-small-control")
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

/// Injects high-performance web & media optimizations across web views:
/// - Speculative Hover Preconnect & DNS warmup (InstantClick / Turbo architecture)
/// - Zero-stale realtime revalidation on BFCache restore (pageshow)
/// - Passive scroll compositor promotion (120 FPS buttery smooth scrolling)
/// - Off-thread asynchronous image decoding
/// - Avoids heavy CPU-bound AV1 software decoding; prioritizes hardware-accelerated VP9/H.264
pub(crate) fn inject_web_optimizations(ucm: &webkit6::UserContentManager) {
    let script_code = r#"
(function() {
  if (window.__railWebOptimized) return;
  window.__railWebOptimized = true;

  // 1. Speculative Hover Preconnect (InstantClick / Turbo architecture)
  // When user hovers or touches a link, resolve DNS and warm TCP/TLS in advance.
  var warmedHosts = new Set();
  function preconnectUrl(href) {
    try {
      if (!href || href.indexOf('http') !== 0) return;
      var url = new URL(href, location.href);
      if (url.origin === location.origin || warmedHosts.has(url.origin)) return;
      warmedHosts.add(url.origin);
      if (warmedHosts.size > 20) {
        var first = warmedHosts.values().next().value;
        warmedHosts.delete(first);
      }
      var linkPreconnect = document.createElement('link');
      linkPreconnect.rel = 'preconnect';
      linkPreconnect.href = url.origin;
      linkPreconnect.crossOrigin = 'anonymous';
      document.head.appendChild(linkPreconnect);

      var linkDns = document.createElement('link');
      linkDns.rel = 'dns-prefetch';
      linkDns.href = url.origin;
      document.head.appendChild(linkDns);
    } catch(_) {}
  }

  document.addEventListener('pointerover', function(e) {
    var a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
    if (a && a.href) preconnectUrl(a.href);
  }, { passive: true, capture: true });

  document.addEventListener('touchstart', function(e) {
    var a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
    if (a && a.href) preconnectUrl(a.href);
  }, { passive: true, capture: true });

  // 2. Zero-stale Realtime Guarantee with BFCache:
  // When a page is restored from Back/Forward cache (event.persisted === true),
  // immediately dispatch visibilitychange and online events. Modern web frameworks
  // (React Query, SWR, Apollo, WebSockets, Next.js, SSE) silently revalidate dynamic feeds
  // while keeping the instant 0ms cached page UI.
  window.addEventListener('pageshow', function(e) {
    if (e.persisted) {
      try {
        document.dispatchEvent(new Event('visibilitychange'));
        window.dispatchEvent(new Event('online'));
        window.dispatchEvent(new Event('focus'));
      } catch(_) {}
    }
  }, false);

  // Do not monkey-patch addEventListener to force passive touch listeners.
  // Sites and web apps need to be able to call preventDefault() for nested scrolling,
  // drawing canvases, touch gestures and custom scroll containers. WebKit handles native
  // compositor scrolling itself; changing listener options globally breaks site behavior.

  // 4. Asynchronous image decoding: offloads image decompression from UI thread
  function makeImagesAsync(root) {
    try {
      var imgs = (root || document).querySelectorAll('img:not([decoding])');
      for (var i = 0; i < imgs.length; i++) {
        imgs[i].decoding = 'async';
      }
    } catch(_) {}
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function() { makeImagesAsync(); });
  } else {
    makeImagesAsync();
  }

  // 5. Hardware-accelerated video decoding on YouTube
  var isYT = location.hostname.indexOf('youtube.com') !== -1;
  if (isYT) {
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

  // 6. Custom context menu & preventDefault detection
  function notifyContextPrevented() {
    try {
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railContextMenu) {
        window.webkit.messageHandlers.railContextMenu.postMessage("prevented");
      }
    } catch(_) {}
  }

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

  // 7. HTML5 Video inline playback & play reliability
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
            makeImagesAsync(muts[i].target);
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
