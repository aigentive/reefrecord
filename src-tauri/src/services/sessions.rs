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
    pub transcription_prompt_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_model: Option<String>,
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

const TRANSCRIPT_PREVIEW_BYTES: usize = 320;
const TRANSCRIPT_PREVIEW_CHARS: usize = 180;

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
            let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !fname.starts_with("session_") || !fname.ends_with(".json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<SessionSummary>(&text) {
                    Ok(mut s) => {
                        // Drop wavPath if the file has actually been removed so
                        // the ui can render accordingly.
                        if let Some(wp) = &s.wav_path {
                            if !std::path::Path::new(wp).exists() {
                                s.wav_path = None;
                            }
                        }
                        if let Some(tp) = &s.transcript_path {
                            if let Some(preview) = read_transcript_preview(tp) {
                                s.transcript_preview = Some(preview);
                            } else if !std::path::Path::new(tp).exists() {
                                s.transcript_path = None;
                            }
                        }
                        sessions.push(s);
                    }
                    Err(e) => tracing::warn!("bad session json {path:?}: {e}"),
                },
                Err(e) => tracing::warn!("cannot read {path:?}: {e}"),
            }
        }
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        *self.cache.write().unwrap() = sessions;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let sessions_dir = self.sessions_dir()?;
        let summary = self
            .cache
            .read()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned();
        if let Some(s) = summary {
            if let Some(wav) = &s.wav_path {
                let _ = std::fs::remove_file(wav);
            }
            if let Some(t) = &s.transcript_path {
                let _ = std::fs::remove_file(t);
            }
            let meta = s.metadata_path(&sessions_dir);
            let _ = std::fs::remove_file(&meta);
        } else {
            // Fallback: remove the metadata file by id if we didn't find it in
            // the cache.
            let _ = std::fs::remove_file(sessions_dir.join(format!("{id}.json")));
        }
        let mut cache = self.cache.write().unwrap();
        cache.retain(|s| s.id != id);
        Ok(())
    }

    pub fn clear_wav(&self, id: &str) -> AppResult<SessionSummary> {
        let mut summary = self
            .get(id)
            .ok_or_else(|| AppError::NotFound(format!("session {id} not found")))?;
        if let Some(wav) = &summary.wav_path {
            if std::path::Path::new(wav).exists() {
                std::fs::remove_file(wav)?;
            }
        }
        summary.wav_path = None;
        self.upsert(summary.clone())?;
        Ok(summary)
    }

    pub fn delete_all(&self) -> AppResult<usize> {
        let ids: Vec<String> = self
            .cache
            .read()
            .unwrap()
            .iter()
            .map(|s| s.id.clone())
            .collect();
        let count = ids.len();
        for id in ids {
            self.delete(&id)?;
        }
        Ok(count)
    }

    pub fn clear_all_wavs(&self) -> AppResult<usize> {
        let ids: Vec<String> = self
            .cache
            .read()
            .unwrap()
            .iter()
            .filter(|s| s.wav_path.is_some())
            .map(|s| s.id.clone())
            .collect();
        let count = ids.len();
        for id in ids {
            self.clear_wav(&id)?;
        }
        Ok(count)
    }

    pub fn list(&self) -> Vec<SessionSummary> {
        self.cache.read().unwrap().clone()
    }

    pub fn get(&self, id: &str) -> Option<SessionSummary> {
        self.cache
            .read()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned()
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
        let sessions_dir = self.sessions_dir()?;
        let parent = match &summary.wav_path {
            Some(wav) => Path::new(wav)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or(sessions_dir.clone()),
            None => sessions_dir.clone(),
        };
        if !parent.exists() {
            std::fs::create_dir_all(&parent)?;
        }
        let path = summary.metadata_path(&sessions_dir);
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

fn read_transcript_preview(path: &str) -> Option<String> {
    let p = Path::new(path);
    if !p.exists() {
        return None;
    }
    // Read only the first N bytes so a very long transcript doesn't slow us
    // down on every reload.
    use std::io::Read;
    let mut f = std::fs::File::open(p).ok()?;
    let mut buf = vec![0u8; TRANSCRIPT_PREVIEW_BYTES];
    let n = f.read(&mut buf).ok()?;
    buf.truncate(n);
    let text = String::from_utf8_lossy(&buf);
    // Strip common transcript prefixes like "[MM:SS] [Speaker N]:" so the
    // preview reads as the actual first words.
    let stripped = text
        .trim_start()
        .lines()
        .map(strip_leading_markers)
        .collect::<Vec<_>>()
        .join(" ");
    let compacted = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    if compacted.is_empty() {
        return None;
    }
    let truncated: String = compacted.chars().take(TRANSCRIPT_PREVIEW_CHARS).collect();
    if compacted.chars().count() > TRANSCRIPT_PREVIEW_CHARS {
        Some(format!("{truncated}…"))
    } else {
        Some(truncated)
    }
}

fn strip_leading_markers(line: &str) -> String {
    let mut s = line.trim_start().to_string();
    for _ in 0..4 {
        if let Some(rest) = s.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                s = rest[end + 1..].trim_start().to_string();
                continue;
            }
        }
        break;
    }
    // Drop a trailing ":" after any speaker label prefix like "Speaker 1:".
    if let Some(pos) = s.find(':') {
        if pos <= 20 {
            s = s[pos + 1..].trim_start().to_string();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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
            wav_path: Some(
                sessions_dir
                    .join(format!("{id}.wav"))
                    .to_string_lossy()
                    .to_string(),
            ),
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
            transcription_prompt_tokens: Some(10),
            transcription_output_tokens: Some(5),
            transcription_total_tokens: Some(15),
            transcription_cost_usd: Some(0.001),
            transcription_model: Some("gemini-test".into()),
            sync_status: SyncStatus::NotEnabled,
            sync_error: None,
            transcript_preview: None,
        }
    }

    #[test]
    fn metadata_path_uses_wav_parent_or_sessions_dir() {
        let sessions_dir = PathBuf::from("/tmp/sessions");
        let with_wav = SessionSummary {
            wav_path: Some("/tmp/audio/session_1.wav".into()),
            ..summary("session_1", "2026-05-11T10:00:00Z", &sessions_dir)
        };
        let without_wav = SessionSummary {
            wav_path: None,
            ..summary("session_2", "2026-05-11T10:00:00Z", &sessions_dir)
        };

        assert_eq!(
            with_wav.metadata_path(&sessions_dir),
            PathBuf::from("/tmp/audio/session_1.json")
        );
        assert_eq!(
            without_wav.metadata_path(&sessions_dir),
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
        std::fs::write(older.wav_path.as_deref().unwrap(), b"wav").unwrap();
        std::fs::write(newer.wav_path.as_deref().unwrap(), b"wav").unwrap();
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
        std::fs::write(present.wav_path.as_deref().unwrap(), b"wav").unwrap();
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
        assert!(missing.wav_path.is_none());
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
        let wav = PathBuf::from(session.wav_path.as_deref().unwrap());
        let transcript = PathBuf::from(session.transcript_path.as_deref().unwrap());
        std::fs::write(&wav, b"wav").unwrap();
        std::fs::write(&transcript, "transcript").unwrap();
        store.upsert(session.clone()).unwrap();

        let cleared = store.clear_wav(&session.id).unwrap();
        assert!(cleared.wav_path.is_none());
        assert!(!wav.exists());
        assert_eq!(store.clear_all_wavs().unwrap(), 0);

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
}
