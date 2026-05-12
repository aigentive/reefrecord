use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::{AppError, AppResult};
use crate::settings::SettingsStore;

use super::preview::read_transcript_preview;
use super::types::SessionSummary;

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
                        normalize_disk_paths(&mut s);
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
            if Path::new(wav).exists() {
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

fn normalize_disk_paths(summary: &mut SessionSummary) {
    if let Some(wp) = &summary.wav_path {
        if !Path::new(wp).exists() {
            summary.wav_path = None;
        }
    }
    if let Some(tp) = &summary.transcript_path {
        if let Some(preview) = read_transcript_preview(tp) {
            summary.transcript_preview = Some(preview);
        } else if !Path::new(tp).exists() {
            summary.transcript_path = None;
        }
    }
}
