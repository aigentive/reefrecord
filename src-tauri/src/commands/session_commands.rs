use serde::Serialize;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::gemini::client::GeminiClient;
use crate::gemini::transcription::{transcribe, TranscriptionJob};
use crate::git_sync::{self, GitSyncStatus, SyncOutcome};
use crate::services::recording::RecordingInput;
use crate::services::secrets;
use crate::services::sessions::{SessionSummary, SyncStatus, TranscriptionStatus};
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptResult {
    pub session_id: String,
    pub transcript_path: String,
    pub status: TranscriptionStatus,
}

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
) -> AppResult<TranscriptResult> {
    let key = secrets::read_gemini_key()?
        .ok_or_else(|| AppError::Invalid("No Gemini API key saved.".into()))?;
    let settings = state.settings.get();
    let mut summary = state
        .sessions
        .get(&session_id)
        .ok_or_else(|| AppError::NotFound(format!("session {session_id} not found")))?;

    summary.transcription_status = TranscriptionStatus::Transcribing;
    summary.transcription_error = None;
    summary.transcription_prompt_tokens = None;
    summary.transcription_output_tokens = None;
    summary.transcription_total_tokens = None;
    summary.transcription_cost_usd = None;
    summary.transcription_model = None;
    state.sessions.upsert(summary.clone())?;

    let client = GeminiClient::new(key)?;
    let wav_path = std::path::PathBuf::from(&summary.wav_path);
    let job = TranscriptionJob {
        wav_path: wav_path.clone(),
        primary_model: settings.gemini_model.clone(),
        fallback_model: settings.gemini_fallback_model.clone(),
        chunk_minutes: settings.chunk_minutes,
        language_hint: settings.language_hint.clone(),
        include_speaker_labels: settings.include_speaker_labels,
        include_timestamps: settings.include_timestamps,
    };
    let transcript_path = wav_path.with_file_name(format!(
        "{}_gemini.txt",
        wav_path.file_stem().and_then(|s| s.to_str()).unwrap_or(&summary.id)
    ));

    match transcribe(&client, job).await {
        Ok(outcome) => {
            std::fs::write(&transcript_path, &outcome.text)?;
            let usd = (outcome.usage.prompt_tokens as f64
                * settings.gemini_input_cost_per_million_usd
                + outcome.usage.output_tokens as f64
                    * settings.gemini_output_cost_per_million_usd)
                / 1_000_000.0;
            summary.transcript_path = Some(transcript_path.to_string_lossy().to_string());
            summary.transcription_status = TranscriptionStatus::Complete;
            summary.transcription_error = None;
            summary.transcription_prompt_tokens = Some(outcome.usage.prompt_tokens);
            summary.transcription_output_tokens = Some(outcome.usage.output_tokens);
            summary.transcription_total_tokens = Some(outcome.usage.total_tokens);
            summary.transcription_cost_usd = Some(usd);
            summary.transcription_model = Some(outcome.model_used);
            state.sessions.upsert(summary.clone())?;
            Ok(TranscriptResult {
                session_id: summary.id,
                transcript_path: transcript_path.to_string_lossy().to_string(),
                status: TranscriptionStatus::Complete,
            })
        }
        Err(e) => {
            summary.transcription_status = TranscriptionStatus::Failed;
            summary.transcription_error = Some(e.to_string());
            state.sessions.upsert(summary.clone())?;
            Err(e)
        }
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
pub async fn sync_session(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<SyncResult> {
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

    let result = tokio::task::spawn_blocking({
        let settings = settings.clone();
        let session = summary.clone();
        move || git_sync::push_session(&settings, &session)
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
