use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::commands::status_commands::ProviderStatusDto;
use crate::error::{AppError, AppResult};
use crate::gemini::client::GeminiClient;
use crate::services::devices::{self, AudioDevice};
use crate::services::permissions::{self, PermissionKind};
use crate::services::secrets;
use crate::settings::{Settings, SettingsInput};
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
pub async fn save_gemini_key(
    state: State<'_, AppState>,
    key: String,
) -> AppResult<ProviderStatusDto> {
    secrets::save_gemini_key(&key)?;
    // Validate after save.
    let settings = state.settings.get();
    let client = GeminiClient::new(key.trim().to_string())?;
    let status = match client.validate(&settings.gemini_model).await {
        Ok(msg) => ProviderStatusDto::ready(msg),
        Err(e) => ProviderStatusDto::warning(format!("Saved. Validation: {e}")),
    };
    let mut cached = state.gemini_last_validation.write().await;
    *cached = Some(status.clone());
    Ok(status)
}

#[tauri::command]
pub async fn has_gemini_key() -> AppResult<bool> {
    Ok(secrets::has_gemini_key())
}

#[tauri::command]
pub async fn delete_gemini_key(state: State<'_, AppState>) -> AppResult<()> {
    secrets::delete_gemini_key()?;
    let mut cached = state.gemini_last_validation.write().await;
    *cached = None;
    Ok(())
}

#[tauri::command]
pub async fn validate_gemini_key(state: State<'_, AppState>) -> AppResult<ProviderStatusDto> {
    let key = secrets::read_gemini_key()?
        .ok_or_else(|| AppError::Invalid("No Gemini API key saved.".into()))?;
    let settings = state.settings.get();
    let client = GeminiClient::new(key)?;
    let status = match client.validate(&settings.gemini_model).await {
        Ok(msg) => ProviderStatusDto::ready(msg),
        Err(e) => ProviderStatusDto::warning(format!("Validation failed: {e}")),
    };
    let mut cached = state.gemini_last_validation.write().await;
    *cached = Some(status.clone());
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
