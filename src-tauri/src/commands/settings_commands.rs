use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::commands::status_commands::ProviderStatusDto;
use crate::error::{AppError, AppResult};
use crate::services::devices::{self, AudioDevice};
use crate::services::permissions::{self, PermissionKind};
use crate::services::secrets;
use crate::settings::{Settings, SettingsInput};
use crate::transcription;
use crate::transcription::types::TranscriptionProvider;
use crate::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.settings.get())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    input: SettingsInput,
) -> AppResult<Settings> {
    state.settings.update(input)
}

#[tauri::command]
pub async fn save_transcription_key(
    state: State<'_, AppState>,
    provider: TranscriptionProvider,
    key: String,
) -> AppResult<ProviderStatusDto> {
    secrets::save_transcription_key(provider, &key)?;
    let settings = state.settings.get();
    let status = match transcription::validate_key(provider, key.trim(), &settings).await {
        Ok(msg) => ProviderStatusDto::ready(msg),
        Err(e) => ProviderStatusDto::warning(format!("Saved. Validation: {e}")),
    };
    let mut cached = state.transcription_last_validation.write().await;
    cached.insert(provider, status.clone());
    Ok(status)
}

#[tauri::command]
pub async fn has_transcription_key(provider: TranscriptionProvider) -> AppResult<bool> {
    Ok(secrets::has_transcription_key(provider))
}

#[tauri::command]
pub async fn delete_transcription_key(
    state: State<'_, AppState>,
    provider: TranscriptionProvider,
) -> AppResult<()> {
    secrets::delete_transcription_key(provider)?;
    let mut cached = state.transcription_last_validation.write().await;
    cached.remove(&provider);
    Ok(())
}

#[tauri::command]
pub async fn validate_transcription_key(
    state: State<'_, AppState>,
    provider: TranscriptionProvider,
    key: Option<String>,
) -> AppResult<ProviderStatusDto> {
    let settings = state.settings.get();
    let transient = matches!(&key, Some(k) if !k.trim().is_empty());
    let effective = match key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => secrets::read_transcription_key(provider)?
            .ok_or_else(|| AppError::Invalid(format!("No {} API key saved.", provider.label())))?,
    };
    let status = match transcription::validate_key(provider, &effective, &settings).await {
        Ok(msg) => ProviderStatusDto::ready(msg),
        Err(e) => ProviderStatusDto::warning(format!("Validation failed: {e}")),
    };
    if !transient {
        let mut cached = state.transcription_last_validation.write().await;
        cached.insert(provider, status.clone());
    }
    Ok(status)
}

#[tauri::command]
pub async fn select_sessions_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<String>> {
    let current = state.settings.get().sessions_dir;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let dialog = app.dialog().file();
    let dialog = match current.as_deref() {
        Some(p) if !p.is_empty() => dialog.set_directory(p),
        _ => dialog,
    };
    dialog.pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let selected = rx
        .await
        .map_err(|_| AppError::msg("folder picker cancelled"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path_str = selected.to_string();
    state.settings.set_sessions_dir(path_str.clone())?;
    // Create the directory if missing.
    if !std::path::Path::new(&path_str).exists() {
        std::fs::create_dir_all(&path_str)?;
    }
    state.sessions.reload()?;
    Ok(Some(path_str))
}

#[tauri::command]
pub async fn reveal_sessions_folder(state: State<'_, AppState>) -> AppResult<()> {
    let path = state
        .settings
        .get()
        .sessions_dir
        .ok_or_else(|| AppError::msg("No sessions folder set."))?;
    permissions::open_folder(&path)
}

#[tauri::command]
pub async fn reveal_path(path: String) -> AppResult<()> {
    permissions::reveal_path(&path)
}

#[tauri::command]
pub async fn open_permissions_settings(kind: PermissionKind) -> AppResult<()> {
    permissions::open(kind)
}

#[tauri::command]
pub async fn list_audio_devices() -> AppResult<Vec<AudioDevice>> {
    devices::list_input_devices()
}
