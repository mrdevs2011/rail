//! Local / Unpacked Chrome and WebExtensions support for Rail.
//!
//! Loads unpacked extension directories (containing manifest.json),
//! parses content_scripts, styles, and injects them via WebKit UserContentManager.
//! Provides a lightweight Chrome/WebExtensions runtime & storage polyfill.

use crate::minijson::{self, Json};
use crate::{load_html_css, user_style_sheet, STYLE_MANAGERS};
use crate::rail_data_dir;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Clone, Debug)]
pub(crate) struct ContentScript {
    pub matches: Vec<String>,
    pub js: Vec<String>,
    pub css: Vec<String>,
    pub run_at: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub content_scripts: Vec<ContentScript>,
}

static EXTENSIONS: RwLock<Vec<Extension>> = RwLock::new(Vec::new());

pub(crate) fn extensions_dir() -> PathBuf {
    rail_data_dir().join("extensions")
}

pub(crate) fn extensions_json_path() -> PathBuf {
    rail_data_dir().join("extensions.json")
}

/// Parse an unpacked extension directory with manifest.json.
pub(crate) fn parse_manifest(dir: &Path) -> Result<Extension, String> {
    let manifest_file = dir.join("manifest.json");
    if !manifest_file.is_file() {
        return Err(format!("manifest.json topilmadi: {}", dir.display()));
    }
    let content = std::fs::read_to_string(&manifest_file)
        .map_err(|e| format!("manifest.json o'qib bo'lmadi: {e}"))?;

    let root = minijson::parse(&content)
        .ok_or_else(|| "manifest.json formati noto'g'ri (JSON parsing xatosi)".to_string())?;

    let fallback_name = dir.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Nomsiz kengaytma".to_string());

    let name = root.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&fallback_name)
        .to_string();

    let version = root.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0")
        .to_string();

    let description = root.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut content_scripts = Vec::new();
    if let Some(cs_arr) = root.get("content_scripts").and_then(|v| v.as_arr()) {
        for cs_val in cs_arr {
            let mut matches = Vec::new();
            if let Some(m_arr) = cs_val.get("matches").and_then(|v| v.as_arr()) {
                for m in m_arr {
                    if let Some(s) = m.as_str() {
                        matches.push(s.to_string());
                    }
                }
            }

            let mut js = Vec::new();
            if let Some(js_arr) = cs_val.get("js").and_then(|v| v.as_arr()) {
                for j in js_arr {
                    if let Some(s) = j.as_str() {
                        js.push(s.to_string());
                    }
                }
            }

            let mut css = Vec::new();
            if let Some(css_arr) = cs_val.get("css").and_then(|v| v.as_arr()) {
                for c in css_arr {
                    if let Some(s) = c.as_str() {
                        css.push(s.to_string());
                    }
                }
            }

            let run_at = cs_val.get("run_at")
                .and_then(|v| v.as_str())
                .unwrap_or("document_idle")
                .to_string();

            if !matches.is_empty() && (!js.is_empty() || !css.is_empty()) {
                content_scripts.push(ContentScript {
                    matches,
                    js,
                    css,
                    run_at,
                });
            }
        }
    }

    let id = {
        let clean: String = name.chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect();
        let trimmed = clean.trim_matches('-');
        if trimmed.is_empty() {
            format!("ext-{}", dir.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "1".into()))
        } else {
            trimmed.to_string()
        }
    };

    Ok(Extension {
        id,
        name,
        version,
        description,
        path: dir.to_path_buf(),
        enabled: true,
        content_scripts,
    })
}

