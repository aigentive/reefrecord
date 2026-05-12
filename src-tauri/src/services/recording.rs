use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{Local, Utc};
use serde::Deserialize;

use crate::audio::capture::{start_capture, InputCapture};
use crate::audio::format::AudioFormat;
use crate::audio::writer::{mix_mono_i16, write_wav_mono_i16};
use crate::error::{AppError, AppResult};
use crate::services::sessions::{SessionStore, SessionSummary, SyncStatus, TranscriptionStatus};
use crate::settings::SettingsStore;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInput {
    pub capture_system_audio: bool,
    pub mic_device_selector: Option<String>,
    pub system_audio_device_selector: Option<String>,
}

struct ActiveRecording {
    session_id: String,
    started_at: Instant,
    started_at_utc: chrono::DateTime<Utc>,
    mic: InputCapture,
    system: Option<InputCapture>,
}

pub struct RecordingService {
    sessions: Arc<SessionStore>,
    settings: Arc<SettingsStore>,
    active: Mutex<Option<ActiveRecording>>,
}

impl RecordingService {
    pub fn new(sessions: Arc<SessionStore>, settings: Arc<SettingsStore>) -> Self {
        Self {
            sessions,
            settings,
            active: Mutex::new(None),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.active.lock().unwrap().is_some()
    }

    pub fn start(&self, input: RecordingInput) -> AppResult<String> {
        let mut slot = self.active.lock().unwrap();
        if slot.is_some() {
            return Err(AppError::msg("A recording is already in progress."));
        }
        let sessions_dir = self.sessions.sessions_dir()?;
        if !sessions_dir.exists() {
            std::fs::create_dir_all(&sessions_dir)?;
        }
        let started_at_utc = Utc::now();
        // Session id uses local wall-clock time to match user expectation and
        // the python reference's filename format.
        let session_id = format!("session_{}", Local::now().format("%Y%m%d_%H%M%S"));

        let mic =
            start_capture(input.mic_device_selector.clone(), false, "mic").map_err(
                |e| match e {
                    AppError::Audio(msg) => AppError::Audio(format!("microphone: {msg}")),
                    other => other,
                },
            )?;

        let system = if input.capture_system_audio {
            match start_capture(input.system_audio_device_selector.clone(), true, "system") {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::info!("system audio not available: {e}");
                    None
                }
            }
        } else {
            None
        };

        tracing::info!(
            session = %session_id,
            mic = %mic.device_name(),
            system = system.as_ref().map(|s| s.device_name()),
            "recording started"
        );

        *slot = Some(ActiveRecording {
            session_id: session_id.clone(),
            started_at: Instant::now(),
            started_at_utc,
            mic,
            system,
        });
        Ok(session_id)
    }

    pub fn stop(&self, session_id: &str) -> AppResult<SessionSummary> {
        let mut slot = self.active.lock().unwrap();
        let active = slot
            .take()
            .ok_or_else(|| AppError::msg("No recording is active."))?;
        if active.session_id != session_id {
            // Put it back, don't destroy an unrelated session.
            *slot = Some(active);
            return Err(AppError::Invalid("Session id mismatch.".into()));
        }
        drop(slot);
        let ActiveRecording {
            session_id,
            started_at,
            started_at_utc,
            mic,
            system,
        } = active;

        let duration_seconds = started_at.elapsed().as_secs();
        let mic_name = mic.device_name().to_string();
        let system_name = system.as_ref().map(|s| s.device_name().to_string());

        let mic_samples = mic.stop();
        let mixed = match system {
            Some(s) => mix_mono_i16(&mic_samples, &s.stop()),
            None => mic_samples,
        };

        let sessions_dir = self.sessions.sessions_dir()?;
        let wav_path: PathBuf = sessions_dir.join(format!("{session_id}.wav"));
        write_wav_mono_i16(&wav_path, &mixed)?;

        let s = self.settings.get();
        let sync_status = if s.github_sync_enabled {
            SyncStatus::Queued
        } else {
            SyncStatus::NotEnabled
        };

        let summary = SessionSummary {
            id: session_id,
            started_at: started_at_utc,
            duration_seconds,
            audio_path: Some(wav_path.to_string_lossy().to_string()),
            audio_format: AudioFormat::Wav,
            transcript_path: None,
            mic_device_name: Some(mic_name),
            system_device_name: system_name,
            transcription_status: TranscriptionStatus::Pending,
            transcription_error: None,
            transcription_provider: None,
            transcription_prompt_tokens: None,
            transcription_output_tokens: None,
            transcription_total_tokens: None,
            transcription_cost_usd: None,
            transcription_model: None,
            transcription_usage: None,
            sync_status,
            sync_error: None,
            transcript_preview: None,
        };
        self.sessions.upsert(summary.clone())?;
        tracing::info!(session = %summary.id, seconds = duration_seconds, "recording saved");
        Ok(summary)
    }
}
