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
                    if cmd.name == "_execute_action" || ext.default_popup.is_some() {
                        self.open_extension_popup(&ext);
                        return true;
                    }
                }
            }
        }
        false
    }
}
