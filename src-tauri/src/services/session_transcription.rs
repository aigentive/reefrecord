use std::path::{Path, PathBuf};

use crate::audio::format::AudioFormat;
use crate::audio::writer::{transcode_flac_to_wav, transcode_wav_to_flac};
use crate::error::{AppError, AppResult};
use crate::services::secrets;
use crate::services::sessions::{SessionStore, SessionSummary, TranscriptionStatus};
use crate::settings::SettingsStore;
use crate::transcription;
use crate::transcription::types::{
    TranscriptionOutcome, TranscriptionProvider, TranscriptionUsage,
};

pub async fn transcribe_session(
    settings: &SettingsStore,
    sessions: &SessionStore,
    session_id: String,
    provider: Option<TranscriptionProvider>,
) -> AppResult<SessionSummary> {
    let settings_snapshot = settings.get();
    let provider = provider.unwrap_or(settings_snapshot.transcription_provider);
    let mut summary = sessions
        .get(&session_id)
        .ok_or_else(|| AppError::NotFound(format!("session {session_id} not found")))?;

    let key = match secrets::read_transcription_key(provider) {
        Ok(Some(key)) => key,
        Ok(None) => {
            return fail_transcription_preflight(
                sessions,
                summary,
                provider,
                AppError::Invalid(format!("No {} API key saved.", provider.label())),
            );
        }
        Err(e) => return fail_transcription_preflight(sessions, summary, provider, e),
    };

    let audio_path = match summary.audio_path.clone() {
        Some(p) if Path::new(&p).exists() => PathBuf::from(p),
        _ => {
            return fail_transcription_preflight(
                sessions,
                summary,
                provider,
                AppError::Invalid(
                    "Audio file is no longer on disk. Clear the session and re-record.".into(),
                ),
            );
        }
    };
    let audio_format = AudioFormat::from_path(&audio_path).unwrap_or(summary.audio_format);

    begin_transcription(sessions, &mut summary, provider)?;
    let transcript_path = transcript_path_for(&summary, &audio_path, provider);

    match transcription::transcribe(provider, key, &settings_snapshot, audio_path, audio_format)
        .await
    {
        Ok(outcome) => {
            let cost_usd = transcription::estimate_cost(&settings_snapshot, &outcome);
            complete_transcription(
                sessions,
                &mut summary,
                &transcript_path,
                settings_snapshot.audio_storage_format,
                outcome,
                cost_usd,
            )?;
            Ok(summary)
        }
        Err(e) => fail_transcription(sessions, summary, e),
    }
}

fn begin_transcription(
    sessions: &SessionStore,
    summary: &mut SessionSummary,
    provider: TranscriptionProvider,
) -> AppResult<()> {
    summary.transcription_status = TranscriptionStatus::Transcribing;
    summary.transcription_error = None;
    summary.transcription_prompt_tokens = None;
    summary.transcription_output_tokens = None;
    summary.transcription_total_tokens = None;
    summary.transcription_cost_usd = None;
    summary.transcription_model = None;
    summary.transcription_provider = Some(provider);
    summary.transcription_usage = None;
    sessions.upsert(summary.clone())
}

fn complete_transcription(
    sessions: &SessionStore,
    summary: &mut SessionSummary,
    transcript_path: &Path,
    desired_audio_format: AudioFormat,
    outcome: TranscriptionOutcome,
    cost_usd: f64,
) -> AppResult<()> {
    remove_replaced_transcript(summary, transcript_path);
    std::fs::write(transcript_path, &outcome.text)?;
    ensure_audio_storage(summary, desired_audio_format)?;

    summary.transcript_path = Some(transcript_path.to_string_lossy().to_string());
    summary.transcription_status = TranscriptionStatus::Complete;
    summary.transcription_error = None;
    set_legacy_usage_fields(summary, &outcome.usage);
    summary.transcription_cost_usd = Some(cost_usd);
    summary.transcription_model = Some(outcome.model_used);
    summary.transcription_provider = Some(outcome.provider);
    summary.transcription_usage = Some(outcome.usage);
    sessions.upsert(summary.clone())
}

