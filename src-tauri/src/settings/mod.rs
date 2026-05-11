use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::transcription::types::TranscriptionProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub sessions_dir: Option<String>,
    pub capture_system_audio: bool,
    pub mic_device_selector: Option<String>,
    pub system_audio_device_selector: Option<String>,
    #[serde(default = "default_transcription_provider")]
    pub transcription_provider: TranscriptionProvider,
    pub gemini_model: String,
    pub gemini_fallback_model: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    #[serde(default)]
    pub openai_fallback_model: String,
    #[serde(default = "default_deepgram_model")]
    pub deepgram_model: String,
    #[serde(default = "default_true")]
    pub deepgram_smart_format: bool,
    #[serde(default = "default_true")]
    pub deepgram_diarize: bool,
    #[serde(default = "default_true")]
    pub deepgram_utterances: bool,
    pub chunk_minutes: u32,
    pub language_hint: String,
    pub include_speaker_labels: bool,
    pub include_timestamps: bool,
    pub gemini_input_cost_per_million_usd: f64,
    pub gemini_output_cost_per_million_usd: f64,
    #[serde(default = "default_openai_cost_per_minute_usd")]
    pub openai_cost_per_minute_usd: f64,
    #[serde(default)]
    pub openai_input_cost_per_million_usd: f64,
    #[serde(default)]
    pub openai_output_cost_per_million_usd: f64,
    #[serde(default)]
    pub deepgram_cost_per_hour_usd: f64,
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
            transcription_provider: TranscriptionProvider::Gemini,
            gemini_model: "gemini-3-flash-preview".to_string(),
            gemini_fallback_model: "gemini-2.5-flash".to_string(),
            openai_model: "whisper-1".to_string(),
            openai_fallback_model: String::new(),
            deepgram_model: "nova-3".to_string(),
            deepgram_smart_format: true,
            deepgram_diarize: true,
            deepgram_utterances: true,
            chunk_minutes: 15,
            language_hint: "Romanian with possible English".to_string(),
            include_speaker_labels: true,
            include_timestamps: true,
            // Defaults match Gemini 3 Flash Preview audio-input + text-output
            // public pricing. Override in Settings if you switch models.
            gemini_input_cost_per_million_usd: 1.00,
            gemini_output_cost_per_million_usd: 3.00,
            openai_cost_per_minute_usd: 0.006,
            openai_input_cost_per_million_usd: 0.0,
            openai_output_cost_per_million_usd: 0.0,
            deepgram_cost_per_hour_usd: 0.0,
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
    pub transcription_provider: Option<TranscriptionProvider>,
    pub gemini_model: Option<String>,
    pub gemini_fallback_model: Option<String>,
    pub openai_model: Option<String>,
    pub openai_fallback_model: Option<String>,
    pub deepgram_model: Option<String>,
    pub deepgram_smart_format: Option<bool>,
    pub deepgram_diarize: Option<bool>,
    pub deepgram_utterances: Option<bool>,
    pub chunk_minutes: Option<u32>,
    pub language_hint: Option<String>,
    pub include_speaker_labels: Option<bool>,
    pub include_timestamps: Option<bool>,
    pub gemini_input_cost_per_million_usd: Option<f64>,
    pub gemini_output_cost_per_million_usd: Option<f64>,
    pub openai_cost_per_minute_usd: Option<f64>,
    pub openai_input_cost_per_million_usd: Option<f64>,
    pub openai_output_cost_per_million_usd: Option<f64>,
    pub deepgram_cost_per_hour_usd: Option<f64>,
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
        if let Some(v) = input.transcription_provider {
            current.transcription_provider = v;
        }
        if let Some(v) = input.gemini_model {
            current.gemini_model = normalize_model(v, &current.gemini_model);
        }
        if let Some(v) = input.gemini_fallback_model {
            current.gemini_fallback_model = normalize_model(v, &current.gemini_fallback_model);
        }
        if let Some(v) = input.openai_model {
            current.openai_model = normalize_model(v, &current.openai_model);
        }
        if let Some(v) = input.openai_fallback_model {
            current.openai_fallback_model = v.trim().to_string();
        }
        if let Some(v) = input.deepgram_model {
            current.deepgram_model = normalize_model(v, &current.deepgram_model);
        }
        if let Some(v) = input.deepgram_smart_format {
            current.deepgram_smart_format = v;
        }
        if let Some(v) = input.deepgram_diarize {
            current.deepgram_diarize = v;
        }
        if let Some(v) = input.deepgram_utterances {
            current.deepgram_utterances = v;
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
        if let Some(v) = input.openai_cost_per_minute_usd {
            current.openai_cost_per_minute_usd = v.max(0.0);
        }
        if let Some(v) = input.openai_input_cost_per_million_usd {
            current.openai_input_cost_per_million_usd = v.max(0.0);
        }
        if let Some(v) = input.openai_output_cost_per_million_usd {
            current.openai_output_cost_per_million_usd = v.max(0.0);
        }
        if let Some(v) = input.deepgram_cost_per_hour_usd {
            current.deepgram_cost_per_hour_usd = v.max(0.0);
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

fn normalize_model(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn default_transcription_provider() -> TranscriptionProvider {
    TranscriptionProvider::Gemini
}

fn default_openai_model() -> String {
    "whisper-1".to_string()
}

fn default_deepgram_model() -> String {
    "nova-3".to_string()
}

fn default_true() -> bool {
    true
}

fn default_openai_cost_per_minute_usd() -> f64 {
    0.006
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_or_invalid_settings_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load_or_default(dir.path());

        assert_eq!(store.get().gemini_model, "gemini-3-flash-preview");
        assert_eq!(store.get().github_target_folder, "sessions");
        assert!(dir.path().exists());

        std::fs::write(dir.path().join("settings.json"), "{bad json").unwrap();
        let invalid = SettingsStore::load_or_default(dir.path());
        assert_eq!(
            invalid.get().language_hint,
            "Romanian with possible English"
        );
    }

    #[test]
    fn update_applies_normalization_and_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load_or_default(dir.path());

        let mut input = SettingsInput::default();
        input.sessions_dir = Some(Some("/tmp/reef sessions".into()));
        input.capture_system_audio = Some(false);
        input.mic_device_selector = Some(Some(" USB Mic ".into()));
        input.system_audio_device_selector = Some(None);
        input.gemini_model = Some("primary".into());
        input.gemini_fallback_model = Some("fallback".into());
        input.chunk_minutes = Some(999);
        input.language_hint = Some("Romanian".into());
        input.include_speaker_labels = Some(false);
        input.include_timestamps = Some(false);
        input.gemini_input_cost_per_million_usd = Some(-1.0);
        input.gemini_output_cost_per_million_usd = Some(4.5);
        input.github_sync_enabled = Some(true);
        input.github_repo_url = Some("  git@github.com:org/repo.git  ".into());
        input.github_target_folder = Some("/meetings/".into());
        input.git_lfs_enabled = Some(false);

        let saved = store.update(input).unwrap();

        assert_eq!(saved.sessions_dir.as_deref(), Some("/tmp/reef sessions"));
        assert!(!saved.capture_system_audio);
        assert_eq!(saved.mic_device_selector.as_deref(), Some(" USB Mic "));
        assert_eq!(saved.system_audio_device_selector, None);
        assert_eq!(saved.gemini_model, "primary");
        assert_eq!(saved.gemini_fallback_model, "fallback");
        assert_eq!(saved.chunk_minutes, 60);
        assert_eq!(saved.language_hint, "Romanian");
        assert!(!saved.include_speaker_labels);
        assert!(!saved.include_timestamps);
        assert_eq!(saved.gemini_input_cost_per_million_usd, 0.0);
        assert_eq!(saved.gemini_output_cost_per_million_usd, 4.5);
        assert!(saved.github_sync_enabled);
        assert_eq!(saved.github_repo_url, "git@github.com:org/repo.git");
        assert_eq!(saved.github_target_folder, "meetings");
        assert!(!saved.git_lfs_enabled);

        let raw = std::fs::read_to_string(store.path()).unwrap();
        assert!(raw.contains("\"githubRepoUrl\": \"git@github.com:org/repo.git\""));
        assert!(!raw.contains("gemini_api_key"));

        let mut clear = SettingsInput::default();
        clear.github_target_folder = Some("   ".into());
        clear.chunk_minutes = Some(0);
        let saved = store.update(clear).unwrap();
        assert_eq!(saved.github_target_folder, "sessions");
        assert_eq!(saved.chunk_minutes, 1);
    }

    #[test]
    fn set_sessions_dir_uses_settings_update_path() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load_or_default(dir.path());

        let saved = store.set_sessions_dir("/tmp/next".into()).unwrap();

        assert_eq!(saved.sessions_dir.as_deref(), Some("/tmp/next"));
        assert_eq!(store.get().sessions_dir.as_deref(), Some("/tmp/next"));
    }
}
