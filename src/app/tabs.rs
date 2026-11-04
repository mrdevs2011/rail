//! Creating web views and opening tabs.

use crate::*;

/// Decide whether a WebKit response is a file download rather than a document
/// navigation. HTML documents must never be diverted just because WebKit cannot
/// render a particular MIME variant or the URL happens to look like a filename.
fn should_download_response(
    mime_type: Option<&str>,
    content_disposition: Option<&str>,
    is_main_frame: bool,
    mime_type_supported: bool,
) -> bool {
    // Subresources (fetch, XHR, RSC components, images, scripts, fonts) must NEVER be diverted into browser file downloads.
    if !is_main_frame {
        return false;
    }

    let is_explicit_attachment = content_disposition
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("attachment"));

    // Server explicitly commanded the browser to download as attachment
    if is_explicit_attachment {
        return true;
    }

    let mime = mime_type.unwrap_or_default().trim().to_ascii_lowercase();
    let is_html_or_text = mime.starts_with("text/html")
        || mime.starts_with("application/xhtml+xml")
        || mime.starts_with("text/plain")
        || mime.starts_with("application/json");

    // Ordinary web pages and supported documents must always be rendered in the view
    if is_html_or_text || mime_type_supported {
        return false;
    }

    // Binary / unsupported files (apk, exe, zip, deb, iso, etc.)
    true
}

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
        settings.set_allow_top_navigation_to_data_urls(true);
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
        ucm.register_script_message_handler("railDownloads", None);
        ucm.register_script_message_handler("railPwaInstall", None);

        let html_css = load_html_css();
        if css_has_active_content(&html_css) {
            ucm.add_style_sheet(&user_style_sheet(&html_css));
        }
        crate::web::inject_context_menu_bridge(&ucm);
        crate::pwa::inject_pwa_bridge(&ucm);
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

    #[allow(deprecated)]
    pub(crate) fn configure_native_web_view(view: &WebView) {
        // Paint the WebView before its first document commits. Without this, GTK/WebKit
        // can expose an unpainted black frame while a new tab's initial HTML is parsed.
        view.set_background_color(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0));

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
                        if let Some(obj) = model.item(i)
                            && let Ok(file) = obj.downcast::<gio::File>()
                        {
                            uris.push(file.uri().to_string());
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
            if decision_type == webkit6::PolicyDecisionType::Response {
                if let Some(resp_decision) =
                    decision.downcast_ref::<webkit6::ResponsePolicyDecision>()
                {
                    if !resp_decision.is_main_frame_main_resource() {
                        return false;
                    }
                    if let Some(response) = resp_decision.response() {
                        let mime = response.mime_type();
                        let disposition = response
                            .http_headers()
                            .and_then(|headers| headers.one("Content-Disposition"));

                        if should_download_response(
                            mime.as_deref(),
                            disposition.as_deref(),
                            true,
                            resp_decision.is_mime_type_supported(),
                        ) {
                            resp_decision.download();
                            return true;
                        }
                    }
                }
            } else if decision_type == webkit6::PolicyDecisionType::NavigationAction {
                if let Some(nav_decision) =
                    decision.downcast_ref::<webkit6::NavigationPolicyDecision>()
                    && let Some(req) = nav_decision.navigation_action().and_then(|a| a.request())
                    && let Some(uri) = req.uri()
                {
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
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        // Attach the WebView to GTK before starting its first navigation. Loading
        // while the widget is still unparented can leave a visible blank/black frame.
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view.clone(), prof_id.clone(), false);
        self.set_view_saved_uri(&view, "rail://start/");
        view.load_html(&start_page(), Some("rail://start/"));
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
                    if !app.ov.get() && app.focused_view().as_ref() == Some(&v) {
                        app.focus_welcome_input(false);
                    }
                }
            });
        }
    }

    pub(crate) fn split_vertical(self: &Rc<Self>) {
        let prof_id = self.profiles.borrow().active_id().to_string();
        let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
        let view = Self::create_web_view(&session, &prof_id);
        *self.fresh.borrow_mut() = Some(view.clone());
        self.add_view(view.clone(), prof_id.clone(), true);
        view.load_html(&start_page(), Some("rail://start/"));
        self.select_palette_profile(&prof_id);
        if self.palette_open() {
            self.hide_palette();
        }
        self.entry.set_text("");
        self.last_entry_len.set(0);
        for ms in [30u64, 120, 350] {
            let app = self.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
                if !app.ov.get()
                    && app.focused_is_welcome()
                    && let Some(v) = app.focused_view()
                {
                    app.push_start_state(&v);
                    app.focus_welcome_input(false);
                }
            });
        }
        if !self.ov.get() {
            self.entry.grab_focus();
        }
    }

    pub(crate) fn new_tab_with_url_and_profile(self: &Rc<Self>, url: &str, profile_id: &str) {
        let session = self.profiles.borrow_mut().get_or_create_session(profile_id);
        let view = Self::create_web_view(&session, profile_id);
        if is_internal_uri(url) {
            load_internal_page(&view, url);
        } else {
            view.load_uri(url);
        }
        self.add_view(view.clone(), profile_id.to_string(), false);
        self.set_view_saved_uri(&view, url);
        self.pill_label.set_text(&self.focused_display_uri());
    }

    pub(crate) fn new_tab_with_url(self: &Rc<Self>, url: &str) {
        let prof_id = self.profiles.borrow().active_id().to_string();
        self.new_tab_with_url_and_profile(url, &prof_id);
    }

    pub(crate) fn open_image_in_new_tab(self: &Rc<Self>, opener_view: &WebView, image_url: &str) {
        let parent_prof = self
            .find_view_profile(opener_view)
            .unwrap_or_else(|| self.profiles.borrow().active_profile().id);

        let resolved_url = if !image_url.contains("://")
            && !image_url.starts_with("data:")
            && !image_url.starts_with("blob:")
        {
            if let Some(base_uri) = opener_view.uri()
                && let Ok(resolved) = glib::Uri::resolve_relative(
                    Some(base_uri.as_str()),
                    image_url,
                    glib::UriFlags::NONE,
                )
            {
                resolved.to_string()
            } else {
                image_url.to_string()
            }
        } else {
            image_url.to_string()
        };

        let new_view = Self::create_related_web_view(opener_view);
        if (resolved_url.starts_with("http://") || resolved_url.starts_with("https://"))
            && let Some(opener_uri) = opener_view.uri()
            && !opener_uri.is_empty()
        {
            let req = webkit6::URIRequest::new(&resolved_url);
            if let Some(headers) = req.http_headers() {
                headers.append("Referer", &opener_uri);
            }
            new_view.load_request(&req);
        } else {
            new_view.load_uri(&resolved_url);
        }

        self.add_view(new_view.clone(), parent_prof, false);
        self.mark_keep_alive(opener_view);
        self.mark_keep_alive(&new_view);
        self.set_view_saved_uri(&new_view, &resolved_url);
        self.pill_label.set_text(&self.focused_display_uri());
    }

    pub(crate) fn open_link_in_new_tab(self: &Rc<Self>, opener_view: &WebView, url: &str) {
        let parent_prof = self
            .find_view_profile(opener_view)
            .unwrap_or_else(|| self.profiles.borrow().active_profile().id);
        let new_view = Self::create_related_web_view(opener_view);
        if is_internal_uri(url) {
            load_internal_page(&new_view, url);
        } else {
            new_view.load_uri(url);
        }
        self.add_view(new_view.clone(), parent_prof, false);
        self.mark_keep_alive(opener_view);
        self.mark_keep_alive(&new_view);
        self.set_view_saved_uri(&new_view, url);
        self.pill_label.set_text(&self.focused_display_uri());
    }

    pub(crate) fn open_link_in_split_view(self: &Rc<Self>, opener_view: &WebView, url: &str) {
        let parent_prof = self
            .find_view_profile(opener_view)
            .unwrap_or_else(|| self.profiles.borrow().active_profile().id);
        let new_view = Self::create_related_web_view(opener_view);
        if is_internal_uri(url) {
            load_internal_page(&new_view, url);
        } else {
            new_view.load_uri(url);
        }
        self.add_view(new_view.clone(), parent_prof, true);
        self.mark_keep_alive(opener_view);
        self.mark_keep_alive(&new_view);
        self.set_view_saved_uri(&new_view, url);
    }

    /// View Page Source (Ctrl+U): open immediately, fetch the original response off-thread.
    pub(crate) fn view_source_of_focused(self: &Rc<Self>) {
        let Some(v) = self.focused_view() else { return };
        let uri = v
            .uri()
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.saved_uri_of(&v));
        if uri.is_empty() || uri.starts_with("rail://") || uri.starts_with("view-source:") {
            return;
        }
        self.new_tab_with_url(&format!("view-source:{uri}"));
    }
}