fn fail_transcription(
    sessions: &SessionStore,
    mut summary: SessionSummary,
    error: AppError,
) -> AppResult<SessionSummary> {
    summary.transcription_status = TranscriptionStatus::Failed;
    summary.transcription_error = Some(error.to_string());
    sessions.upsert(summary)?;
    Err(error)
}

fn fail_transcription_preflight(
    sessions: &SessionStore,
    mut summary: SessionSummary,
    provider: TranscriptionProvider,
    error: AppError,
) -> AppResult<SessionSummary> {
    clear_transcription_result(&mut summary);
    summary.transcription_status = TranscriptionStatus::Failed;
    summary.transcription_error = Some(error.to_string());
    summary.transcription_provider = Some(provider);
    sessions.upsert(summary)?;
    Err(error)
}

fn clear_transcription_result(summary: &mut SessionSummary) {
    summary.transcription_prompt_tokens = None;
    summary.transcription_output_tokens = None;
    summary.transcription_total_tokens = None;
    summary.transcription_cost_usd = None;
    summary.transcription_model = None;
    summary.transcription_usage = None;
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

fn transcript_path_for(
    summary: &SessionSummary,
    audio_path: &Path,
    provider: TranscriptionProvider,
) -> PathBuf {
    audio_path.with_file_name(format!(
        "{}_{}.txt",
        audio_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&summary.id),
        provider
    ))
}

fn ensure_audio_storage(
    summary: &mut SessionSummary,
    desired_audio_format: AudioFormat,
) -> AppResult<()> {
    let Some(audio_path) = summary.audio_path.clone() else {
        return Ok(());
    };
    let path = PathBuf::from(audio_path);
    if !path.exists() {
        return Err(AppError::Invalid(
            "Audio file is no longer on disk. Clear the session and re-record.".into(),
        ));
    }
    let current_format = AudioFormat::from_path(&path).unwrap_or(summary.audio_format);
    if current_format == desired_audio_format {
        summary.audio_format = current_format;
        return Ok(());
    }
    let next_path = match (current_format, desired_audio_format) {
        (AudioFormat::Wav, AudioFormat::Flac) => transcode_wav_to_flac(&path)?,
        (AudioFormat::Flac, AudioFormat::Wav) => transcode_flac_to_wav(&path)?,
        _ => path,
    };
    summary.audio_path = Some(next_path.to_string_lossy().to_string());
    summary.audio_format = desired_audio_format;
    Ok(())
}

