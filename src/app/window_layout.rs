//! Niri-inspired column and stacked-window actions for Rail's 2D browser workspace.

use crate::*;

impl App {
    fn focused_column(&self) -> Option<(usize, usize, usize, f64)> {
        let row_index = self.focus_row.get();
        let tab_index = self.focus_col.get();
        let viewport = self.strip.width() as f64;
        if viewport <= 0.0 {
            return None;
        }
        let rows = self.rows.borrow();
        let row = rows.get(row_index)?;
        if tab_index >= row.tabs.len() {
            return None;
        }
        let (start, end) = row.col_span(tab_index);
        let head = &row.tabs[start];
        let width = clamp_column_width(
            head.custom_width
                .get()
                .unwrap_or_else(|| col_width(head.container.has_css_class("full"), viewport)),
            viewport,
        );
        Some((row_index, start, end, width))
    }

    /// Niri center-column: center only the focused column when explicitly requested.
    pub(crate) fn center_focused_column(self: &Rc<Self>) {
        let row_index = self.focus_row.get();
        let column_index = self.focus_col.get();
        let viewport = self.strip.width() as f64;
        let zoom = self.target_zoom.get().max(0.01);
        if viewport <= 0.0 {
            return;
        }
        let rows = self.rows.borrow();
        let Some(row) = rows.get(row_index) else {
            return;
        };
        let columns = row_cols(row, viewport, gap(zoom));
        let Some((_, _, x, width)) = columns
            .iter()
            .find(|&&(start, len, _, _)| column_index >= start && column_index < start + len)
        else {
            return;
        };
        let row_width = columns
            .last()
            .map_or(0.0, |&(_, _, left, width)| left + width);
        let visible_width = viewport / zoom;
        let target = centered_offset(visible_width, *x, *x + *width, row_width - visible_width);
        drop(rows);
        self.target_x.set(target);
        self.animate();
    }

    /// Niri center-visible-columns: center the currently visible horizontal group as one span.
    pub(crate) fn center_visible_columns(self: &Rc<Self>) {
        let row_index = self.focus_row.get();
        let viewport = self.strip.width() as f64;
        let zoom = self.target_zoom.get().max(0.01);
        if viewport <= 0.0 {
            return;
        }
        let visible_width = viewport / zoom;
        let offset = self.target_x.get();
        let rows = self.rows.borrow();
        let Some(row) = rows.get(row_index) else {
            return;
        };
        let columns = row_cols(row, viewport, gap(zoom));
        let mut visible_start = f64::INFINITY;
        let mut visible_end = f64::NEG_INFINITY;
        for &(_, _, x, width) in &columns {
            if x + width > offset && x < offset + visible_width {
                visible_start = visible_start.min(x);
                visible_end = visible_end.max(x + width);
            }
        }
        if !visible_start.is_finite() || !visible_end.is_finite() {
            return;
        }
        let row_width = columns
            .last()
            .map_or(0.0, |&(_, _, left, width)| left + width);
        let target = centered_offset(
            visible_width,
            visible_start,
            visible_end,
            row_width - visible_width,
        );
        drop(rows);
        self.target_x.set(target);
        self.animate();
    }

    /// Niri switch-preset-column-width: cycle between the configured 50% and 100% widths.
    pub(crate) fn cycle_column_width_preset(self: &Rc<Self>) {
        let Some((row, start, _, width)) = self.focused_column() else {
            return;
        };
        let viewport = self.strip.width() as f64;
        let ratio = width / viewport.max(1.0);
        let Some(next) = next_layout_preset(&COLUMN_WIDTH_PRESETS, ratio) else {
            return;
        };
        self.set_tab_width(row, start, viewport * next);
        self.schedule_save_session();
        self.reveal_focus();
    }

    /// Niri set-column-width +/-10%: the step is relative to the viewport, not the pixel width.
    pub(crate) fn adjust_column_width(self: &Rc<Self>, direction: i32) {
        if direction == 0 {
            return;
        }
        let Some((row, start, _, width)) = self.focused_column() else {
            return;
        };
        let viewport = self.strip.width() as f64;
        let next = clamp_column_width(
            width + viewport * 0.10 * direction.signum() as f64,
            viewport,
        );
        self.set_tab_width(row, start, next);
        self.schedule_save_session();
        self.reveal_focus();
    }

    /// Niri expand-column-to-available-width: fill the horizontal space left by peer columns.
    pub(crate) fn expand_column_to_available_width(self: &Rc<Self>) {
        let Some((row_index, start, _end, _)) = self.focused_column() else {
            return;
        };
        let viewport = self.strip.width() as f64;
        let gap_size = gap(1.0);
        let rows = self.rows.borrow();
        let Some(row) = rows.get(row_index) else {
            return;
        };
        let other_width: f64 = row
            .columns()
            .into_iter()
            .filter(|(column_start, _)| *column_start != start)
            .map(|(column_start, _)| {
                row.tabs[column_start]
                    .custom_width
                    .get()
                    .unwrap_or_else(|| {
                        col_width(
                            row.tabs[column_start].container.has_css_class("full"),
                            viewport,
                        )
                    })
                    + gap_size
            })
            .sum();
        let target = (viewport - other_width).max(MIN_COLUMN_WIDTH.min(viewport));
        drop(rows);
        self.set_tab_width(row_index, start, target.min(viewport));
        self.schedule_save_session();
        self.reveal_focus();
    }

