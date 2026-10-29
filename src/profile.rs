use gtk4::glib;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use webkit6::NetworkSession;

pub const PROFILE_COLORS: &[(&str, &str)] = &[
    ("#ededed", "White"),
    ("#c084fc", "Purple"),
    ("#4ade80", "Green"),
    ("#facc15", "Yellow"),
    ("#f87171", "Red"),
    ("#22d3ee", "Sky"),
    ("#fb923c", "Orange"),
    ("#2dd4bf", "Teal"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color_idx: usize,
}

impl Profile {
    #[allow(dead_code)]
    pub fn hex_color(&self) -> &'static str {
        PROFILE_COLORS
            .get(self.color_idx)
            .map(|(c, _)| *c)
            .unwrap_or("#7aa2f7")
    }

    pub fn initial_letter(&self) -> String {
        let trimmed = self.name.trim();
        if trimmed.is_empty() {
            return "P".into();
        }

        // 1. If trimmed string consists entirely of digits (e.g. "10", "100", "7"):
        // take up to 3 digits directly!
        let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
        let non_digits: String = trimmed.chars().filter(|c| c.is_alphabetic()).collect();

        if !digits.is_empty() && non_digits.is_empty() {
            return digits.chars().take(3).collect();
        }

        // 2. If multi-word name:
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() > 1 {
            if let Some(last) = words.last() {
                if last.chars().all(|c| c.is_ascii_digit()) && last.len() <= 3 {
                    // e.g. "Profile 10" -> "10"
                    if words.len() == 2 && words[0].eq_ignore_ascii_case("profile") {
                        return (*last).to_string();
                    }
                }
            }
            // Multi-word initials up to 3: "John Doe" -> "JD", "My Work Space" -> "MWS"
            let initials: String = words
                .iter()
                .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
                .take(3)
                .map(|c| c.to_uppercase().to_string())
                .collect();
            if !initials.is_empty() {
                return initials;
            }
        }

        // 3. Short alphanumeric names <= 3 chars (e.g. "QA", "DEV", "10", "PRO"):
        let clean: String = trimmed.chars().filter(|c| c.is_alphanumeric()).collect();
        if clean.len() <= 3 && !clean.is_empty() {
            return clean.to_uppercase();
        }

        // 4. Long single-word names (e.g. "Default", "Personal", "Work"):
        clean
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "P".into())
    }

    pub fn text_color(&self) -> &'static str {
        profile_text_color_for_bg(self.hex_color())
    }
}

pub fn profile_text_color_for_bg(hex: &str) -> &'static str {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            let lum = 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64);
            if lum > 140.0 {
                "#050811" // Light background -> solid pure deep black
            } else {
                "#ffffff" // Dark background -> solid crisp white
            }
        } else {
            "#050811"
        }
    } else {
        "#050811"
    }
}

pub struct ProfileManager {
    profiles: Vec<Profile>,
    active_id: String,
    sessions: HashMap<String, NetworkSession>,
    config_file: PathBuf,
    active_file: PathBuf,
    base_dir: PathBuf,
}

impl ProfileManager {
    pub fn new(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        let base_dir = root.join("profiles");
        let _ = fs::create_dir_all(&base_dir);

        let config_file = root.join("profiles.txt");
        let active_file = root.join("active_profile.txt");

        let mut mgr = Self {
            profiles: Vec::new(),
            active_id: "default".into(),
            sessions: HashMap::new(),
            config_file,
            active_file,
            base_dir,
        };

        mgr.load();
        mgr
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_file) {
            for line in content.lines() {
                let parts: Vec<&str> = line.trim().split('|').collect();
                if parts.len() >= 3 {
                    let id = parts[0].trim().to_string();
                    let name = parts[1].trim().to_string();
                    let color_idx = parts[2].trim().parse::<usize>().unwrap_or(0);
                    if !id.is_empty() && !name.is_empty() {
                        self.profiles.push(Profile {
                            id,
                            name,
                            color_idx,
                        });
                    }
                }
            }
        }

