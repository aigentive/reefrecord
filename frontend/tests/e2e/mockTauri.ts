import type { Page } from "@playwright/test";
import type {
  AppStatus,
  AudioDevice,
  GitSyncStatus,
  ProviderStatus,
  SessionSummary,
  Settings,
  SyncResult,
} from "../../src/api/types";

export type CommandCall = {
  cmd: string;
  args: Record<string, unknown>;
};

type CommandFailure = {
  message: string;
  once: boolean;
};

export type BrowserMockState = {
  settings: Settings;
  hasGeminiKey: boolean;
  geminiValidation: ProviderStatus | null;
  micStatus: ProviderStatus;
  systemAudioStatus: ProviderStatus | null;
  gitInstalled: boolean;
  gitLfsInstalled: boolean;
  devices: AudioDevice[];
  sessions: SessionSummary[];
  transcripts: Record<string, string>;
  selectFolderResult: string | null;
  nextSessionId: string;
  nextStartedAt: string;
  commandFailures: Record<string, CommandFailure>;
  syncFailureOnce: boolean;
  syncResolveStatus: SyncResult["status"];
  syncFailureMessage: string;
  transcribeFailureOnce: boolean;
  transcribeFailureMessage: string;
  calls: CommandCall[];
  opened: string[];
};

type MockStateInput = Partial<
  Omit<
    BrowserMockState,
    | "settings"
    | "micStatus"
    | "systemAudioStatus"
    | "devices"
    | "sessions"
    | "transcripts"
    | "commandFailures"
  >
> & {
  settings?: Partial<Settings>;
  micStatus?: ProviderStatus;
  systemAudioStatus?: ProviderStatus | null;
  devices?: AudioDevice[];
  sessions?: SessionSummary[];
  transcripts?: Record<string, string>;
  commandFailures?: Record<string, CommandFailure>;
};

export function makeSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    sessionsDir: "/tmp/reef-recorder-e2e",
    captureSystemAudio: false,
    micDeviceSelector: null,
    systemAudioDeviceSelector: "blackhole",
    geminiModel: "gemini-3-flash-preview",
    geminiFallbackModel: "gemini-2.5-flash",
    chunkMinutes: 15,
    languageHint: "Romanian with possible English",
    includeSpeakerLabels: true,
    includeTimestamps: true,
    geminiInputCostPerMillionUsd: 0.3,
    geminiOutputCostPerMillionUsd: 2.5,
    githubSyncEnabled: false,
    githubRepoUrl: "",
    githubTargetFolder: "sessions",
    gitLfsEnabled: true,
    ...overrides,
  };
}

export function mockSession(
  id = "session_20260511_101500",
  overrides: Partial<SessionSummary> = {}
): SessionSummary {
  return {
    id,
    startedAt: "2026-05-11T10:15:00.000Z",
    durationSeconds: 7,
    wavPath: `/tmp/reef-recorder-e2e/${id}.wav`,
    transcriptPath: `/tmp/reef-recorder-e2e/${id}_gemini.txt`,
    transcriptPreview: "Speaker 1: We covered the app workflow.",
    micDeviceName: "Studio Mic",
    systemDeviceName: null,
    transcriptionStatus: "complete",
    transcriptionPromptTokens: 1200,
    transcriptionOutputTokens: 240,
    transcriptionTotalTokens: 1440,
    transcriptionCostUsd: 0.00096,
    transcriptionModel: "gemini-3-flash-preview",
    syncStatus: "not_enabled",
    ...overrides,
  };
}

