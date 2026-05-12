use std::path::PathBuf;

use keyring::Entry;

use crate::error::{AppError, AppResult};
use crate::transcription::types::TranscriptionProvider;

const SERVICE: &str = "com.aigentive.reefrecord";

/// Retained as a setup hook so app initialization can remove the legacy
/// encrypted-file secret store. Secrets now live only in the OS credential
/// store, not the app config directory.
pub fn set_config_dir(path: PathBuf) {
    let legacy = path.join("secrets.bin");
    if legacy.exists() {
        let _ = std::fs::remove_file(legacy);
    }
}

pub fn save_transcription_key(provider: TranscriptionProvider, key: &str) -> AppResult<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::Invalid(format!(
            "{} key is empty.",
            provider.label()
        )));
    }
    entry(provider)?.set_password(key).map_err(keyring_error)?;
    match read_transcription_key(provider)? {
        Some(saved) if saved == key => Ok(()),
        Some(_) => Err(AppError::Invalid(format!(
            "{} key was saved but did not round-trip from the credential store.",
            provider.label()
        ))),
        None => Err(AppError::Invalid(format!(
            "{} key was not readable after saving.",
            provider.label()
        ))),
    }
}

pub fn read_transcription_key(provider: TranscriptionProvider) -> AppResult<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keyring_error(e)),
    }
}

pub fn delete_transcription_key(provider: TranscriptionProvider) -> AppResult<()> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keyring_error(e)),
    }
}

pub fn has_transcription_key(provider: TranscriptionProvider) -> bool {
    matches!(read_transcription_key(provider), Ok(Some(_)))
}

pub fn read_gemini_key() -> AppResult<Option<String>> {
    read_transcription_key(TranscriptionProvider::Gemini)
}

fn entry(provider: TranscriptionProvider) -> AppResult<Entry> {
    Entry::new(SERVICE, provider.key_account()).map_err(keyring_error)
}

fn keyring_error(e: keyring::Error) -> AppError {
    AppError::Invalid(format!("credential store error: {e}"))
}
