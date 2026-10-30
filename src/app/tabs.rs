//! Creating web views and opening tabs.

use crate::*;

impl App {
    // ---- tabs & workspaces -----------------------------------------------

    #[allow(dead_code)]
    pub(crate) fn focused_tab_profile(&self) -> Option<String> {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        rows.get(cur_r)
            .and_then(|r| r.tabs.get(cur_c))
            .map(|t| t.profile_id.clone())
    }

    pub(crate) fn find_view_profile(&self, view: &WebView) -> Option<String> {
        let rows = self.rows.borrow();
        for row in rows.iter() {
            for tab in row.tabs.iter() {
                if &tab.view == view {
                    return Some(tab.profile_id.clone());
                }
            }
        }
        None
    }

    pub(crate) fn create_web_view(session: &webkit6::NetworkSession, _profile_id: &str) -> WebView {
        attach_download_handlers_once(session);

        let settings = webkit6::Settings::new();
        // Native WebKitGTK user agent ensures websites deliver JavaScriptCore-compatible
        // scripts with zero V8 syntax errors.
        settings.set_enable_webrtc(webrtc_allowed());
        // High-speed background DNS prefetching eliminates lookup latency on all link clicks
        settings.set_enable_dns_prefetching(true);
        settings.set_property("enable-offline-web-application-cache", true);
        // Smooth, kinetic scrolling across all pages and web views:
        settings.set_enable_smooth_scrolling(load_app_settings().smooth_scrolling);
        settings.set_enable_webgl(true); // needed for smooth compositing on many sites
        settings.set_property("enable-2d-canvas-acceleration", true);
        settings.set_enable_hyperlink_auditing(false);
        settings.set_enable_mediasource(true);
        settings.set_enable_media(true);
        settings.set_enable_media_capabilities(true);
        settings.set_media_playback_allows_inline(true);
        settings.set_media_playback_requires_user_gesture(false);
        // Widevine CDM is absent on desktop Linux WebKitGTK; enabling EME causes YouTube to request DRM/ClearKey and fail
        settings.set_enable_encrypted_media(false);
        settings.set_enable_developer_extras(true);
        settings.set_enable_back_forward_navigation_gestures(true);
        settings.set_enable_webaudio(true);
        settings.set_enable_media_stream(true); // getUserMedia (mic/camera) still needs the user prompt
        settings.set_enable_html5_local_storage(true);
        settings.set_enable_html5_database(true);
        settings.set_enable_site_specific_quirks(true);
        settings.set_javascript_can_access_clipboard(true);
        // Allow local offline files (e.g. index.html) to load relative CSS, JS and images like Chrome / Firefox
        settings.set_allow_file_access_from_file_urls(true);
        settings.set_allow_universal_access_from_file_urls(true);
        // Always: smooth page scroll/compositing; falls back when GPU unavailable
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
        // Ensure images, web icons, fonts, and inline SVGs load unconditionally
        settings.set_auto_load_images(true);
        settings.set_load_icons_ignoring_image_load_setting(true);
        // Back/forward uses the page cache: instant history navigation instead of a reload
        let dev_no_cache = rail_data_dir().join("dev-disable-cache").exists();
        settings.set_enable_page_cache(!dev_no_cache);

        if rail_data_dir().join("dev-cors-bypass").exists()
            || rail_data_dir().join("dev-mixed-content").exists()
        {
            settings.set_disable_web_security(true);
            settings.set_allow_universal_access_from_file_urls(true);
            settings.set_allow_file_access_from_file_urls(true);
            settings.set_allow_top_navigation_to_data_urls(true);
        }

        let ucm = webkit6::UserContentManager::new();
        ucm.register_script_message_handler("railgo", None);
        ucm.register_script_message_handler("railScreenshot", None);
        ucm.register_script_message_handler("railContextMenu", None);

        // Cross-tab/webpage image drags often arrive as text/uri-list + text/html,
        // not as files. Best-effort fetch the dragged image and route it through the
        // page's normal file input. If CORS or the site blocks it, native drop remains
        // available as a fallback instead of swallowing the drag.
        let drop_image_script = r#"(function(){
          document.addEventListener('drop', function(ev) {
            try {
              var d=ev.dataTransfer;
              if(!d || (d.files && d.files.length)) return;
              var url=(d.getData('text/uri-list')||'').split(/\r?\n/).find(function(s){return s && s[0]!=='#';})||'';
              var html=d.getData('text/html')||'';
              if(html){var doc=new DOMParser().parseFromString(html,'text/html');var img=doc.querySelector('img[src]');if(img && img.src)url=img.src;}
              if(!url || !/^https?:\/\//i.test(url)) return;
              var looksImage=/\.(png|jpe?g|gif|webp|bmp|avif|svg)(?:[?#]|$)/i.test(url) || /<img\b/i.test(html);
              if(!looksImage) return;
              fetch(url,{mode:'cors',credentials:'omit'}).then(function(r){if(!r.ok)throw new Error('image fetch failed');return r.blob();}).then(function(blob){
                var name='dropped-image';try{name=decodeURIComponent(new URL(url).pathname.split('/').pop())||name;}catch(_){}
                name=name.replace(/[\\\/'"\r\n]/g,'_');
                var file=new File([blob],name,{type:blob.type||'image/png'}),dt=new DataTransfer();dt.items.add(file);
                var ins=Array.from(document.querySelectorAll('input[type=file]')).filter(function(i){return !i.disabled;});
                var input=ins.find(function(i){return !i.accept||i.accept.indexOf('image')>=0;})||ins[0];
                if(input){input.files=dt.files;input.dispatchEvent(new Event('input',{bubbles:true}));input.dispatchEvent(new Event('change',{bubbles:true}));}
                else {var t=document.activeElement;if(t)t.dispatchEvent(new ClipboardEvent('paste',{clipboardData:dt,bubbles:true,cancelable:true,composed:true}));}
              }).catch(function(err){console.debug('Rail image drop fallback:',err);});
            } catch(err) { console.debug('Rail drop fallback:',err); }
          }, true);
        })();"#;
        let drop_image_user_script = webkit6::UserScript::new(
            drop_image_script,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        );
        ucm.add_script(&drop_image_user_script);

        let html_css = load_html_css();
        if css_has_active_content(&html_css) {
            ucm.add_style_sheet(&user_style_sheet(&html_css));
        }
        crate::web::inject_web_optimizations(&ucm);
        STYLE_MANAGERS.with(|l| l.borrow_mut().push(ucm.downgrade()));
        ADBLOCK_FILTER.with(|f| {
            if let Some(f) = f.borrow().as_ref() {
                ucm.add_filter(f);
            }
        });

        let website_policies = webkit6::WebsitePolicies::builder()
            .autoplay(webkit6::AutoplayPolicy::Allow)
            .build();

        let view = WebView::builder()
            .network_session(session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .website_policies(&website_policies)
            .build();

        // Use a native GTK chooser for every web file input. This is more reliable
        // than WebKit's default chooser on Linux and supports multi-file uploads.
        view.connect_run_file_chooser(|view, request| {
            let chooser =
                gtk::FileChooserNative::builder()
                    .title("Choose files to upload")
                    .transient_for(&view.root().and_downcast::<gtk::Window>().unwrap_or_else(
                        || {
                            // WebView is always embedded in the application window; this is
                            // only a defensive fallback for detached views.
                            gtk::Window::new()
                        },
                    ))
                    .action(gtk::FileChooserAction::Open)
                    .accept_label("Open")
                    .cancel_label("Cancel")
                    .build();
            chooser.set_select_multiple(request.selects_multiple());
            if let Some(filter) = request.mime_types_filter() {
                chooser.set_filter(&filter);
            } else {
                let mime_types = request.mime_types();
                if !mime_types.is_empty() {
                    let filter = gtk::FileFilter::new();
                    for mime in mime_types {
                        filter.add_mime_type(mime.as_str());
                    }
                    chooser.set_filter(&filter);
                }
            }
            let req = request.clone();
            chooser.connect_response(move |dialog, response| {
                if response == gtk::ResponseType::Accept {
                    let model = dialog.files();
                    let mut uris = Vec::new();
                    for i in 0..model.n_items() {
                        if let Some(obj) = model.item(i) {
                            if let Ok(file) = obj.downcast::<gio::File>() {
                                uris.push(file.uri().to_string());
                            }
                        }
                    }
                    let refs: Vec<&str> = uris.iter().map(String::as_str).collect();
                    if !refs.is_empty() {
                        req.select_files(&refs);
                    } else {
                        req.cancel();
                    }
                } else {
                    req.cancel();
                }
                dialog.destroy();
            });
            chooser.show();
            true
        });

        view.connect_decide_policy(|_view, decision, decision_type| {
            if decision_type == webkit6::PolicyDecisionType::NavigationAction {
                if let Some(nav_decision) =
                    decision.downcast_ref::<webkit6::NavigationPolicyDecision>()
                {
                    if let Some(req) = nav_decision.navigation_action().and_then(|a| a.request()) {
                        if let Some(uri) = req.uri() {
                            let uri_s = uri.to_string();
                            let scheme = uri_s.split(':').next().unwrap_or("").to_lowercase();
                            let is_external = matches!(
                                scheme.as_str(),
                                "mailto"
                                    | "tel"
                                    | "tg"
                                    | "telegram"
                                    | "magnet"
                                    | "slack"
                                    | "discord"
                                    | "zoommtg"
                                    | "skype"
                                    | "callto"
                            );
                            if is_external {
                                let _ = gio::AppInfo::launch_default_for_uri(
                                    &uri_s,
                                    None::<&gio::AppLaunchContext>,
                                );
                                decision.ignore();
                                return true;
                            }
                        }
                    }
                }
                let policies = webkit6::WebsitePolicies::builder()
                    .autoplay(webkit6::AutoplayPolicy::Allow)
                    .build();
                decision.use_with_policies(&policies);
                return true;
            }
            false
        });

        view
    }

    pub(crate) fn new_tab(self: &Rc<Self>) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_html(&start_page(), Some("rail://start/"));
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view.clone(), prof_id.clone(), false);
        self.set_view_saved_uri(&view, "rail://start/");
        self.select_palette_profile(&prof_id);
        // In-page omnibar only — never show GTK palette over rail://start
        if self.palette_open() {
            self.hide_palette();
        }
        self.entry.set_text("");
        self.last_entry_len.set(0);
        // Retry state inject until page JS is ready (covers init race)
        for ms in [30u64, 120, 350] {
            let app = self.clone();
            let v = view.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
                let mut u = v.uri().map(|s| s.to_string()).unwrap_or_default();
                if u.is_empty() || u.starts_with("about:") {
                    u = app.saved_uri_of(&v);
                }
                if u.starts_with("rail://start") {
                    app.push_start_state(&v);
                    if app.focused_view().as_ref() == Some(&v) {
                        app.focus_welcome_input(false);
                    }
                }
            });
        }
    }

