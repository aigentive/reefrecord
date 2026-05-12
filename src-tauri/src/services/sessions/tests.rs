use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::audio::format::AudioFormat;
use crate::settings::SettingsStore;
use crate::transcription::types::TranscriptionProvider;

use super::*;

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

fn summary(id: &str, started_at: &str, sessions_dir: &Path) -> SessionSummary {
    SessionSummary {
        id: id.to_string(),
        started_at: started_at.parse().unwrap(),
        duration_seconds: 42,
        audio_path: Some(
            sessions_dir
                .join(format!("{id}.wav"))
                .to_string_lossy()
                .to_string(),
        ),
        audio_format: AudioFormat::Wav,
        transcript_path: Some(
            sessions_dir
                .join(format!("{id}_gemini.txt"))
                .to_string_lossy()
                .to_string(),
        ),
        mic_device_name: Some("Studio Mic".into()),
        system_device_name: None,
        transcription_status: TranscriptionStatus::Complete,
        transcription_error: None,
        transcription_provider: Some(TranscriptionProvider::Gemini),
        transcription_prompt_tokens: Some(10),
        transcription_output_tokens: Some(5),
        transcription_total_tokens: Some(15),
        transcription_cost_usd: Some(0.001),
        transcription_model: Some("gemini-test".into()),
        transcription_usage: None,
        sync_status: SyncStatus::NotEnabled,
        sync_error: None,
        transcript_preview: None,
    }
}

#[test]
fn metadata_path_uses_wav_parent_or_sessions_dir() {
    let sessions_dir = PathBuf::from("/tmp/sessions");
    let with_audio = SessionSummary {
        audio_path: Some("/tmp/audio/session_1.flac".into()),
        audio_format: AudioFormat::Flac,
        ..summary("session_1", "2026-05-11T10:00:00Z", &sessions_dir)
    };
    let without_audio = SessionSummary {
        audio_path: None,
        ..summary("session_2", "2026-05-11T10:00:00Z", &sessions_dir)
    };

    assert_eq!(
        with_audio.metadata_path(&sessions_dir),
        PathBuf::from("/tmp/audio/session_1.json")
    );
    assert_eq!(
        without_audio.metadata_path(&sessions_dir),
        sessions_dir.join("session_2.json")
    );
}

#[test]
fn upsert_writes_metadata_and_lists_newest_first() {
    let (_config, store, sessions_dir) = store_with_dir();
    std::fs::create_dir_all(&sessions_dir).unwrap();

    let older = summary(
        "session_20260511_100000",
        "2026-05-11T10:00:00Z",
        &sessions_dir,
    );
    let newer = summary(
        "session_20260511_110000",
        "2026-05-11T11:00:00Z",
        &sessions_dir,
    );
    std::fs::write(older.audio_path.as_deref().unwrap(), b"wav").unwrap();
    std::fs::write(newer.audio_path.as_deref().unwrap(), b"wav").unwrap();
    std::fs::write(
        older.transcript_path.as_deref().unwrap(),
        "[00:00] [Speaker 1]: Hello",
    )
    .unwrap();
    std::fs::write(
        newer.transcript_path.as_deref().unwrap(),
        "[00:00] [Speaker 2]: Later",
    )
    .unwrap();

    store.upsert(older.clone()).unwrap();
    store.upsert(newer.clone()).unwrap();

    let ids = store.list().into_iter().map(|s| s.id).collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec!["session_20260511_110000", "session_20260511_100000"]
    );
    assert!(older.metadata_path(&sessions_dir).exists());
}

#[test]
fn reload_populates_preview_and_drops_missing_paths() {
    let (_config, store, sessions_dir) = store_with_dir();
    std::fs::create_dir_all(&sessions_dir).unwrap();

    let present = summary("session_present", "2026-05-11T10:00:00Z", &sessions_dir);
    std::fs::write(present.audio_path.as_deref().unwrap(), b"wav").unwrap();
    std::fs::write(
        present.transcript_path.as_deref().unwrap(),
        "[00:00] [Speaker 1]: Hello there\n[00:03] [Speaker 2]: General Kenobi",
    )
    .unwrap();
    store.upsert(present).unwrap();

    let missing = summary("session_missing", "2026-05-11T11:00:00Z", &sessions_dir);
    store.upsert(missing).unwrap();
    store.reload().unwrap();

    let present = store.get("session_present").unwrap();
    let missing = store.get("session_missing").unwrap();
    assert_eq!(
        present.transcript_preview.as_deref(),
        Some("Hello there General Kenobi")
    );
    assert!(missing.audio_path.is_none());
    assert!(missing.transcript_path.is_none());
}

#[test]
fn clear_and_delete_session_files_update_cache_and_disk() {
    let (_config, store, sessions_dir) = store_with_dir();
    std::fs::create_dir_all(&sessions_dir).unwrap();

    let session = summary(
        "session_20260511_120000",
        "2026-05-11T12:00:00Z",
        &sessions_dir,
    );
    let audio = PathBuf::from(session.audio_path.as_deref().unwrap());
    let transcript = PathBuf::from(session.transcript_path.as_deref().unwrap());
    std::fs::write(&audio, b"wav").unwrap();
    std::fs::write(&transcript, "transcript").unwrap();
    store.upsert(session.clone()).unwrap();

    let cleared = store.clear_audio(&session.id).unwrap();
    assert!(cleared.audio_path.is_none());
    assert!(!audio.exists());
    assert_eq!(store.clear_all_audio().unwrap(), 0);

    assert_eq!(store.delete_all().unwrap(), 1);
    assert!(store.list().is_empty());
    assert!(!transcript.exists());
    assert!(!session.metadata_path(&sessions_dir).exists());
}

#[test]
fn sessions_dir_requires_setting_and_creates_directory() {
    let config = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::load_or_default(config.path()));
    let store = SessionStore::new(settings.clone());
    assert!(store.sessions_dir().is_err());

    let sessions_dir = config.path().join("created");
    settings
        .set_sessions_dir(sessions_dir.to_string_lossy().to_string())
        .unwrap();
    assert_eq!(store.sessions_dir().unwrap(), sessions_dir);
    assert!(sessions_dir.exists());
}
