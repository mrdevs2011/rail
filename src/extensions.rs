//! Local / Unpacked Chrome and WebExtensions support for Rail.
//!
//! Loads unpacked extension directories (containing manifest.json),
//! supports direct .zip and .crx installation, parses content_scripts, styles,
//! and injects them via WebKit UserContentManager.
//! Provides a full-featured Chrome and WebExtensions API polyfill (storage.local,
//! storage.sync, storage.session, runtime.getManifest, runtime.getURL, runtime.sendMessage,
//! runtime.connect, tabs, i18n, action, browserAction, window.browser).

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
    pub all_frames: bool,
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
    pub manifest_raw: String,
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

    let mut name = root.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&fallback_name)
        .to_string();

    let version = root.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0")
        .to_string();

    let mut description = root.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Check _locales if name or description has __MSG_...__
    if name.starts_with("__MSG_") || description.starts_with("__MSG_") {
        let locales_dir = dir.join("_locales");
        for lang in ["en", "en_US", "en_GB"] {
            let msg_file = locales_dir.join(lang).join("messages.json");
            if let Ok(msg_text) = std::fs::read_to_string(&msg_file) {
                if let Some(msg_json) = minijson::parse(&msg_text) {
                    if name.starts_with("__MSG_") {
                        let key = name.trim_start_matches("__MSG_").trim_end_matches("__");
                        if let Some(msg_val) = msg_json.get(key).and_then(|o| o.get("message")).and_then(|m| m.as_str()) {
                            name = msg_val.to_string();
                        }
                    }
                    if description.starts_with("__MSG_") {
                        let key = description.trim_start_matches("__MSG_").trim_end_matches("__");
                        if let Some(msg_val) = msg_json.get(key).and_then(|o| o.get("message")).and_then(|m| m.as_str()) {
                            description = msg_val.to_string();
                        }
                    }
                }
                break;
            }
        }
    }

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

            let all_frames = cs_val.get("all_frames")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !matches.is_empty() && (!js.is_empty() || !css.is_empty()) {
                content_scripts.push(ContentScript {
                    matches,
                    js,
                    css,
                    run_at,
                    all_frames,
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
        manifest_raw: content,
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

/// Unpack and register a .zip or .crx extension archive or unpacked folder.
pub(crate) fn register_unpacked_or_archive(path: &Path) -> Result<Extension, String> {
    if path.is_file() {
        let ext_str = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext_str == "zip" || ext_str == "crx" {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("extension");
            let target_dir = extensions_dir().join(stem);
            let _ = std::fs::create_dir_all(&target_dir);

            // Python zip extractor handles both plain .zip and .crx (finds zip magic bytes)
            let status = std::process::Command::new("python3")
                .arg("-c")
                .arg(r#"
import sys, zipfile, io
p, out = sys.argv[1], sys.argv[2]
with open(p, 'rb') as f:
    data = f.read()
idx = data.find(b'PK\x03\x04')
if idx == -1:
    sys.exit(1)
with zipfile.ZipFile(io.BytesIO(data[idx:])) as z:
    z.extractall(out)
"#)
                .arg(path)
                .arg(&target_dir)
                .status();

            if status.map(|s| s.success()).unwrap_or(false) && target_dir.join("manifest.json").is_file() {
                return register_unpacked_extension(&target_dir);
            } else {
                return Err("Arxivni ochib bo'lmadi yoki ichida manifest.json topilmadi".into());
            }
        }
    }
    register_unpacked_extension(path)
}

/// Comprehensive Chrome & WebExtensions runtime, storage, tabs, and i18n polyfill.
fn polyfill_js(ext: &Extension) -> String {
    let base_url = format!("file://{}", ext.path.display());
    let manifest_clean = if ext.manifest_raw.trim().is_empty() {
        "{}".to_string()
    } else {
        ext.manifest_raw.trim().to_string()
    };

    format!(
        r#"(function() {{
  if (window.__rail_ext_{ext_id}) return;
  window.__rail_ext_{ext_id} = true;
  var extId = "{ext_id}";
  var extBaseUrl = "{base_url}";
  var manifest = {manifest_json};

  window.chrome = window.chrome || {{}};
  window.browser = window.browser || window.chrome;

  var storageListeners = [];

  function makeStorageArea(areaName, persistence) {{
    return {{
      get: function(keys, cb) {{
        return new Promise(function(resolve) {{
          var res = {{}};
          try {{
            var getVal = function(k) {{
              if (persistence === 'memory') return window['__rail_mem_' + extId + '_' + k];
              var raw = localStorage.getItem('__rail_ext_' + extId + '_' + k);
              return raw !== null ? JSON.parse(raw) : undefined;
            }};
            if (keys === null || keys === undefined) {{
              var prefix = '__rail_ext_' + extId + '_';
              for (var i = 0; i < localStorage.length; i++) {{
                var k = localStorage.key(i);
                if (k && k.indexOf(prefix) === 0) {{
                  var realKey = k.slice(prefix.length);
                  res[realKey] = JSON.parse(localStorage.getItem(k));
                }}
              }}
            }} else if (typeof keys === 'string') {{
              var val = getVal(keys);
              if (val !== undefined) res[keys] = val;
            }} else if (Array.isArray(keys)) {{
              keys.forEach(function(k) {{
                var val = getVal(k);
                if (val !== undefined) res[k] = val;
              }});
            }} else if (typeof keys === 'object') {{
              Object.keys(keys).forEach(function(k) {{
                var val = getVal(k);
                res[k] = val !== undefined ? val : keys[k];
              }});
            }}
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(function() {{ cb(res); }}, 0);
          resolve(res);
        }});
      }},
      set: function(items, cb) {{
        return new Promise(function(resolve) {{
          var changes = {{}};
          try {{
            Object.keys(items || {{}}).forEach(function(k) {{
              var oldVal = persistence === 'memory' ? window['__rail_mem_' + extId + '_' + k] : (function() {{
                var r = localStorage.getItem('__rail_ext_' + extId + '_' + k);
                return r !== null ? JSON.parse(r) : undefined;
              }})();
              var newVal = items[k];
              if (persistence === 'memory') {{
                window['__rail_mem_' + extId + '_' + k] = newVal;
              }} else {{
                localStorage.setItem('__rail_ext_' + extId + '_' + k, JSON.stringify(newVal));
              }}
              changes[k] = {{ oldValue: oldVal, newValue: newVal }};
            }});
            storageListeners.forEach(function(fn) {{
              try {{ fn(changes, areaName); }} catch(e) {{}}
            }});
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          resolve();
        }});
      }},
      remove: function(keys, cb) {{
        return new Promise(function(resolve) {{
          try {{
            var arr = typeof keys === 'string' ? [keys] : (Array.isArray(keys) ? keys : []);
            var changes = {{}};
            arr.forEach(function(k) {{
              if (persistence === 'memory') {{
                delete window['__rail_mem_' + extId + '_' + k];
              }} else {{
                localStorage.removeItem('__rail_ext_' + extId + '_' + k);
              }}
              changes[k] = {{ oldValue: undefined, newValue: undefined }};
            }});
            storageListeners.forEach(function(fn) {{
              try {{ fn(changes, areaName); }} catch(e) {{}}
            }});
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          resolve();
        }});
      }},
      clear: function(cb) {{
        return new Promise(function(resolve) {{
          try {{
            var prefix = '__rail_ext_' + extId + '_';
            for (var i = localStorage.length - 1; i >= 0; i--) {{
              var k = localStorage.key(i);
              if (k && k.indexOf(prefix) === 0) {{
                localStorage.removeItem(k);
              }}
            }}
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          resolve();
        }});
      }},
      getBytesInUse: function(keys, cb) {{
        var size = 1024;
        if (typeof cb === 'function') setTimeout(function() {{ cb(size); }}, 0);
        return Promise.resolve(size);
      }}
    }};
  }}

  if (!window.chrome.storage) {{
    var localStorageArea = makeStorageArea('local', 'local');
    var syncStorageArea = makeStorageArea('sync', 'local');
    var sessionStorageArea = makeStorageArea('session', 'memory');
    window.chrome.storage = {{
      local: localStorageArea,
      sync: syncStorageArea,
      session: sessionStorageArea,
      managed: localStorageArea,
      onChanged: {{
        addListener: function(fn) {{ if (typeof fn === 'function' && storageListeners.indexOf(fn) === -1) storageListeners.push(fn); }},
        removeListener: function(fn) {{ var idx = storageListeners.indexOf(fn); if (idx !== -1) storageListeners.splice(idx, 1); }},
        hasListener: function(fn) {{ return storageListeners.indexOf(fn) !== -1; }}
      }}
    }};
  }}

  var msgListeners = [];
  var connectListeners = [];

  if (!window.chrome.runtime) {{
    window.chrome.runtime = {{
      id: extId,
      lastError: null,
      getURL: function(path) {{
        if (!path) return extBaseUrl;
        if (path.charAt(0) === '/') path = path.slice(1);
        return extBaseUrl + '/' + path;
      }},
      getManifest: function() {{
        return manifest;
      }},
      getPlatformInfo: function(cb) {{
        var info = {{ os: 'linux', arch: 'x86-64', nacl_arch: 'x86-64' }};
        if (typeof cb === 'function') setTimeout(function() {{ cb(info); }}, 0);
        return Promise.resolve(info);
      }},
      sendMessage: function(msg, cb) {{
        return new Promise(function(resolve) {{
          var handled = false;
          msgListeners.forEach(function(fn) {{
            try {{
              fn(msg, {{ id: extId, url: location.href }}, function(res) {{
                handled = true;
                if (typeof cb === 'function') cb(res);
                resolve(res);
              }});
            }} catch(e) {{}}
          }});
          if (!handled) {{
            var defaultRes = {{ status: 'ok' }};
            if (typeof cb === 'function') setTimeout(function() {{ cb(defaultRes); }}, 0);
            resolve(defaultRes);
          }}
        }});
      }},
      onMessage: {{
        addListener: function(fn) {{ if (typeof fn === 'function' && msgListeners.indexOf(fn) === -1) msgListeners.push(fn); }},
        removeListener: function(fn) {{ var idx = msgListeners.indexOf(fn); if (idx !== -1) msgListeners.splice(idx, 1); }},
        hasListener: function(fn) {{ return msgListeners.indexOf(fn) !== -1; }}
      }},
      connect: function(connectInfo) {{
        var name = (connectInfo && connectInfo.name) || '';
        var portListeners = [];
        var portDisconnectListeners = [];
        var port = {{
          name: name,
          postMessage: function(m) {{
            portListeners.forEach(function(l) {{ try {{ l(m); }} catch(e) {{}} }});
          }},
          disconnect: function() {{
            portDisconnectListeners.forEach(function(l) {{ try {{ l(); }} catch(e) {{}} }});
          }},
          onMessage: {{
            addListener: function(fn) {{ portListeners.push(fn); }},
            removeListener: function(fn) {{ var i = portListeners.indexOf(fn); if (i !== -1) portListeners.splice(i, 1); }}
          }},
          onDisconnect: {{
            addListener: function(fn) {{ portDisconnectListeners.push(fn); }},
            removeListener: function(fn) {{ var i = portDisconnectListeners.indexOf(fn); if (i !== -1) portDisconnectListeners.splice(i, 1); }}
          }}
        }};
        connectListeners.forEach(function(l) {{ try {{ l(port); }} catch(e) {{}} }});
        return port;
      }},
      onConnect: {{
        addListener: function(fn) {{ if (typeof fn === 'function') connectListeners.push(fn); }},
        removeListener: function(fn) {{ var i = connectListeners.indexOf(fn); if (i !== -1) connectListeners.splice(i, 1); }}
      }},
      onInstalled: {{
        addListener: function() {{}},
        removeListener: function() {{}}
      }}
    }};
  }}

  if (!window.chrome.extension) {{
    window.chrome.extension = {{
      getURL: window.chrome.runtime.getURL,
      inIncognitoContext: false
    }};
  }}

  if (!window.chrome.i18n) {{
    window.chrome.i18n = {{
      getMessage: function(msgKey, substitutions) {{
        if (!msgKey) return '';
        return String(msgKey);
      }},
      getUILanguage: function() {{
        return navigator.language || 'en';
      }},
      detectLanguage: function(text, cb) {{
        var res = {{ isReliable: true, languages: [{{ language: 'en', percentage: 100 }}] }};
        if (typeof cb === 'function') setTimeout(function() {{ cb(res); }}, 0);
        return Promise.resolve(res);
      }}
    }};
  }}

  if (!window.chrome.tabs) {{
    window.chrome.tabs = {{
      query: function(queryInfo, cb) {{
        var tab = [{{ id: 1, active: true, currentWindow: true, url: location.href, title: document.title }}];
        if (typeof cb === 'function') setTimeout(function() {{ cb(tab); }}, 0);
        return Promise.resolve(tab);
      }},
      sendMessage: function(tabId, msg, cb) {{
        return window.chrome.runtime.sendMessage(msg, cb);
      }},
      create: function(props, cb) {{
        if (props && props.url) window.open(props.url, '_blank');
        if (typeof cb === 'function') setTimeout(cb, 0);
        return Promise.resolve();
      }}
    }};
  }}

  if (!window.chrome.action) {{
    window.chrome.action = {{
      setIcon: function(_, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      setTitle: function(_, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      setBadgeText: function(_, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      setBadgeBackgroundColor: function(_, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      onClicked: {{ addListener: function() {{}}, removeListener: function() {{}} }}
    }};
  }}
  if (!window.chrome.browserAction) {{
    window.chrome.browserAction = window.chrome.action;
  }}
}})();
"#,
        ext_id = ext.id,
        base_url = base_url,
        manifest_json = manifest_clean
    )
}

/// Apply all enabled extensions' content scripts and stylesheets to a UserContentManager.
pub(crate) fn apply_extensions_to_ucm(ucm: &webkit6::UserContentManager) {
    let list = get_all_extensions();
    for ext in list.iter().filter(|e| e.enabled) {
        let polyfill = polyfill_js(ext);

        for cs in &ext.content_scripts {
            let matches_str = format!(
                "[{}]",
                cs.matches
                    .iter()
                    .map(|s| format!("\"{}\"", minijson::escape(s)))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            let frames = if cs.all_frames {
                webkit6::UserContentInjectedFrames::AllFrames
            } else {
                webkit6::UserContentInjectedFrames::TopFrame
            };

            // Inject JavaScript
            for js_file in &cs.js {
                let p = ext.path.join(js_file);
                if let Ok(code) = std::fs::read_to_string(&p) {
                    let wrapped_code = format!(
                        r#"(function() {{
  var patterns = {matches_str};
  function urlMatches(url, patterns) {{
    if (!patterns || !patterns.length) return true;
    for (var i = 0; i < patterns.length; i++) {{
      var p = patterns[i];
      if (p === '<all_urls>' || p === '*://*/*' || p === '*://*') return true;
      var glob = p.replace(/[.+^${{}}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*');
      try {{
        if (new RegExp('^' + glob + '$').test(url)) return true;
      }} catch(e) {{}}
    }}
    return false;
  }}
  if (!urlMatches(location.href, patterns)) return;

{polyfill}

{code}
}})();"#,
                        matches_str = matches_str,
                        polyfill = polyfill,
                        code = code
                    );

                    let time = if cs.run_at == "document_start" {
                        webkit6::UserScriptInjectionTime::Start
                    } else {
                        webkit6::UserScriptInjectionTime::End
                    };

                    let script = webkit6::UserScript::new(
                        &wrapped_code,
                        frames,
                        time,
                        &[],
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
                        frames,
                        webkit6::UserStyleLevel::User,
                        &[],
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
