use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::settings::SettingsStore;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStatus {
    NotStarted,
    Pending,
    Transcribing,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    pub wav_path: String,
    pub transcript_path: Option<String>,
    pub mic_device_name: Option<String>,
    pub system_device_name: Option<String>,
    pub transcription_status: TranscriptionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription_error: Option<String>,
    pub sync_status: SyncStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_error: Option<String>,
}

impl SessionSummary {
    pub fn metadata_path(&self) -> PathBuf {
        let p = PathBuf::from(&self.wav_path);
        p.with_file_name(format!("{}.json", self.id))
    }
}

pub struct SessionStore {
    settings: Arc<SettingsStore>,
    cache: RwLock<Vec<SessionSummary>>,
}

impl SessionStore {
    pub fn new(settings: Arc<SettingsStore>) -> Self {
        let store = Self {
            settings,
            cache: RwLock::new(Vec::new()),
        };
        if let Err(e) = store.reload() {
            tracing::warn!("session store reload failed: {e}");
        }
        store
    }

    pub fn settings(&self) -> &SettingsStore {
        &self.settings
    }

    pub fn reload(&self) -> AppResult<()> {
        let dir = match self.settings.get().sessions_dir {
            Some(d) => PathBuf::from(d),
            None => {
                *self.cache.write().unwrap() = Vec::new();
                return Ok(());
            }
        };
        if !dir.exists() {
            *self.cache.write().unwrap() = Vec::new();
            return Ok(());
        }
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let Some(fname) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if !fname.starts_with("session_") || !fname.ends_with(".json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<SessionSummary>(&text) {
                    Ok(s) => sessions.push(s),
                    Err(e) => tracing::warn!("bad session json {path:?}: {e}"),
                },
                Err(e) => tracing::warn!("cannot read {path:?}: {e}"),
            }
        }
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        *self.cache.write().unwrap() = sessions;
        Ok(())
    }

    pub fn list(&self) -> Vec<SessionSummary> {
        self.cache.read().unwrap().clone()
    }

    pub fn get(&self, id: &str) -> Option<SessionSummary> {
        self.cache.read().unwrap().iter().find(|s| s.id == id).cloned()
    }

    pub fn upsert(&self, summary: SessionSummary) -> AppResult<()> {
        self.write_metadata(&summary)?;
        let mut cache = self.cache.write().unwrap();
        if let Some(idx) = cache.iter().position(|s| s.id == summary.id) {
            cache[idx] = summary;
        } else {
            cache.insert(0, summary);
        }
        cache.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(())
    }

    pub fn sessions_dir(&self) -> AppResult<PathBuf> {
        let s = self.settings.get();
        let dir = s
            .sessions_dir
            .ok_or_else(|| AppError::msg("Sessions folder is not set."))?;
        let path = PathBuf::from(dir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    fn write_metadata(&self, summary: &SessionSummary) -> AppResult<()> {
        let wav_path = Path::new(&summary.wav_path);
        let parent = wav_path.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
        let path = summary.metadata_path();
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(summary)?;
        let mut f = std::fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut f, text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}