    /// Apply one Niri-style active-window height proportion to a stacked column.
    fn set_active_window_height_share(
        &self,
        row_index: usize,
        start: usize,
        end: usize,
        share: f64,
    ) {
        let count = end.saturating_sub(start);
        if count < 2 {
            return;
        }
        let viewport = self.strip.height() as f64;
        if viewport <= 0.0 {
            return;
        }
        let available = (viewport - (count as f64 - 1.0) * gap(1.0)).max(1.0);
        let active = self.focus_col.get().saturating_sub(start).min(count - 1);
        let heights =
            window_height_preset_distribution(available, count, active, share, MIN_WINDOW_HEIGHT);
        let mut rows = self.rows.borrow_mut();
        let Some(row) = rows.get_mut(row_index) else {
            return;
        };
        for (index, height) in heights.into_iter().enumerate() {
            let ratio = height / available;
            row.tabs[start + index].custom_height.set(Some(ratio));
            self.layout
                .set_tab_height(row_index, start + index, Some(ratio));
        }
        drop(rows);
        self.strip.queue_allocate();
    }

    /// Niri switch-preset-window-height: cycle active window through 50%, 66.7%, 100%.
    pub(crate) fn cycle_window_height_preset(self: &Rc<Self>) {
        let Some((row_index, start, end, _)) = self.focused_column() else {
            return;
        };
        let count = end - start;
        if count < 2 {
            return;
        }
        let viewport = self.strip.height() as f64;
        let available = (viewport - (count as f64 - 1.0) * gap(1.0)).max(1.0);
        let weights: Vec<f64> = {
            let rows = self.rows.borrow();
            let row = &rows[row_index];
            (start..end)
                .map(|index| {
                    row.tabs[index]
                        .custom_height
                        .get()
                        .unwrap_or(1.0 / count as f64)
                })
                .collect()
        };
        let heights = distribute_height_weights(&weights, available, MIN_WINDOW_HEIGHT);
        let active = self.focus_col.get().saturating_sub(start).min(count - 1);
        let current = heights[active] / available;
        let Some(next) = next_layout_preset(&WINDOW_HEIGHT_PRESETS, current) else {
            return;
        };
        self.set_active_window_height_share(row_index, start, end, next);
        self.schedule_save_session();
    }

    /// Niri set-window-height +/-10%: adjust the focused split and share the remainder equally.
    pub(crate) fn adjust_window_height(self: &Rc<Self>, direction: i32) {
        if direction == 0 {
            return;
        }
        let Some((row_index, start, end, _)) = self.focused_column() else {
            return;
        };
        let count = end - start;
        if count < 2 {
            return;
        }
        let viewport = self.strip.height() as f64;
        let available = (viewport - (count as f64 - 1.0) * gap(1.0)).max(1.0);
        let weights: Vec<f64> = {
            let rows = self.rows.borrow();
            let row = &rows[row_index];
            (start..end)
                .map(|index| {
                    row.tabs[index]
                        .custom_height
                        .get()
                        .unwrap_or(1.0 / count as f64)
                })
                .collect()
        };
        let heights = distribute_height_weights(&weights, available, MIN_WINDOW_HEIGHT);
        let active = self.focus_col.get().saturating_sub(start).min(count - 1);
        let minimum_share = (MIN_WINDOW_HEIGHT / available).min(1.0 / count as f64);
        let maximum_share = (1.0 - minimum_share * (count - 1) as f64).max(minimum_share);
        let next = (heights[active] / available + 0.10 * direction.signum() as f64)
            .clamp(minimum_share, maximum_share);
        self.set_active_window_height_share(row_index, start, end, next);
        self.schedule_save_session();
    }

    /// Niri reset-window-height: clear manual ratios and restore equal shares.
    pub(crate) fn reset_window_height(self: &Rc<Self>) {
        let Some((row_index, start, end, _)) = self.focused_column() else {
            return;
        };
        let mut rows = self.rows.borrow_mut();
        let Some(row) = rows.get_mut(row_index) else {
            return;
        };
        for index in start..end {
            row.tabs[index].custom_height.set(None);
            self.layout.set_tab_height(row_index, index, None);
        }
        drop(rows);
        self.strip.queue_allocate();
        self.schedule_save_session();
    }

