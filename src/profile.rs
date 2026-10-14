use gtk4::glib;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use webkit6::NetworkSession;

pub const PROFILE_COLORS: &[(&str, &str)] = &[
    ("#38bdf8", "Cyan"),
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
        self.name
            .chars()
            .find(|c| c.is_alphanumeric())
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "P".into())
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
                        self.profiles.push(Profile { id, name, color_idx });
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
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
            .collect();

        self.profiles.iter().any(|p| {
            p.name.trim().eq_ignore_ascii_case(clean)
                || p.id.trim().eq_ignore_ascii_case(clean)
                || (!slug.is_empty() && p.id.trim().eq_ignore_ascii_case(&slug))
        })
    }

    pub fn create_profile(&mut self, name: &str, color_idx: usize) -> Result<Profile, &'static str> {
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
        self.profiles.push(p.clone());
        self.save();
        Ok(p)
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
        let old_cache = glib::user_cache_dir().join("rail").join("profiles").join(profile_id).join("cache");
        if old_cache.exists() && (!p_cache.exists() || fs::read_dir(&p_cache).map(|mut d| d.next().is_none()).unwrap_or(false)) {
            let _ = fs::rename(&old_cache, &p_cache);
        }

        let session = NetworkSession::new(
            Some(p_data.to_str().unwrap_or("")),
            Some(p_cache.to_str().unwrap_or("")),
        );

        if let Some(cookies) = session.cookie_manager() {
            let cookie_path = p_data.join("cookies.sqlite");
            cookies.set_persistent_storage(
                cookie_path.to_str().unwrap_or("cookies.sqlite"),
                webkit6::CookiePersistentStorage::Sqlite,
            );
        }

        self.sessions.insert(profile_id.to_string(), session.clone());
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_uniqueness() {
        let mut mgr = ProfileManager {
            profiles: vec![
                Profile { id: "default".into(), name: "Default".into(), color_idx: 0 },
                Profile { id: "work".into(), name: "Work".into(), color_idx: 1 },
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
        assert_eq!(mgr.create_profile("Default", 0), Err("Profile already exists"));
        assert_eq!(mgr.create_profile("", 0), Err("Profile name cannot be empty"));

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
}
