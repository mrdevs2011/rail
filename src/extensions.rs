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
pub(crate) struct ExtensionCommand {
    pub name: String,
    pub suggested_key: String,
    pub description: String,
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
    pub default_popup: Option<String>,
    pub default_icon: Option<String>,
    pub background_script: Option<String>,
    pub commands: Vec<ExtensionCommand>,
    pub manifest_raw: String,
}

static EXTENSIONS: RwLock<Vec<Extension>> = RwLock::new(Vec::new());

pub(crate) fn ext_data_dir() -> PathBuf {
    rail_data_dir().join("ext_data")
}

pub(crate) fn read_ext_storage(ext_id: &str, area: &str) -> String {
    let p = ext_data_dir().join(format!("{}_{}.json", ext_id, area));
    let s = std::fs::read_to_string(p).unwrap_or_else(|_| "{}".to_string());
    if s.trim().is_empty() {
        "{}".to_string()
    } else {
        s
    }
}

pub(crate) fn write_ext_storage(ext_id: &str, area: &str, data: &str) {
    let dir = ext_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(format!("{}_{}.json", ext_id, area));
    let _ = std::fs::write(p, data);
}

pub(crate) fn extensions_dir() -> PathBuf {
    rail_data_dir().join("extensions")
}

pub(crate) fn extensions_json_path() -> PathBuf {
    rail_data_dir().join("extensions.json")
}