fn remove_replaced_transcript(summary: &SessionSummary, next_path: &Path) {
    let Some(existing) = summary.transcript_path.as_deref() else {
        return;
    };
    let existing_path = Path::new(existing);
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::audio::writer::{read_flac_i32, write_wav_mono_i16};
    use crate::services::sessions::{SyncStatus, TranscriptionStatus};

    fn store_with_dir() -> (tempfile::TempDir, SessionStore, PathBuf) {
        let config = tempfile::tempdir().unwrap();
        let sessions_dir = config.path().join("sessions");
        let settings = Arc::new(SettingsStore::load_or_default(config.path()));
        settings
            .set_sessions_dir(sessions_dir.to_string_lossy().to_string())
            .unwrap();
        let store = SessionStore::new(settings);
        (config, store, sessions_dir)
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
            transcription_status: TranscriptionStatus::Pending,
            transcription_error: None,
            transcription_provider: None,
            transcription_prompt_tokens: Some(1),
            transcription_output_tokens: Some(2),
            transcription_total_tokens: Some(3),
            transcription_cost_usd: Some(0.10),
            transcription_model: Some("old-model".into()),
            transcription_usage: Some(TranscriptionUsage::Duration { seconds: 12.0 }),
            sync_status: SyncStatus::NotEnabled,
            sync_error: None,
            transcript_preview: None,
        }
    }

    #[test]
    fn preflight_failure_persists_failed_status_and_clears_result_fields() {
        let (_config, store, sessions_dir) = store_with_dir();
        let session = summary("session_20260511_100000", &sessions_dir);
        store.upsert(session.clone()).unwrap();

        let err = fail_transcription_preflight(
            &store,
            session,
            TranscriptionProvider::Deepgram,
            AppError::Invalid("missing key".into()),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "invalid input: missing key");
        let saved = store.get("session_20260511_100000").unwrap();
        assert!(matches!(
            saved.transcription_status,
            TranscriptionStatus::Failed
        ));
        assert_eq!(
            saved.transcription_provider,
            Some(TranscriptionProvider::Deepgram)
        );
        assert_eq!(
            saved.transcription_error.as_deref(),
            Some("invalid input: missing key")
        );
        assert!(saved.transcription_prompt_tokens.is_none());
        assert!(saved.transcription_model.is_none());
        assert!(saved.transcription_usage.is_none());
    }

    #[test]
    fn legacy_usage_fields_only_follow_token_usage() {
        let (_config, _store, sessions_dir) = store_with_dir();
        let mut session = summary("session_20260511_100000", &sessions_dir);

        set_legacy_usage_fields(
            &mut session,
            &TranscriptionUsage::Tokens {
                prompt_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
                audio_tokens: None,
                text_tokens: None,
            },
        );
        assert_eq!(session.transcription_prompt_tokens, Some(10));
        assert_eq!(session.transcription_output_tokens, Some(20));
        assert_eq!(session.transcription_total_tokens, Some(30));

        set_legacy_usage_fields(&mut session, &TranscriptionUsage::Duration { seconds: 5.0 });
        assert!(session.transcription_prompt_tokens.is_none());
        assert!(session.transcription_output_tokens.is_none());
        assert!(session.transcription_total_tokens.is_none());
    }

    #[test]
    fn transcript_path_uses_provider_suffix() {
        let (_config, _store, sessions_dir) = store_with_dir();
        let session = summary("session_20260511_100000", &sessions_dir);
        let wav = sessions_dir.join("session_20260511_100000.wav");

        assert_eq!(
            transcript_path_for(&session, &wav, TranscriptionProvider::Openai),
            sessions_dir.join("session_20260511_100000_openai.txt")
        );
    }

    #[test]
    fn ensure_audio_storage_converts_wav_to_flac_for_archival() {
        let (_config, _store, sessions_dir) = store_with_dir();
        std::fs::create_dir_all(&sessions_dir).unwrap();
        let mut session = summary("session_20260511_100000", &sessions_dir);
        let wav = PathBuf::from(session.audio_path.as_deref().unwrap());
        let samples = (0..32).map(|n| n as i16 - 16).collect::<Vec<i16>>();
        write_wav_mono_i16(&wav, &samples).unwrap();

        ensure_audio_storage(&mut session, AudioFormat::Flac).unwrap();

        let audio = PathBuf::from(session.audio_path.as_deref().unwrap());
        assert_eq!(session.audio_format, AudioFormat::Flac);
        assert_eq!(audio.extension().and_then(|s| s.to_str()), Some("flac"));
        assert!(!wav.exists());
        assert_eq!(
            read_flac_i32(&audio).unwrap().0,
            (0..32).map(|n| n - 16).collect::<Vec<i32>>()
        );
    }

    #[test]
    fn remove_replaced_transcript_only_removes_same_session_file() {
        let (_config, _store, sessions_dir) = store_with_dir();
        std::fs::create_dir_all(&sessions_dir).unwrap();
        let old = sessions_dir.join("session_20260511_100000_gemini.txt");
        let unrelated = sessions_dir.join("session_other_gemini.txt");
        let next = sessions_dir.join("session_20260511_100000_deepgram.txt");
        std::fs::write(&old, "old").unwrap();
        std::fs::write(&unrelated, "keep").unwrap();

        let mut session = summary("session_20260511_100000", &sessions_dir);
        session.transcript_path = Some(old.to_string_lossy().to_string());
        remove_replaced_transcript(&session, &next);

        assert!(!old.exists());
        assert!(unrelated.exists());
    }
}
