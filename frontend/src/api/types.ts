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

export type TranscriptionProvider = "gemini" | "openai" | "deepgram";

export type AppStatus = {
  mic: ProviderStatus;
  systemAudio: ProviderStatus;
  transcription: ProviderStatus;
  providers: Record<TranscriptionProvider, ProviderStatus>;
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
  transcriptionProvider: TranscriptionProvider;
  geminiModel: string;
  geminiFallbackModel: string;
  openaiModel: string;
  openaiFallbackModel: string;
  deepgramModel: string;
  deepgramSmartFormat: boolean;
  deepgramDiarize: boolean;
  deepgramUtterances: boolean;
  chunkMinutes: number;
  languageHint: string;
  includeSpeakerLabels: boolean;
  includeTimestamps: boolean;
  geminiInputCostPerMillionUsd: number;
  geminiOutputCostPerMillionUsd: number;
  openaiCostPerMinuteUsd: number;
  openaiInputCostPerMillionUsd: number;
  openaiOutputCostPerMillionUsd: number;
  deepgramCostPerHourUsd: number;
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

export type TranscriptionUsage =
  | {
      kind: "tokens";
      promptTokens: number;
      outputTokens: number;
      totalTokens: number;
      audioTokens?: number | null;
      textTokens?: number | null;
    }
  | {
      kind: "duration";
      seconds: number;
    }
  | {
      kind: "deepgram";
      requestId?: string | null;
      durationSeconds?: number | null;
      confidence?: number | null;
    }
  | {
      kind: "unknown";
    };

export type SessionSummary = {
  id: string;
  startedAt: string;
  durationSeconds: number;
  wavPath: string | null;
  transcriptPath: string | null;
  transcriptPreview?: string;
  micDeviceName: string | null;
  systemDeviceName: string | null;
  transcriptionStatus: TranscriptionStatus;
  transcriptionError?: string;
  transcriptionProvider?: TranscriptionProvider | null;
  transcriptionPromptTokens?: number;
  transcriptionOutputTokens?: number;
  transcriptionTotalTokens?: number;
  transcriptionCostUsd?: number;
  transcriptionModel?: string;
  transcriptionUsage?: TranscriptionUsage;
  syncStatus: SyncStatus;
  syncError?: string;
};

export type RecordingInput = {
  captureSystemAudio: boolean;
  micDeviceSelector: string | null;
  systemAudioDeviceSelector: string | null;
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
