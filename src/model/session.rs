//! Saved session format and parser.

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SessionTabItem {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) full: bool,
    pub(crate) profile_id: String,
    pub(crate) url: String,
    pub(crate) custom_width: Option<f64>,
    /// Column-relative vertical share (0.0..=1.0), stable across viewport resizes.
    pub(crate) custom_height: Option<f64>,
    pub(crate) tabbed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SessionState {
    pub(crate) focus: (usize, usize),
    pub(crate) sidebar_open: Option<bool>,
    pub(crate) search_engine: Option<String>,
    pub(crate) win_size: Option<(i32, i32, bool)>,
    pub(crate) tabs: Vec<SessionTabItem>,
    /// (row, col) of tabs that are stacked under the tab before them in the same column
    pub(crate) stacked: Vec<(usize, usize)>,
}

pub(crate) fn parse_session_content(content: &str) -> SessionState {
    let mut focus = (0usize, 0usize);
    let mut sidebar_open = None;
    let mut search_engine = None;
    let mut win_size = None;
    let mut tabs: Vec<SessionTabItem> = Vec::new();
    let mut stacked: Vec<(usize, usize)> = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "focus" if parts.len() >= 3 => {
                let r = parts[1].parse().unwrap_or(0);
                let c = parts[2].parse().unwrap_or(0);
                focus = (r, c);
            }
            "sidebar" if parts.len() >= 2 => {
                sidebar_open = Some(parts[1] == "1");
            }
            "engine" | "search_engine" if parts.len() >= 2 => {
                search_engine = Some(parts[1].trim().to_string());
            }
            "win" if parts.len() >= 4 => {
                let w = parts[1].parse().unwrap_or(1400);
                let h = parts[2].parse().unwrap_or(850);
                let max = parts[3] == "1";
                win_size = Some((w, h, max));
            }
            "tab" if parts.len() >= 6 => {
                let r = parts[1].parse().unwrap_or(0);
                let c = parts[2].parse().unwrap_or(0);
                let full = parts[3] == "1";
                let pid = parts[4].to_string();
                let url = parts[5].to_string();
                let custom_width = parts.get(6).and_then(|s| s.parse::<f64>().ok());
                let custom_height = parts
                    .get(7)
                    .and_then(|s| s.parse::<f64>().ok())
                    .filter(|height| height.is_finite() && *height > 0.0 && *height <= 1.0);
                tabs.push(SessionTabItem {
                    row: r,
                    col: c,
                    full,
                    profile_id: pid,
                    url,
                    custom_width,
                    custom_height,
                    tabbed: parts.get(8).is_some_and(|value| *value == "1"),
                });
            }
            "tab" if parts.len() == 4 => {
                let c = parts[1].parse().unwrap_or(0);
                let pid = parts[2].to_string();
                let url = parts[3].to_string();
                tabs.push(SessionTabItem {
                    row: 0,
                    col: c,
                    full: true,
                    profile_id: pid,
                    url,
                    custom_width: None,
                    custom_height: None,
                    tabbed: false,
                });
            }
            "stack" if parts.len() >= 3 => {
                stacked.push((parts[1].parse().unwrap_or(0), parts[2].parse().unwrap_or(0)));
            }
            _ => {}
        }
    }
    SessionState {
        focus,
        sidebar_open,
        search_engine,
        win_size,
        tabs,
        stacked,
    }
}
