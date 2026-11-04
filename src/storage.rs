//! Atomic transactional file persistence and self-healing backup subsystem.
//!
//! Writes use unique same-directory temporary files, sync file contents before rename,
//! and sync the containing directory after replacement. Recovery never overwrites the
//! known-good backup with the corrupt primary file it is repairing.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Helper to produce a backup path for a given file (e.g. `settings.json` -> `settings.json.bak`).
pub(crate) fn backup_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    let mut os_str = p.as_os_str().to_os_string();
    os_str.push(".bak");
    PathBuf::from(os_str)
}

/// Produce a process- and thread-safe temporary path beside the target.
pub(crate) fn temp_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    let mut os_str = p.as_os_str().to_os_string();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    os_str.push(format!(".tmp.{}.{}", std::process::id(), sequence));
    PathBuf::from(os_str)
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn sync_parent_dir(path: &Path) -> io::Result<()> {
    File::open(parent_dir(path))?.sync_all()
}

/// Create and fully sync a unique temporary file in the target directory.
fn write_temp_file(target: &Path, content: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(parent_dir(target))?;
    loop {
        let tmp = temp_path(target);
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(mut file) => {
                let result = file
                    .write_all(content.as_bytes())
                    .and_then(|_| file.sync_all());
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&tmp);
                    return Err(error);
                }
                return Ok(tmp);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Atomically replace a file without changing its backup. Used for recovery.
fn atomic_replace<P: AsRef<Path>>(path: P, content: &str) -> io::Result<()> {
    let target = path.as_ref();
    let tmp = write_temp_file(target, content)?;
    let result = fs::rename(&tmp, target).and_then(|_| sync_parent_dir(target));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Atomically copy a file to its backup path without exposing a partial backup.
fn atomic_copy<P: AsRef<Path>, Q: AsRef<Path>>(source: P, destination: Q) -> io::Result<()> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    fs::create_dir_all(parent_dir(destination))?;
    loop {
        let tmp = temp_path(destination);
        let mut input = File::open(source)?;
        let mut output = match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let copy_result = io::copy(&mut input, &mut output).and_then(|_| output.sync_all());
        drop(output);
        if let Err(error) = copy_result {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        let result = fs::rename(&tmp, destination).and_then(|_| sync_parent_dir(destination));
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        return result;
    }
}

/// Atomically writes content to `path` and preserves the previous non-empty file as `.bak`.
/// The write is aborted if the previous file cannot be backed up safely.
pub(crate) fn atomic_write_with_backup<P: AsRef<Path>>(path: P, content: &str) -> io::Result<()> {
    let target = path.as_ref();
    let tmp = write_temp_file(target, content)?;
    let result = (|| {
        if target.exists() && fs::metadata(target)?.len() > 0 {
            atomic_copy(target, backup_path(target))?;
        }
        fs::rename(&tmp, target)?;
        sync_parent_dir(target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Reads `path` and validates its content using `validator`.
/// If the primary file is missing or invalid, a valid `.bak` is restored without
/// modifying that backup, so a failed recovery attempt cannot destroy the last good copy.
pub(crate) fn read_with_auto_healing<P: AsRef<Path>, F: Fn(&str) -> bool>(
    path: P,
    validator: F,
) -> Option<String> {
    let target = path.as_ref();

    if let Ok(content) = fs::read_to_string(target) {
        if validator(&content) {
            return Some(content);
        }
        eprintln!(
            "[rail] Warning: Primary file {:?} failed validation/corrupted.",
            target
        );
    }

    let bak = backup_path(target);
    if let Ok(bak_content) = fs::read_to_string(&bak)
        && validator(&bak_content)
    {
        eprintln!(
            "[rail] Self-healing: Restoring {:?} from {:?}.",
            target, bak
        );
        if let Err(error) = atomic_replace(target, &bak_content) {
            eprintln!(
                "[rail] Warning: Could not persist recovered file {:?}: {error}",
                target
            );
        }
        return Some(bak_content);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rail_test_storage_{name}_{}_{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn atomic_write_preserves_previous_content_in_backup() {
        let dir = test_dir("atomic");
        let path = dir.join("settings.json");

        atomic_write_with_backup(&path, "payload 1").unwrap();
        atomic_write_with_backup(&path, "payload 2").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "payload 2");
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), "payload 1");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recovery_restores_primary_without_overwriting_good_backup() {
        let dir = test_dir("recovery");
        let path = dir.join("data.json");
        let backup = backup_path(&path);

        atomic_write_with_backup(&path, "{\"valid\": true}").unwrap();
        atomic_write_with_backup(&path, "{\"valid\": 2}").unwrap();
        fs::write(&path, "{corrupted json...").unwrap();

        let validator = |content: &str| content.contains("\"valid\":");
        let recovered = read_with_auto_healing(&path, validator).unwrap();
        assert_eq!(recovered, "{\"valid\": true}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"valid\": true}");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "{\"valid\": true}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn temporary_paths_are_unique_within_one_process() {
        let path = Path::new("settings.json");
        let first = temp_path(path);
        let second = temp_path(path);
        assert_ne!(first, second);
    }

    #[test]
    fn concurrent_writes_never_share_temporary_files_or_leave_partial_json() {
        let dir = test_dir("concurrent");
        let path = dir.join("state.json");
        let mut workers = Vec::new();
        for index in 0..8 {
            let path = path.clone();
            workers.push(std::thread::spawn(move || {
                let content = format!("{{\"writer\":{index},\"complete\":true}}");
                atomic_write_with_backup(path, &content).unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("{\"writer\":"));
        assert!(content.ends_with(",\"complete\":true}"));
        if let Ok(backup) = fs::read_to_string(backup_path(&path)) {
            assert!(backup.ends_with(",\"complete\":true}"));
        }
        assert!(fs::read_dir(&dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp.")
        }));
        fs::remove_dir_all(dir).unwrap();
    }
}
