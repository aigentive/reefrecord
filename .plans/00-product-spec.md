# Phase 0: Product Spec

## Goal

Create a mini desktop app that makes this project easy to configure and run without editing `.env` or using the terminal.

The first screen is the working recorder experience, not a marketing landing page. Missing setup items appear as actionable readiness chips and inline setup panels.

## Target User Workflow

1. Launch app.
2. See readiness status for microphone, system audio, Gemini key, save folder, and GitHub sync.
3. Fix missing items from the same screen.
4. Click Record.
5. Stop recording.
6. App saves WAV locally.
7. App transcribes with Gemini.
8. App shows transcript and file locations.
9. If GitHub sync is enabled, app commits and pushes the session.

## MVP Scope

- Tauri v2 desktop shell with React + TypeScript frontend.
- macOS-first audio setup.
- Microphone recording.
- Optional system audio recording via BlackHole input device.
- Local session folder picker.
- Gemini API key setup and validation.
- Post-recording Gemini transcription.
- GitHub repository sync via local `git` and `git-lfs`.
- Settings screen and first-run progressive setup.
- Clean orange minimal UI with status-driven flows.

## Non-Goals

- Real-time transcription. Gemini docs currently state the Gemini API is not for real-time transcription; MVP transcribes after stop.
- Multi-user accounts.
- Cloud storage other than GitHub.
- Full GitHub OAuth/device flow.
- Windows/Linux system-audio support.
- Large RalphX-style task orchestration.
- Complex audio editing.

## App Name

Working name: `Reef Recorder`.

Bundle identifier: `com.aigentive.reefrecord`.

## Architecture

Use Tauri as the boundary:

- React frontend owns UI state, setup flow, validation display, and session list.
- Rust backend owns filesystem writes, audio capture, Gemini requests, and Git operations.
- Frontend never receives raw API keys except while the user is typing before save.
- Rust commands expose stable app operations through `invoke`.

Suggested structure:

```text
frontend/
  src/
    api/
    components/
    features/
    styles/
src-tauri/
  src/
    commands/
    services/
    audio/
    gemini/
    git_sync/
    settings/
```

This mirrors the useful shape of `../ralphx` without adopting its larger domain layers.

## Rust Services

- `settings_service`: load/save non-secret settings.
- `secret_service`: save/read/delete Gemini key and optional Git token.
- `permission_service`: detect microphone access state where possible and open system settings.
- `device_service`: list input devices and detect BlackHole-like devices.
- `recording_service`: start/stop capture, mix streams, write WAV.
- `gemini_service`: upload/transcribe audio, retry transient failures, write transcript.
- `git_sync_service`: validate git tools, clone/pull target repo, copy session files, commit, push.

## Tauri Commands

Minimal command surface:

- `get_app_status() -> AppStatus`
- `get_settings() -> Settings`
- `save_settings(input: SettingsInput) -> Settings`
- `save_gemini_key(key: String) -> SecretStatus`
- `validate_gemini_key() -> ProviderStatus`
- `select_sessions_folder() -> Option<String>`
- `open_permissions_settings(kind: PermissionKind) -> ()`
- `list_audio_devices() -> Vec<AudioDevice>`
- `start_recording(input: RecordingInput) -> SessionId`
- `stop_recording(session_id: SessionId) -> SessionSummary`
- `transcribe_session(session_id: SessionId) -> TranscriptResult`
- `sync_session(session_id: SessionId) -> SyncResult`

## Settings Model

Non-secret settings:

```json
{
  "sessionsDir": "/Users/admin/Documents/Github/deepgram/sessions",
  "captureSystemAudio": true,
  "micDeviceSelector": null,
  "systemAudioDeviceSelector": "blackhole",
  "geminiModel": "gemini-3-flash-preview",
  "geminiFallbackModel": "gemini-2.5-flash",
  "chunkMinutes": 15,
  "languageHint": "Romanian with possible English",
  "githubSyncEnabled": false,
  "githubRepoUrl": "",
  "githubTargetFolder": "sessions",
  "gitLfsEnabled": true
}
```

Secrets:

- `gemini_api_key`
- optional future `github_token`

## Secret Storage Decision

MVP should prefer OS keychain storage from Rust for the lowest-friction UX. Tauri Stronghold remains the official Tauri plugin fallback if a plugin-only solution is required, but it adds a vault password flow and is not needed for the first polished app.

## UI Experience

Main screen regions:

- Top status bar: app status, settings button, sync indicator.
- Setup rail: compact readiness chips for Mic, System Audio, Gemini, Folder, GitHub.
- Recorder panel: large circular record/stop control, elapsed timer, selected devices.
- Session list: latest sessions with transcript state and sync state.
- Transcript drawer: view transcript, reveal files, retry transcription, retry sync.

Progressive setup behavior:

- If all required items are ready, setup rail collapses to chips.
- If Gemini key is missing, open inline key form.
- If microphone access fails, show "Open Settings" button.
- If BlackHole is missing, system audio chip is warning, not blocking.
- If save folder is missing, open native folder picker.
- If GitHub sync is off, show it as optional.

## Visual Direction

Clean, minimal, orange-accented desktop utility:

- Background: `#FAFAF8`
- Surface: `#FFFFFF`
- Text: `#171717`
- Muted text: `#6F6A63`
- Border: `#E7E0D6`
- Accent: `#F97316`
- Accent dark: `#C2410C`
- Success: `#15803D`
- Warning: `#D97706`
- Error: `#DC2626`

Rules:

- Use orange as the action/accent color, not as the whole palette.
- Keep cards to individual functional panels, no nested cards.
- Use icons from `lucide-react` for settings, folder, key, mic, GitHub, refresh, alert, check.
- Use compact headings inside panels.
- Make button labels short and stable.
- Never hide failed states behind toast-only feedback.

## Acceptance Criteria

- A new user can configure required settings from the app without editing `.env`.
- A user can record a mic-only session and get a WAV plus transcript.
- If BlackHole is available, a user can include system audio.
- A user can choose where sessions are saved.
- A user can enable GitHub sync and see clear success/failure state.
- The app handles missing microphone permission, missing Gemini key, missing save folder, missing Git, missing Git LFS, and Git push failure.

