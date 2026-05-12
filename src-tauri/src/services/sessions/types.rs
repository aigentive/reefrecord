use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::transcription::types::{TranscriptionProvider, TranscriptionUsage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStatus {
    NotStarted,
    Pending,
    Transcribing,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    NotEnabled,
    Queued,
    Syncing,
    Synced,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: u64,
    #[serde(default)]
    pub wav_path: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    pub mic_device_name: Option<String>,
    pub system_device_name: Option<String>,
    pub transcription_status: TranscriptionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_provider: Option<TranscriptionProvider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_prompt_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_usage: Option<TranscriptionUsage>,
    pub sync_status: SyncStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_error: Option<String>,
    /// Computed at reload time; not persisted.
    #[serde(default, skip_serializing, skip_deserializing)]
    pub transcript_preview: Option<String>,
}

impl SessionSummary {
    pub fn metadata_path(&self, sessions_dir: &Path) -> PathBuf {
        if let Some(wav) = &self.wav_path {
            let p = PathBuf::from(wav);
            return p.with_file_name(format!("{}.json", self.id));
        }
        sessions_dir.join(format!("{}.json", self.id))
    }
}
