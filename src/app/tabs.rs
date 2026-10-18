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
        // Real WebKitGTK user agent: Rail does not spoof UA, platform, GPU or any other browser property.
        settings.set_enable_webrtc(webrtc_allowed());
        settings.set_enable_hyperlink_auditing(false);
        settings.set_enable_dns_prefetching(false);
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_webgl(true); // needed for smooth compositing on many sites
        settings.set_enable_developer_extras(true);
        settings.set_enable_back_forward_navigation_gestures(true);
        settings.set_enable_webaudio(true);
        settings.set_enable_html5_local_storage(true);
        settings.set_enable_html5_database(true);
        settings.set_enable_site_specific_quirks(true);
        settings.set_javascript_can_access_clipboard(true);
        // Always: smooth page scroll/compositing; falls back when GPU unavailable
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
        // Back/forward uses the page cache: instant history navigation instead of a reload
        settings.set_enable_page_cache(true);

        let ucm = webkit6::UserContentManager::new();
        ucm.register_script_message_handler("railgo", None);
        let html_css = load_html_css();
        if !html_css.trim().is_empty() {
            ucm.add_style_sheet(&user_style_sheet(&html_css));
        }
        STYLE_MANAGERS.with(|l| l.borrow_mut().push(ucm.downgrade()));
        ADBLOCK_FILTER.with(|f| {
            if let Some(f) = f.borrow().as_ref() {
                ucm.add_filter(f);
            }
        });

        WebView::builder()
            .network_session(session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .build()
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
        // Show the omnibar fixed under the brand on the welcome page
        self.select_palette_profile(&prof_id);
        self.show_palette();
        self.entry.set_text("");
        self.last_entry_len.set(0);
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
        self.show_palette();
        self.entry.set_text("");
        self.last_entry_len.set(0);
        self.entry.grab_focus();
    }

    pub(crate) fn new_tab_with_url_and_profile(self: &Rc<Self>, url: &str, profile_id: &str) {
        self.exit_overview();
        let session = self.profiles.borrow_mut().get_or_create_session(profile_id);
        let view = Self::create_web_view(&session, profile_id);
        view.load_uri(url);
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
        view.load_uri(url);
        self.add_view(view, prof_id, true);
    }
}
