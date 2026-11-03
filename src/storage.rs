//! Atomic transactional file persistence and self-healing backup subsystem.
//!
//! Conforms to Phase 4 (Structural Architectural Hardening):
//! 1. All file writes are atomic (temporary file in same directory + fsync + atomic rename).
//! 2. Redundant `.bak` copies are automatically maintained.
//! 3. On startup / load, if the primary file is corrupted or truncated, the subsystem
//!    automatically self-heals by restoring from the `.bak` replica.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Helper to produce a backup path for a given file (e.g. `settings.json` -> `settings.json.bak`).
pub(crate) fn backup_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    let mut os_str = p.as_os_str().to_os_string();
    os_str.push(".bak");
    PathBuf::from(os_str)
}

/// Helper to produce a temporary path in the same directory (guarantees atomic rename).
pub(crate) fn temp_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    let mut os_str = p.as_os_str().to_os_string();
    os_str.push(format!(".tmp.{}", std::process::id()));
    PathBuf::from(os_str)
}

/// Atomically writes content to `path`.
/// - Writes to a temporary file in the same directory.
/// - Flushes and syncs to physical storage (`sync_all`).
/// - Backs up any existing valid file to `<path>.bak`.
/// - Atomically renames temporary file to `path`.
pub(crate) fn atomic_write_with_backup<P: AsRef<Path>>(
    path: P,
    content: &str,
) -> std::io::Result<()> {
    let target = path.as_ref();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = temp_path(target);
    {
        let mut file = File::create(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }

    // Maintain backup of existing data if it exists and has content
    if target.exists()
        && let Ok(meta) = fs::metadata(target)
        && meta.len() > 0
    {
        let bak = backup_path(target);
        let _ = fs::copy(target, &bak);
    }

    // Atomic replace on POSIX
    fs::rename(&tmp, target)?;
    Ok(())
}

/// Reads `path` and validates its content using `validator`.
/// If `path` is missing or fails validation (corrupted, truncated, 0 bytes):
/// - Checks if `<path>.bak` exists and satisfies `validator`.
/// - If valid, automatically restores `<path>` from the backup (self-healing).
/// - Returns the restored content.
pub(crate) fn read_with_auto_healing<P: AsRef<Path>, F: Fn(&str) -> bool>(
    path: P,
    validator: F,
) -> Option<String> {
    let target = path.as_ref();

    // 1. Try primary file
    if let Ok(content) = fs::read_to_string(target) {
        if validator(&content) {
            return Some(content);
        }
        eprintln!(
            "[rail] Warning: Primary file {:?} failed validation/corrupted.",
            target
        );
    }

    // 2. Try backup file for self-healing
    let bak = backup_path(target);
    if let Ok(bak_content) = fs::read_to_string(&bak)
        && validator(&bak_content)
    {
        eprintln!(
            "[rail] Self-healing: Restoring {:?} from {:?}.",
            target, bak
        );
        let _ = atomic_write_with_backup(target, &bak_content);
        return Some(bak_content);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_write_and_read() {
        let dir = std::env::temp_dir().join(format!("rail_test_storage_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_file.txt");

        let res = atomic_write_with_backup(&path, "payload 1");
        assert!(res.is_ok());
        assert_eq!(fs::read_to_string(&path).unwrap(), "payload 1");

        // Second write creates backup of payload 1
        let res2 = atomic_write_with_backup(&path, "payload 2");
        assert!(res2.is_ok());
        assert_eq!(fs::read_to_string(&path).unwrap(), "payload 2");
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), "payload 1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_self_healing_from_backup() {
        let dir = std::env::temp_dir().join(format!("rail_test_heal_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("data.json");

        // Valid initial data
        let _ = atomic_write_with_backup(&path, "{\"valid\": true}");
        // Second write to create .bak
        let _ = atomic_write_with_backup(&path, "{\"valid\": 2}");

        // Deliberately corrupt primary file (simulate power loss or partial flush)
        fs::write(&path, "{corrupted json...").unwrap();

        let validator = |c: &str| c.contains("\"valid\":");
        let healed = read_with_auto_healing(&path, validator);

        assert!(healed.is_some());
        // Should have healed back to the valid backup
        assert_eq!(healed.unwrap(), "{\"valid\": true}");
        // And the primary file should now be restored!
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"valid\": true}");

        let _ = fs::remove_dir_all(&dir);
    }
}
