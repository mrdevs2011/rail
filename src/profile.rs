use gtk4::glib;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use webkit6::NetworkSession;

pub const PROFILE_COLORS: &[(&str, &str)] = &[
    ("#7aa2f7", "Blue"),
    ("#bb9af7", "Purple"),
    ("#9ece6a", "Green"),
    ("#e0af68", "Yellow"),
    ("#f7768e", "Pink"),
    ("#7dcfff", "Cyan"),
    ("#ff9e64", "Orange"),
    ("#73daca", "Teal"),
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
    base_data_dir: PathBuf,
    base_cache_dir: PathBuf,
}

impl ProfileManager {
    pub fn new() -> Self {
        let base_data = glib::user_data_dir().join("rail").join("profiles");
        let base_cache = glib::user_cache_dir().join("rail").join("profiles");
        let config_dir = glib::user_data_dir().join("rail");
        let _ = fs::create_dir_all(&config_dir);
        let _ = fs::create_dir_all(&base_data);
        let _ = fs::create_dir_all(&base_cache);

        let config_file = config_dir.join("profiles.txt");
        let active_file = config_dir.join("active_profile.txt");

        let mut mgr = Self {
            profiles: Vec::new(),
            active_id: "default".into(),
            sessions: HashMap::new(),
            config_file,
            active_file,
            base_data_dir: base_data,
            base_cache_dir: base_cache,
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

    pub fn create_profile(&mut self, name: &str, color_idx: usize) -> Profile {
        let clean_name = name.trim();
        let slug: String = clean_name
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
            .collect();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let id = format!("{}_{}", slug, timestamp);

        let p = Profile {
            id: id.clone(),
            name: clean_name.to_string(),
            color_idx: color_idx % PROFILE_COLORS.len(),
        };
        self.profiles.push(p.clone());
        self.save();
        p
    }

    pub fn delete_profile(&mut self, id: &str) -> bool {
        if self.profiles.len() <= 1 || id == "default" {
            return false;
        }

        if let Some(pos) = self.profiles.iter().position(|p| p.id == id) {
            self.profiles.remove(pos);
            self.sessions.remove(id);

            // Clean up directories in background
            let d = self.base_data_dir.join(id);
            let c = self.base_cache_dir.join(id);
            let _ = fs::remove_dir_all(&d);
            let _ = fs::remove_dir_all(&c);

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

        let p_data = self.base_data_dir.join(profile_id).join("data");
        let p_cache = self.base_cache_dir.join(profile_id).join("cache");
        let _ = fs::create_dir_all(&p_data);
        let _ = fs::create_dir_all(&p_cache);

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