/// Load registered extensions from extensions.json, plus scan extensions_dir().
pub(crate) fn init_extensions() {
    let mut list = Vec::new();

    // 1) Read extensions.json
    if let Ok(text) = std::fs::read_to_string(extensions_json_path()) {
        if let Some(Json::Arr(items)) = minijson::parse(&text) {
            for item in items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let path_str = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let enabled = item.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let p = PathBuf::from(path_str);
                if p.is_dir() {
                    if let Ok(mut ext) = parse_manifest(&p) {
                        if !id.is_empty() {
                            ext.id = id;
                        }
                        ext.enabled = enabled;
                        list.push(ext);
                    }
                }
            }
        }
    }

    // 2) Auto-discover folders directly in extensions_dir()
    let edir = extensions_dir();
    if let Ok(entries) = std::fs::read_dir(&edir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && p.join("manifest.json").is_file() {
                if !list.iter().any(|e| e.path == p) {
                    if let Ok(ext) = parse_manifest(&p) {
                        list.push(ext);
                    }
                }
            }
        }
    }

    if let Ok(mut lock) = EXTENSIONS.write() {
        *lock = list;
    }
}

pub(crate) fn get_all_extensions() -> Vec<Extension> {
    EXTENSIONS.read().map(|l| l.clone()).unwrap_or_default()
}

pub(crate) fn save_extensions() {
    let list = get_all_extensions();
    let mut out = String::from("[\n");
    for (i, ext) in list.iter().enumerate() {
        out.push_str("  {\n");
        out.push_str(&format!("    \"id\": \"{}\",\n", minijson::escape(&ext.id)));
        out.push_str(&format!("    \"name\": \"{}\",\n", minijson::escape(&ext.name)));
        out.push_str(&format!("    \"version\": \"{}\",\n", minijson::escape(&ext.version)));
        out.push_str(&format!("    \"description\": \"{}\",\n", minijson::escape(&ext.description)));
        out.push_str(&format!("    \"path\": \"{}\",\n", minijson::escape(&ext.path.to_string_lossy())));
        out.push_str(&format!("    \"enabled\": {}\n", ext.enabled));
        if i + 1 < list.len() {
            out.push_str("  },\n");
        } else {
            out.push_str("  }\n");
        }
    }
    out.push_str("]\n");

    let p = extensions_json_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(p, out);
}

pub(crate) fn toggle_extension(id: &str, enabled: bool) {
    if let Ok(mut lock) = EXTENSIONS.write() {
        if let Some(item) = lock.iter_mut().find(|e| e.id == id) {
            item.enabled = enabled;
        }
    }
    save_extensions();
    apply_extensions_all();
}

pub(crate) fn remove_extension(id: &str) {
    if let Ok(mut lock) = EXTENSIONS.write() {
        lock.retain(|e| e.id != id);
    }
    save_extensions();
    apply_extensions_all();
}

pub(crate) fn register_unpacked_extension(folder: &Path) -> Result<Extension, String> {
    let ext = parse_manifest(folder)?;
    if let Ok(mut lock) = EXTENSIONS.write() {
        lock.retain(|e| e.id != ext.id && e.path != ext.path);
        lock.push(ext.clone());
    }
    save_extensions();
    apply_extensions_all();
    Ok(ext)
}

