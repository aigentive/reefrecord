use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub sessions_dir: Option<String>,
    pub capture_system_audio: bool,
    pub mic_device_selector: Option<String>,
    pub system_audio_device_selector: Option<String>,
    pub gemini_model: String,
    pub gemini_fallback_model: String,
    pub chunk_minutes: u32,
    pub language_hint: String,
    pub include_speaker_labels: bool,
    pub include_timestamps: bool,
    pub gemini_input_cost_per_million_usd: f64,
    pub gemini_output_cost_per_million_usd: f64,
    pub github_sync_enabled: bool,
    pub github_repo_url: String,
    pub github_target_folder: String,
    pub git_lfs_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sessions_dir: None,
            capture_system_audio: true,
            mic_device_selector: None,
            system_audio_device_selector: Some("blackhole".to_string()),
            gemini_model: "gemini-3-flash-preview".to_string(),
            gemini_fallback_model: "gemini-2.5-flash".to_string(),
            chunk_minutes: 15,
            language_hint: "Romanian with possible English".to_string(),
            include_speaker_labels: true,
            include_timestamps: true,
            // Defaults match Gemini 3 Flash Preview audio-input + text-output
            // public pricing. Override in Settings if you switch models.
            gemini_input_cost_per_million_usd: 1.00,
            gemini_output_cost_per_million_usd: 3.00,
            github_sync_enabled: false,
            github_repo_url: String::new(),
            github_target_folder: "sessions".to_string(),
            git_lfs_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub sessions_dir: Option<Option<String>>,
    pub capture_system_audio: Option<bool>,
    pub mic_device_selector: Option<Option<String>>,
    pub system_audio_device_selector: Option<Option<String>>,
    pub gemini_model: Option<String>,
    pub gemini_fallback_model: Option<String>,
    pub chunk_minutes: Option<u32>,
    pub language_hint: Option<String>,
    pub include_speaker_labels: Option<bool>,
    pub include_timestamps: Option<bool>,
    pub gemini_input_cost_per_million_usd: Option<f64>,
    pub gemini_output_cost_per_million_usd: Option<f64>,
    pub github_sync_enabled: Option<bool>,
    pub github_repo_url: Option<String>,
    pub github_target_folder: Option<String>,
    pub git_lfs_enabled: Option<bool>,
}

pub struct SettingsStore {
    path: PathBuf,
    inner: RwLock<Settings>,
}

impl SettingsStore {
    pub fn load_or_default(config_dir: &Path) -> Self {
        let path = config_dir.join("settings.json");
        let inner = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("invalid settings.json, using defaults: {e}");
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        if !config_dir.exists() {
            let _ = std::fs::create_dir_all(config_dir);
        }
        Self {
            path,
            inner: RwLock::new(inner),
        }
    }

    pub fn get(&self) -> Settings {
        match self.inner.read() {
            Ok(g) => g.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn update(&self, input: SettingsInput) -> AppResult<Settings> {
        let mut current = match self.inner.write() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(v) = input.sessions_dir {
            current.sessions_dir = v;
        }
        if let Some(v) = input.capture_system_audio {
            current.capture_system_audio = v;
        }
        if let Some(v) = input.mic_device_selector {
            current.mic_device_selector = v;
        }
        if let Some(v) = input.system_audio_device_selector {
            current.system_audio_device_selector = v;
        }
        if let Some(v) = input.gemini_model {
            current.gemini_model = v;
        }
        if let Some(v) = input.gemini_fallback_model {
            current.gemini_fallback_model = v;
        }
        if let Some(v) = input.chunk_minutes {
            current.chunk_minutes = v.clamp(1, 60);
        }
        if let Some(v) = input.language_hint {
            current.language_hint = v;
        }
        if let Some(v) = input.include_speaker_labels {
            current.include_speaker_labels = v;
        }
        if let Some(v) = input.include_timestamps {
            current.include_timestamps = v;
        }
        if let Some(v) = input.gemini_input_cost_per_million_usd {
            current.gemini_input_cost_per_million_usd = v.max(0.0);
        }
        if let Some(v) = input.gemini_output_cost_per_million_usd {
            current.gemini_output_cost_per_million_usd = v.max(0.0);
        }
        if let Some(v) = input.github_sync_enabled {
            current.github_sync_enabled = v;
        }
        if let Some(v) = input.github_repo_url {
            current.github_repo_url = v.trim().to_string();
        }
        if let Some(v) = input.github_target_folder {
            let trimmed = v.trim().trim_matches('/');
            current.github_target_folder = if trimmed.is_empty() {
                "sessions".to_string()
            } else {
                trimmed.to_string()
            };
        }
        if let Some(v) = input.git_lfs_enabled {
            current.git_lfs_enabled = v;
        }
        let next = current.clone();
        drop(current);
        self.write_to_disk(&next)?;
        Ok(next)
    }

    pub fn set_sessions_dir(&self, dir: String) -> AppResult<Settings> {
        let mut input = SettingsInput::default();
        input.sessions_dir = Some(Some(dir));
        self.update(input)
    }

    fn write_to_disk(&self, settings: &Settings) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let text = serde_json::to_string_pretty(settings)?;
        let tmp = self.path.with_extension("json.tmp");
        let mut f = std::fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut f, text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

pub fn resolve_config_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::msg(format!("no app config dir: {e}")))?;
    Ok(dir)
}