export function makeMockState(overrides: MockStateInput = {}): BrowserMockState {
  const settings = makeSettings(overrides.settings);
  const defaultTranscript =
    "[00:00] [Speaker 1] We covered the complete Reef Recorder workflow.";
  const sessions = overrides.sessions
    ? overrides.sessions.map((session) => ({ ...session }))
    : [];

  return {
    settings,
    hasGeminiKey: true,
    geminiValidation: null,
    micStatus: { state: "ready", detail: "Studio Mic" },
    systemAudioStatus: null,
    gitInstalled: true,
    gitLfsInstalled: true,
    devices: [
      {
        id: "mic-1",
        name: "Studio Mic",
        inputChannels: 1,
        isDefault: true,
        isBlackhole: false,
      },
    ],
    sessions,
    transcripts: Object.fromEntries(
      sessions.map((session) => [session.id, defaultTranscript])
    ),
    selectFolderResult: "/tmp/reef-recorder-e2e",
    nextSessionId: "session_20260511_101500",
    nextStartedAt: "2026-05-11T10:15:00.000Z",
    commandFailures: {},
    syncFailureOnce: false,
    syncResolveStatus: "synced",
    syncFailureMessage: "Push rejected.",
    transcribeFailureOnce: false,
    transcribeFailureMessage: "Gemini rate limit.",
    calls: [],
    opened: [],
    ...overrides,
    settings,
    sessions,
    transcripts: {
      ...Object.fromEntries(
        sessions.map((session) => [session.id, defaultTranscript])
      ),
      ...overrides.transcripts,
    },
    commandFailures: { ...overrides.commandFailures },
  };
}

