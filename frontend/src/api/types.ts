export type ReadinessState =
  | "ready"
  | "missing"
  | "denied"
  | "warning"
  | "checking"
  | "optional";

export type ProviderStatus = {
  state: ReadinessState;
  detail?: string;
  lastValidatedAt?: string;
};

export type AppStatus = {
  mic: ProviderStatus;
  systemAudio: ProviderStatus;
  gemini: ProviderStatus;
  folder: ProviderStatus;
  github: ProviderStatus;
  git: ProviderStatus;
  gitLfs: ProviderStatus;
  canRecord: boolean;
  blockingReason?: string;
};

export type Settings = {
  sessionsDir: string | null;
  captureSystemAudio: boolean;
  micDeviceSelector: string | null;
  systemAudioDeviceSelector: string | null;
  geminiModel: string;
  geminiFallbackModel: string;
  chunkMinutes: number;
  languageHint: string;
  githubSyncEnabled: boolean;
  githubRepoUrl: string;
  githubTargetFolder: string;
  gitLfsEnabled: boolean;
};

export type SettingsInput = Partial<Settings>;

export type AudioDevice = {
  id: string;
  name: string;
  inputChannels: number;
  isDefault: boolean;
  isBlackhole: boolean;
};

export type PermissionKind =
  | "microphone"
  | "soundSettings"
  | "audioMidiSetup";

export type TranscriptionStatus =
  | "not_started"
  | "pending"
  | "transcribing"
  | "complete"
  | "failed";

export type SyncStatus =
  | "not_enabled"
  | "queued"
  | "syncing"
  | "synced"
  | "skipped"
  | "failed";

export type SessionSummary = {
  id: string;
  startedAt: string;
  durationSeconds: number;
  wavPath: string;
  transcriptPath: string | null;
  micDeviceName: string | null;
  systemDeviceName: string | null;
  transcriptionStatus: TranscriptionStatus;
  transcriptionError?: string;
  syncStatus: SyncStatus;
  syncError?: string;
};

export type RecordingInput = {
  captureSystemAudio: boolean;
  micDeviceSelector: string | null;
  systemAudioDeviceSelector: string | null;
};

export type TranscriptResult = {
  sessionId: string;
  transcriptPath: string;
  status: TranscriptionStatus;
};

export type SyncResult = {
  sessionId: string;
  status: SyncStatus;
  message?: string;
};

export type GitSyncStatus = {
  gitInstalled: boolean;
  gitLfsInstalled: boolean;
  repoUrlValid: boolean;
  targetFolderSafe: boolean;
  message?: string;
};
