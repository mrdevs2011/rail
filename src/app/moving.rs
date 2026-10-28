//! Moving tabs and columns: reorder, stack / unstack, move between rows.

use crate::*;

impl App {
    /// Moves the whole column (all of its stacked windows) one place left or right.
    pub(crate) fn move_column(self: &Rc<Self>, dir: isize) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        let new_c;
        {
            let mut rows = self.rows.borrow_mut();
            let Some(row) = rows.get_mut(cur_r) else { return };
            if cur_c >= row.tabs.len() {
                return;
            }
            let (s, e) = row.col_span(cur_c);
            if dir > 0 {
                if e >= row.tabs.len() {
                    return;
                }
                let (_, e2) = row.col_span(e);
                row.tabs[s..e2].rotate_left(e - s);
                new_c = cur_c + (e2 - e);
            } else {
                if s == 0 {
                    return;
                }
                let (p, _) = row.col_span(s - 1);
                row.tabs[p..e].rotate_right(e - s);
                new_c = cur_c - (s - p);
            }
        }
        self.sync_strip();
        self.focus_coords(cur_r, new_c);
        self.schedule_save_session();
    }

    /// Alt+Shift+J/K: swap the window with its neighbour inside a stacked column; when it is
    /// not stacked in that direction, move it to the neighbouring workspace instead.
    pub(crate) fn move_window_vertical(self: &Rc<Self>, dir: isize) {
        let r = self.focus_row.get();
        let c = self.focus_col.get();
        let mut swapped = None;
        {
            let mut rows = self.rows.borrow_mut();
            if let Some(row) = rows.get_mut(r) {
                if c < row.tabs.len() {
                    let (s, e) = row.col_span(c);
                    let t = c as isize + dir;
                    if t >= s as isize && (t as usize) < e {
                        row.swap_keep_props(c, t as usize);
                        swapped = Some(t as usize);
                    }
                }
            }
        }
        if let Some(t) = swapped {
            self.sync_strip();
            self.focus_coords(r, t);
            self.schedule_save_session();
        } else if dir > 0 {
            self.move_tab_to_row(r + 1);
        } else if r > 0 {
            self.move_tab_to_row(r - 1);
        }
    }

    /// niri's consume-or-expel-window: a window that is alone in its column joins the
    /// neighbouring column (stacked); a stacked window leaves its column and becomes a new
    /// column on that side.
    pub(crate) fn consume_or_expel(self: &Rc<Self>, dir: isize) {
        let r = self.focus_row.get();
        let c = self.focus_col.get();
        let new_c;
        {
            let mut rows = self.rows.borrow_mut();
            let Some(row) = rows.get_mut(r) else { return };
            if c >= row.tabs.len() {
                return;
            }
            let (s, e) = row.col_span(c);
            if e - s == 1 {
                if dir < 0 {
                    if s == 0 {
                        return;
                    }
                    let left_id = row.tabs[s - 1].col_id.get();
                    row.tabs[c].reset_col_props();
                    row.tabs[c].col_id.set(left_id);
                } else {
                    if e >= row.tabs.len() {
                        return;
                    }
                    // this tab becomes the top (first) window of the right column and takes its width
                    let right_id = row.tabs[e].col_id.get();
                    row.move_col_props(e, c);
                    row.tabs[c].col_id.set(right_id);
                }
                new_c = c;
            } else if dir < 0 {
                if c == s {
                    row.move_col_props(s, s + 1);
                    row.tabs[c].col_id.set(new_col_id());
                    new_c = c;
                } else {
                    let tab = row.tabs.remove(c);
                    tab.reset_col_props();
                    tab.col_id.set(new_col_id());
                    row.tabs.insert(s, tab);
                    new_c = s;
                }
            } else if c == e - 1 {
                row.tabs[c].reset_col_props();
                row.tabs[c].col_id.set(new_col_id());
                new_c = c;
            } else {
                if c == s {
                    row.move_col_props(s, s + 1);
                }
                let tab = row.tabs.remove(c);
                tab.reset_col_props();
                tab.col_id.set(new_col_id());
                row.tabs.insert(e - 1, tab);
                new_c = e - 1;
            }
        }
        self.sync_strip();
        self.focus_coords(r, new_c);
        self.schedule_save_session();
    }

    pub(crate) fn reorder_tab_in_row(self: &Rc<Self>, r: usize, from_c: usize, to_c: usize) {
        if from_c == to_c {
            return;
        }
        let mut rows = self.rows.borrow_mut();
        if r >= rows.len() || from_c >= rows[r].tabs.len() || to_c >= rows[r].tabs.len() {
            return;
        }
        let tab = rows[r].detach(from_c);
        rows[r].tabs.insert(to_c, tab);
        drop(rows);
        self.sync_strip();
        self.focus_coords(r, to_c);
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    pub(crate) fn move_tab_to_row(self: &Rc<Self>, target_r: usize) {
        let cur_r = self.focus_row.get();
        let cur_c = self.focus_col.get();
        if cur_r == target_r {
            return;
        }

        let mut rows = self.rows.borrow_mut();
        if cur_r >= rows.len() || cur_c >= rows[cur_r].tabs.len() {
            return;
        }

        let tab = rows[cur_r].detach(cur_c);
        let cur_row_empty = rows[cur_r].tabs.is_empty();

        while rows.len() <= target_r {
            rows.push(Row { tabs: Vec::new() });
        }
        rows[target_r].tabs.push(tab);
        let dest_c = rows[target_r].tabs.len() - 1;

        let need_fresh_in_row_0 = cur_row_empty && cur_r == 0;
        drop(rows);

        if need_fresh_in_row_0 {
            let prof_id = self.profiles.borrow().active_profile().id;
            let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
            let view = Self::create_web_view(&session, &prof_id);
            view.load_html(&start_page(), Some("rail://start/"));
            view.set_can_target(!self.ov.get());
            let tw = Self::create_tab_widget(&view);
            self.setup_tab_signals(
                &tw.container,
                &view,
                &tw.resize_handle,
                &tw.height_handle,
                &tw.dt_handle,
                &tw.dt_box,
            );

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab {
                container: tw.container,
                view,
                freeze: tw.freeze,
                height_handle: tw.height_handle,
                profile_id: prof_id,
                custom_width: Cell::new(None),
                custom_height: Cell::new(None),
                col_id: Cell::new(new_col_id()),
                saved_uri: RefCell::new("rail://start/".to_string()),
                suspended: Cell::new(false),
                keep_alive: Cell::new(false),
                last_active: Cell::new(std::time::Instant::now()),
                dt_box: tw.dt_box,
                dt_handle: tw.dt_handle,
                dt_open: Cell::new(false),
                dt_height: Cell::new(self.default_devtools_height.get()),
            });
            drop(rows);
        } else if cur_row_empty && cur_r > 0 {
            let mut rows = self.rows.borrow_mut();
            while rows.len() > 1 && rows.last().map_or(false, |r| r.tabs.is_empty()) {
                rows.pop();
            }
            drop(rows);
        }

        self.sync_strip();
        self.focus_coords(target_r, dest_c);
    }

    pub(crate) fn move_tab_to_row_pos(self: &Rc<Self>, from_r: usize, from_c: usize, to_r: usize, to_c: usize) {
        if from_r == to_r {
            self.reorder_tab_in_row(from_r, from_c, to_c);
            return;
        }

        let mut rows = self.rows.borrow_mut();
        if from_r >= rows.len() || from_c >= rows[from_r].tabs.len() {
            return;
        }

        let tab = rows[from_r].detach(from_c);
        let cur_row_empty = rows[from_r].tabs.is_empty();

        while rows.len() <= to_r {
            rows.push(Row { tabs: Vec::new() });
        }
        let insert_c = to_c.min(rows[to_r].tabs.len());
        rows[to_r].tabs.insert(insert_c, tab);

        let need_fresh_in_row_0 = cur_row_empty && from_r == 0;
        drop(rows);

        if need_fresh_in_row_0 {
            let prof_id = self.profiles.borrow().active_profile().id;
            let session = self.profiles.borrow_mut().get_or_create_session(&prof_id);
            let view = Self::create_web_view(&session, &prof_id);
            view.load_html(&start_page(), Some("rail://start/"));
            view.set_can_target(!self.ov.get());
            let tw = Self::create_tab_widget(&view);
            self.setup_tab_signals(
                &tw.container,
                &view,
                &tw.resize_handle,
                &tw.height_handle,
                &tw.dt_handle,
                &tw.dt_box,
            );

            let mut rows = self.rows.borrow_mut();
            rows[0].tabs.push(Tab {
                container: tw.container,
                view,
                freeze: tw.freeze,
                height_handle: tw.height_handle,
                profile_id: prof_id,
                custom_width: Cell::new(None),
                custom_height: Cell::new(None),
                col_id: Cell::new(new_col_id()),
                saved_uri: RefCell::new("rail://start/".to_string()),
                suspended: Cell::new(false),
                keep_alive: Cell::new(false),
                last_active: Cell::new(std::time::Instant::now()),
                dt_box: tw.dt_box,
                dt_handle: tw.dt_handle,
                dt_open: Cell::new(false),
                dt_height: Cell::new(self.default_devtools_height.get()),
            });
            drop(rows);
        } else if cur_row_empty && from_r > 0 {
            let mut rows = self.rows.borrow_mut();
            while rows.len() > 1 && rows.last().map_or(false, |r| r.tabs.is_empty()) {
                rows.pop();
            }
            drop(rows);
        }

        self.sync_strip();
        self.focus_coords(to_r, insert_c);
        self.schedule_save_session();
    }
}
