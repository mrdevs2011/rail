//! Focus helpers, clipboard image paste and window title.

use crate::*;

pub(crate) struct TabWidgets {
    pub(crate) container: gtk::Box,
    pub(crate) resize_handle: gtk::Box,
    pub(crate) height_handle: gtk::Box,
    pub(crate) freeze: gtk::Picture,
    pub(crate) dt_box: gtk::Box,
    pub(crate) dt_handle: gtk::Box,
}

impl App {
    pub(crate) fn focused_view(&self) -> Option<WebView> {
        let rows = self.rows.borrow();
        rows.get(self.focus_row.get())
            .and_then(|r| r.tabs.get(self.focus_col.get()))
            .map(|t| t.view.clone())
    }

    /// True when the clipboard offers an image but no text. WebKitGTK does not expose
    /// clipboard images through DataTransfer (WebKit bug 218519), so such a paste must be
    /// replayed into the page by Rail.
    pub(crate) fn clipboard_is_image_only(&self) -> bool {
        let f = self.win.clipboard().formats();
        f.contains_type(<gdk::Texture as glib::prelude::StaticType>::static_type())
            && !f.contain_mime_type("text/plain")
            && !f.contain_mime_type("text/plain;charset=utf-8")
    }

    /// Reads the clipboard image natively and dispatches a synthetic `paste` event
    /// carrying it as a File, so pages (Gemini etc.) see it in clipboardData.files.
    pub(crate) fn paste_clipboard_image(self: &Rc<Self>, view: WebView) {
        self.win.clipboard().read_texture_async(gio::Cancellable::NONE, move |res| {
            let Ok(Some(tex)) = res else { return };
            let png = tex.save_to_png_bytes();
            let b64 = glib::base64_encode(&png[..]);
            let js = format!(
                "(function(){{try{{var b=atob('{}');var u=new Uint8Array(b.length);\
                 for(var i=0;i<b.length;i++)u[i]=b.charCodeAt(i);\
                 var f=new File([u],'image.png',{{type:'image/png'}});\
                 var dt=new DataTransfer();dt.items.add(f);\
                 var t=document.activeElement||document.body;\
                 t.dispatchEvent(new ClipboardEvent('paste',{{clipboardData:dt,bubbles:true,cancelable:true,composed:true}}));\
                 }}catch(e){{}}}})();",
                b64
            );
            view.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        });
    }

    /// Ctrl+V with an image-only clipboard while a page has focus: handled by Rail.
    pub(crate) fn try_paste_image(self: &Rc<Self>) -> bool {
        let Some(view) = self.focused_view() else { return false };
        let page_focused = match gtk::prelude::GtkWindowExt::focus(&self.win) {
            Some(w) => w == *view.upcast_ref::<gtk::Widget>() || w.is_ancestor(&view),
            None => false,
        };
        if !page_focused || !self.clipboard_is_image_only() {
            return false;
        }
        self.paste_clipboard_image(view);
        true
    }

    pub(crate) fn palette_open(&self) -> bool {
        gtk::prelude::WidgetExt::is_visible(&self.palette)
    }

    pub(crate) fn overview_open(&self) -> bool {
        self.ov.get()
    }