#[cfg(test)]
mod response_policy_tests {
    use super::should_download_response;

    #[test]
    fn subresources_are_never_downloaded() {
        assert!(!should_download_response(
            Some("application/octet-stream"),
            Some("attachment"),
            false,
            false,
        ));
        assert!(!should_download_response(
            Some("text/x-component"),
            None,
            false,
            false,
        ));
    }

    #[test]
    fn html_navigation_is_never_downloaded_for_mime_or_filename_reasons() {
        assert!(!should_download_response(
            Some("text/html"),
            None,
            true,
            false,
        ));
        assert!(!should_download_response(
            Some("text/html; charset=utf-8"),
            None,
            true,
            false,
        ));
        assert!(!should_download_response(
            Some("application/xhtml+xml"),
            None,
            true,
            false,
        ));
    }

    #[test]
    fn explicit_attachment_header_wins_for_main_frame() {
        assert!(should_download_response(
            Some("text/html"),
            Some(" attachment; filename=project.html"),
            true,
            true,
        ));
        assert!(should_download_response(
            Some("application/pdf"),
            Some("ATTACHMENT; filename=report.pdf"),
            true,
            true,
        ));
    }

    #[test]
    fn unsupported_main_frame_payloads_download() {
        assert!(should_download_response(
            Some("application/vnd.android.package-archive"),
            None,
            true,
            false,
        ));
        assert!(should_download_response(
            Some("application/octet-stream"),
            None,
            true,
            false,
        ));
        assert!(!should_download_response(
            Some("application/pdf"),
            Some("inline"),
            true,
            true,
        ));
    }
}
