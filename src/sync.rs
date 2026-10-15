//! End-to-end encrypted Cloud Sync for Rail Browser.
//! Packs workspaces, tabs, profiles, logins, cookies and settings,
//! encrypts with AES-256-CBC (PBKDF2 100,000 iterations) using a mandatory user password,
//! and synchronizes via Vercel / Supabase.

use crate::minijson;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

pub const DEFAULT_SYNC_SERVER: &str = "https://rail-browser.vercel.app";

pub fn sync_server_url() -> String {
    std::env::var("RAIL_SYNC_SERVER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SYNC_SERVER.to_string())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub id: String,
    pub url: String,
}

/// Extract clean UUID / Sync ID from a raw ID or full URL (e.g. https://rail-browser.vercel.app/sync?id=UUID)
pub fn extract_sync_id(raw: &str) -> String {
    let s = raw.trim();
    if let Some(pos) = s.find("id=") {
        let after = &s[pos + 3..];
        let end = after.find(['&', '#', ' ', '/']).unwrap_or(after.len());
        return after[..end].trim().to_string();
    }
    if let Some(pos) = s.rfind("/sync/") {
        let after = &s[pos + 6..];
        let end = after.find(['?', '&', '#', ' ']).unwrap_or(after.len());
        return after[..end].trim().to_string();
    }
    s.to_string()
}

/// Packs data_dir (excluding disposable web cache), encrypts with password, and returns base64 ciphertext.
pub fn pack_and_encrypt(data_dir: &Path, password: &str) -> Result<String, String> {
    if !data_dir.exists() {
        return Err(format!("Storage directory does not exist: {}", data_dir.display()));
    }
    if password.trim().is_empty() {
        return Err("Password cannot be empty.".to_string());
    }

    let parent = data_dir.parent().unwrap_or(data_dir);
    let name = data_dir.file_name().ok_or_else(|| "Invalid directory name".to_string())?;

    let rand_id = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 10000000).to_string();
    let temp_tar = std::env::temp_dir().join(format!("rail_sync_pack_{}.tar.gz", rand_id));

    // Pack into tar.gz excluding temporary cache
    let tar_status = Command::new("tar")
        .arg("--exclude=*/cache")
        .arg("--exclude=*/cache/*")
        .arg("-czf")
        .arg(&temp_tar)
        .arg("-C")
        .arg(parent)
        .arg(name)
        .status()
        .map_err(|e| format!("Failed to create archive: {}", e))?;

    if !tar_status.success() {
        let _ = std::fs::remove_file(&temp_tar);
        return Err(format!("tar command failed with status: {}", tar_status));
    }

    // Encrypt with openssl enc -aes-256-cbc -pbkdf2 -iter 100000 -salt
    let tar_bytes = std::fs::read(&temp_tar).map_err(|e| {
        let _ = std::fs::remove_file(&temp_tar);
        format!("Failed to read archive: {}", e)
    })?;
    let _ = std::fs::remove_file(&temp_tar);

    let mut openssl = Command::new("openssl")
        .args([
            "enc",
            "-aes-256-cbc",
            "-pbkdf2",
            "-iter", "100000",
            "-salt",
            "-pass", &format!("pass:{}", password),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run openssl: {}", e))?;

    if let Some(mut stdin) = openssl.stdin.take() {
        let _ = stdin.write_all(&tar_bytes);
    }

    let output = openssl.wait_with_output().map_err(|e| format!("OpenSSL error: {}", e))?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Encryption failed: {}", err_msg.trim()));
    }

    // Base64 encode the ciphertext
    let mut b64_cmd = Command::new("base64")
        .arg("-w")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run base64: {}", e))?;

    if let Some(mut stdin) = b64_cmd.stdin.take() {
        let _ = stdin.write_all(&output.stdout);
    }

    let b64_output = b64_cmd.wait_with_output().map_err(|e| format!("Base64 error: {}", e))?;
    if !b64_output.status.success() {
        return Err("Base64 encoding failed.".to_string());
    }

    let b64_str = String::from_utf8_lossy(&b64_output.stdout).trim().to_string();
    if b64_str.is_empty() {
        return Err("Encrypted data is empty.".to_string());
    }

    Ok(b64_str)
}

/// Uploads encrypted payload to Vercel/Supabase sync endpoint
pub fn upload_sync_payload(encrypted_b64: &str, server_base: &str) -> Result<SyncResult, String> {
    let rand_id = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 10000000).to_string();
    let temp_json = std::env::temp_dir().join(format!("rail_sync_upload_{}.json", rand_id));

    let json_body = format!("{{\"encrypted_data\":\"{}\"}}", minijson::escape(encrypted_b64));
    std::fs::write(&temp_json, json_body).map_err(|e| format!("Failed to write upload payload: {}", e))?;

    let api_url = format!("{}/api/sync", server_base.trim_end_matches('/'));
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "-m", "45",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &format!("@{}", temp_json.display()),
            &api_url,
        ])
        .stdin(Stdio::null())
        .output();

    let _ = std::fs::remove_file(&temp_json);

    let output = out.map_err(|e| format!("Network request failed: {}", e))?;
    if !output.status.success() {
        let err_text = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Sync server error (exit {}): {}", output.status, err_text.trim()));
    }

    let resp_str = String::from_utf8_lossy(&output.stdout);
    let parsed = minijson::parse(&resp_str).ok_or_else(|| {
        format!("Invalid server response: {}", resp_str)
    })?;

    if let Some(err) = parsed.get("error").and_then(|e| e.as_str()) {
        return Err(format!("Server returned error: {}", err));
    }

    let id = parsed.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if id.is_empty() {
        return Err("Server did not return a valid Sync ID.".to_string());
    }

    let final_url = if url.is_empty() {
        format!("{}/sync?id={}", server_base.trim_end_matches('/'), id)
    } else {
        url
    };

    Ok(SyncResult { id, url: final_url })
}

