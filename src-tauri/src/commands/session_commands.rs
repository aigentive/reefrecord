use serde::Serialize;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::git_sync::{self, GitSyncStatus, SyncOutcome};
use crate::services::recording::RecordingInput;
use crate::services::secrets;
use crate::services::sessions::{SessionSummary, SyncStatus, TranscriptionStatus};
use crate::transcription;
use crate::transcription::types::{TranscriptionProvider, TranscriptionUsage};
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub session_id: String,
    pub status: SyncStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

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
    let settings = state.settings.get();
    let provider = provider.unwrap_or(settings.transcription_provider);
    let key = secrets::read_transcription_key(provider)?
        .ok_or_else(|| AppError::Invalid(format!("No {} API key saved.", provider.label())))?;
    let mut summary = state
        .sessions
        .get(&session_id)
        .ok_or_else(|| AppError::NotFound(format!("session {session_id} not found")))?;

    let wav_path = match summary.wav_path.clone() {
        Some(p) if std::path::Path::new(&p).exists() => std::path::PathBuf::from(p),
        _ => {
            return Err(AppError::Invalid(
                "WAV file is no longer on disk. Clear the session and re-record.".into(),
            ));
        }
    };

    summary.transcription_status = TranscriptionStatus::Transcribing;
    summary.transcription_error = None;
    summary.transcription_prompt_tokens = None;
    summary.transcription_output_tokens = None;
    summary.transcription_total_tokens = None;
    summary.transcription_cost_usd = None;
    summary.transcription_model = None;
    summary.transcription_provider = Some(provider);
    summary.transcription_usage = None;
    state.sessions.upsert(summary.clone())?;

    let transcript_path = wav_path.with_file_name(format!(
        "{}_{}.txt",
        wav_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&summary.id),
        provider
    ));

    match transcription::transcribe(provider, key, &settings, wav_path.clone()).await {
        Ok(outcome) => {
            remove_replaced_transcript(&summary, &transcript_path);
            std::fs::write(&transcript_path, &outcome.text)?;
            let usd = transcription::estimate_cost(&settings, &outcome);
            summary.transcript_path = Some(transcript_path.to_string_lossy().to_string());
            summary.transcription_status = TranscriptionStatus::Complete;
            summary.transcription_error = None;
            set_legacy_usage_fields(&mut summary, &outcome.usage);
            summary.transcription_cost_usd = Some(usd);
            summary.transcription_model = Some(outcome.model_used);
            summary.transcription_provider = Some(outcome.provider);
            summary.transcription_usage = Some(outcome.usage);
            state.sessions.upsert(summary.clone())?;
            Ok(summary)
        }
        Err(e) => {
            summary.transcription_status = TranscriptionStatus::Failed;
            summary.transcription_error = Some(e.to_string());
            state.sessions.upsert(summary.clone())?;
            Err(e)
        }
    }
}

fn set_legacy_usage_fields(summary: &mut SessionSummary, usage: &TranscriptionUsage) {
    match usage {
        TranscriptionUsage::Tokens {
            prompt_tokens,
            output_tokens,
            total_tokens,
            ..
        } => {
            summary.transcription_prompt_tokens = Some(*prompt_tokens);
            summary.transcription_output_tokens = Some(*output_tokens);
            summary.transcription_total_tokens = Some(*total_tokens);
        }
        _ => {
            summary.transcription_prompt_tokens = None;
            summary.transcription_output_tokens = None;
            summary.transcription_total_tokens = None;
        }
    }
}

fn remove_replaced_transcript(summary: &SessionSummary, next_path: &std::path::Path) {
    let Some(existing) = summary.transcript_path.as_deref() else {
        return;
    };
    let existing_path = std::path::Path::new(existing);
    if existing_path == next_path || !existing_path.exists() {
        return;
    }
    let Some(name) = existing_path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let same_parent = existing_path.parent() == next_path.parent();
    if same_parent && name.starts_with(&summary.id) && name.ends_with(".txt") {
        let _ = std::fs::remove_file(existing_path);
    }
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
    let settings = state.settings.get();
    let mut summary = state
        .sessions
        .get(&session_id)
        .ok_or_else(|| AppError::NotFound(format!("session {session_id} not found")))?;

    if !settings.github_sync_enabled {
        summary.sync_status = SyncStatus::NotEnabled;
        summary.sync_error = None;
        state.sessions.upsert(summary.clone())?;
        return Ok(SyncResult {
            session_id: summary.id,
            status: SyncStatus::NotEnabled,
            message: Some("GitHub sync is disabled.".into()),
        });
    }

    summary.sync_status = SyncStatus::Syncing;
    summary.sync_error = None;
    state.sessions.upsert(summary.clone())?;

    let sessions_dir = state.sessions.sessions_dir()?;
    let result = tokio::task::spawn_blocking({
        let settings = settings.clone();
        let session = summary.clone();
        let sessions_dir = sessions_dir.clone();
        move || git_sync::push_session(&settings, &session, &sessions_dir)
    })
    .await
    .map_err(|e| AppError::Git(format!("join error: {e}")))?;

    match result {
        Ok(SyncOutcome::Synced) => {
            summary.sync_status = SyncStatus::Synced;
            summary.sync_error = None;
            state.sessions.upsert(summary.clone())?;
            Ok(SyncResult {
                session_id: summary.id,
                status: SyncStatus::Synced,
                message: None,
            })
        }
        Ok(SyncOutcome::Skipped) => {
            summary.sync_status = SyncStatus::Skipped;
            summary.sync_error = None;
            state.sessions.upsert(summary.clone())?;
            Ok(SyncResult {
                session_id: summary.id,
                status: SyncStatus::Skipped,
                message: Some("Nothing new to push.".into()),
            })
        }
        Err(e) => {
            summary.sync_status = SyncStatus::Failed;
            summary.sync_error = Some(e.to_string());
            state.sessions.upsert(summary.clone())?;
            Err(e)
        }
    }
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
pub async fn clear_session_wav(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<SessionSummary> {
    state.sessions.clear_wav(&session_id)
}

#[tauri::command]
pub async fn delete_all_sessions(state: State<'_, AppState>) -> AppResult<usize> {
    state.sessions.delete_all()
}

#[tauri::command]
pub async fn clear_all_wavs(state: State<'_, AppState>) -> AppResult<usize> {
    state.sessions.clear_all_wavs()
}