    pub(crate) fn create_tab_widget(view: &WebView) -> TabWidgets {
        view.add_css_class("tab");
        view.set_overflow(gtk::Overflow::Hidden);
        view.set_vexpand(true);
        view.set_hexpand(true);

        let freeze = gtk::Picture::new();
        freeze.add_css_class("tab-freeze");
        freeze.set_can_target(false);
        freeze.set_content_fit(gtk::ContentFit::Fill);
        freeze.set_hexpand(true);
        freeze.set_vexpand(true);
        freeze.set_visible(false);

        let height_handle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        height_handle.add_css_class("tab-height-handle");
        height_handle.set_cursor_from_name(Some("row-resize"));
        height_handle.set_valign(gtk::Align::End);
        height_handle.set_halign(gtk::Align::Fill);
        height_handle.set_hexpand(true);
        height_handle.set_size_request(-1, 8);
        height_handle.set_visible(false);

        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_child(Some(view));
        overlay.add_overlay(&freeze);
        overlay.add_overlay(&height_handle);

        let dt_handle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dt_handle.add_css_class("devtools-resize-handle");
        dt_handle.set_cursor_from_name(Some("ns-resize"));
        dt_handle.set_visible(false);

        let dt_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        dt_box.add_css_class("devtools-content-area");
        dt_box.add_css_class("devtools-tab-dock");
        dt_box.set_hexpand(true);
        dt_box.set_vexpand(false);
        dt_box.set_visible(false);

        let tab_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        tab_vbox.set_hexpand(true);
        tab_vbox.set_vexpand(true);
        tab_vbox.append(&overlay);
        tab_vbox.append(&dt_handle);
        tab_vbox.append(&dt_box);

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.add_css_class("rail-col");
        container.add_css_class("full");
        container.set_vexpand(true);
        container.set_hexpand(true);

        let resize_handle = gtk::Box::new(gtk::Orientation::Vertical, 0);
        resize_handle.add_css_class("tab-resize-handle");
        resize_handle.set_cursor_from_name(Some("col-resize"));

        container.append(&tab_vbox);
        container.append(&resize_handle);

        TabWidgets {
            container,
            resize_handle,
            height_handle,
            freeze,
            dt_box,
            dt_handle,
        }
    }

    pub(crate) fn set_tab_width(&self, r: usize, c: usize, width: f64) {
        let vw = self.strip.width() as f64;
        let clamped = width.clamp(280.0, vw);
        let rows = self.rows.borrow();
        if let Some(row) = rows.get(r) {
            if let Some(tab) = row.tabs.get(c) {
                tab.custom_width.set(Some(clamped));
                if (clamped - vw).abs() < 10.0 {
                    tab.container.add_css_class("full");
                } else {
                    tab.container.remove_css_class("full");
                }
            }
        }
        drop(rows);
        self.layout.set_column_width(r, c, Some(clamped));
        self.strip.queue_allocate();
    }

    pub(crate) fn update_window_title(&self) {
        let focused_title = self.focused_view().and_then(|v| {
            v.title()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty())
                .or_else(|| v.uri().and_then(|u| host_of(&u)))
        });
        let title = match focused_title {
            Some(t) => {
                let clean = t.trim();
                if clean.to_lowercase().contains("rail") {
                    clean.to_string()
                } else {
                    format!("{} — rail", clean)
                }
            }
            None => "rail".to_string(),
        };
        self.win.set_title(Some(&title));
    }

    pub(crate) fn sync_strip(&self) {
        let rows = self.rows.borrow();
        let mut grid = Vec::new();
        let mut custom_widths = Vec::new();
        let mut custom_heights = Vec::new();
        let mut col_ids = Vec::new();
        let mut prev: Option<gtk::Widget> = None;
        for row in rows.iter() {
            let mut row_grid = Vec::new();
            let mut row_widths = Vec::new();
            let mut row_heights = Vec::new();
            let mut row_ids = Vec::new();
            for (ci, tab) in row.tabs.iter().enumerate() {
                row_grid.push(tab.container.has_css_class("full"));
                row_widths.push(tab.custom_width.get());
                row_heights.push(tab.custom_height.get());
                row_ids.push(tab.col_id.get());
                // Height handle only between stacked neighbors
                let has_below = row
                    .tabs
                    .get(ci + 1)
                    .map(|t| t.col_id.get() == tab.col_id.get())
                    .unwrap_or(false);
                tab.height_handle.set_visible(has_below);
                if tab.container.parent().as_ref() != Some(self.strip.upcast_ref()) {
                    self.strip.append(&tab.container);
                }
                self.strip.reorder_child_after(&tab.container, prev.as_ref());
                prev = Some(tab.container.clone().upcast());
            }
            grid.push(row_grid);
            custom_widths.push(row_widths);
            custom_heights.push(row_heights);
            col_ids.push(row_ids);
        }
        drop(rows);
        self.layout.set_grid(grid);
        self.layout.set_custom_widths(custom_widths);
        self.layout.set_custom_heights(custom_heights);
        self.layout.set_col_ids(col_ids);
        self.strip.queue_allocate();
    }
}
