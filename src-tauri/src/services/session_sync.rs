use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::git_sync::{self, SyncOutcome};
use crate::services::sessions::{SessionStore, SyncStatus};
use crate::settings::SettingsStore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub session_id: String,
    pub status: SyncStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub async fn sync_session(
    settings: &SettingsStore,
    sessions: &SessionStore,
    session_id: String,
) -> AppResult<SyncResult> {
    let settings_snapshot = settings.get();
    let mut summary = sessions
        .get(&session_id)
        .ok_or_else(|| AppError::NotFound(format!("session {session_id} not found")))?;

    if !settings_snapshot.github_sync_enabled {
        summary.sync_status = SyncStatus::NotEnabled;
        summary.sync_error = None;
        sessions.upsert(summary.clone())?;
        return Ok(SyncResult {
            session_id: summary.id,
            status: SyncStatus::NotEnabled,
            message: Some("GitHub sync is disabled.".into()),
        });
    }

    summary.sync_status = SyncStatus::Syncing;
    summary.sync_error = None;
    sessions.upsert(summary.clone())?;

    let sessions_dir = sessions.sessions_dir()?;
    let result = tokio::task::spawn_blocking({
        let settings = settings_snapshot.clone();
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
            sessions.upsert(summary.clone())?;
            Ok(SyncResult {
                session_id: summary.id,
                status: SyncStatus::Synced,
                message: None,
            })
        }
        Ok(SyncOutcome::Skipped) => {
            summary.sync_status = SyncStatus::Skipped;
            summary.sync_error = None;
            sessions.upsert(summary.clone())?;
            Ok(SyncResult {
                session_id: summary.id,
                status: SyncStatus::Skipped,
                message: Some("Nothing new to push.".into()),
            })
        }
        Err(e) => {
            summary.sync_status = SyncStatus::Failed;
            summary.sync_error = Some(e.to_string());
            sessions.upsert(summary.clone())?;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use super::*;
    use crate::audio::format::AudioFormat;
    use crate::services::sessions::{SessionSummary, TranscriptionStatus};
    use crate::transcription::types::TranscriptionProvider;

    fn store_with_dir() -> (
        tempfile::TempDir,
        Arc<SettingsStore>,
        SessionStore,
        std::path::PathBuf,
    ) {
        let config = tempfile::tempdir().unwrap();
        let sessions_dir = config.path().join("sessions");
        let settings = Arc::new(SettingsStore::load_or_default(config.path()));
        settings
            .set_sessions_dir(sessions_dir.to_string_lossy().to_string())
            .unwrap();
        let store = SessionStore::new(settings.clone());
        (config, settings, store, sessions_dir)
    }

    fn summary(id: &str, sessions_dir: &Path) -> SessionSummary {
        SessionSummary {
            id: id.to_string(),
            started_at: "2026-05-11T10:00:00Z".parse().unwrap(),
            duration_seconds: 42,
            audio_path: Some(
                sessions_dir
                    .join(format!("{id}.wav"))
                    .to_string_lossy()
                    .to_string(),
            ),
            audio_format: AudioFormat::Wav,
            transcript_path: None,
            mic_device_name: Some("Studio Mic".into()),
            system_device_name: None,
            transcription_status: TranscriptionStatus::Complete,
            transcription_error: None,
            transcription_provider: Some(TranscriptionProvider::Gemini),
            transcription_prompt_tokens: None,
            transcription_output_tokens: None,
            transcription_total_tokens: None,
            transcription_cost_usd: None,
            transcription_model: Some("gemini-test".into()),
            transcription_usage: None,
            sync_status: SyncStatus::Queued,
            sync_error: Some("old error".into()),
            transcript_preview: None,
        }
    }

    #[tokio::test]
    async fn sync_disabled_persists_not_enabled_and_clears_error() {
        let (_config, settings, store, sessions_dir) = store_with_dir();
        let session = summary("session_20260511_100000", &sessions_dir);
        store.upsert(session).unwrap();

        let result = sync_session(&settings, &store, "session_20260511_100000".into())
            .await
            .unwrap();

        assert_eq!(result.status, SyncStatus::NotEnabled);
        assert_eq!(result.message.as_deref(), Some("GitHub sync is disabled."));
        let saved = store.get("session_20260511_100000").unwrap();
        assert_eq!(saved.sync_status, SyncStatus::NotEnabled);
        assert!(saved.sync_error.is_none());
    }
}
