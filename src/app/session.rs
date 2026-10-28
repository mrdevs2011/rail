//! Saving and restoring the session.

use crate::*;

impl App {
    pub(crate) fn save_session_internal(&self, sync: bool) {
        let rows = self.rows.borrow();
        let total_tabs: usize = rows.iter().map(|r| r.tabs.len()).sum();
        let dir = rail_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.txt");
        let temp_path = dir.join("session.txt.tmp");

        if total_tabs == 0 {
            *self.last_saved_session.borrow_mut() = String::new();
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(&temp_path);
            return;
        }

        let mut out = String::new();
        out.push_str(&format!("focus\t{}\t{}\n", self.focus_row.get(), self.focus_col.get()));
        out.push_str(&format!("sidebar\t{}\n", if self.side_open.get() { 1 } else { 0 }));
        out.push_str(&format!("engine\t{}\n", self.search_engine.borrow()));
        let win_w = self.win.width();
        let win_h = self.win.height();
        let is_max = if self.win.is_maximized() { 1 } else { 0 };
        out.push_str(&format!("win\t{}\t{}\t{}\n", win_w, win_h, is_max));

        for (ri, row) in rows.iter().enumerate() {
            for (ci, tab) in row.tabs.iter().enumerate() {
                let is_full = if tab.container.has_css_class("full") { 1 } else { 0 };
                let uri = tab.real_uri();
                if let Some(cw) = tab.custom_width.get() {
                    out.push_str(&format!("tab\t{}\t{}\t{}\t{}\t{}\t{:.1}\n", ri, ci, is_full, tab.profile_id, uri, cw));
                } else {
                    out.push_str(&format!("tab\t{}\t{}\t{}\t{}\t{}\n", ri, ci, is_full, tab.profile_id, uri));
                }
                if ci > 0 && row.tabs[ci - 1].col_id.get() == tab.col_id.get() {
                    out.push_str(&format!("stack\t{}\t{}\n", ri, ci));
                }
            }
        }
        drop(rows);

        // Optimization: if session content hasn't changed, skip disk write completely (prevents SSD wear)
        if *self.last_saved_session.borrow() == out {
            return;
        }

        // Atomic write via temp file + atomic rename
        let write_ok = if let Ok(mut file) = std::fs::File::create(&temp_path) {
            use std::io::Write;
            if file.write_all(out.as_bytes()).is_ok() {
                if sync {
                    let _ = file.sync_all(); // Flush physical hardware to SSD before exit / reboot
                }
                std::fs::rename(&temp_path, &path).is_ok()
            } else {
                false
            }
        } else {
            false
        };

        if !write_ok {
            let _ = std::fs::write(&path, &out);
        }

        *self.last_saved_session.borrow_mut() = out;
    }

