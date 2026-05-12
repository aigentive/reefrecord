use tauri::State;

use crate::error::{AppError, AppResult};
use crate::git_sync::{self, GitSyncStatus};
use crate::services::recording::RecordingInput;
use crate::services::session_sync::{self, SyncResult};
use crate::services::session_transcription;
use crate::services::sessions::SessionSummary;
use crate::transcription::types::TranscriptionProvider;
use crate::AppState;

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> AppResult<Vec<SessionSummary>> {
    state.sessions.reload()?;
    Ok(state.sessions.list())
}

#[tauri::command]
pub async fn start_recording(
    state: State<'_, AppState>,
    input: RecordingInput,
) -> AppResult<String> {
    state.recording.start(input)
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<SessionSummary> {
    state.recording.stop(&session_id)
}

#[tauri::command]
pub async fn transcribe_session(
    state: State<'_, AppState>,
    session_id: String,
    provider: Option<TranscriptionProvider>,
) -> AppResult<SessionSummary> {
    session_transcription::transcribe_session(
        state.settings.as_ref(),
        state.sessions.as_ref(),
        session_id,
        provider,
    )
    .await
}

#[tauri::command]
pub async fn read_transcript(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<Option<String>> {
    let summary = state
        .sessions
        .get(&session_id)
        .ok_or_else(|| AppError::NotFound(format!("session {session_id} not found")))?;
    match summary.transcript_path {
        Some(p) => {
            if !std::path::Path::new(&p).exists() {
                return Ok(None);
            }
            let text = std::fs::read_to_string(&p)?;
            Ok(Some(text))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn sync_session(state: State<'_, AppState>, session_id: String) -> AppResult<SyncResult> {
    session_sync::sync_session(state.settings.as_ref(), state.sessions.as_ref(), session_id).await
}

#[tauri::command]
pub async fn validate_git_sync_settings(state: State<'_, AppState>) -> AppResult<GitSyncStatus> {
    let settings = state.settings.get();
    Ok(git_sync::validate(&settings))
}

#[tauri::command]
pub async fn delete_session(state: State<'_, AppState>, session_id: String) -> AppResult<()> {
    state.sessions.delete(&session_id)
}

#[tauri::command]
pub async fn clear_session_audio(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<SessionSummary> {
    state.sessions.clear_audio(&session_id)
}

#[tauri::command]
pub async fn delete_all_sessions(state: State<'_, AppState>) -> AppResult<usize> {
    state.sessions.delete_all()
}

#[tauri::command]
pub async fn clear_all_audio(state: State<'_, AppState>) -> AppResult<usize> {
    state.sessions.clear_all_audio()
}