/// Create an icon widget for an extension: uses real default_icon/disk icon, or fallback to symbolic
pub(crate) fn extension_icon_widget(ext: &Extension, size: i32) -> gtk4::Image {
    if let Some(rel) = &ext.default_icon {
        let p = ext.path.join(rel);
        if p.is_file() {
            let img = gtk4::Image::from_file(&p);
            img.set_pixel_size(size);
            return img;
        }
    }
    let candidates = [
        "icon128.png", "icon48.png", "icon32.png", "icon16.png",
        "icons/128.png", "icons/48.png", "icons/32.png", "icons/16.png",
        "icons/icon128.png", "icons/icon48.png", "icons/icon32.png", "icons/icon16.png",
        "icon.png", "assets/icon.png", "images/icon.png",
    ];
    for cand in candidates {
        let p = ext.path.join(cand);
        if p.is_file() {
            let img = gtk4::Image::from_file(&p);
            img.set_pixel_size(size);
            return img;
        }
    }
    let img = gtk4::Image::from_icon_name("application-x-addon-symbolic");
    img.set_pixel_size(size);
    img
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

    let action_obj = root.get("action").or_else(|| root.get("browser_action"));
    let default_popup = action_obj
        .and_then(|a| a.get("default_popup"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let default_icon = action_obj
        .and_then(|a| a.get("default_icon"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else {
                v.get("128")
                    .or_else(|| v.get("48"))
                    .or_else(|| v.get("32"))
                    .or_else(|| v.get("16"))
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string())
            }
        })
        .or_else(|| {
            root.get("icons").and_then(|v| {
                v.get("128")
                    .or_else(|| v.get("48"))
                    .or_else(|| v.get("32"))
                    .or_else(|| v.get("16"))
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string())
            })
        });

    let background_script = root.get("background").and_then(|b| {
        b.get("service_worker")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                b.get("scripts")
                    .and_then(|v| v.as_arr())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    });

    let mut commands = Vec::new();
    if let Some(cmds) = root.get("commands").and_then(|v| v.as_obj()) {
        for (cmd_name, cmd_val) in cmds {
            let desc = cmd_val.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let key = cmd_val.get("suggested_key").and_then(|k| {
                if let Some(s) = k.as_str() {
                    Some(s.to_string())
                } else {
                    k.get("linux")
                        .or_else(|| k.get("default"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }
            }).unwrap_or_default();

            if !key.is_empty() {
                commands.push(ExtensionCommand {
                    name: cmd_name.to_string(),
                    suggested_key: key,
                    description: desc,
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
        default_popup,
        default_icon,
        background_script,
        commands,
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

/// Download, extract and register an extension from Chrome Web Store by its 32-char ID.
pub(crate) fn install_from_webstore(ext_id: &str) -> Result<Extension, String> {
    let clean_id = ext_id.trim();
    if clean_id.len() != 32 || !clean_id.chars().all(|c| c.is_ascii_lowercase()) {
        return Err("Noto'g'ri Chrome Web Store ID (32 ta kichik harf bo'lishi kerak)".into());
    }

    let tmp_file = std::env::temp_dir().join(format!("rail_ext_{}.crx", clean_id));
    let url = format!(
        "https://clients2.google.com/service/update2/crx?response=redirect&prodversion=128.0.6613.120&acceptformat=crx2,crx3&x=id%3D{}%26uc",
        clean_id
    );

    let status = std::process::Command::new("curl")
        .args([
            "-4",
            "-fsSL",
            "-L",
            "--compressed",
            "-m", "60",
            "-A", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
            "-o", tmp_file.to_str().unwrap_or(""),
            &url,
        ])
        .status()
        .map_err(|e| format!("curl bilan yuklab bo'lmadi: {e}"))?;

    if !status.success() || !tmp_file.is_file() {
        let _ = std::fs::remove_file(&tmp_file);
        return Err("Chrome Web Store serveridan yuklab bo'lmadi".into());
    }

    let target_dir = extensions_dir().join(clean_id);
    let _ = std::fs::remove_dir_all(&target_dir);
    let _ = std::fs::create_dir_all(&target_dir);

    let py_status = std::process::Command::new("python3")
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
        .arg(&tmp_file)
        .arg(&target_dir)
        .status();

    let _ = std::fs::remove_file(&tmp_file);

    if py_status.map(|s| s.success()).unwrap_or(false) && target_dir.join("manifest.json").is_file() {
        register_unpacked_extension(&target_dir)
    } else {
        let _ = std::fs::remove_dir_all(&target_dir);
        Err("Kengaytma arxivini ochib bo'lmadi yoki manifest.json topilmadi".into())
    }
}

/// Comprehensive Chrome & WebExtensions runtime, storage, tabs, and i18n polyfill.
pub(crate) fn polyfill_for_ext(ext: &Extension) -> String {
    let base_url = format!("file://{}", ext.path.display());
    let manifest_clean = if ext.manifest_raw.trim().is_empty() {
        "{}".to_string()
    } else {
        ext.manifest_raw.trim().to_string()
    };

    let initial_local = read_ext_storage(&ext.id, "local");
    let initial_sync = read_ext_storage(&ext.id, "sync");

    format!(
        r#"(function() {{
  var extId = "{ext_id}";
  if (window["__rail_ext_" + extId]) return;
  window["__rail_ext_" + extId] = true;
  var extBaseUrl = "{base_url}";
  var manifest = {manifest_json};
  var initialLocal = {initial_local};
  var initialSync = {initial_sync};

  window.chrome = window.chrome || {{}};
  window.browser = window.browser || window.chrome;

  var storageListeners = [];

  function postBridge(action, payload) {{
    try {{
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {{
        var obj = Object.assign({{ action: action, extId: extId }}, payload || {{}});
        window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify(obj));
      }}
    }} catch(e) {{}}
  }}

  function makeStorageArea(areaName, initialStore) {{
    var store = Object.assign({{}}, initialStore || {{}});
    return {{
      get: function(keys, cb) {{
        return new Promise(function(resolve) {{
          var res = {{}};
          try {{
            if (keys === null || keys === undefined) {{
              res = Object.assign({{}}, store);
            }} else if (typeof keys === 'string') {{
              if (store[keys] !== undefined) res[keys] = store[keys];
            }} else if (Array.isArray(keys)) {{
              keys.forEach(function(k) {{
                if (store[k] !== undefined) res[k] = store[k];
              }});
            }} else if (typeof keys === 'object') {{
              Object.keys(keys).forEach(function(k) {{
                res[k] = store[k] !== undefined ? store[k] : keys[k];
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
              var oldVal = store[k];
              var newVal = items[k];
              store[k] = newVal;
              changes[k] = {{ oldValue: oldVal, newValue: newVal }};
            }});
            postBridge('storageSet', {{ area: areaName, data: JSON.stringify(store) }});
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
          var arr = typeof keys === 'string' ? [keys] : (Array.isArray(keys) ? keys : []);
          var changes = {{}};
          try {{
            arr.forEach(function(k) {{
              var oldVal = store[k];
              delete store[k];
              changes[k] = {{ oldValue: oldVal, newValue: undefined }};
            }});
            postBridge('storageSet', {{ area: areaName, data: JSON.stringify(store) }});
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
            store = {{}};
            postBridge('storageSet', {{ area: areaName, data: JSON.stringify(store) }});
          }} catch(e) {{}}
          if (typeof cb === 'function') setTimeout(cb, 0);
          resolve();
        }});
      }},
      getBytesInUse: function(keys, cb) {{
        var size = JSON.stringify(store).length;
        if (typeof cb === 'function') setTimeout(function() {{ cb(size); }}, 0);
        return Promise.resolve(size);
      }}
    }};
  }}

  if (!window.chrome.storage) {{
    var localStorageArea = makeStorageArea('local', initialLocal);
    var syncStorageArea = makeStorageArea('sync', initialSync);
    var sessionStorageArea = makeStorageArea('session', {{}});
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
        var url = (props && props.url) || 'rail://start/';
        postBridge('openTab', {{ url: url }});
        if (typeof cb === 'function') setTimeout(cb, 0);
        return Promise.resolve({{ id: 1, url: url }});
      }}
    }};
  }}

  if (!window.chrome.search) {{
    window.chrome.search = {{
      query: function(options, cb) {{
        var q = (options && options.text) || '';
        postBridge('search', {{ text: q }});
        if (typeof cb === 'function') setTimeout(cb, 0);
        return Promise.resolve();
      }}
    }};
  }}

  if (!window.chrome.notifications) {{
    window.chrome.notifications = {{
      create: function(id, options, cb) {{
        var title = (options && options.title) || 'Rail Extension';
        var message = (options && options.message) || '';
        postBridge('notify', {{ title: title, message: message }});
        if (typeof cb === 'function') setTimeout(function() {{ cb(id || '1'); }}, 0);
        return Promise.resolve(id || '1');
      }},
      clear: function(id, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(true); }}
    }};
  }}

  if (!window.chrome.commands) {{
    var cmdListeners = [];
    window.chrome.commands = {{
      onCommand: {{
        addListener: function(fn) {{ if (typeof fn === 'function' && cmdListeners.indexOf(fn) === -1) cmdListeners.push(fn); }},
        removeListener: function(fn) {{ var i = cmdListeners.indexOf(fn); if (i !== -1) cmdListeners.splice(i, 1); }},
        hasListener: function(fn) {{ return cmdListeners.indexOf(fn) !== -1; }}
      }},
      getAll: function(cb) {{
        var list = [
          {{ name: "crystal-fix", description: "Crystal Text — fix selected text with AI", shortcut: "Alt+F" }},
          {{ name: "_execute_action", description: "Activate extension", shortcut: "Ctrl+Comma" }}
        ];
        if (typeof cb === 'function') setTimeout(function() {{ cb(list); }}, 0);
        return Promise.resolve(list);
      }},
      __dispatch: function(cmd) {{
        cmdListeners.forEach(function(fn) {{ try {{ fn(cmd); }} catch(e) {{}} }});
      }}
    }};
  }}

  if (!window.chrome.contextMenus) {{
    window.chrome.contextMenus = {{
      create: function(props, cb) {{ if (cb) setTimeout(cb, 0); return 1; }},
      update: function(id, props, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      remove: function(id, cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      removeAll: function(cb) {{ if (cb) setTimeout(cb, 0); return Promise.resolve(); }},
      onClicked: {{ addListener: function() {{}}, removeListener: function() {{}} }}
    }};
  }}

  if (!window.chrome.scripting) {{
    window.chrome.scripting = {{
      executeScript: function(details, cb) {{
        var res = [];
        if (typeof details?.func === 'function') {{
          try {{
            var r = details.func.apply(null, details.args || []);
            res = [{{ result: r }}];
          }} catch(e) {{}}
        }}
        if (typeof cb === 'function') setTimeout(function() {{ cb(res); }}, 0);
        return Promise.resolve(res);
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
        manifest_json = manifest_clean,
        initial_local = initial_local,
        initial_sync = initial_sync
    )
}

/// Apply all enabled extensions' content scripts and stylesheets to a UserContentManager.
pub(crate) fn apply_extensions_to_ucm(ucm: &webkit6::UserContentManager) {
    let list = get_all_extensions();
    for ext in list.iter().filter(|e| e.enabled) {
        let polyfill = polyfill_for_ext(ext);

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

    // Chrome Web Store 1-Click Install Helper script
    const WEBSTORE_HELPER_JS: &str = r#"(function() {
  if (!location.hostname.includes('chromewebstore.google.com') && !location.hostname.includes('chrome.google.com')) return;

  function getExtId() {
    var m = location.href.match(/([a-z]{32})(?:[/?#]|$)/i);
    return m ? m[1].toLowerCase() : null;
  }

  function queryAllDeep(selector, root) {
    root = root || document;
    var results = [];
    try {
      results = Array.prototype.slice.call(root.querySelectorAll(selector));
    } catch(e) {}
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, null);
    var node = walker.nextNode();
    while (node) {
      if (node.shadowRoot) {
        results = results.concat(queryAllDeep(selector, node.shadowRoot));
      }
      node = walker.nextNode();
    }
    return results;
  }

  function triggerInstall(btn, extId) {
    if (btn) {
      btn.disabled = true;
      btn.setAttribute('aria-disabled', 'true');
      btn.innerHTML = '<span style="display:flex;align-items:center;justify-content:center;gap:8px;">⏳ Rail\'ga o\'rnatilmoqda...</span>';
      btn.style.opacity = '0.9';
    }
    var otherBtns = document.querySelectorAll('#rail-webstore-install-btn, [data-rail-webstore-hijacked="true"]');
    for (var i = 0; i < otherBtns.length; i++) {
      if (otherBtns[i] !== btn) {
        otherBtns[i].disabled = true;
        otherBtns[i].setAttribute('aria-disabled', 'true');
        otherBtns[i].innerHTML = '<span style="display:flex;align-items:center;justify-content:center;gap:8px;">⏳ Rail\'ga o\'rnatilmoqda...</span>';
      }
    }
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railWebstore) {
      window.webkit.messageHandlers.railWebstore.postMessage(JSON.stringify({ action: "install", id: extId }));
    }
  }

  function hijackWebstorePage() {
    var extId = getExtId();
    if (!extId) return;

    // 1. Hijack native gray/disabled "Add to Chrome" button on chromewebstore.google.com
    var candidates = queryAllDeep('button, [role="button"], a[role="button"]', document);
    for (var i = 0; i < candidates.length; i++) {
      var el = candidates[i];
      if (el.id === 'rail-webstore-install-btn' || el.getAttribute('data-rail-webstore-hijacked') === 'true') continue;
      var txt = (el.textContent || '').trim().toLowerCase();
      var aria = (el.getAttribute('aria-label') || '').toLowerCase();
      var isChromeBtn = txt.includes('chrome') || txt.includes('add to') || txt.includes('добавить') ||
                        txt.includes('доступно') || txt.includes('available') || aria.includes('chrome') ||
                        aria.includes('add to') || aria.includes('install');
      var isCtA = el.getAttribute('jsname') === 'wQO0od' || el.classList.contains('F9O37c') || (el.closest && el.closest('[data-item-id]'));

      if (isChromeBtn || isCtA) {
        el.setAttribute('data-rail-webstore-hijacked', 'true');
        el.removeAttribute('disabled');
        el.setAttribute('aria-disabled', 'false');
        el.style.setProperty('background', '#2563eb', 'important');
        el.style.setProperty('background-color', '#2563eb', 'important');
        el.style.setProperty('color', '#ffffff', 'important');
        el.style.setProperty('opacity', '1', 'important');
        el.style.setProperty('cursor', 'pointer', 'important');
        el.style.setProperty('pointer-events', 'auto', 'important');
        el.style.setProperty('border-radius', '24px', 'important');
        el.style.setProperty('font-weight', '700', 'important');
        el.style.setProperty('box-shadow', '0 4px 14px rgba(37, 99, 235, 0.45)', 'important');
        el.innerHTML = '<span style="display:flex;align-items:center;justify-content:center;gap:8px;font-weight:700;"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg> Rail\'ga qo\'shish</span>';

        el.addEventListener('click', function(e) {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          triggerInstall(this, extId);
        }, true);
        break;
      }
    }

    // 2. Guaranteed floating backup button at bottom-right
    if (!document.getElementById('rail-webstore-install-btn') && document.body) {
      var btn = document.createElement('button');
      btn.id = 'rail-webstore-install-btn';
      btn.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="margin-right:8px;"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg> Rail\'ga qo\'shish';
      btn.style.cssText = 'position: fixed; bottom: 32px; right: 32px; z-index: 2147483647; background: #2563eb; color: #ffffff; border: none; border-radius: 9999px; padding: 14px 28px; font-size: 15px; font-weight: 700; box-shadow: 0 10px 30px rgba(37, 99, 235, 0.5); cursor: pointer; transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1); display: flex; align-items: center; font-family: system-ui, -apple-system, sans-serif; letter-spacing: 0.3px;';

      btn.onmouseenter = function() { btn.style.transform = 'translateY(-2px) scale(1.04)'; btn.style.boxShadow = '0 14px 36px rgba(37, 99, 235, 0.65)'; };
      btn.onmouseleave = function() { btn.style.transform = 'none'; btn.style.boxShadow = '0 10px 30px rgba(37, 99, 235, 0.5)'; };

      btn.onclick = function(e) {
        e.preventDefault();
        e.stopPropagation();
        triggerInstall(btn, extId);
      };

      document.body.appendChild(btn);
    }
  }

  window.__railWebstoreInstalled = function(id) {
    var btns = document.querySelectorAll('#rail-webstore-install-btn, [data-rail-webstore-hijacked="true"]');
    for (var i = 0; i < btns.length; i++) {
      var b = btns[i];
      b.innerHTML = '<span style="display:flex;align-items:center;justify-content:center;gap:8px;font-weight:700;">✓ Rail\'ga o\'rnatildi</span>';
      b.style.setProperty('background', '#059669', 'important');
      b.style.setProperty('background-color', '#059669', 'important');
      b.style.setProperty('box-shadow', '0 10px 30px rgba(5, 150, 105, 0.5)', 'important');
      b.disabled = true;
    }
  };

  window.__railWebstoreFailed = function(err) {
    var btns = document.querySelectorAll('#rail-webstore-install-btn, [data-rail-webstore-hijacked="true"]');
    for (var i = 0; i < btns.length; i++) {
      var b = btns[i];
      b.innerHTML = '<span style="display:flex;align-items:center;justify-content:center;gap:8px;font-weight:700;">❌ ' + (err || "Xatolik yuz berdi") + '</span>';
      b.style.setProperty('background', '#dc2626', 'important');
      b.style.setProperty('background-color', '#dc2626', 'important');
      b.disabled = false;
    }
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', hijackWebstorePage);
  } else {
    hijackWebstorePage();
  }
  setInterval(hijackWebstorePage, 800);
})();"#;

    let ws_script = webkit6::UserScript::new(
        WEBSTORE_HELPER_JS,
        webkit6::UserContentInjectedFrames::TopFrame,
        webkit6::UserScriptInjectionTime::End,
        &[],
        &[],
    );
    ucm.add_script(&ws_script);
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
