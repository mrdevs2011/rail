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
            .default_width(400)
            .default_height(540)
            .destroy_with_parent(true)
            .build();
        win.add_css_class("settings-modal-window");

        let root_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Header bar
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.add_css_class("settings-header");
        header.set_margin_start(12);
        header.set_margin_end(12);
        header.set_margin_top(8);
        header.set_margin_bottom(8);

        let icon = gtk::Image::from_icon_name("application-x-addon-symbolic");
        let title_lbl = gtk::Label::new(Some(&ext.name));
        title_lbl.add_css_class("settings-card-title");
        title_lbl.set_halign(gtk::Align::Start);
        title_lbl.set_hexpand(true);

        let ver_lbl = gtk::Label::new(Some(&format!("v{}", ext.version)));
        ver_lbl.add_css_class("settings-badge");

        let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("flat-icon");
        let win_close = win.clone();
        close_btn.connect_clicked(move |_| win_close.close());

        header.append(&icon);
        header.append(&title_lbl);
        header.append(&ver_lbl);
        header.append(&close_btn);
        root_box.append(&header);

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
        let a_bridge = self.clone();
        ucm.connect_script_message_received(Some("railExtBridge"), move |_, val| {
            a_bridge.handle_ext_bridge_message(val.to_str().as_str());
        });

        let polyfill = crate::extensions::polyfill_for_ext(ext);
        let script = webkit6::UserScript::new(
            &polyfill,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        );
        ucm.add_script(&script);

        let view = WebView::builder()
            .network_session(&session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .hexpand(true)
            .vexpand(true)
            .build();

        let uri = format!("file://{}", file_path.display());
        view.load_uri(&uri);

        root_box.append(&view);
        win.set_child(Some(&root_box));

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

        win.present();
    }

    /// Check if keyboard combination matches any registered extension command
    pub(crate) fn try_extension_command(self: &Rc<Self>, key: gdk::Key, state: gdk::ModifierType) -> bool {
        use gdk::ModifierType as M;
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
