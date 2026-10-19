use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

pub const POPULAR_DOMAINS: &[(&str, &str)] = &[
    ("google.com", "Google Search"),
    ("github.com", "GitHub - Where software is built"),
    ("youtube.com", "YouTube"),
    ("chatgpt.com", "ChatGPT"),
    ("wikipedia.org", "Wikipedia"),
    ("x.com", "X / Twitter"),
    ("twitter.com", "Twitter"),
    ("reddit.com", "Reddit"),
    ("instagram.com", "Instagram"),
    ("facebook.com", "Facebook"),
    ("linkedin.com", "LinkedIn"),
    ("telegram.org", "Telegram"),
    ("amazon.com", "Amazon"),
    ("netflix.com", "Netflix"),
    ("duckduckgo.com", "DuckDuckGo"),
    ("apple.com", "Apple"),
    ("microsoft.com", "Microsoft"),
    ("gitlab.com", "GitLab"),
    ("stackoverflow.com", "Stack Overflow"),
    ("archlinux.org", "Arch Linux"),
    ("crates.io", "crates.io: Rust Package Registry"),
    ("docs.rs", "docs.rs"),
    ("rust-lang.org", "Rust Programming Language"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visit_count: u32,
    pub last_visit: i64,
    // Lower-cased / derived forms, computed once when the entry is created or its title changes, so
    // `suggest()` (runs on every keystroke in the omnibar) does no per-entry allocation or lower-casing.
    url_lower: String,
    domain: String,
    title_lower: String,
}

impl HistoryEntry {
    fn new(url: String, title: String, visit_count: u32, last_visit: i64) -> Self {
        let url_lower = url.to_lowercase();
        let domain = extract_clean_domain(&url_lower);
        let title_lower = title.to_lowercase();
        Self { url, title, visit_count, last_visit, url_lower, domain, title_lower }
    }
    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        self.title_lower = self.title.to_lowercase();
    }
}

/// One serialized-on-the-writer-thread history row: (url, title, visit_count, last_visit).
type Row = (String, String, u32, i64);

/// Background history writer. The UI thread only clones a snapshot and sends it; sorting, formatting and the
/// (atomic tmp + rename) file write happen on this thread. Newer snapshots replace queued older ones.
/// The mutex holds the "epoch": `clear()` bumps it under the lock so a stale queued snapshot can never
/// resurrect cleared history.
struct DiskWriter {
    tx: Option<mpsc::Sender<(u64, Vec<Row>)>>,
    handle: Option<JoinHandle<()>>,
    epoch: Arc<Mutex<u64>>,
}

fn write_rows(path: &PathBuf, mut rows: Vec<Row>) {
    // Keep most recently and frequently visited entries, limit to 2000
    rows.sort_by(|a, b| {
        let score_b = (b.2 as i64 * 3600) + (b.3 / 1_000_000);
        let score_a = (a.2 as i64 * 3600) + (a.3 / 1_000_000);
        score_b.cmp(&score_a)
    });
    let mut out = String::new();
    for (url, title, count, last) in rows.into_iter().take(2000) {
        let clean_title = title.replace('\t', " ").replace('\n', " ");
        out.push_str(&format!("{}\t{}\t{}\t{}\n", url, clean_title, count, last));
    }
    let tmp = path.with_extension("txt.tmp");
    if fs::write(&tmp, &out).is_ok() {
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::write(path, &out);
        }
    } else {
        let _ = fs::write(path, &out);
    }
}