fn polyfill_js(ext_id: &str) -> String {
    format!(
        r#"(function() {{
  if (window.__rail_ext_{ext_id}) return;
  window.__rail_ext_{ext_id} = true;
  var extId = "{ext_id}";
  window.chrome = window.chrome || {{}};
  window.browser = window.browser || window.chrome;
  if (!window.chrome.runtime) {{
    window.chrome.runtime = {{
      id: extId,
      getURL: function(path) {{
        if (path && path.charAt(0) === '/') path = path.slice(1);
        return 'rail-extension://' + extId + '/' + (path || '');
      }},
      sendMessage: function(msg, cb) {{
        if (typeof cb === 'function') setTimeout(function() {{ cb({{ status: 'ok' }}); }}, 0);
        return Promise.resolve({{ status: 'ok' }});
      }},
      onMessage: {{
        addListener: function() {{}},
        removeListener: function() {{}}
      }}
    }};
  }}
  if (!window.chrome.storage) {{
    window.chrome.storage = {{
      local: {{
        get: function(keys, cb) {{
          var res = {{}};
          try {{
            if (typeof keys === 'string') {{
              var v = localStorage.getItem('__rail_ext_' + extId + '_' + keys);
              res[keys] = v !== null ? JSON.parse(v) : null;
            }} else if (Array.isArray(keys)) {{
              keys.forEach(function(k) {{
                var v = localStorage.getItem('__rail_ext_' + extId + '_' + k);
                res[k] = v !== null ? JSON.parse(v) : null;
              }});
            }} else if (keys && typeof keys === 'object') {{
              Object.keys(keys).forEach(function(k) {{
                var v = localStorage.getItem('__rail_ext_' + extId + '_' + k);
                res[k] = v !== null ? JSON.parse(v) : keys[k];
              }});
            }}
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(function() {{ cb(res); }}, 0);
          return Promise.resolve(res);
        }},
        set: function(items, cb) {{
          try {{
            Object.keys(items || {{}}).forEach(function(k) {{
              localStorage.setItem('__rail_ext_' + extId + '_' + k, JSON.stringify(items[k]));
            }});
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          return Promise.resolve();
        }},
        remove: function(keys, cb) {{
          try {{
            var arr = typeof keys === 'string' ? [keys] : (Array.isArray(keys) ? keys : []);
            arr.forEach(function(k) {{
              localStorage.removeItem('__rail_ext_' + extId + '_' + k);
            }});
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          return Promise.resolve();
        }},
        clear: function(cb) {{
          try {{
            for (var i = localStorage.length - 1; i >= 0; i--) {{
              var k = localStorage.key(i);
              if (k && k.indexOf('__rail_ext_' + extId + '_') === 0) {{
                localStorage.removeItem(k);
              }}
            }}
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          return Promise.resolve();
        }}
      }}
    }};
  }}
}})();
"#,
        ext_id = ext_id
    )
}

/// Apply all enabled extensions' content scripts and stylesheets to a UserContentManager.
pub(crate) fn apply_extensions_to_ucm(ucm: &webkit6::UserContentManager) {
    let list = get_all_extensions();
    for ext in list.iter().filter(|e| e.enabled) {
        let polyfill = polyfill_js(&ext.id);

        for cs in &ext.content_scripts {
            let mut allow_list_vec: Vec<String> = Vec::new();
            let mut match_all = false;
            for m in &cs.matches {
                if m == "<all_urls>" || m == "*://*/*" || m == "*://*" {
                    match_all = true;
                } else {
                    allow_list_vec.push(m.clone());
                }
            }
            let allow_refs: Vec<&str> = allow_list_vec.iter().map(|s| s.as_str()).collect();
            let allow_slice: &[&str] = if match_all { &[] } else { &allow_refs };

            // Inject JavaScript
            for js_file in &cs.js {
                let p = ext.path.join(js_file);
                if let Ok(code) = std::fs::read_to_string(&p) {
                    let full_code = format!("{}\n{}", polyfill, code);
                    let time = if cs.run_at == "document_start" {
                        webkit6::UserScriptInjectionTime::Start
                    } else {
                        webkit6::UserScriptInjectionTime::End
                    };
                    let script = webkit6::UserScript::new(
                        &full_code,
                        webkit6::UserContentInjectedFrames::AllFrames,
                        time,
                        allow_slice,
                        &[],
                    );
                    ucm.add_script(&script);
                }
            }

            // Inject CSS
            for css_file in &cs.css {
                let p = ext.path.join(css_file);
                if let Ok(css_code) = std::fs::read_to_string(&p) {
                    let sheet = webkit6::UserStyleSheet::new(
                        &css_code,
                        webkit6::UserContentInjectedFrames::AllFrames,
                        webkit6::UserStyleLevel::User,
                        allow_slice,
                        &[],
                    );
                    ucm.add_style_sheet(&sheet);
                }
            }
        }
    }
}

/// Re-apply all user scripts, user styles and extensions to all open WebViews.
pub(crate) fn apply_extensions_all() {
    let html_css = load_html_css();
    STYLE_MANAGERS.with(|list| {
        let mut list = list.borrow_mut();
        list.retain(|w| w.upgrade().is_some());
        for w in list.iter() {
            if let Some(ucm) = w.upgrade() {
                ucm.remove_all_scripts();
                ucm.remove_all_style_sheets();
                if !html_css.trim().is_empty() {
                    ucm.add_style_sheet(&user_style_sheet(&html_css));
                }
                apply_extensions_to_ucm(&ucm);
            }
        }
    });
}
