//! Command-line data export / import / info.

use crate::*;

pub(crate) fn export_data(archive_path: &Path) -> Result<(), String> {
    let dir = rail_data_dir();
    if !dir.exists() {
        return Err(trf(
            "Storage directory does not exist: {}",
            &[&dir.display().to_string()],
        ));
    }
    let parent = dir.parent().unwrap_or(&dir);
    let name = dir
        .file_name()
        .ok_or_else(|| tr("Invalid directory name").to_string())?;

    let abs_archive = if archive_path.is_absolute() {
        archive_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(archive_path)
    };

    let status = std::process::Command::new("tar")
        .arg("--exclude=*/cache")
        .arg("--exclude=*/cache/*")
        .arg("-czf")
        .arg(&abs_archive)
        .arg("-C")
        .arg(parent)
        .arg(name)
        .status()
        .map_err(|e| trf("Failed to run tar command: {}", &[&e.to_string()]))?;

    if status.success() {
        println!(
            "{}",
            trf(
                "Successfully exported all Rail data to: {}",
                &[&abs_archive.display().to_string()]
            )
        );
        println!(
            "{}",
            tr("This file contains all your tabs, workspaces, profiles, logins, and cookies.")
        );
        Ok(())
    } else {
        Err(trf(
            "tar command failed with status: {}",
            &[&status.to_string()],
        ))
    }
}

pub(crate) fn import_data(archive_path: &Path) -> Result<(), String> {
    let abs_archive = if archive_path.is_absolute() {
        archive_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(archive_path)
    };

    if !abs_archive.exists() {
        return Err(trf(
            "Archive file does not exist: {}",
            &[&abs_archive.display().to_string()],
        ));
    }

    let dest_dir = rail_data_dir();
    let parent = dest_dir.parent().unwrap_or(&dest_dir);
    let _ = std::fs::create_dir_all(parent);

    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&abs_archive)
        .arg("-C")
        .arg(parent)
        .status()
        .map_err(|e| trf("Failed to run tar command: {}", &[&e.to_string()]))?;

    if status.success() {
        println!(
            "{}",
            trf(
                "Successfully imported all Rail data into: {}",
                &[&dest_dir.display().to_string()]
            )
        );
        println!(
            "{}",
            tr(
                "You can now launch 'rail' and all tabs, workspaces, logins, and accounts will be active."
            )
        );
        Ok(())
    } else {
        Err(trf(
            "tar command failed with status: {}",
            &[&status.to_string()],
        ))
    }
}

pub(crate) fn print_data_info() {
    let dir = rail_data_dir();
    println!(
        "{}",
        trf(
            "Rail Unified Storage Directory: {}",
            &[&dir.display().to_string()]
        )
    );
    println!("--------------------------------------------------");
    if !dir.exists() {
        println!(
            "{}",
            tr("Status: Directory does not exist yet (clean slate)")
        );
        return;
    }

    let session_file = dir.join("session.txt");
    if session_file.exists() {
        if let Ok(c) = std::fs::read_to_string(&session_file) {
            let tabs = c.lines().filter(|l| l.starts_with("tab\t")).count();
            println!(
                "{}",
                trf("  Session:       {} saved tabs", &[&tabs.to_string()])
            );
        }
    } else {
        println!(
            "{}",
            tr("  Session:       No saved session (clean start on next launch)")
        );
    }

    let profiles_file = dir.join("profiles.txt");
    if profiles_file.exists() {
        if let Ok(c) = std::fs::read_to_string(&profiles_file) {
            let count = c.lines().filter(|l| !l.trim().is_empty()).count();
            println!(
                "{}",
                trf(
                    "  Profiles:      {} profiles configured",
                    &[&count.to_string()]
                )
            );
        }
    }

    let profiles_dir = dir.join("profiles");
    if profiles_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    let has_cookies = p.join("data").join("cookies.sqlite").exists();
                    let has_storage = p.join("data").join("storage").exists();
                    println!(
                        "{}",
                        trf(
                            "    - Profile '{}': accounts/cookies={}, localStorage/IndexedDB={}",
                            &[&name, &has_cookies.to_string(), &has_storage.to_string()]
                        )
                    );
                }
            }
        }
    }

    println!();
    println!("{}", tr("To share your exact environment with a friend:"));
    println!("  Export:   rail --export rail_backup.tar.gz");
    println!("  Import:   rail --import rail_backup.tar.gz");
    println!(
        "{}",
        trf(
            "  Or copy:  cp -r {} /path/to/destination",
            &[&dir.display().to_string()]
        )
    );
}
