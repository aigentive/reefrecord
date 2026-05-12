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
  GitSyncStatus,
  ProviderStatus,
  TranscriptionProvider,
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

export function saveTranscriptionKey(
  provider: TranscriptionProvider,
  key: string
): Promise<ProviderStatus> {
  return invoke("save_transcription_key", { provider, key });
}

export function hasTranscriptionKey(
  provider: TranscriptionProvider
): Promise<boolean> {
  return invoke("has_transcription_key", { provider });
}

export function deleteTranscriptionKey(
  provider: TranscriptionProvider
): Promise<void> {
  return invoke("delete_transcription_key", { provider });
}

export function validateTranscriptionKey(
  provider: TranscriptionProvider,
  key?: string
): Promise<ProviderStatus> {
  return invoke("validate_transcription_key", key ? { provider, key } : { provider });
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

export function transcribeSession(
  sessionId: string,
  provider?: TranscriptionProvider
): Promise<SessionSummary> {
  return invoke("transcribe_session", provider ? { sessionId, provider } : { sessionId });
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

export function deleteSession(sessionId: string): Promise<void> {
  return invoke("delete_session", { sessionId });
}

export function clearSessionAudio(sessionId: string): Promise<SessionSummary> {
  return invoke("clear_session_audio", { sessionId });
}

export function deleteAllSessions(): Promise<number> {
  return invoke("delete_all_sessions");
}

export function clearAllAudio(): Promise<number> {
  return invoke("clear_all_audio");
}