        if self.profiles.is_empty() {
            self.profiles.push(Profile {
                id: "default".into(),
                name: "Default".into(),
                color_idx: 0,
            });
            self.save();
        }

        if let Ok(act) = fs::read_to_string(&self.active_file) {
            let act = act.trim();
            if self.profiles.iter().any(|p| p.id == act) {
                self.active_id = act.to_string();
            } else {
                self.active_id = self.profiles[0].id.clone();
            }
        } else {
            self.active_id = self.profiles[0].id.clone();
        }
    }

    pub fn save(&self) {
        let mut out = String::new();
        for p in &self.profiles {
            out.push_str(&format!("{}|{}|{}\n", p.id, p.name, p.color_idx));
        }
        let _ = fs::write(&self.config_file, out);
        let _ = fs::write(&self.active_file, &self.active_id);
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn active_profile(&self) -> Profile {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_id)
            .cloned()
            .unwrap_or_else(|| self.profiles[0].clone())
    }

    pub fn active_id(&self) -> &str {
        &self.active_id
    }

    pub fn set_active(&mut self, id: &str) {
        if self.profiles.iter().any(|p| p.id == id) {
            self.active_id = id.to_string();
            self.save();
        }
    }

    pub fn get_profile(&self, id: &str) -> Option<Profile> {
        self.profiles.iter().find(|p| p.id == id).cloned()
    }

    pub fn profile_exists(&self, name: &str) -> bool {
        let clean = name.trim();
        if clean.is_empty() {
            return false;
        }
        let slug: String = clean
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();

        self.profiles.iter().any(|p| {
            p.name.trim().eq_ignore_ascii_case(clean)
                || p.id.trim().eq_ignore_ascii_case(clean)
                || (!slug.is_empty() && p.id.trim().eq_ignore_ascii_case(&slug))
        })
    }

    pub fn create_profile(
        &mut self,
        name: &str,
        color_idx: usize,
    ) -> Result<Profile, &'static str> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err("Profile name cannot be empty");
        }
        if clean_name.len() > 32 {
            return Err("Profile name is too long (max 32 characters)");
        }
        if self.profile_exists(clean_name) {
            return Err("Profile already exists");
        }

        let mut slug: String = clean_name
            .chars()
            .filter_map(|c| {
                if c.is_alphanumeric() {
                    Some(c.to_ascii_lowercase())
                } else if c == '-' || c == '_' {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();
        if slug.is_empty() {
            slug = "profile".into();
        }

        let mut id = slug.clone();
        let mut counter = 1;
        while self.profiles.iter().any(|p| p.id == id) {
            id = format!("{}_{}", slug, counter);
            counter += 1;
        }

        let p = Profile {
            id,
            name: clean_name.to_string(),
            color_idx: color_idx % PROFILE_COLORS.len(),
        };
        // Always append newly created profile to the very bottom
        self.profiles.push(p.clone());
        self.save();
        Ok(p)
    }

    pub fn reorder_profile(&mut self, from_idx: usize, to_idx: usize) -> bool {
        if from_idx >= self.profiles.len() || to_idx >= self.profiles.len() || from_idx == to_idx {
            return false;
        }
        let p = self.profiles.remove(from_idx);
        self.profiles.insert(to_idx, p);
        self.save();
        true
    }

    pub fn delete_profile(&mut self, id: &str) -> bool {
        if self.profiles.len() <= 1 || id == "default" {
            return false;
        }

        if let Some(pos) = self.profiles.iter().position(|p| p.id == id) {
            self.profiles.remove(pos);
            self.sessions.remove(id);

            // Clean up profile directory
            let prof_dir = self.base_dir.join(id);
            let _ = fs::remove_dir_all(&prof_dir);

            if self.active_id == id {
                self.active_id = self.profiles[0].id.clone();
            }
            self.save();
            true
        } else {
            false
        }
    }

    pub fn get_or_create_session(&mut self, profile_id: &str) -> NetworkSession {
        if let Some(s) = self.sessions.get(profile_id) {
            return s.clone();
        }

        let prof_dir = self.base_dir.join(profile_id);
        let p_data = prof_dir.join("data");
        let p_cache = prof_dir.join("cache");
        let _ = fs::create_dir_all(&p_data);
        let _ = fs::create_dir_all(&p_cache);

        // Migrate old cache from ~/.cache if present
        let old_cache = glib::user_cache_dir()
            .join("rail")
            .join("profiles")
            .join(profile_id)
            .join("cache");
        if old_cache.exists()
            && (!p_cache.exists()
                || fs::read_dir(&p_cache)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false))
        {
            let _ = fs::rename(&old_cache, &p_cache);
        }

        let session = NetworkSession::new(
            Some(p_data.to_str().unwrap_or("")),
            Some(p_cache.to_str().unwrap_or("")),
        );

        session.set_itp_enabled(true);
        // Favicons are off by default: without this WebView::favicon() is always None and the
        // sidebar falls back to letter badges instead of the site's icon.
        if let Some(dm) = session.website_data_manager() {
            dm.set_favicons_enabled(true);
        }
        session.set_persistent_credential_storage_enabled(false);

        if let Some(cookies) = session.cookie_manager() {
            let cookie_path = p_data.join("cookies.sqlite");
            cookies.set_persistent_storage(
                cookie_path.to_str().unwrap_or("cookies.sqlite"),
                webkit6::CookiePersistentStorage::Sqlite,
            );
            cookies.set_accept_policy(webkit6::CookieAcceptPolicy::Always);
        }

        self.sessions
            .insert(profile_id.to_string(), session.clone());
        session
    }
}

