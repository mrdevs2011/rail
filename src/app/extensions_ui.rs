//! Extension popups and keyboard shortcuts runner.

use crate::extensions::Extension;
use crate::*;
use std::rc::Rc;

impl App {
    /// Open the action popup of an extension (e.g. Quick Open, Crystal Text, Shortcut Blocker)
    pub(crate) fn open_extension_popup(self: &Rc<Self>, ext: &Extension) {
        let Some(popup_file) = &ext.default_popup else {
            return;
        };
        let file_path = ext.path.join(popup_file);
        if !file_path.is_file() {
            return;
        }

        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);

        let win = gtk::Window::builder()
            .transient_for(&self.win)
            .modal(false)
            .title(&ext.name)
            .decorated(false)
            .destroy_with_parent(true)
            .build();
        win.add_css_class("extension-popup-window");

        let root_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Web view for the popup
        let settings = webkit6::Settings::new();
        settings.set_enable_javascript(true);
        settings.set_allow_file_access_from_file_urls(true);
        settings.set_allow_universal_access_from_file_urls(true);
        settings.set_enable_html5_local_storage(true);
        settings.set_enable_html5_database(true);
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);

        let ucm = webkit6::UserContentManager::new();
        ucm.register_script_message_handler("railExtBridge", None);

        let polyfill = crate::extensions::polyfill_for_ext(ext);
        let script = webkit6::UserScript::new(
            &polyfill,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        );
        ucm.add_script(&script);

        // Inject auto-sizing script that reports document dimensions
        let resize_script_code = r#"
