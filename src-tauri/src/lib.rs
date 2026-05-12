#![allow(clippy::too_many_arguments)]

pub mod audio;
pub mod commands;
pub mod error;
pub mod gemini;
pub mod git_sync;
pub mod services;
pub mod settings;
pub mod transcription;

use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::Manager;
use tokio::sync::RwLock;

use crate::services::recording::RecordingService;
use crate::services::sessions::SessionStore;
use crate::settings::SettingsStore;
use crate::transcription::types::TranscriptionProvider;

pub struct AppState {
    pub settings: Arc<SettingsStore>,
    pub sessions: Arc<SessionStore>,
    pub recording: Arc<RecordingService>,
    pub transcription_last_validation: Arc<
        RwLock<
            BTreeMap<TranscriptionProvider, crate::commands::status_commands::ProviderStatusDto>,
        >,
    >,
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("info,reef_recorder_lib=debug")
            }),
        )
        .compact()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = settings::resolve_config_dir(app.handle())
                .expect("failed to resolve config directory");
            crate::services::secrets::set_config_dir(config_dir.clone());
            let settings_store = Arc::new(SettingsStore::load_or_default(&config_dir));
            let session_store = Arc::new(SessionStore::new(settings_store.clone()));
            let recording_service = Arc::new(RecordingService::new(
                session_store.clone(),
                settings_store.clone(),
            ));
            app.manage(AppState {
                settings: settings_store,
                sessions: session_store,
                recording: recording_service,
                transcription_last_validation: Arc::new(RwLock::new(BTreeMap::new())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status_commands::get_app_status,
            commands::settings_commands::get_settings,
            commands::settings_commands::save_settings,
            commands::settings_commands::save_transcription_key,
            commands::settings_commands::has_transcription_key,
            commands::settings_commands::delete_transcription_key,
            commands::settings_commands::validate_transcription_key,
            commands::settings_commands::select_sessions_folder,
            commands::settings_commands::reveal_sessions_folder,
            commands::settings_commands::reveal_path,
            commands::settings_commands::open_permissions_settings,
            commands::settings_commands::list_audio_devices,
            commands::session_commands::list_sessions,
            commands::session_commands::start_recording,
            commands::session_commands::stop_recording,
            commands::session_commands::transcribe_session,
            commands::session_commands::read_transcript,
            commands::session_commands::sync_session,
            commands::session_commands::validate_git_sync_settings,
            commands::session_commands::delete_session,
            commands::session_commands::clear_session_audio,
            commands::session_commands::delete_all_sessions,
            commands::session_commands::clear_all_audio,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
