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
            let Some(row) = rows.get_mut(cur_r) else {
                return;
            };
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

    /// Niri move-window-up/down: swap only with a neighbour inside the focused stack.
    pub(crate) fn move_window_vertical(self: &Rc<Self>, dir: isize) {
        let r = self.focus_row.get();
        let c = self.focus_col.get();
        let mut swapped = None;
        {
            let mut rows = self.rows.borrow_mut();
            if let Some(row) = rows.get_mut(r)
                && c < row.tabs.len()
            {
                let (s, e) = row.col_span(c);
                let t = c as isize + dir;
                if t >= s as isize && (t as usize) < e {
                    row.swap_keep_props(c, t as usize);
                    swapped = Some(t as usize);
                }
            }
        }
        if let Some(t) = swapped {
            self.sync_strip();
            self.focus_coords(r, t);
            self.schedule_save_session();
        }
    }

    /// Niri consume-window-into-column: take the top window from the column to the right
    /// and append it to the focused column. The focused window remains focused.
    pub(crate) fn consume_window_into_column(self: &Rc<Self>) {
        let row_index = self.focus_row.get();
        let focused_index = self.focus_col.get();
        let mut rows = self.rows.borrow_mut();
        let Some(row) = rows.get_mut(row_index) else {
            return;
        };
        if focused_index >= row.tabs.len() {
            return;
        }
        let (target_start, target_end) = row.col_span(focused_index);
        if target_end >= row.tabs.len() {
            return;
        }
        let target_id = row.tabs[target_start].col_id.get();
        // Row::detach transfers the right column's width/tabbed properties to its next
        // window when it is a stack, then makes the moved window independent.
        let moved = row.detach(target_end);
        moved.reset_col_props();
        moved.custom_height.set(None);
        moved.tabbed.set(false);
        moved.col_id.set(target_id);
        row.tabs.insert(target_end, moved);
        drop(rows);

        self.sync_strip();
        self.focus_coords(row_index, focused_index);
        self.schedule_save_session();
    }

    /// Niri expel-window-from-column: move the bottom window into its own column to the right.
    pub(crate) fn expel_window_from_column(self: &Rc<Self>) {
        let row_index = self.focus_row.get();
        let focused_index = self.focus_col.get();
        let new_focus;
        {
            let mut rows = self.rows.borrow_mut();
            let Some(row) = rows.get_mut(row_index) else {
                return;
            };
            if focused_index >= row.tabs.len() {
                return;
            }
            let (start, end) = row.col_span(focused_index);
            if end - start < 2 {
                return;
            }
            let bottom = end - 1;
            let focused_moves = focused_index == bottom;
            let moved = row.detach(bottom);
            moved.reset_col_props();
            moved.custom_height.set(None);
            moved.tabbed.set(false);
            let insert_at = bottom;
            row.tabs.insert(insert_at, moved);
            new_focus = if focused_moves {
                insert_at
            } else {
                focused_index
            };
        }
        self.sync_strip();
        self.focus_coords(row_index, new_focus);
        self.schedule_save_session();
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
                    tab.custom_height.set(None);
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
        self.move_tab_to_row_with_focus(target_r, true);
    }

    /// Niri move-window-to-workspace --focus=false: move the focused tab but keep focus
    /// on the source workspace (or its replacement if that workspace becomes empty).
    pub(crate) fn move_tab_to_row_keep_focus(self: &Rc<Self>, target_r: usize) {
        self.move_tab_to_row_with_focus(target_r, false);
    }

    fn move_tab_to_row_with_focus(self: &Rc<Self>, target_r: usize, follow: bool) {
        let source_r = self.focus_row.get();
        let source_c = self.focus_col.get();
        if source_r == target_r {
            return;
        }

        let (dest_r, dest_c, source_empty) = {
            let mut rows = self.rows.borrow_mut();
            if source_r >= rows.len() || source_c >= rows[source_r].tabs.len() {
                return;
            }
            let tab = rows[source_r].detach(source_c);
            let source_empty = rows[source_r].tabs.is_empty();
            while rows.len() <= target_r {
                rows.push(Row { tabs: Vec::new() });
            }
            rows[target_r].tabs.push(tab);
            let dest_c = rows[target_r].tabs.len() - 1;
            let mut dest_r = target_r;

            // Non-home empty workspaces disappear; keep all workspace indices coherent.
            if source_empty && source_r > 0 {
                rows.remove(source_r);
                if dest_r > source_r {
                    dest_r -= 1;
                }
            }
            (dest_r, dest_c, source_empty)
        };

        if source_empty && source_r > 0 {
            self.remap_previous_workspace_after_remove(source_r);
            let old = self.row_last_col.borrow().clone();
            let mut remapped = std::collections::HashMap::new();
            for (row, col) in old {
                if row == source_r {
                    continue;
                }
                remapped.insert(if row > source_r { row - 1 } else { row }, col);
            }
            *self.row_last_col.borrow_mut() = remapped;
        }

        self.sync_strip();
        if source_empty && source_r == 0 {
            // Rail always keeps its home workspace alive; create a replacement tab first.
            self.new_tab();
            if follow {
                self.focus_coords(dest_r, dest_c);
            }
        } else if follow {
            if source_empty {
                self.focus_row.set(dest_r);
            }
            self.focus_coords(dest_r, dest_c);
        } else if source_empty {
            let rows = self.rows.borrow();
            if rows.is_empty() {
                return;
            }
            let keep_r = source_r.min(rows.len() - 1);
            let keep_c = self
                .row_last_col
                .borrow()
                .get(&keep_r)
                .copied()
                .unwrap_or(0)
                .min(rows[keep_r].tabs.len().saturating_sub(1));
            drop(rows);
            self.focus_row.set(keep_r);
            self.focus_coords(keep_r, keep_c);
        } else {
            let rows = self.rows.borrow();
            let keep_c = source_c.min(rows[source_r].tabs.len().saturating_sub(1));
            drop(rows);
            self.focus_coords(source_r, keep_c);
        }
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    pub(crate) fn move_tab_to_row_pos(
        self: &Rc<Self>,
        from_r: usize,
        from_c: usize,
        to_r: usize,
        to_c: usize,
    ) {
        if from_r == to_r {
            self.reorder_tab_in_row(from_r, from_c, to_c);
            return;
        }

        let (dest_r, insert_c, source_empty) = {
            let mut rows = self.rows.borrow_mut();
            if from_r >= rows.len() || from_c >= rows[from_r].tabs.len() {
                return;
            }
            let tab = rows[from_r].detach(from_c);
            let source_empty = rows[from_r].tabs.is_empty();
            while rows.len() <= to_r {
                rows.push(Row { tabs: Vec::new() });
            }
            let insert_c = to_c.min(rows[to_r].tabs.len());
            rows[to_r].tabs.insert(insert_c, tab);
            let mut dest_r = to_r;
            if source_empty && from_r > 0 {
                rows.remove(from_r);
                if dest_r > from_r {
                    dest_r -= 1;
                }
            }
            (dest_r, insert_c, source_empty)
        };

        if source_empty && from_r > 0 {
            self.remap_previous_workspace_after_remove(from_r);
            let old = self.row_last_col.borrow().clone();
            let mut remapped = std::collections::HashMap::new();
            for (row, col) in old {
                if row == from_r {
                    continue;
                }
                remapped.insert(if row > from_r { row - 1 } else { row }, col);
            }
            *self.row_last_col.borrow_mut() = remapped;
            self.focus_row.set(dest_r);
        }

        if source_empty && from_r == 0 {
            // Keep Rail's permanent home workspace alive when its last tab is dragged away.
            self.new_tab();
        } else {
            self.sync_strip();
        }
        self.focus_coords(dest_r, insert_c);
        self.refresh_sidebar();
        self.schedule_save_session();
    }
}