(function() {
  function sendSize() {
    try {
      var b = document.body;
      var d = document.documentElement;
      var w = Math.max(b ? b.scrollWidth : 0, d ? d.scrollWidth : 0, b ? b.offsetWidth : 0, d ? d.offsetWidth : 0);
      var h = Math.max(b ? b.scrollHeight : 0, d ? d.scrollHeight : 0, b ? b.offsetHeight : 0, d ? d.offsetHeight : 0);
      if (w > 20 && h > 20) {
        if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {
          window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify({ action: "resize", width: Math.ceil(w), height: Math.ceil(h) }));
        }
      }
    } catch(_) {}
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', sendSize);
  } else {
    sendSize();
  }
  window.addEventListener('load', sendSize);
  if (window.ResizeObserver && document.documentElement) {
    new ResizeObserver(sendSize).observe(document.documentElement);
  }
  setTimeout(sendSize, 40);
  setTimeout(sendSize, 180);
  setTimeout(sendSize, 500);
})();
"#;
        let resize_script = webkit6::UserScript::new(
            resize_script_code,
            webkit6::UserContentInjectedFrames::TopFrame,
            webkit6::UserScriptInjectionTime::End,
            &[],
            &[],
        );
        ucm.add_script(&resize_script);

        let view = WebView::builder()
            .network_session(&session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .hexpand(true)
            .vexpand(true)
            .build();

        let a_bridge = self.clone();
        let win_bridge = win.downgrade();
        let view_bridge = view.downgrade();
        ucm.connect_script_message_received(Some("railExtBridge"), move |_, val| {
            let msg = val.to_str();
            if let Some(json) = crate::minijson::parse(&msg) {
                if json.get("action").and_then(|a| a.as_str()) == Some("resize") {
                    let w = json.get("width").and_then(|v| v.as_f64()).unwrap_or(360.0) as i32;
                    let h = json.get("height").and_then(|v| v.as_f64()).unwrap_or(480.0) as i32;
                    let clamped_w = w.clamp(200, 800);
                    let clamped_h = h.clamp(100, 600);
                    if let Some(wnd) = win_bridge.upgrade() {
                        wnd.set_default_size(clamped_w, clamped_h);
                        if let Some(wv) = view_bridge.upgrade() {
                            wv.set_size_request(clamped_w, clamped_h);
                        }
                        wnd.queue_resize();
                    }
                }
            }
            a_bridge.handle_ext_bridge_message(msg.as_str());
        });

        // Trigger size measurement when page load finishes
        let win_load = win.downgrade();
        let view_load = view.downgrade();
        view.connect_load_changed(move |v, evt| {
            if evt == webkit6::LoadEvent::Finished {
                let win_cb = win_load.clone();
                let view_cb = view_load.clone();
                v.evaluate_javascript(
                    r#"
(function() {
  var b = document.body;
  var d = document.documentElement;
  var w = Math.max(b ? b.scrollWidth : 0, d ? d.scrollWidth : 0, b ? b.offsetWidth : 0, d ? d.offsetWidth : 0);
  var h = Math.max(b ? b.scrollHeight : 0, d ? d.scrollHeight : 0, b ? b.offsetHeight : 0, d ? d.offsetHeight : 0);
  return JSON.stringify({ w: Math.ceil(w), h: Math.ceil(h) });
})()
"#,
                    None,
                    None,
                    gio::Cancellable::NONE,
                    move |res| {
                        if let Ok(js_val) = res {
                            let json_str = js_val.to_string();
                            if let Some(parsed) = crate::minijson::parse(&json_str) {
                                let w = parsed.get("w").and_then(|v| v.as_f64()).unwrap_or(360.0) as i32;
                                let h = parsed.get("h").and_then(|v| v.as_f64()).unwrap_or(480.0) as i32;
                                let clamped_w = w.clamp(200, 800);
                                let clamped_h = h.clamp(100, 600);
                                if let Some(wnd) = win_cb.upgrade() {
                                    wnd.set_default_size(clamped_w, clamped_h);
                                    if let Some(wv) = view_cb.upgrade() {
                                        wv.set_size_request(clamped_w, clamped_h);
                                    }
                                    wnd.queue_resize();
                                }
                            }
                        }
                    },
                );
            }
        });

        let uri = format!("file://{}", file_path.display());
        view.load_uri(&uri);

        root_box.append(&view);
        win.set_child(Some(&root_box));

        let win_close_vw = win.clone();
        view.connect_close(move |_| {
            win_close_vw.close();
        });

        // Close on Escape
        let key_ctl = gtk::EventControllerKey::new();
        let win_esc = win.clone();
        key_ctl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                win_esc.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        win.add_controller(key_ctl);

        // Dismiss on focus leave (click outside), matching Chrome/Brave extension popups
        let win_focus = win.clone();
        let opened_at = std::time::Instant::now();
        win.connect_is_active_notify(move |w| {
            if !w.is_active() && opened_at.elapsed().as_millis() > 350 {
                win_focus.close();
            }
        });

        self.sidebar_locked.set(true);
        let a_lock = self.clone();
        win.connect_close_request(move |_| {
            a_lock.sidebar_locked.set(false);
            glib::Propagation::Proceed
        });

        win.present();
        let v_focus = view.clone();
        glib::idle_add_local_once(move || {
            v_focus.grab_focus();
        });
    }

    /// Ctrl+Shift+E: switch between normal mode (extension shortcuts blocked) and extension
    /// mode (Rail shortcuts off, only extension shortcuts work).
    pub(crate) fn toggle_extension_mode(self: &Rc<Self>) {
        let on = !crate::extensions::ext_keys_on();
        crate::extensions::set_ext_keys_on(on);
        // Pages opened from now on start in the new state (content scripts are regenerated).
        crate::extensions::apply_extensions_all();
        // Pages that are already open flip live.
        let msg = format!("window.postMessage({{__railExtKeys:{}}},'*');", on);
        for row in self.rows.borrow().iter() {
            for t in row.tabs.iter() {
                t.view.evaluate_javascript(&msg, None, None, None::<&gio::Cancellable>, |_| {});
            }
        }
        if on {
            self.show_copied_toast(
                tr("Extension mode ON"),
                tr("Rail shortcuts are off, only extension shortcuts work. Ctrl+Shift+E to exit."),
            );
        } else {
            self.show_copied_toast(
                tr("Extension mode OFF"),
                tr("Rail shortcuts are back, extension shortcuts are blocked."),
            );
        }
    }

    /// Check if keyboard combination matches any registered extension command
    pub(crate) fn try_extension_command(self: &Rc<Self>, key: gdk::Key, state: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
        if !crate::extensions::ext_keys_on() {
            return false; // extension shortcuts are blocked outside extension mode
        }
        let ctrl = state.contains(M::CONTROL_MASK);
        let alt = state.contains(M::ALT_MASK);
        let shift = state.contains(M::SHIFT_MASK);

        let exts = crate::extensions::get_all_extensions();
        for ext in exts {
            if !ext.enabled {
                continue;
            }
            for cmd in &ext.commands {
                let sk = cmd.suggested_key.to_lowercase();
                let need_ctrl = sk.contains("ctrl") || sk.contains("control") || sk.contains("command");
                let need_alt = sk.contains("alt");
                let need_shift = sk.contains("shift");

                if need_ctrl != ctrl || need_alt != alt || need_shift != shift {
                    continue;
                }

                let key_name = sk.split('+').last().unwrap_or("").trim();
                let matched = match key_name {
                    "comma" => key == gdk::Key::comma,
                    "f" => key == gdk::Key::f || key == gdk::Key::F,
                    "s" => key == gdk::Key::s || key == gdk::Key::S,
                    "o" => key == gdk::Key::o || key == gdk::Key::O,
                    "p" => key == gdk::Key::p || key == gdk::Key::P,
                    k => key.name().map(|n| n.eq_ignore_ascii_case(k)).unwrap_or(false),
                };

                if matched {
                    if cmd.name == "_execute_action" || cmd.name == "_execute_browser_action" {
                        self.open_extension_popup(&ext);
                        return true;
                    } else {
                        self.dispatch_extension_command(&ext.id, &cmd.name);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Dispatch non-popup extension commands (e.g. Crystal Text Alt+F)
    pub(crate) fn dispatch_extension_command(self: &Rc<Self>, ext_id: &str, cmd_name: &str) {
        let Some(view) = self.focused_view() else { return };
        if ext_id == "crystal-text" && cmd_name == "crystal-fix" {
            let script = r#"
(async function() {
  var settings = await chrome.storage.sync.get({ apiKey: "", model: "openai/gpt-oss-20b" });
  if (!settings.apiKey || !settings.apiKey.startsWith("gsk_")) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {
      window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify({
        action: "notify",
        title: "Crystal Text",
        message: "Groq API key kerak. Extension menyusidan Crystal Text'ni ochib, key kiriting."
      }));
    }
    return;
  }
  var ae = document.activeElement;
  var selText = "";
  var isInput = false;
  var start = 0, end = 0;
  if (ae && (ae.tagName === "TEXTAREA" || (ae.tagName === "INPUT" && /^(text|search|url|tel|password|email|number)$/i.test(ae.type || "text"))) && typeof ae.selectionStart === "number") {
    start = ae.selectionStart;
    end = ae.selectionEnd;
    if (end > start) {
      selText = ae.value.slice(start, end);
      isInput = true;
    }
  } else if (window.getSelection) {
    var s = window.getSelection();
    if (s && !s.isCollapsed && s.rangeCount > 0) {
      selText = s.toString();
    }
  }
  if (!selText.trim()) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {
      window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify({
        action: "notify",
        title: "Crystal Text",
        message: "Avval matnni tanlang (select qiling)."
      }));
    }
    return;
  }
  if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {
    window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify({
      action: "notify",
      title: "Crystal Text",
      message: "Tuzatilmoqda..."
    }));
  }
  try {
    var prompt = "You are an expert editor for Uzbek, English, and mixed text.\nRewrite the selected text so it is crystal clear, grammatically correct, and easy for AI to understand.\nRules:\n1. Fix ALL spelling and punctuation errors.\n2. Fix grammar carefully, especially Uzbek case endings.\n3. Rewrite into natural, correct sentences.\n4. Keep the ORIGINAL language.\n5. Keep technical terms as-is.\n6. Return ONLY the rewritten text.";
    var res = await fetch("https://api.groq.com/openai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": "Bearer " + settings.apiKey
      },
      body: JSON.stringify({
        model: settings.model || "openai/gpt-oss-20b",
        messages: [
          { role: "system", content: prompt },
          { role: "user", content: selText }
        ],
        temperature: 0.35,
        max_tokens: 4096
      })
    });
    if (!res.ok) {
      var err = await res.text();
      throw new Error("Groq " + res.status + ": " + err.slice(0, 100));
    }
    var data = await res.json();
    var fixed = data?.choices?.[0]?.message?.content?.trim();
    if (!fixed) throw new Error("Bo'sh javob keldi");

    if (isInput) {
      var before = ae.value.slice(0, start);
      var after = ae.value.slice(end);
      ae.value = before + fixed + after;
      ae.dispatchEvent(new Event("input", { bubbles: true }));
      ae.dispatchEvent(new Event("change", { bubbles: true }));
      ae.setSelectionRange(start + fixed.length, start + fixed.length);
      ae.focus();
    } else if (window.getSelection) {
      var sel = window.getSelection();
      if (sel && sel.rangeCount > 0) {
        var range = sel.getRangeAt(0);
        range.deleteContents();
        var node = document.createTextNode(fixed);
        range.insertNode(node);
        range.setStartAfter(node);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
      }
    }
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {
      window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify({
        action: "notify",
        title: "Crystal Text — tuzatildi",
        message: fixed.length > 80 ? fixed.slice(0, 77) + "..." : fixed
      }));
    }
  } catch(e) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railExtBridge) {
      window.webkit.messageHandlers.railExtBridge.postMessage(JSON.stringify({
        action: "notify",
        title: "Crystal Text xato",
        message: String(e.message || e).slice(0, 150)
      }));
    }
  }
})();
"#;
            view.evaluate_javascript(script, None, None, gio::Cancellable::NONE, |_| {});
        } else {
            let js = format!(
                "if (window.chrome && window.chrome.commands && window.chrome.commands.__dispatch) {{ window.chrome.commands.__dispatch('{}'); }}",
                cmd_name.replace('\'', "\\'")
            );
            view.evaluate_javascript(&js, None, None, gio::Cancellable::NONE, |_| {});
        }
    }

    /// Process bridge messages from extensions (storage, tabs, search, notifications)
    pub(crate) fn handle_ext_bridge_message(self: &Rc<Self>, raw: &str) {
        if let Some(json) = crate::minijson::parse(raw) {
            let action = json.get("action").and_then(|a| a.as_str()).unwrap_or("");
            match action {
                "storageSet" => {
                    if let (Some(ext_id), Some(area), Some(data)) = (
                        json.get("extId").and_then(|i| i.as_str()),
                        json.get("area").and_then(|a| a.as_str()),
                        json.get("data").and_then(|d| d.as_str()),
                    ) {
                        crate::extensions::write_ext_storage(ext_id, area, data);
                    }
                }
                "openTab" => {
                    if let Some(url) = json.get("url").and_then(|u| u.as_str()) {
                        let uri = to_uri(url, &self.search_engine.borrow());
                        self.new_tab_with_url(&uri);
                    }
                }
                "search" => {
                    if let Some(q) = json.get("text").and_then(|t| t.as_str()) {
                        let uri = to_uri(q, &self.search_engine.borrow());
                        self.new_tab_with_url(&uri);
                    }
                }
                "notify" => {
                    let title = json.get("title").and_then(|t| t.as_str()).unwrap_or("Extension");
                    let msg = json.get("message").and_then(|m| m.as_str()).unwrap_or("");
                    self.show_copied_toast(title, msg);
                }
                _ => {}
            }
        }
    }
}
