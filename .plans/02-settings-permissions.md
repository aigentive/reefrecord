# Phase 2: Settings, Secrets, Permissions

## Objective

Replace manual `.env` setup with guided desktop configuration.

## Required Settings

- Gemini API key.
- Local sessions folder.
- Microphone device preference.

## Optional Settings

- System audio capture.
- BlackHole/system audio device selector.
- GitHub sync enablement.
- GitHub repo URL.
- GitHub target folder.
- Gemini model and fallback model.
- Transcript language hint.

## Secret Handling

Implement Rust-side secret commands:

- `save_gemini_key`
- `has_gemini_key`
- `delete_gemini_key`
- `validate_gemini_key`

Rules:

- Never write the Gemini key to `.env`, logs, transcript files, or non-secret settings JSON.
- Show only key presence and last validation result after save.
- Support paste, reveal/hide, and clear actions in UI.

## Non-Secret Settings Storage

Use a JSON settings file under the app config directory.

Example:

```text
~/Library/Application Support/com.aigentive.reefrecord/settings.json
```

Rust owns the read/write path so the frontend does not need filesystem permission.

## Folder Picker

Use Tauri dialog plugin to select the local sessions folder.

UX:

- "Choose Folder" opens native folder picker.
- If no folder is selected, default to this repo's `sessions/` when running in dev.
- "Reveal Folder" opens the selected folder with Tauri opener.

## Permissions UX

Readiness states:

- `ready`
- `missing`
- `denied`
- `warning`
- `checking`

Microphone:

- Show current microphone device if available.
- If opening the stream fails with a permission-like error, mark as `denied`.
- Provide "Open Microphone Settings".

System audio:

- Detect input devices with names containing `blackhole` or `black hole`.
- Missing BlackHole is a warning, not a blocker.
- Provide "Open Sound Settings" and short setup checklist.

Open settings targets:

- macOS microphone privacy settings.
- macOS sound settings.
- Audio MIDI Setup if available.

Use Tauri opener where possible and Rust OS-specific fallback commands only when needed.

## UI Flow

Setup rail chip behavior:

- Clicking `Gemini` opens inline key panel.
- Clicking `Folder` opens folder picker.
- Clicking `Mic` opens device and permission panel.
- Clicking `System Audio` opens BlackHole help and device selector.
- Clicking `GitHub` opens sync settings, but does not block recording.

## Commands

- `get_app_status`
- `get_settings`
- `save_settings`
- `save_gemini_key`
- `delete_gemini_key`
- `validate_gemini_key`
- `select_sessions_folder`
- `reveal_sessions_folder`
- `open_permissions_settings`
- `list_audio_devices`

## Tests

Rust:

- Settings round-trip.
- Missing settings file returns defaults.
- Secret save/read/delete with test backend.
- Device matching detects BlackHole names.

Frontend:

- Key panel save state.
- Folder picker success/cancel states with mocked Tauri calls.
- Permission denied panel shows open-settings action.

## Acceptance Criteria

- A user can configure required app settings without editing `.env`.
- The app clearly shows whether recording is available.
- Missing BlackHole does not prevent mic-only recording.
- Secrets are not written into settings JSON.