export async function installTauriMock(
  page: Page,
  initialState: BrowserMockState = makeMockState()
): Promise<void> {
  await page.addInitScript((state: BrowserMockState) => {
    const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

    window.__REEF_TEST_STATE__ = clone(state);

    function ready(detail: string): ProviderStatus {
      return { state: "ready", detail };
    }

    function missing(detail: string): ProviderStatus {
      return { state: "missing", detail };
    }

    function warning(detail: string): ProviderStatus {
      return { state: "warning", detail };
    }

    function optional(detail: string): ProviderStatus {
      return { state: "optional", detail };
    }

    function currentState(): BrowserMockState {
      const current = window.__REEF_TEST_STATE__;
      if (!current) throw new Error("Tauri mock state is not installed.");
      return current;
    }

    function validateGitSync(settings: Settings): GitSyncStatus {
      const state = currentState();
      const repoUrlValid =
        !settings.githubSyncEnabled || settings.githubRepoUrl.trim().length > 0;
      const target = settings.githubTargetFolder.trim();
      const targetFolderSafe =
        target.length > 0 && !target.startsWith("/") && !target.includes("..");
      let message: string | undefined;
      if (!state.gitInstalled) message = "git not installed";
      else if (settings.gitLfsEnabled && !state.gitLfsInstalled) {
        message = "git-lfs not installed";
      } else if (!repoUrlValid) message = "Repository URL is required.";
      else if (!targetFolderSafe) message = "Target folder is not safe.";
      return {
        gitInstalled: state.gitInstalled,
        gitLfsInstalled: state.gitLfsInstalled,
        repoUrlValid,
        targetFolderSafe,
        message,
      };
    }

    function appStatus(): AppStatus {
      const state = currentState();
      const settings = state.settings;
      const gemini = state.hasGeminiKey
        ? state.geminiValidation ?? ready("Key saved. Press Validate to test.")
        : missing("Paste a Gemini API key to enable transcription.");
      const folder = settings.sessionsDir
        ? ready(settings.sessionsDir)
        : missing("Pick a folder to save sessions.");
      const mic = state.micStatus;

      const blackhole = state.devices.find((device) => device.isBlackhole);
      const systemAudio =
        state.systemAudioStatus ??
        (!settings.captureSystemAudio
          ? optional("System audio capture is disabled.")
          : blackhole
          ? ready(blackhole.name)
          : warning("BlackHole not detected - recording will be mic-only."));

      const gitSync = validateGitSync(settings);
      const github = !settings.githubSyncEnabled
        ? optional("Sync is off.")
        : gitSync.gitInstalled &&
          (gitSync.gitLfsInstalled || !settings.gitLfsEnabled) &&
          gitSync.repoUrlValid &&
          gitSync.targetFolderSafe
        ? ready(`Will push to ${settings.githubRepoUrl}`)
        : warning(gitSync.message ?? "Sync settings incomplete.");

      const blocking: string[] = [];
      if (mic.state !== "ready") blocking.push("microphone");
      if (gemini.state !== "ready" && gemini.state !== "warning") {
        blocking.push("Gemini key");
      }
      if (folder.state !== "ready") blocking.push("sessions folder");

      return {
        mic,
        systemAudio,
        gemini,
        folder,
        github,
        git: state.gitInstalled ? ready("git found") : missing("git not installed"),
        gitLfs: state.gitLfsInstalled
          ? ready("git-lfs found")
          : missing("git-lfs not installed"),
        canRecord: blocking.length === 0,
        blockingReason:
          blocking.length === 0 ? undefined : `Missing: ${blocking.join(", ")}`,
      };
    }

    function maybeFail(command: string): void {
      const state = currentState();
      const failure = state.commandFailures[command];
      if (!failure) return;
      if (failure.once) delete state.commandFailures[command];
      throw failure.message;
    }

    function upsertSession(next: SessionSummary): SessionSummary {
      const state = currentState();
      const index = state.sessions.findIndex((session) => session.id === next.id);
      if (index >= 0) state.sessions[index] = next;
      else state.sessions.unshift(next);
      return next;
    }

    function findSession(sessionId: string): SessionSummary {
      const state = currentState();
      const session = state.sessions.find((candidate) => candidate.id === sessionId);
      if (!session) throw `session ${sessionId} not found`;
      return session;
    }

    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
        const state = currentState();
        state.calls.push({ cmd, args: clone(args) });
        maybeFail(cmd);

        switch (cmd) {
          case "get_app_status":
            return appStatus();
          case "get_settings":
            return clone(state.settings);
          case "save_settings": {
            const input = (args.input ?? {}) as Partial<Settings>;
            state.settings = { ...state.settings, ...input };
            return clone(state.settings);
          }
          case "save_gemini_key": {
            const key = String(args.key ?? "").trim();
            if (!key) throw "No Gemini API key provided.";
            state.hasGeminiKey = true;
            state.geminiValidation = key.includes("bad")
              ? warning("Saved. Validation: invalid Gemini key.")
              : ready("Key saved and validated.");
            return clone(state.geminiValidation);
          }
          case "has_gemini_key":
            return state.hasGeminiKey;
          case "delete_gemini_key":
            state.hasGeminiKey = false;
            state.geminiValidation = null;
            return undefined;
          case "validate_gemini_key": {
            const key = typeof args.key === "string" ? args.key.trim() : "";
            if (!key && !state.hasGeminiKey) throw "No Gemini API key saved.";
            const result = key.includes("bad")
              ? warning("Validation failed: invalid Gemini key.")
              : ready("Validation complete.");
            if (!key) state.geminiValidation = result;
            return clone(result);
          }
          case "select_sessions_folder":
            if (state.selectFolderResult === null) return null;
            state.settings.sessionsDir = state.selectFolderResult;
            return state.selectFolderResult;
          case "reveal_sessions_folder":
            if (!state.settings.sessionsDir) throw "No sessions folder set.";
            state.opened.push(state.settings.sessionsDir);
            return undefined;
          case "reveal_path":
            state.opened.push(String(args.path ?? ""));
            return undefined;
          case "open_permissions_settings":
            state.opened.push(String(args.kind ?? ""));
            return undefined;
          case "list_audio_devices":
            return clone(state.devices);
          case "list_sessions":
            return clone(state.sessions);
          case "start_recording":
            return state.nextSessionId;
          case "stop_recording": {
            const sessionId = String(args.sessionId ?? state.nextSessionId);
            const summary: SessionSummary = {
              id: sessionId,
              startedAt: state.nextStartedAt,
              durationSeconds: 7,
              wavPath: `${state.settings.sessionsDir ?? "/tmp"}/${sessionId}.wav`,
              transcriptPath: null,
              micDeviceName: state.micStatus.detail ?? "Studio Mic",
              systemDeviceName: state.settings.captureSystemAudio
                ? appStatus().systemAudio.detail ?? null
                : null,
              transcriptionStatus: "pending",
              syncStatus: state.settings.githubSyncEnabled ? "queued" : "not_enabled",
            };
            return clone(upsertSession(summary));
          }
          case "transcribe_session": {
            const sessionId = String(args.sessionId ?? "");
            const state = currentState();
            if (state.transcribeFailureOnce) {
              state.transcribeFailureOnce = false;
              const failed = {
                ...findSession(sessionId),
                transcriptionStatus: "failed" as const,
                transcriptionError: state.transcribeFailureMessage,
              };
              upsertSession(failed);
              throw state.transcribeFailureMessage;
            }
            const existing = findSession(sessionId);
            if (!existing.wavPath) {
              throw "WAV file is no longer on disk. Clear the session and re-record.";
            }
            const transcript =
              state.transcripts[sessionId] ??
              "[00:00] [Speaker 1] We covered the complete Reef Recorder workflow.";
            state.transcripts[sessionId] = transcript;
            const updated: SessionSummary = {
              ...existing,
              transcriptPath:
                existing.transcriptPath ??
                `${state.settings.sessionsDir ?? "/tmp"}/${sessionId}_gemini.txt`,
              transcriptPreview: "Speaker 1: We covered the app workflow.",
              transcriptionStatus: "complete",
              transcriptionError: undefined,
              transcriptionPromptTokens: 1200,
              transcriptionOutputTokens: 240,
              transcriptionTotalTokens: 1440,
              transcriptionCostUsd: 0.00096,
              transcriptionModel: state.settings.geminiModel,
            };
            return clone(upsertSession(updated));
          }
          case "read_transcript": {
            const sessionId = String(args.sessionId ?? "");
            return state.transcripts[sessionId] ?? null;
          }
          case "sync_session": {
            const sessionId = String(args.sessionId ?? "");
            const existing = findSession(sessionId);
            if (!state.settings.githubSyncEnabled) {
              const disabled = {
                ...existing,
                syncStatus: "not_enabled" as const,
                syncError: undefined,
              };
              upsertSession(disabled);
              return {
                sessionId,
                status: "not_enabled",
                message: "GitHub sync is disabled.",
              };
            }
            if (state.syncFailureOnce) {
              state.syncFailureOnce = false;
              const failed = {
                ...existing,
                syncStatus: "failed" as const,
                syncError: state.syncFailureMessage,
              };
              upsertSession(failed);
              throw state.syncFailureMessage;
            }
            const status = state.syncResolveStatus;
            const updated = {
              ...existing,
              syncStatus: status,
              syncError: status === "failed" ? state.syncFailureMessage : undefined,
            };
            upsertSession(updated);
            return {
              sessionId,
              status,
              message: status === "failed" ? state.syncFailureMessage : undefined,
            };
          }
          case "validate_git_sync_settings":
            return validateGitSync(state.settings);
          case "delete_session": {
            const sessionId = String(args.sessionId ?? "");
            state.sessions = state.sessions.filter((session) => session.id !== sessionId);
            return undefined;
          }
          case "clear_session_wav": {
            const sessionId = String(args.sessionId ?? "");
            const updated = { ...findSession(sessionId), wavPath: null };
            return clone(upsertSession(updated));
          }
          case "delete_all_sessions": {
            const count = state.sessions.length;
            state.sessions = [];
            return count;
          }
          case "clear_all_wavs": {
            let count = 0;
            state.sessions = state.sessions.map((session) => {
              if (!session.wavPath) return session;
              count += 1;
              return { ...session, wavPath: null };
            });
            return count;
          }
          default:
            throw `Unhandled Tauri command: ${cmd}`;
        }
      },
      transformCallback: () => 1,
      unregisterCallback: () => undefined,
      runCallback: () => undefined,
      callbacks: {},
      convertFileSrc: (filePath: string) => filePath,
    };
  }, initialState);
}

export async function setMockState(
  page: Page,
  patch: Partial<BrowserMockState>
): Promise<void> {
  await page.evaluate((next) => {
    const state = window.__REEF_TEST_STATE__;
    if (!state) throw new Error("Tauri mock state is not installed.");
    Object.assign(state, next);
  }, patch);
}

export async function commandCalls(page: Page): Promise<CommandCall[]> {
  return page.evaluate(() => {
    const state = window.__REEF_TEST_STATE__;
    if (!state) throw new Error("Tauri mock state is not installed.");
    return state.calls;
  });
}

declare global {
  interface Window {
    __REEF_TEST_STATE__?: BrowserMockState;
    __TAURI_INTERNALS__?: {
      invoke: (
        cmd: string,
        args?: Record<string, unknown>,
        options?: unknown
      ) => Promise<unknown>;
      transformCallback: () => number;
      unregisterCallback: () => void;
      runCallback: () => void;
      callbacks: Record<string, unknown>;
      convertFileSrc: (filePath: string) => string;
    };
  }
}