impl DiskWriter {
    fn spawn(path: PathBuf) -> Self {
        let epoch = Arc::new(Mutex::new(0u64));
        let (tx, rx) = mpsc::channel::<(u64, Vec<Row>)>();
        let ep = epoch.clone();
        let handle = std::thread::Builder::new()
            .name("rail-history".into())
            .spawn(move || {
                while let Ok(mut msg) = rx.recv() {
                    while let Ok(newer) = rx.try_recv() {
                        msg = newer;
                    }
                    let guard = ep.lock().unwrap_or_else(|e| e.into_inner());
                    if msg.0 == *guard {
                        write_rows(&path, msg.1);
                    }
                    drop(guard);
                }
            })
            .ok();
        Self { tx: Some(tx), handle, epoch }
    }
    fn send(&self, rows: Vec<Row>) {
        let ep = *self.epoch.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = &self.tx {
            let _ = tx.send((ep, rows));
        }
    }
    /// Drops queued writes and runs `f` (which truncates the file) while the writer is locked out.
    fn invalidate_and<F: FnOnce()>(&self, f: F) {
        let mut g = self.epoch.lock().unwrap_or_else(|e| e.into_inner());
        *g += 1;
        f();
    }
    fn finish(&mut self) {
        self.tx.take();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestionKind {
    History,
    Popular,
    Search,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistorySuggestion {
    pub display_title: String,
    pub display_url: String,
    pub target_url: String,
    pub autofill_text: String,
    pub kind: SuggestionKind,
}

pub struct HistoryManager {
    entries: HashMap<String, HistoryEntry>,
    file_path: PathBuf,
    /// Visits are written to disk lazily (flush()), by the background writer, never on the UI thread.
    dirty: bool,
    writer: DiskWriter,
}

impl HistoryManager {
    pub fn new(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        let file_path = root.join("history.txt");
        let writer = DiskWriter::spawn(file_path.clone());
        let mut mgr = Self {
            entries: HashMap::new(),
            file_path,
            dirty: false,
            writer,
        };
        mgr.load();
        mgr
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.file_path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 4 {
                    let url = parts[0].trim().to_string();
                    let title = parts[1].trim().to_string();
                    let visit_count = parts[2].trim().parse::<u32>().unwrap_or(1);
                    let last_visit = parts[3].trim().parse::<i64>().unwrap_or(0);
                    if !url.is_empty() {
                        self.entries.insert(
                            url.clone(),
                            HistoryEntry::new(url, title, visit_count, last_visit),
                        );
                    }
                }
            }
        }
    }

    /// Hands a snapshot to the background writer (non-blocking).
    pub fn save(&self) {
        let rows: Vec<Row> = self
            .entries
            .values()
            .map(|e| (e.url.clone(), e.title.clone(), e.visit_count, e.last_visit))
            .collect();
        self.writer.send(rows);
    }

    pub fn record_visit(&mut self, url: &str, title: &str) {
        let trimmed = url.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("about:")
            || trimmed.starts_with("file:")
            || trimmed.starts_with("data:")
        {
            return;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let clean_title = title.trim();
        if let Some(existing) = self.entries.get_mut(trimmed) {
            existing.visit_count = existing.visit_count.saturating_add(1);
            existing.last_visit = now;
            if !clean_title.is_empty() && existing.title.is_empty() {
                existing.set_title(clean_title);
            }
        } else {
            self.entries.insert(
                trimmed.to_string(),
                HistoryEntry::new(trimmed.to_string(), clean_title.to_string(), 1, now),
            );
        }
        self.dirty = true;
    }

    /// Writes history to disk if something changed since the last write.
    pub fn flush(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.save();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.dirty = false;
        let path = self.file_path.clone();
        self.writer.invalidate_and(|| {
            let _ = fs::write(&path, "");
        });
    }

    pub fn suggest(&self, query: &str) -> Vec<HistorySuggestion> {
        let raw_q = query.trim();
        if raw_q.is_empty() {
            return Vec::new();
        }

        let q_lower = raw_q.to_lowercase();
        let has_scheme = q_lower.starts_with("https://") || q_lower.starts_with("http://");
        let q_clean = q_lower
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.");

        if q_clean.is_empty() {
            return Vec::new();
        }

        struct ScoredCandidate {
            display_title: String,
            display_url: String,
            target_url: String,
            autofill_text: String,
            score: i64,
            kind: SuggestionKind,
        }

        let mut candidates: Vec<ScoredCandidate> = Vec::new();
        let mut seen_domains: HashMap<String, usize> = HashMap::new();

        // 1. Check history entries
        for entry in self.entries.values() {
            let url_lower = entry.url_lower.as_str();
            let domain = entry.domain.as_str();
            let mut score = 0i64;
            let mut autofill = String::new();

            if domain.starts_with(q_clean) {
                // Exact prefix match on domain: Highest rank!
                score = 10000 + (entry.visit_count as i64 * 100) + (100 - domain.len() as i64);
                autofill = domain.to_string();
            } else if domain.contains(q_clean) {
                score = 5000 + (entry.visit_count as i64 * 50);
                autofill = domain.to_string();
            } else if url_lower.contains(q_clean) {
                score = 3000 + (entry.visit_count as i64 * 20);
                autofill = entry.url.clone();
            } else if entry.title_lower.contains(q_clean) {
                score = 1500 + (entry.visit_count as i64 * 10);
                autofill = domain.to_string();
            }

            if score > 0 {
                if has_scheme && !autofill.starts_with("http") {
                    autofill = format!("https://{}", autofill);
                }
                let title = if entry.title.is_empty() {
                    domain.to_string()
                } else {
                    entry.title.clone()
                };

                candidates.push(ScoredCandidate {
                    display_title: title,
                    display_url: entry.url.clone(),
                    target_url: entry.url.clone(),
                    autofill_text: autofill,
                    score,
                    kind: SuggestionKind::History,
                });
                seen_domains.insert(domain.to_string(), candidates.len() - 1);
            }
        }

        // 2. Check built-in popular domains
        for &(pop_domain, pop_title) in POPULAR_DOMAINS {
            let domain_lower = pop_domain.to_lowercase();
            let mut score = 0i64;

            if domain_lower.starts_with(q_clean) {
                score = 8000 + (100 - domain_lower.len() as i64);
            } else if domain_lower.contains(q_clean) {
                score = 4000;
            } else if pop_title.to_lowercase().contains(q_clean) {
                score = 2000;
            }

            if score > 0 {
                let target = format!("https://{}", pop_domain);
                let mut autofill = pop_domain.to_string();
                if has_scheme {
                    autofill = format!("https://{}", autofill);
                }

                if let Some(&idx) = seen_domains.get(&domain_lower) {
                    // History already has this domain, boost history's score
                    candidates[idx].score += 1500;
                } else {
                    candidates.push(ScoredCandidate {
                        display_title: pop_title.to_string(),
                        display_url: pop_domain.to_string(),
                        target_url: target,
                        autofill_text: autofill,
                        score,
                        kind: SuggestionKind::Popular,
                    });
                    seen_domains.insert(domain_lower, candidates.len() - 1);
                }
            }
        }

        // Sort by score descending
        candidates.sort_by(|a, b| b.score.cmp(&a.score));

        let mut results: Vec<HistorySuggestion> = candidates
            .into_iter()
            .take(20)
            .map(|c| HistorySuggestion {
                display_title: c.display_title,
                display_url: c.display_url,
                target_url: c.target_url,
                autofill_text: c.autofill_text,
                kind: c.kind,
            })
            .collect();

        // Always include default web search suggestion at the end
        let query_esc = url_encode(raw_q);
        results.push(HistorySuggestion {
            display_title: crate::i18n::trf("Search web for \"{}\"", &[raw_q]),
            display_url: format!("duckduckgo.com/?q={}", query_esc),
            target_url: format!("https://duckduckgo.com/?q={}", query_esc),
            autofill_text: raw_q.to_string(),
            kind: SuggestionKind::Search,
        });

        results
    }
}

pub fn extract_clean_domain(url: &str) -> String {
    let clean = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    clean
        .split('/')
        .next()
        .unwrap_or(clean)
        .split('?')
        .next()
        .unwrap_or(clean)
        .split(':')
        .next()
        .unwrap_or(clean)
        .to_string()
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_clean_domain() {
        assert_eq!(extract_clean_domain("https://www.google.com/search?q=rust"), "google.com");
        assert_eq!(extract_clean_domain("http://github.com/mrdevs2011/rail"), "github.com");
        assert_eq!(extract_clean_domain("google.com"), "google.com");
        assert_eq!(extract_clean_domain("localhost:8080/test"), "localhost");
    }

    #[test]
    fn test_suggest_prefix_matching() {
        let temp_dir = std::env::temp_dir().join(format!("rail_hist_test_{}", std::process::id()));
        let mut mgr = HistoryManager::new(temp_dir.clone());

        // Suggesting "goo" without any history should match popular google.com
        let suggestions = mgr.suggest("goo");
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].autofill_text, "google.com");
        assert!(suggestions[0].target_url.contains("google.com"));

        // Record a visit to a custom site
        mgr.record_visit("https://goodreads.com/book/123", "Goodreads Book");
        let suggestions2 = mgr.suggest("good");
        assert!(!suggestions2.is_empty());
        assert_eq!(suggestions2[0].autofill_text, "goodreads.com");
        assert_eq!(suggestions2[0].kind, SuggestionKind::History);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_history_persistence() {
        let temp_dir = std::env::temp_dir().join(format!("rail_hist_test2_{}", std::process::id()));
        {
            let mut mgr = HistoryManager::new(temp_dir.clone());
            mgr.record_visit("https://example.org/docs", "Example Docs");
            mgr.record_visit("https://example.org/docs", "Example Docs");
            assert_eq!(mgr.entries.get("https://example.org/docs").unwrap().visit_count, 2);
        }

        // Re-read from disk
        {
            let mgr2 = HistoryManager::new(temp_dir.clone());
            let entry = mgr2.entries.get("https://example.org/docs").expect("Should exist");
            assert_eq!(entry.visit_count, 2);
            assert_eq!(entry.title, "Example Docs");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_history_clear() {
        let temp_dir = std::env::temp_dir().join(format!("rail_hist_test3_{}", std::process::id()));
        let mut mgr = HistoryManager::new(temp_dir.clone());
        mgr.record_visit("https://example.org", "Example");
        assert_eq!(mgr.entries.len(), 1);
        mgr.clear();
        assert_eq!(mgr.entries.len(), 0);

        let mgr2 = HistoryManager::new(temp_dir.clone());
        assert_eq!(mgr2.entries.len(), 0);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

impl Drop for HistoryManager {
    fn drop(&mut self) {
        self.flush();
        // Wait for the queued snapshot to reach the disk before the process exits.
        self.writer.finish();
    }
}
