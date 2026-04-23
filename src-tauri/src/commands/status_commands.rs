use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::services::devices;
use crate::services::secrets;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusDto {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validated_at: Option<String>,
}

impl ProviderStatusDto {
    pub fn ready<S: Into<String>>(detail: S) -> Self {
        Self {
            state: "ready".into(),
            detail: Some(detail.into()),
            last_validated_at: None,
        }
    }
    pub fn missing<S: Into<String>>(detail: S) -> Self {
        Self {
            state: "missing".into(),
            detail: Some(detail.into()),
            last_validated_at: None,
        }
    }
    pub fn warning<S: Into<String>>(detail: S) -> Self {
        Self {
            state: "warning".into(),
            detail: Some(detail.into()),
            last_validated_at: None,
        }
    }
    pub fn denied<S: Into<String>>(detail: S) -> Self {
        Self {
            state: "denied".into(),
            detail: Some(detail.into()),
            last_validated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub mic: ProviderStatusDto,
    pub system_audio: ProviderStatusDto,
    pub gemini: ProviderStatusDto,
    pub folder: ProviderStatusDto,
    pub github: ProviderStatusDto,
    pub git: ProviderStatusDto,
    pub git_lfs: ProviderStatusDto,
    pub can_record: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_reason: Option<String>,
}

#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> AppResult<AppStatusDto> {
    let settings = state.settings.get();
    let devices = devices::list_input_devices().unwrap_or_default();

    // Mic status
    let mic = if devices.is_empty() {
        ProviderStatusDto::missing("No input devices available.")
    } else if let Some(sel) = settings.mic_device_selector.as_deref() {
        match devices::resolve_device_selector(&devices, Some(sel)) {
            Some(d) => ProviderStatusDto::ready(d.name.clone()),
            None => ProviderStatusDto::warning(format!("Selector {:?} not matched; will use default.", sel)),
        }
    } else {
        match devices.iter().find(|d| d.is_default && !d.is_blackhole) {
            Some(d) => ProviderStatusDto::ready(d.name.clone()),
            None => match devices.iter().find(|d| !d.is_blackhole && d.input_channels > 0) {
                Some(d) => ProviderStatusDto::ready(d.name.clone()),
                None => ProviderStatusDto::denied("No microphone-capable device."),
            },
        }
    };

    // System audio / BlackHole
    let system_audio = if !settings.capture_system_audio {
        ProviderStatusDto {
            state: "optional".into(),
            detail: Some("System audio capture is disabled.".into()),
            last_validated_at: None,
        }
    } else {
        match devices::find_blackhole_device(&devices) {
            Some(d) => ProviderStatusDto::ready(d.name.clone()),
            None => ProviderStatusDto::warning("BlackHole not detected — recording will be mic-only."),
        }
    };

    // Gemini key presence
    let gemini_state = if secrets::has_gemini_key() {
        let cached = state.gemini_last_validation.read().await.clone();
        match cached {
            Some(v) => v,
            None => ProviderStatusDto::ready("Key saved. Press Validate to test."),
        }
    } else {
        ProviderStatusDto::missing("Paste a Gemini API key to enable transcription.")
    };

    // Sessions folder
    let folder = match settings.sessions_dir.as_deref() {
        Some(dir) => {
            let exists = std::path::Path::new(dir).exists();
            if exists {
                ProviderStatusDto::ready(dir.to_string())
            } else {
                ProviderStatusDto::warning(format!("{dir} does not exist yet."))
            }
        }
        None => ProviderStatusDto::missing("Pick a folder to save sessions."),
    };

    // GitHub
    let github = if !settings.github_sync_enabled {
        ProviderStatusDto {
            state: "optional".into(),
            detail: Some("Sync is off.".into()),
            last_validated_at: None,
        }
    } else {
        let v = crate::git_sync::validate(&settings);
        if v.git_installed && (v.git_lfs_installed || !settings.git_lfs_enabled) && v.repo_url_valid && v.target_folder_safe {
            ProviderStatusDto::ready(format!("Will push to {}", settings.github_repo_url))
        } else {
            ProviderStatusDto::warning(v.message.unwrap_or_else(|| "Sync settings incomplete.".into()))
        }
    };

    let git_status = if which::which("git").is_ok() {
        ProviderStatusDto::ready("git found")
    } else {
        ProviderStatusDto::missing("git not installed")
    };
    let git_lfs_status = if which::which("git-lfs").is_ok() {
        ProviderStatusDto::ready("git-lfs found")
    } else {
        ProviderStatusDto::missing("git-lfs not installed")
    };

    let mut blocking: Vec<String> = Vec::new();
    if mic.state != "ready" {
        blocking.push("microphone".into());
    }
    if gemini_state.state != "ready" && gemini_state.state != "warning" {
        blocking.push("Gemini key".into());
    }
    if folder.state != "ready" {
        blocking.push("sessions folder".into());
    }
    let can_record = blocking.is_empty();
    let blocking_reason = if can_record {
        None
    } else {
        Some(format!("Missing: {}", blocking.join(", ")))
    };

    Ok(AppStatusDto {
        mic,
        system_audio,
        gemini: gemini_state,
        folder,
        github,
        git: git_status,
        git_lfs: git_lfs_status,
        can_record,
        blocking_reason,
    })
}
