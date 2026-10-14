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

        session.set_itp_enabled(true);
        session.set_persistent_credential_storage_enabled(false);

        if let Some(cookies) = session.cookie_manager() {
            let cookie_path = p_data.join("cookies.sqlite");
            cookies.set_persistent_storage(
                cookie_path.to_str().unwrap_or("cookies.sqlite"),
                webkit6::CookiePersistentStorage::Sqlite,
            );
            cookies.set_accept_policy(webkit6::CookieAcceptPolicy::NoThirdParty);
        }

        self.sessions.insert(profile_id.to_string(), session.clone());
        session
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePersona {
    pub user_agent: &'static str,
    pub platform: &'static str,
    pub oscpu: &'static str,
    pub hardware_concurrency: usize,
    pub device_memory: usize,
    pub webgl_vendor: &'static str,
    pub webgl_renderer: &'static str,
    pub seed: u64,
}

impl ProfilePersona {
    pub fn for_id(profile_id: &str) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in profile_id.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        const PERSONAS: &[(&str, &str, &str, usize, usize, &str, &str)] = &[
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
                "Linux x86_64",
                "Linux x86_64",
                16,
                16,
                "Google Inc. (NVIDIA)",
                "ANGLE (NVIDIA, NVIDIA GeForce RTX 4070, OpenGL 4.6)",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
                "Linux x86_64",
                "Linux x86_64",
                12,
                16,
                "Google Inc. (Intel)",
                "Mesa Intel(R) Iris(R) Xe Graphics (TGL GT2)",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
                "Linux x86_64",
                "Linux x86_64",
                8,
                8,
                "Mesa/X.org",
                "Mesa Intel(R) UHD Graphics 620 (KBL GT2)",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
                "Linux x86_64",
                "Linux x86_64",
                16,
                32,
                "Google Inc. (AMD)",
                "AMD Radeon RX 6700 XT (RADV NAVI22)",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
                "Linux x86_64",
                "Linux x86_64",
                8,
                16,
                "Google Inc. (Intel)",
                "Mesa Intel(R) Arc(TM) A770 Graphics (DG2)",
            ),
            (
                "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:132.0) Gecko/20100101 Firefox/132.0",
                "Linux x86_64",
                "Linux x86_64",
                12,
                16,
                "Google Inc. (NVIDIA)",
                "NVIDIA GeForce RTX 3060/PCIe/SSE2",
            ),
            (
                "Mozilla/5.0 (X11; Fedora; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
                "Linux x86_64",
                "Linux x86_64",
                8,
                8,
                "Google Inc. (NVIDIA)",
                "NVIDIA GeForce GTX 1660 SUPER/PCIe/SSE2",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
                "Linux x86_64",
                "Linux x86_64",
                16,
                16,
                "Google Inc. (AMD)",
                "AMD Radeon Graphics (RADV RENOIR)",
            ),
        ];

        let idx = (hash as usize) % PERSONAS.len();
        let (ua, plat, oscpu, hw, mem, vendor, rend) = PERSONAS[idx];
        Self {
            user_agent: ua,
            platform: plat,
            oscpu,
            hardware_concurrency: hw,
            device_memory: mem,
            webgl_vendor: vendor,
            webgl_renderer: rend,
            seed: hash,
        }
    }

    pub fn summary(&self) -> String {
        let browser = if self.user_agent.contains("Edg/") {
            "Edge 131"
        } else if self.user_agent.contains("Chrome/") {
            "Chrome 131"
        } else if self.user_agent.contains("Firefox/") {
            "Firefox 133"
        } else if self.user_agent.contains("Version/") {
            "Safari 18"
        } else {
            "Browser"
        };

        let os = if self.platform == "Win32" {
            "Windows"
        } else if self.platform == "MacIntel" {
            "macOS"
        } else {
            "Linux"
        };

        format!("{} ({})", browser, os)
    }

    pub fn stealth_script(&self) -> String {
        format!(
            r#"(function() {{
    const seed = {}n;
    const hwConcurrency = {};
    const devMemory = {};
    const spoofPlatform = "{}";
    const webglVendor = "{}";
    const webglRenderer = "{}";

    const makeNative = (fn, name) => {{
        try {{
            Object.defineProperty(fn, 'name', {{ value: name, configurable: true }});
            const nativeStr = 'function ' + name + '() {{ [native code] }}';
            Object.defineProperty(fn, 'toString', {{
                value: function toString() {{ return nativeStr; }},
                configurable: true
            }});
        }} catch(e) {{}}
    }};

    try {{
        Object.defineProperty(navigator, 'hardwareConcurrency', {{ get: () => hwConcurrency, configurable: true }});
        Object.defineProperty(navigator, 'deviceMemory', {{ get: () => devMemory, configurable: true }});
        if (spoofPlatform) {{
            Object.defineProperty(navigator, 'platform', {{ get: () => spoofPlatform, configurable: true }});
        }}
    }} catch(e) {{}}

    function pseudoNoise(offset) {{
        let x = (seed + BigInt(offset)) % 2147483647n;
        x = (x * 16807n) % 2147483647n;
        return Number((x % 3n) - 1n);
    }}

    try {{
        if (typeof CanvasRenderingContext2D !== 'undefined') {{
            const origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
            const patchedGetImageData = function(x, y, w, h) {{
                const imgData = origGetImageData.apply(this, arguments);
                if (w > 64 && h > 64 && imgData.data && imgData.data.length >= 1024) {{
                    const d = imgData.data;
                    const noise = pseudoNoise(0);
                    d[d.length - 4] = Math.max(0, Math.min(255, d[d.length - 4] + noise));
                }}
                return imgData;
            }};
            makeNative(patchedGetImageData, 'getImageData');
            CanvasRenderingContext2D.prototype.getImageData = patchedGetImageData;
        }}

        if (typeof HTMLCanvasElement !== 'undefined') {{
            const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
            const patchedToDataURL = function() {{
                return origToDataURL.apply(this, arguments);
            }};
            makeNative(patchedToDataURL, 'toDataURL');
            HTMLCanvasElement.prototype.toDataURL = patchedToDataURL;
        }}
    }} catch(e) {{}}

    try {{
        if (typeof AudioBuffer !== 'undefined') {{
            const origGetChannelData = AudioBuffer.prototype.getChannelData;
            const patchedGetChannelData = function(channel) {{
                const arr = origGetChannelData.apply(this, arguments);
                if (arr && arr.length > 512) {{
                    const noise = (pseudoNoise(channel) * 0.00000005);
                    arr[arr.length - 1] += noise;
                }}
                return arr;
            }};
            makeNative(patchedGetChannelData, 'getChannelData');
            AudioBuffer.prototype.getChannelData = patchedGetChannelData;
        }}
    }} catch(e) {{}}

    try {{
        const spoofGL = function(proto) {{
            if (!proto) return;
            const origGetParameter = proto.getParameter;
            const patchedGetParameter = function(param) {{
                // UNMASKED_VENDOR_WEBGL (0x9245)
                if (param === 0x9245) return webglVendor;
                // UNMASKED_RENDERER_WEBGL (0x9246)
                if (param === 0x9246) return webglRenderer;
                return origGetParameter.apply(this, arguments);
            }};
            makeNative(patchedGetParameter, 'getParameter');
            proto.getParameter = patchedGetParameter;
        }};
        if (typeof WebGLRenderingContext !== 'undefined') spoofGL(WebGLRenderingContext.prototype);
        if (typeof WebGL2RenderingContext !== 'undefined') spoofGL(WebGL2RenderingContext.prototype);
    }} catch(e) {{}}
}})();"#,
            self.seed,
            self.hardware_concurrency,
            self.device_memory,
            self.platform,
            self.webgl_vendor,
            self.webgl_renderer
        )
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
    fn test_profile_persona_isolation() {
        let p_default = ProfilePersona::for_id("default");
        let p_work = ProfilePersona::for_id("work");
        let p_profile_2 = ProfilePersona::for_id("profile_2");

        // Verify different seeds
        assert_ne!(p_default.seed, p_work.seed);
        assert_ne!(p_work.seed, p_profile_2.seed);

        // Verify stealth script generation includes unique seeds
        let script_default = p_default.stealth_script();
        let script_work = p_work.stealth_script();
        assert!(script_default.contains("const seed ="));
        assert!(script_default.contains("CanvasRenderingContext2D"));
        assert!(script_default.contains("UNMASKED_RENDERER_WEBGL"));
        assert_ne!(script_default, script_work);
    }
}
