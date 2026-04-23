import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  AudioDevice,
  PermissionKind,
  RecordingInput,
  SessionSummary,
  Settings,
  SettingsInput,
  SyncResult,
  TranscriptResult,
  GitSyncStatus,
  ProviderStatus,
} from "./types";

export function getAppStatus(): Promise<AppStatus> {
  return invoke("get_app_status");
}

export function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export function saveSettings(input: SettingsInput): Promise<Settings> {
  return invoke("save_settings", { input });
}

export function saveGeminiKey(key: string): Promise<ProviderStatus> {
  return invoke("save_gemini_key", { key });
}

export function hasGeminiKey(): Promise<boolean> {
  return invoke("has_gemini_key");
}

export function deleteGeminiKey(): Promise<void> {
  return invoke("delete_gemini_key");
}

export function validateGeminiKey(): Promise<ProviderStatus> {
  return invoke("validate_gemini_key");
}

export function selectSessionsFolder(): Promise<string | null> {
  return invoke("select_sessions_folder");
}

export function revealSessionsFolder(): Promise<void> {
  return invoke("reveal_sessions_folder");
}

export function revealPath(path: string): Promise<void> {
  return invoke("reveal_path", { path });
}

export function openPermissionsSettings(kind: PermissionKind): Promise<void> {
  return invoke("open_permissions_settings", { kind });
}

export function listAudioDevices(): Promise<AudioDevice[]> {
  return invoke("list_audio_devices");
}

export function listSessions(): Promise<SessionSummary[]> {
  return invoke("list_sessions");
}

export function startRecording(input: RecordingInput): Promise<string> {
  return invoke("start_recording", { input });
}

export function stopRecording(sessionId: string): Promise<SessionSummary> {
  return invoke("stop_recording", { sessionId });
}

export function transcribeSession(sessionId: string): Promise<TranscriptResult> {
  return invoke("transcribe_session", { sessionId });
}

export function readTranscript(sessionId: string): Promise<string | null> {
  return invoke("read_transcript", { sessionId });
}

export function syncSession(sessionId: string): Promise<SyncResult> {
  return invoke("sync_session", { sessionId });
}

export function validateGitSyncSettings(): Promise<GitSyncStatus> {
  return invoke("validate_git_sync_settings");
}
