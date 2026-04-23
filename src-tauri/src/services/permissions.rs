use std::process::Command;

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionKind {
    Microphone,
    SoundSettings,
    AudioMidiSetup,
}

pub fn open(kind: PermissionKind) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        let arg = match kind {
            PermissionKind::Microphone => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            PermissionKind::SoundSettings => "x-apple.systempreferences:com.apple.preference.sound",
            PermissionKind::AudioMidiSetup => {
                // Open the Audio MIDI Setup app directly.
                let status = Command::new("open")
                    .args(["-a", "Audio MIDI Setup"])
                    .status()
                    .map_err(|e| AppError::msg(format!("cannot open Audio MIDI Setup: {e}")))?;
                if !status.success() {
                    return Err(AppError::msg("Audio MIDI Setup could not be opened."));
                }
                return Ok(());
            }
        };
        let status = Command::new("open")
            .arg(arg)
            .status()
            .map_err(|e| AppError::msg(format!("cannot open settings: {e}")))?;
        if !status.success() {
            return Err(AppError::msg("Could not open system settings."));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = kind;
        Err(AppError::msg(
            "Permission shortcuts are only implemented for macOS in this MVP.",
        ))
    }
}

pub fn reveal_path(path: &str) -> AppResult<()> {
    if path.is_empty() {
        return Err(AppError::Invalid("path is empty".into()));
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|e| AppError::msg(format!("cannot reveal path: {e}")))?;
        if !status.success() {
            return Err(AppError::msg("Finder could not reveal that path."));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(AppError::msg("Reveal is only implemented for macOS in this MVP."))
    }
}

pub fn open_folder(path: &str) -> AppResult<()> {
    if path.is_empty() {
        return Err(AppError::Invalid("path is empty".into()));
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(path)
            .status()
            .map_err(|e| AppError::msg(format!("cannot open folder: {e}")))?;
        if !status.success() {
            return Err(AppError::msg("Finder could not open that folder."));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(AppError::msg("Open folder is only implemented for macOS in this MVP."))
    }
}
