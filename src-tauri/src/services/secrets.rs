use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Filename inside the app config dir. Intentionally opaque.
const SECRET_FILENAME: &str = "secrets.bin";
const SALT: &[u8] = b"com.aigentive.reefrecord::gemini_api_key::v1";
const NONCE_LEN: usize = 12;
const GEMINI_RECORD_MARKER: &[u8] = b"gemini_api_key=";

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Must be called once from the Tauri setup so we know where to persist the
/// encrypted secret file.
pub fn set_config_dir(path: PathBuf) {
    let _ = CONFIG_DIR.set(path);
}

fn config_dir() -> AppResult<&'static PathBuf> {
    CONFIG_DIR
        .get()
        .ok_or_else(|| AppError::msg("secrets config dir not initialized"))
}

fn secret_path() -> AppResult<PathBuf> {
    Ok(config_dir()?.join(SECRET_FILENAME))
}

/// Derive a 32-byte AES-256 key from the machine's `IOPlatformUUID` plus a
/// fixed salt. The UUID is stable across reboots and unique per machine. If
/// the UUID is unavailable we fall back to a salt-only key; the worst-case
/// outcome is that the stored secret file is decryptable if an attacker can
/// read both the file and this source code, which is no weaker than the
/// previous keychain-in-a-no-op-backend state.
fn derive_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SALT);
    if let Some(id) = machine_identifier() {
        hasher.update(id.as_bytes());
    }
    let out = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&out);
    key
}

#[cfg(target_os = "macos")]
fn machine_identifier() -> Option<String> {
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("\"IOPlatformUUID\" = \"") {
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn machine_identifier() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string())
}

fn encrypt(plaintext: &[u8]) -> AppResult<Vec<u8>> {
    let key_bytes = derive_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AppError::msg(format!("encrypt failed: {e}")))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn decrypt(blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() <= NONCE_LEN {
        return Err(AppError::msg("secrets file is truncated"));
    }
    let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
    let key_bytes = derive_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ct)
        .map_err(|_| AppError::msg("could not decrypt secrets — key was saved on a different machine or user"))
}

fn read_record() -> AppResult<Option<Vec<u8>>> {
    let path = secret_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&path)?;
    Ok(Some(decrypt(&raw)?))
}

fn write_record(plaintext: &[u8]) -> AppResult<()> {
    let dir = config_dir()?;
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    let path = secret_path()?;
    let tmp = path.with_extension("bin.tmp");
    let blob = encrypt(plaintext)?;
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&blob)?;
        f.sync_all()?;
    }
    set_owner_only(&tmp)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) -> AppResult<()> {
    Ok(())
}

pub fn save_gemini_key(key: &str) -> AppResult<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::Invalid("Gemini key is empty.".into()));
    }
    let mut record = GEMINI_RECORD_MARKER.to_vec();
    record.extend_from_slice(key.as_bytes());
    write_record(&record)
}

pub fn read_gemini_key() -> AppResult<Option<String>> {
    let record = match read_record()? {
        Some(r) => r,
        None => return Ok(None),
    };
    if !record.starts_with(GEMINI_RECORD_MARKER) {
        return Err(AppError::msg("secrets file has unexpected format"));
    }
    let key_bytes = &record[GEMINI_RECORD_MARKER.len()..];
    let key = std::str::from_utf8(key_bytes)
        .map_err(|_| AppError::msg("secrets file contains invalid utf8"))?;
    Ok(Some(key.to_string()))
}

pub fn delete_gemini_key() -> AppResult<()> {
    let path = secret_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn has_gemini_key() -> bool {
    matches!(read_gemini_key(), Ok(Some(_)))
}