    pub(crate) fn split_vertical(self: &Rc<Self>) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        view.load_html(&start_page(), Some("rail://start/"));
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view, prof_id.clone(), true);
        self.select_palette_profile(&prof_id);
        if self.palette_open() {
            self.hide_palette();
        }
        self.entry.set_text("");
        self.last_entry_len.set(0);
        for ms in [30u64, 120, 350] {
            let app = self.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
                if app.focused_is_welcome() {
                    if let Some(v) = app.focused_view() {
                        app.push_start_state(&v);
                        app.focus_welcome_input(false);
                    }
                }
            });
        }
        self.entry.grab_focus();
    }

    pub(crate) fn new_tab_with_url_and_profile(self: &Rc<Self>, url: &str, profile_id: &str) {
        self.exit_overview();
        let session = self.profiles.borrow_mut().get_or_create_session(profile_id);
        let view = Self::create_web_view(&session, profile_id);
        if is_internal_uri(url) {
            load_internal_page(&view, url);
        } else {
            if is_internal_uri(url) {
                load_internal_page(&view, url);
            } else {
                view.load_uri(url);
            }
        }
        self.set_view_saved_uri(&view, url);
        self.add_view(view, profile_id.to_string(), false);
    }

    pub(crate) fn new_tab_with_url(self: &Rc<Self>, url: &str) {
        let prof_id = self.profiles.borrow().active_id().to_string();
        self.new_tab_with_url_and_profile(url, &prof_id);
    }

    pub(crate) fn split_with_url(self: &Rc<Self>, url: &str) {
        self.exit_overview();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        if is_internal_uri(url) {
            load_internal_page(&view, url);
        } else {
            view.load_uri(url);
        }
        self.add_view(view, prof_id, true);
    }

    /// View Page Source (Ctrl+U): extracts active DOM HTML or fetches raw page, renders with line numbers
    pub(crate) fn view_source_of_focused(self: &Rc<Self>) {
        let Some(v) = self.focused_view() else { return };
        let uri = v
            .uri()
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.saved_uri_of(&v));
        if uri.is_empty() || uri.starts_with("rail://") {
            return;
        }
        let a = self.clone();
        let prof_id = self.profiles.borrow().active_id().to_string();
        let uri_target = uri.clone();
        v.evaluate_javascript(
            "document.documentElement ? (document.doctype ? new XMLSerializer().serializeToString(document.doctype) + '\n' : '<!DOCTYPE html>\n') + document.documentElement.outerHTML : ''",
            None,
            None,
            gio::Cancellable::NONE,
            move |res| {
                let raw_html = match res {
                    Ok(val) => val.to_string(),
                    Err(_) => String::new(),
                };
                let view_source_uri = format!("view-source:{uri_target}");
                if !raw_html.trim().is_empty() {
                    let html = crate::format_view_source_html(&uri_target, &raw_html);
                    let session = a.profiles.borrow_mut().get_or_create_session(&prof_id);
                    let new_view = Self::create_web_view(&session, &prof_id);
                    new_view.load_html(&html, Some("about:blank"));
                    a.set_view_saved_uri(&new_view, &view_source_uri);
                    a.add_view(new_view, prof_id, false);
                } else {
                    a.new_tab_with_url(&view_source_uri);
                }
            },
        );
    }
}