impl Profile {
    /// Ko'rsatiladigan nom: standart profil joriy tilda nomlanadi.
    pub fn display_name(&self) -> String {
        if self.id == "default" && self.name == "Default" {
            crate::i18n::tr("Default").to_string()
        } else {
            self.name.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_uniqueness() {
        let mut mgr = ProfileManager {
            profiles: vec![
                Profile {
                    id: "default".into(),
                    name: "Default".into(),
                    color_idx: 0,
                },
                Profile {
                    id: "work".into(),
                    name: "Work".into(),
                    color_idx: 1,
                },
            ],
            active_id: "default".into(),
            sessions: HashMap::new(),
            config_file: PathBuf::from("/tmp/test_rail_profiles.txt"),
            active_file: PathBuf::from("/tmp/test_rail_active.txt"),
            base_dir: PathBuf::from("/tmp/test_rail_profiles"),
        };

        // Existing matches (case-insensitive name and id)
        assert!(mgr.profile_exists("Default"));
        assert!(mgr.profile_exists("default"));
        assert!(mgr.profile_exists("Work"));
        assert!(mgr.profile_exists("work"));
        assert!(mgr.profile_exists("  WORK  "));

        // Non-existing
        assert!(!mgr.profile_exists("personal"));
        assert!(!mgr.profile_exists("study"));

        // Create duplicate should fail
        assert_eq!(mgr.create_profile("work", 2), Err("Profile already exists"));
        assert_eq!(
            mgr.create_profile("Default", 0),
            Err("Profile already exists")
        );
        assert_eq!(
            mgr.create_profile("", 0),
            Err("Profile name cannot be empty")
        );

        // Create new unique profile should succeed
        let new_p = mgr.create_profile("Personal", 3).expect("Should succeed");
        assert_eq!(new_p.id, "personal");
        assert_eq!(new_p.name, "Personal");
        assert_eq!(new_p.color_idx, 3);
        assert!(mgr.profile_exists("personal"));
        assert!(mgr.profile_exists("Personal"));

        // Clean up test files if any created
        let _ = fs::remove_file(&mgr.config_file);
        let _ = fs::remove_file(&mgr.active_file);
    }

    #[test]
    fn test_profile_active_persistence() {
        let temp_dir = std::env::temp_dir().join(format!("rail_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        {
            let mut mgr = ProfileManager::new(temp_dir.clone());
            let _ = mgr.create_profile("Gaming", 4).expect("create profile");
            mgr.set_active("gaming");
            assert_eq!(mgr.active_id(), "gaming");
        }

        // Re-read from disk
        {
            let mgr2 = ProfileManager::new(temp_dir.clone());
            assert_eq!(mgr2.active_id(), "gaming");
            assert_eq!(mgr2.active_profile().name, "Gaming");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_create_profile_does_not_auto_activate() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rail_prof_no_auto_act_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&temp_dir);

        let mut mgr = ProfileManager::new(temp_dir.clone());
        assert_eq!(mgr.active_id(), "default");

        let p = mgr
            .create_profile("New Account", 2)
            .expect("create profile");
        assert_eq!(p.id, "newaccount");
        // Newly created profile must NOT become active automatically
        assert_eq!(mgr.active_id(), "default");
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_create_profile_appends_to_bottom() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rail_prof_appends_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&temp_dir);

        let mut mgr = ProfileManager::new(temp_dir.clone());
        let _ = mgr.create_profile("Profile A", 1).expect("create");
        let _ = mgr.create_profile("Profile B", 2).expect("create");
        mgr.set_active("profilea");

        let p_new = mgr.create_profile("Profile C", 3).expect("create");
        assert_eq!(p_new.id, "profilec");

        let ids: Vec<&str> = mgr.profiles().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["default", "profilea", "profileb", "profilec"]);

        // Test reorder
        mgr.reorder_profile(3, 1);
        let ids_reordered: Vec<&str> = mgr.profiles().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids_reordered,
            vec!["default", "profilec", "profilea", "profileb"]
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_profile_contrast_color() {
        // Bright colors (cyan, yellow, green, teal, etc.) must yield high contrast dark text (#050811)
        assert_eq!(profile_text_color_for_bg("#38bdf8"), "#050811");
        assert_eq!(profile_text_color_for_bg("#facc15"), "#050811");
        assert_eq!(profile_text_color_for_bg("#4ade80"), "#050811");
        assert_eq!(profile_text_color_for_bg("#ffffff"), "#050811");

        // Dark backgrounds (black, deep navy, etc.) must yield crisp white text (#ffffff)
        assert_eq!(profile_text_color_for_bg("#000000"), "#ffffff");
        assert_eq!(profile_text_color_for_bg("#090b10"), "#ffffff");
        assert_eq!(profile_text_color_for_bg("#1e293b"), "#ffffff");

        let prof = Profile {
            id: "work".into(),
            name: "Work".into(),
            color_idx: 0, // #38bdf8
        };
        assert_eq!(prof.text_color(), "#050811");
    }

    #[test]
    fn test_profile_initial_letter_multi_char() {
        let p1 = Profile {
            id: "1".into(),
            name: "1".into(),
            color_idx: 0,
        };
        assert_eq!(p1.initial_letter(), "1");

        let p10 = Profile {
            id: "10".into(),
            name: "10".into(),
            color_idx: 0,
        };
        assert_eq!(p10.initial_letter(), "10");

        let p100 = Profile {
            id: "100".into(),
            name: "100".into(),
            color_idx: 0,
        };
        assert_eq!(p100.initial_letter(), "100");

        let p_prof10 = Profile {
            id: "prof10".into(),
            name: "Profile 10".into(),
            color_idx: 0,
        };
        assert_eq!(p_prof10.initial_letter(), "10");

        let p_qa = Profile {
            id: "qa".into(),
            name: "QA".into(),
            color_idx: 0,
        };
        assert_eq!(p_qa.initial_letter(), "QA");

        let p_dev = Profile {
            id: "dev".into(),
            name: "Dev".into(),
            color_idx: 0,
        };
        assert_eq!(p_dev.initial_letter(), "DEV");

        let p_default = Profile {
            id: "default".into(),
            name: "Default".into(),
            color_idx: 0,
        };
        assert_eq!(p_default.initial_letter(), "D");
    }
}
