//! Creating web views and opening tabs.

use crate::*;

pub(crate) fn is_external_uri_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "mailto"
            | "tel"
            | "sms"
            | "sip"
            | "sips"
            | "webcal"
            | "xmpp"
            | "facetime"
            | "tg"
            | "telegram"
            | "magnet"
            | "slack"
            | "discord"
            | "zoommtg"
            | "zoomus"
            | "skype"
            | "callto"
            | "signal"
            | "viber"
            | "whatsapp"
            | "msteams"
            | "ms-teams"
            | "steam"
            | "spotify"
    )
}

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

    /// One site-capability configuration shared by browser tabs, popups and PWA windows.
    pub(crate) fn configure_web_settings(settings: &webkit6::Settings) {
        settings.set_enable_webrtc(webrtc_allowed());
        settings.set_enable_dns_prefetching(true);
        settings.set_enable_smooth_scrolling(load_app_settings().smooth_scrolling);
        settings.set_enable_webgl(true);
        settings.set_property("enable-2d-canvas-acceleration", true);
        settings.set_enable_hyperlink_auditing(false);
        settings.set_enable_mediasource(true);
        settings.set_enable_media(true);
        settings.set_enable_media_capabilities(true);
        settings.set_media_playback_allows_inline(true);
        // EME stays disabled because this Linux environment has no Widevine CDM.
        settings.set_enable_encrypted_media(false);
        settings.set_enable_developer_extras(true);
        settings.set_enable_back_forward_navigation_gestures(true);
        settings.set_enable_webaudio(true);
        settings.set_enable_media_stream(true);
        settings.set_enable_html5_local_storage(true);
        settings.set_enable_html5_database(true);
        settings.set_enable_site_specific_quirks(true);
        settings.set_javascript_can_access_clipboard(true);
        settings.set_allow_file_access_from_file_urls(true);
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
        settings.set_auto_load_images(true);
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
    }

    fn build_web_view(
        session: Option<&webkit6::NetworkSession>,
        related_view: Option<&WebView>,
    ) -> WebView {
        let settings = webkit6::Settings::new();
        Self::configure_web_settings(&settings);

        let ucm = webkit6::UserContentManager::new();
        ucm.register_script_message_handler("railgo", None);
        ucm.register_script_message_handler("railScreenshot", None);
        ucm.register_script_message_handler("railContextMenu", None);

        let html_css = load_html_css();
        if css_has_active_content(&html_css) {
            ucm.add_style_sheet(&user_style_sheet(&html_css));
        }
        crate::web::inject_context_menu_bridge(&ucm);
        STYLE_MANAGERS.with(|l| l.borrow_mut().push(ucm.downgrade()));
        ADBLOCK_FILTER.with(|f| {
            if let Some(f) = f.borrow().as_ref() {
                ucm.add_filter(f);
            }
        });

        let website_policies = webkit6::WebsitePolicies::builder()
            .autoplay(webkit6::AutoplayPolicy::AllowWithoutSound)
            .build();

        let view = if let Some(parent) = related_view {
            WebView::builder()
                .related_view(parent)
                .settings(&settings)
                .user_content_manager(&ucm)
                .website_policies(&website_policies)
                .build()
        } else {
            WebView::builder()
                .network_session(session.expect("a new WebView needs a NetworkSession"))
                .settings(&settings)
                .user_content_manager(&ucm)
                .website_policies(&website_policies)
                .build()
        };

        Self::configure_native_web_view(&view);
        view
    }

    pub(crate) fn create_web_view(session: &webkit6::NetworkSession, _profile_id: &str) -> WebView {
        attach_download_handlers_once(session);
        Self::build_web_view(Some(session), None)
    }

    /// Create a `window.open()` target with exactly the same settings, user scripts,
    /// content filters, stylesheet and native chooser/navigation handlers as a normal tab.
    /// WebKit's `related_view` shares only the opener's web process and network session;
    /// those other per-view settings must be attached explicitly.
    pub(crate) fn create_related_web_view(parent: &WebView) -> WebView {
        if let Some(session) = parent.network_session() {
            attach_download_handlers_once(&session);
        }
        Self::build_web_view(None, Some(parent))
    }

    pub(crate) fn configure_native_web_view(view: &WebView) {
        // Use a native GTK chooser for every web file input. This is more reliable
        // than WebKit's default chooser on Linux and supports multi-file uploads.
        view.connect_run_file_chooser(|view, request| {
            let chooser =
                gtk::FileChooserNative::builder()
                    .title("Choose files to upload")
                    .transient_for(&view.root().and_downcast::<gtk::Window>().unwrap_or_else(
                        || {
                            // Defensive fallback for detached views.
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
                            if is_external_uri_scheme(&scheme)
                                && gio::AppInfo::launch_default_for_uri(
                                    &uri_s,
                                    None::<&gio::AppLaunchContext>,
                                )
                                .is_ok()
                            {
                                decision.ignore();
                                return true;
                            }
                        }
                    }
                }
                let policies = webkit6::WebsitePolicies::builder()
                    .autoplay(webkit6::AutoplayPolicy::AllowWithoutSound)
                    .build();
                decision.use_with_policies(&policies);
                return true;
            }
            false
        });
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