    pub(crate) fn schedule_save_session(self: &Rc<Self>) {
        let current_gen = self.save_session_gen.get().wrapping_add(1);
        self.save_session_gen.set(current_gen);
        let app = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(1000), move || {
            if app.save_session_gen.get() == current_gen {
                app.save_session_internal(false);
            }
        });
    }

    pub(crate) fn save_session_immediate(&self) {
        self.save_session_gen.set(self.save_session_gen.get().wrapping_add(1));
        self.save_session_internal(true);
    }

    pub(crate) fn restore_session(self: &Rc<Self>) -> bool {
        let path = rail_data_dir().join("session.txt");
        let Ok(content) = std::fs::read_to_string(path) else { return false; };
        let state = parse_session_content(&content);
        if state.tabs.is_empty() {
            return false;
        }
        *self.last_saved_session.borrow_mut() = content;

        if let Some((w, h, max)) = state.win_size {
            if w > 400 && h > 300 {
                self.win.set_default_size(w, h);
            }
            if max {
                self.win.maximize();
            }
        }


        if let Some(engine) = state.search_engine {
            *self.search_engine.borrow_mut() = engine;
        }

        if let Some(open) = state.sidebar_open {
            self.side_open.set(open);
            self.side.set_visible(open);
            self.shell_layout.set_side(if open { SIDE_W as f64 } else { 0.0 });
        }

        let mut tab_items = state.tabs;
        tab_items.sort_by_key(|t| (t.row, t.col));

        let mut max_row = 0;
        for t in &tab_items {
            if t.row > max_row { max_row = t.row; }
        }

        // Clear any existing tabs before loading restored session
        {
            let mut rows = self.rows.borrow_mut();
            for r in rows.iter_mut() {
                for t in r.tabs.drain(..) {
                    self.strip.remove(&t.container);
                }
            }
            rows.clear();
        }

        let mut loaded_tabs: Vec<(usize, Tab)> = Vec::new();
        // Lazy restore: only the focused tab and its nearest neighbours in the same row load now.
        // Every other web tab stays suspended (no process, no network) and wakes when focused.
        // Loading dozens of tabs at once is what made startup freeze.
        let (focus_r, focus_c) = state.focus;
        let mut row_pos: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for item in tab_items {
            let pos = {
                let e = row_pos.entry(item.row).or_insert(0);
                let v = *e;
                *e += 1;
                v
            };
            let lazy = item.url.starts_with("http")
                && !(item.row == focus_r && pos.abs_diff(focus_c) <= 2);
            let session = self.profiles.borrow_mut().get_or_create_session(&item.profile_id);
            let view = Self::create_web_view(&session, &item.profile_id);
            if lazy {
                // no load_uri: the page is fetched on first focus (see focus_coords)
            } else if item.url.is_empty() || item.url.starts_with("about:") || item.url.starts_with("rail://start") {
                view.load_html(&start_page(), Some("rail://start/"));
            } else if item.url.starts_with("rail://settings") {
                view.load_html(&settings_shell_page(), Some(&item.url));
            } else if item.url.starts_with("rail://downloads") {
                // downloads is pure HTML; will be refreshed later if needed
                view.load_html(&start_page(), Some("rail://downloads")); // placeholder; real content loaded on focus/refresh
            } else {
                view.load_uri(&item.url);
            }
            let tw = Self::create_tab_widget(&view);
            if !item.full {
                tw.container.remove_css_class("full");
            }
            self.setup_tab_signals(
                &tw.container,
                &view,
                &tw.resize_handle,
                &tw.height_handle,
                &tw.dt_handle,
                &tw.dt_box,
            );
            if lazy {
                // free the idle web process WebKit may have spawned for the empty view
                view.terminate_web_process();
            }
            loaded_tabs.push((item.row, Tab {
                container: tw.container,
                view,
                freeze: tw.freeze,
                height_handle: tw.height_handle,
                profile_id: item.profile_id,
                custom_width: Cell::new(item.custom_width),
                custom_height: Cell::new(None),
                col_id: Cell::new(new_col_id()),
                saved_uri: RefCell::new(if item.url.starts_with("http") || item.url.starts_with("rail://") { item.url.clone() } else if item.url.is_empty() || item.url.starts_with("about:") { "rail://start/".to_string() } else { String::new() }),
                suspended: Cell::new(lazy),
                keep_alive: Cell::new(false),
                last_active: Cell::new(if lazy { std::time::Instant::now().checked_sub(Duration::from_secs(SUSPEND_AFTER_SECS + 1)).unwrap_or_else(std::time::Instant::now) } else { std::time::Instant::now() }),
                dt_box: tw.dt_box,
                dt_handle: tw.dt_handle,
                dt_open: Cell::new(false),
                dt_height: Cell::new(self.default_devtools_height.get()),
            }));
        }

        let mut rows = self.rows.borrow_mut();
        while rows.len() <= max_row {
            rows.push(Row { tabs: Vec::new() });
        }
        for (row_idx, tab) in loaded_tabs {
            rows[row_idx].tabs.push(tab);
        }
        let mut stacked = state.stacked.clone();
        stacked.sort();
        for (sr, sc) in stacked {
            if let Some(row) = rows.get(sr) {
                if sc > 0 && sc < row.tabs.len() {
                    let id = row.tabs[sc - 1].col_id.get();
                    row.tabs[sc].col_id.set(id);
                }
            }
        }
        drop(rows);

        self.sync_strip();
        self.focus_coords(state.focus.0, state.focus.1);
        // Ensure settings chrome is attached for any restored settings tabs
        self.sync_settings_chrome();
        true
    }

    pub(crate) fn toggle_full(self: &Rc<Self>) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let rows = self.rows.borrow();
        let Some(row) = rows.get(cur_r) else { return };
        if cur_c >= row.tabs.len() {
            return;
        }
        // the width belongs to the whole column: it lives on its first tab
        let head = row.col_span(cur_c).0;
        let tab = &row.tabs[head];
        tab.custom_width.set(None);
        if tab.container.has_css_class("full") {
            tab.container.remove_css_class("full");
        } else {
            tab.container.add_css_class("full");
        }
        drop(rows);
        self.layout.set_column_width(cur_r, head, None);
        self.sync_strip();
        self.reveal_focus();
        self.schedule_save_session();
    }
}