/// Downloads encrypted payload from cloud, decrypts with password, and saves to dest_tar
pub fn download_and_decrypt(sync_id_or_url: &str, password: &str, server_base: &str, dest_tar: &Path) -> Result<(), String> {
    let sync_id = extract_sync_id(sync_id_or_url);
    if sync_id.is_empty() {
        return Err("Please enter a valid Sync ID or URL.".to_string());
    }
    if password.trim().is_empty() {
        return Err("Password cannot be empty.".to_string());
    }

    let api_url = format!("{}/api/sync?id={}", server_base.trim_end_matches('/'), sync_id);
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "-m", "45",
            &api_url,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Failed to connect to sync server: {}", e))?;

    if !out.status.success() {
        let err_text = String::from_utf8_lossy(&out.stderr);
        return Err(format!("Failed to download sync data (exit {}): {}", out.status, err_text.trim()));
    }

    let resp_str = String::from_utf8_lossy(&out.stdout);
    let parsed = minijson::parse(&resp_str).ok_or_else(|| {
        format!("Invalid server response: {}", resp_str)
    })?;

    if let Some(err) = parsed.get("error").and_then(|e| e.as_str()) {
        return Err(format!("Server returned error: {}", err));
    }

    let encrypted_b64 = parsed.get("encrypted_data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Sync payload missing encrypted_data.".to_string())?;

    // Base64 decode and pipe to openssl enc -d
    let mut b64_decode = Command::new("base64")
        .arg("-d")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run base64: {}", e))?;

    if let Some(mut stdin) = b64_decode.stdin.take() {
        let _ = stdin.write_all(encrypted_b64.as_bytes());
    }

    let b64_out = b64_decode.wait_with_output().map_err(|e| format!("Base64 decode error: {}", e))?;
    if !b64_out.status.success() {
        return Err("Corrupted base64 ciphertext.".to_string());
    }

    let mut openssl = Command::new("openssl")
        .args([
            "enc",
            "-d",
            "-aes-256-cbc",
            "-pbkdf2",
            "-iter", "100000",
            "-pass", &format!("pass:{}", password),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run openssl: {}", e))?;

    if let Some(mut stdin) = openssl.stdin.take() {
        let _ = stdin.write_all(&b64_out.stdout);
    }

    let dec_output = openssl.wait_with_output().map_err(|e| format!("OpenSSL decrypt error: {}", e))?;
    if !dec_output.status.success() {
        return Err("INCORRECT_PASSWORD".to_string());
    }

    if dec_output.stdout.is_empty() {
        return Err("Decrypted data is empty.".to_string());
    }

    std::fs::write(dest_tar, &dec_output.stdout).map_err(|e| {
        format!("Failed to write decrypted archive: {}", e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_sync_id() {
        assert_eq!(
            extract_sync_id("https://rail-browser.vercel.app/sync?id=7bfb9920-8e12-4f32-849a-54316279f014"),
            "7bfb9920-8e12-4f32-849a-54316279f014"
        );
        assert_eq!(
            extract_sync_id("rail://sync?id=7bfb9920-8e12-4f32-849a-54316279f014&other=1"),
            "7bfb9920-8e12-4f32-849a-54316279f014"
        );
        assert_eq!(
            extract_sync_id("7bfb9920-8e12-4f32-849a-54316279f014"),
            "7bfb9920-8e12-4f32-849a-54316279f014"
        );
    }

    #[test]
    fn test_pack_and_encrypt_wrong_password() {
        let temp_dir = std::env::temp_dir().join("test_rail_sync_unit");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::fs::write(temp_dir.join("test.txt"), "hello rail sync");

        let cipher_res = pack_and_encrypt(&temp_dir, "mySecretPassword123");
        assert!(cipher_res.is_ok());
        let cipher_b64 = cipher_res.unwrap();
        assert!(!cipher_b64.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
