//! User style sheets and ad & tracker blocking for every web view.

use crate::*;

/// Developer-oriented HTML/CSS starter kit. All recipes are intentionally commented
/// out: resetting the editor must never silently restyle every website.
pub(crate) const DEFAULT_HTML_CSS: &str = r###"/* ============================================================================
   RAIL · HTML.CSS — DEVELOPER STARTER KIT
   Luxury / Modern / Classic · Standards-first · WebKit compatible

   This file is injected at AUTHOR cascade level when saved; the site can keep its own design.
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

/// Identifies the previous Rail-wide reset that could override site-authored CSS.
fn is_legacy_global_reset(css: &str) -> bool {
    css.contains("Rail HTML5 Default Standards & Base Stylesheet")
        || css.contains("/* ===== 1. Box Sizing")
        || css.contains("Rail leaves website styling untouched by default.")
        || css.contains("font: -webkit-small-control")
        || css.contains("/* ===== form controls ===== */")
        || css.contains(
            "Sites that set their own CSS still win; this layer only replaces the defaults.",
        )
}

/// Preserve the old file before replacing it. Write-and-rename prevents a partial template
/// if the process exits during migration. Existing backup is never overwritten.
fn migrated_css_from_legacy(old_css: &str) -> String {
    // Keep intentional user rules written after the old template's explicit custom area.
    // The old built-in reset itself is discarded; the original full file is backed up.
    const CUSTOM_MARKER: &str = "/* ===== 7. User Custom Overrides ===== */";
    let custom = old_css
        .find(CUSTOM_MARKER)
        .map(|at| &old_css[at + CUSTOM_MARKER.len()..])
        .unwrap_or("");
    if css_has_active_content(custom) {
        format!(
            "{}\n/* Preserved user rules from the previous Rail html.css. */\n{}",
            DEFAULT_HTML_CSS, custom
        )
    } else {
        DEFAULT_HTML_CSS.to_string()
    }
}

fn migrate_legacy_css(path: &Path, old_css: &str) -> Option<String> {
    let dir = path.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let backup = dir.join("html.css.legacy-backup");
    if !backup.exists() {
        std::fs::write(&backup, old_css).ok()?;
    }
    let migrated = migrated_css_from_legacy(old_css);
    let temp = dir.join("html.css.migrating");
    if std::fs::write(&temp, &migrated).is_err() {
        return None;
    }
    if std::fs::rename(&temp, path).is_err() {
        let _ = std::fs::remove_file(&temp);
        return None;
    }
    // Remove the stale sheet from every already-open tab in this process immediately.
    apply_html_css(&migrated);
    Some(migrated)
}

/// Reads html.css; creates it from the built-in developer template the first time.
pub(crate) fn load_html_css() -> String {
    let path = html_css_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            if is_legacy_global_reset(&text) {
                if let Some(migrated) = migrate_legacy_css(&path, &text) {
                    return migrated;
                }
                // Fail safe: do not apply the known-bad old global reset if migration fails.
                return String::new();
            }
            text
        }
        Err(_) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            // First run after the move: carry over a user file from the previous data root.
            let legacy = rail_data_dir().join("html.css");
            let old = std::fs::read_to_string(&legacy).ok();
            let initial = match old.as_deref() {
                Some(css) if is_legacy_global_reset(css) => {
                    let Some(migrated) = migrate_legacy_css(&path, css) else {
                        return String::new();
                    };
                    let _ = std::fs::remove_file(&legacy);
                    return migrated;
                }
                Some(css) => css.to_string(),
                None => DEFAULT_HTML_CSS.to_string(),
            };
            let temp = path.with_file_name("html.css.migrating");
            if std::fs::write(&temp, &initial).is_ok() {
                let _ = std::fs::rename(&temp, &path);
            }
            if legacy.exists() && old.is_some() {
                let _ = std::fs::remove_file(&legacy);
            }
            initial
        }
    }
}

/// Returns true only when the stylesheet contains tokens outside CSS comments.
/// The built-in developer kit is a reference-only comment document and must never
/// be injected into a page as an active stylesheet.
pub(crate) fn css_has_active_content(css: &str) -> bool {
    let bytes = css.as_bytes();
    let mut i = 0;
    let mut in_comment = false;
    while i < bytes.len() {
        if !in_comment && bytes[i..].starts_with(b"/*") {
            in_comment = true;
            i += 2;
        } else if in_comment && bytes[i..].starts_with(b"*/") {
            in_comment = false;
            i += 2;
        } else {
            if !in_comment && !bytes[i].is_ascii_whitespace() {
                return true;
            }
            i += 1;
        }
    }
    false
}

