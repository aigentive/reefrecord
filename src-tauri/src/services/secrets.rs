use keyring::Entry;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.aigentive.reefrecord";
const GEMINI_ACCOUNT: &str = "gemini_api_key";

fn entry() -> AppResult<Entry> {
    Entry::new(SERVICE, GEMINI_ACCOUNT).map_err(Into::into)
}

pub fn save_gemini_key(key: &str) -> AppResult<()> {
    if key.trim().is_empty() {
        return Err(AppError::Invalid("Gemini key is empty.".into()));
    }
    entry()?.set_password(key.trim())?;
    Ok(())
}

pub fn read_gemini_key() -> AppResult<Option<String>> {
    match entry()?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

pub fn delete_gemini_key() -> AppResult<()> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

pub fn has_gemini_key() -> bool {
    matches!(read_gemini_key(), Ok(Some(_)))
}