    /// Niri focus-column-first / focus-column-last.
    pub(crate) fn focus_column_edge(self: &Rc<Self>, last: bool) {
        let row_index = self.focus_row.get();
        let current = self.focus_col.get();
        let target = {
            let rows = self.rows.borrow();
            let Some(row) = rows.get(row_index) else {
                return;
            };
            let columns = row.columns();
            let Some((start, len)) = (if last {
                columns.last()
            } else {
                columns.first()
            }) else {
                return;
            };
            start + (current.saturating_sub(row.col_span(current).0)).min(len - 1)
        };
        self.focus_coords(row_index, target);
    }

    /// Niri move-column-to-first / move-column-to-last.
    pub(crate) fn move_column_edge(self: &Rc<Self>, last: bool) {
        let row_index = self.focus_row.get();
        let current = self.focus_col.get();
        let new_focus = {
            let mut rows = self.rows.borrow_mut();
            let Some(row) = rows.get_mut(row_index) else {
                return;
            };
            if current >= row.tabs.len() {
                return;
            }
            let (start, end) = row.col_span(current);
            let depth = current - start;
            if last {
                if end == row.tabs.len() {
                    return;
                }
                let group: Vec<Tab> = row.tabs.drain(start..end).collect();
                let destination = row.tabs.len();
                row.tabs.extend(group);
                destination + depth
            } else {
                if start == 0 {
                    return;
                }
                let group: Vec<Tab> = row.tabs.drain(start..end).collect();
                row.tabs.splice(0..0, group);
                depth
            }
        };
        self.sync_strip();
        self.focus_coords(row_index, new_focus);
        self.schedule_save_session();
    }

    /// Move the complete stacked column to another workspace, retaining its widths and splits.
    pub(crate) fn move_column_to_workspace(self: &Rc<Self>, target_row: usize) {
        let source_row = self.focus_row.get();
        let source_col = self.focus_col.get();
        if source_row == target_row {
            return;
        }
        let mut rows = self.rows.borrow_mut();
        if source_row >= rows.len() || source_col >= rows[source_row].tabs.len() {
            return;
        }
        let (start, end) = rows[source_row].col_span(source_col);
        let depth = source_col - start;
        let column: Vec<Tab> = rows[source_row].tabs.drain(start..end).collect();
        let source_is_empty = rows[source_row].tabs.is_empty();
        while rows.len() <= target_row {
            rows.push(Row { tabs: Vec::new() });
        }
        let destination = target_row;
        let insertion = rows[destination].tabs.len();
        rows[destination].tabs.extend(column);

        // Niri removes empty secondary workspaces. Workspace zero is retained as Rail's
        // permanent home row; other empty rows are compacted and all indices are remapped.
        let home_row_empty = source_is_empty && source_row == 0;
        let mut focus_destination = destination;
        if source_is_empty && source_row > 0 {
            rows.remove(source_row);
            if destination > source_row {
                focus_destination -= 1;
            }
            let old_last_focus = self.row_last_col.borrow().clone();
            let mut remapped = std::collections::HashMap::new();
            for (row, col) in old_last_focus {
                if row == source_row {
                    continue;
                }
                remapped.insert(if row > source_row { row - 1 } else { row }, col);
            }
            *self.row_last_col.borrow_mut() = remapped;
        }
        drop(rows);
        if home_row_empty {
            self.focus_row.set(0);
            self.focus_col.set(0);
            self.new_tab();
        }
        self.sync_strip();
        self.focus_coords(focus_destination, insertion + depth);
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    /// Reorder a complete workspace by one position, just as Niri moves workspace up/down.
    pub(crate) fn move_workspace(self: &Rc<Self>, direction: i32) {
        if direction == 0 {
            return;
        }
        let source = self.focus_row.get();
        let rows_len = self.rows.borrow().len();
        let Some(target) = (source as isize + direction.signum() as isize)
            .try_into()
            .ok()
            .filter(|target: &usize| *target < rows_len)
        else {
            return;
        };
        self.rows.borrow_mut().swap(source, target);
        let old = self.row_last_col.borrow().clone();
        let mut remapped = std::collections::HashMap::new();
        for (row, col) in old {
            let new_row = if row == source {
                target
            } else if row == target {
                source
            } else {
                row
            };
            remapped.insert(new_row, col);
        }
        *self.row_last_col.borrow_mut() = remapped;
        self.focus_row.set(target);
        self.sync_strip();
        self.focus_coords(target, self.focus_col.get());
        self.refresh_sidebar();
        self.schedule_save_session();
    }

    /// Niri toggle-column-tabbed-display: show only the focused window in this stacked column.
    pub(crate) fn toggle_column_tabbed(self: &Rc<Self>) {
        let Some((row_index, start, end, _)) = self.focused_column() else {
            return;
        };
        if end - start < 2 {
            return;
        }
        let rows = self.rows.borrow();
        let next = !rows[row_index].tabs[start].tabbed.get();
        rows[row_index].tabs[start].tabbed.set(next);
        drop(rows);
        self.sync_strip();
        self.schedule_save_session();
    }
}