pub(crate) fn user_style_sheet(css: &str) -> webkit6::UserStyleSheet {
    webkit6::UserStyleSheet::new(
        css,
        webkit6::UserContentInjectedFrames::AllFrames,
        // Site-authored rules win normal cascade conflicts; custom CSS cannot silently
        // resize/replace a site's icons, avatars, controls or layout.
        webkit6::UserStyleLevel::Author,
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
                if css_has_active_content(css) {
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

/// A tiny observational hook so Rail can respect a site's custom context menu.
/// It never changes page DOM, media capabilities, listeners, or default actions.
const PAGE_COMPATIBILITY_SCRIPT: &str = r#"
(function() {
  if (window.__railContextMenuObserver) return;
  window.__railContextMenuObserver = true;

  function notifyContextPrevented() {
    try {
      var handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railContextMenu;
      if (handler) handler.postMessage("prevented");
    } catch (_) {}
  }

  // Wrap only this one event instance so notification is synchronous. The original
  // method is always called with the same receiver/arguments; no global prototype,
  // event propagation, or page listener registration is changed.
  window.addEventListener('contextmenu', function(event) {
    var originalPreventDefault = event.preventDefault;
    event.preventDefault = function() {
      notifyContextPrevented();
      return originalPreventDefault.apply(this, arguments);
    };
  }, true);
})();
"#;

pub(crate) fn inject_context_menu_bridge(ucm: &webkit6::UserContentManager) {
    let script = webkit6::UserScript::new(
        PAGE_COMPATIBILITY_SCRIPT,
        webkit6::UserContentInjectedFrames::AllFrames,
        webkit6::UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    ucm.add_script(&script);
}

#[cfg(test)]
mod stylesheet_safety_tests {
    use super::*;

    #[test]
    fn injected_page_hook_only_observes_context_menu_cancellation() {
        assert!(PAGE_COMPATIBILITY_SCRIPT.contains("event.preventDefault = function()"));
        assert!(
            PAGE_COMPATIBILITY_SCRIPT.contains("originalPreventDefault.apply(this, arguments)")
        );
        assert!(PAGE_COMPATIBILITY_SCRIPT.contains("railContextMenu"));
        for forbidden in [
            "Event.prototype.preventDefault =",
            "EventTarget.prototype.addEventListener =",
            "addEventListener =",
            "MediaSource.isTypeSupported",
            "playsinline",
            "decoding =",
            "dispatchEvent(",
            "addEventListener('drop'",
            "preconnect",
        ] {
            assert!(
                !PAGE_COMPATIBILITY_SCRIPT.contains(forbidden),
                "page compatibility hook must not mutate site behavior: {forbidden}"
            );
        }
    }

    #[test]
    fn legacy_global_reset_is_detected_but_developer_template_is_not() {
        let legacy = "/* Rail HTML5 Default Standards & Base Stylesheet */\n:where(svg) { max-width: 100%; height: auto; }";
        assert!(is_legacy_global_reset(legacy));
        assert!(!is_legacy_global_reset(DEFAULT_HTML_CSS));
        assert!(!is_legacy_global_reset(
            ".my-site-card { border-radius: 12px; }"
        ));
    }

    #[test]
    fn built_in_developer_template_has_no_active_css_rules() {
        assert!(!css_has_active_content(DEFAULT_HTML_CSS));
        assert!(css_has_active_content(
            ".custom-card { border-radius: 12px; }"
        ));
        // Tiny CSS-comment lexer: every default recipe must remain inside a valid comment.
        // This prevents accidental global rules from leaking out of a nested/malformed comment.
        let bytes = DEFAULT_HTML_CSS.as_bytes();
        let mut index = 0;
        let mut in_comment = false;
        while index < bytes.len() {
            if !in_comment && bytes[index..].starts_with(b"/*") {
                in_comment = true;
                index += 2;
            } else if in_comment && bytes[index..].starts_with(b"*/") {
                in_comment = false;
                index += 2;
            } else {
                if !in_comment {
                    assert!(
                        bytes[index].is_ascii_whitespace(),
                        "active CSS outside comments at byte {index}"
                    );
                }
                index += 1;
            }
        }
        assert!(!in_comment, "unclosed comment in developer CSS template");
    }

    #[test]
    fn legacy_css_migration_preserves_backup_and_replaces_the_active_file() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "rail-css-migration-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("html.css");
        let old_css = "/* Rail HTML5 Default Standards & Base Stylesheet */\nsvg { height: auto; }\n/* ===== 7. User Custom Overrides ===== */\n.my-panel { border-radius: 12px; }";
        std::fs::write(&path, old_css).unwrap();

        let migrated = migrate_legacy_css(&path, old_css).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), migrated);
        assert!(migrated.contains(".my-panel { border-radius: 12px; }"));
        assert_eq!(
            std::fs::read_to_string(dir.join("html.css.legacy-backup")).unwrap(),
            old_css
        );

        std::fs::remove_dir_all(dir).unwrap();
    }
}
