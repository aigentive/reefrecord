import { useEffect, useMemo, useRef, useState } from "react";
import { Circle, Folder, Square, WandSparkles } from "lucide-react";
import type { AppStatus, SessionSummary, Settings } from "../../api/types";
import {
  startRecording,
  stopRecording,
  transcribeSession,
} from "../../api/bridge";
import type { SettingsSection } from "../settings/SettingsSheet";

type Props = {
  status: AppStatus | null;
  settings: Settings | null;
  onCompleted: (session: SessionSummary) => void;
  onSessionUpdated: (session: SessionSummary) => void;
  onRefreshStatus: () => Promise<void> | void;
  onOpenSettings: (section: SettingsSection) => void;
};

type Phase =
  | "idle"
  | "starting"
  | "recording"
  | "stopping"
  | "saving"
  | "transcribing"
  | "complete"
  | "failed";

export function RecorderPanel({
  status,
  settings,
  onCompleted,
  onSessionUpdated,
  onRefreshStatus,
  onOpenSettings,
}: Props) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [level, setLevel] = useState(0);
  const startRef = useRef<number | null>(null);

  useEffect(() => {
    if (phase !== "recording") return;
    const id = window.setInterval(() => {
      if (startRef.current != null) {
        setElapsed(Math.floor((performance.now() - startRef.current) / 1000));
      }
    }, 250);
    return () => window.clearInterval(id);
  }, [phase]);

  useEffect(() => {
    if (phase !== "recording") {
      setLevel(0);
      return;
    }
    let frame = 0;
    const tick = (now: number) => {
      const t = now / 1000;
      const next =
        0.5 +
        0.24 * Math.sin(t * 2.1) +
        0.16 * Math.sin(t * 5.3 + 1.1) +
        0.05 * Math.sin(t * 11);
      setLevel(Math.max(0.05, Math.min(0.98, next)));
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frame);
  }, [phase]);

  async function doStart() {
    if (!settings) return;
    setError(null);
    setPhase("starting");
    setElapsed(0);
    try {
      const id = await startRecording({
        captureSystemAudio: settings.captureSystemAudio,
        micDeviceSelector: settings.micDeviceSelector,
        systemAudioDeviceSelector: settings.systemAudioDeviceSelector,
      });
      setSessionId(id);
      startRef.current = performance.now();
      setPhase("recording");
    } catch (e) {
      setError(String(e));
      setPhase("failed");
      await onRefreshStatus();
    }
  }

  async function doStop() {
    if (!sessionId) return;
    setError(null);
    setPhase("stopping");
    try {
      const summary = await stopRecording(sessionId);
      setPhase("saving");
      onCompleted(summary);
      setPhase("transcribing");
      try {
        const result = await transcribeSession(sessionId);
        onSessionUpdated(result);
        setPhase("complete");
      } catch (e) {
        const message = String(e);
        onSessionUpdated({
          ...summary,
          transcriptionStatus: "failed",
          transcriptionError: message,
          transcriptionProvider:
            settings?.transcriptionProvider ?? summary.transcriptionProvider,
        });
        setError(message);
        setPhase("failed");
      }
      setSessionId(null);
      setElapsed(0);
      startRef.current = null;
    } catch (e) {
      setError(String(e));
      setPhase("failed");
    }
  }

  const disabled =
    !status?.canRecord ||
    phase === "starting" ||
    phase === "stopping" ||
    phase === "saving" ||
    phase === "transcribing";
  const recording = phase === "recording";
  const busy = phase === "starting" || phase === "stopping" || phase === "saving" || phase === "transcribing";

  const label = recording ? "Stop" : "Record";

  const selectedMic = status?.mic.detail;
  const selectedSys = status?.systemAudio.detail;
  const providerLabel = settings
    ? providerName(settings.transcriptionProvider)
    : "selected parser";
  const modelLabel = settings ? modelName(settings) : "";
  const folderLabel = useMemo(
    () => shortPath(settings?.sessionsDir ?? ""),
    [settings?.sessionsDir]
  );
  const levelState = level > 0.95 ? "clip" : level > 0.78 ? "warn" : "ok";

  return (
    <section className="recorder-bar" aria-label="Recorder">
      <button
        type="button"
        className="record-btn record-btn--sm"
        data-state={recording ? "recording" : "idle"}
        aria-pressed={recording}
        aria-label={label}
        disabled={disabled}
        onClick={recording ? doStop : doStart}
      >
        {recording ? (
          <Square size={20} fill="currentColor" />
        ) : (
          <Circle size={22} fill="currentColor" />
        )}
      </button>

      <div className="recorder-bar__timer">
        <span className="timer-sm" aria-live="polite">
          {formatDuration(elapsed)}
        </span>
        <span className="recorder-bar__hint">
          {recording ? "Recording" : busy ? phaseLabel(phase) : "Press R to record"}
        </span>
      </div>

      <div className="meter meter--inline" aria-hidden={!recording}>
        <div className="meter__track">
          <div
            className="meter__fill"
            data-clip={levelState}
            style={{ width: `${(recording ? level : 0) * 100}%` }}
            role="meter"
            aria-valuemin={-60}
            aria-valuemax={0}
            aria-valuenow={recording ? Math.round(-60 + level * 60) : -60}
            aria-label="Input level"
          />
        </div>
        <div className="meter__scale" aria-hidden>
          <span>-60</span>
          <span>-36</span>
          <span>-18</span>
          <span>-6</span>
          <span>0</span>
        </div>
      </div>

      <div className="recorder-bar__context">
        <div className="recorder-bar__devices">
          <span>
            <span className="muted">In</span> {selectedMic || "No mic"}
          </span>
          <span className="recorder-bar__sep" aria-hidden />
          <span>
            <span className="muted">Sys</span>{" "}
            {settings?.captureSystemAudio ? selectedSys || "No system" : "Off"}
          </span>
        </div>
        <div className="recorder-bar__targets">
          <button
            type="button"
            className="ctx-chip"
            onClick={() => onOpenSettings("transcription")}
            title="Change transcription provider"
          >
            <WandSparkles size={14} />
            <span className="ctx-chip__label">{providerLabel}</span>
            {modelLabel && <span className="ctx-chip__detail">{modelLabel}</span>}
          </button>
          <button
            type="button"
            className="ctx-chip"
            onClick={() => onOpenSettings("storage")}
            title="Change sessions folder"
          >
            <Folder size={14} />
            <span className="ctx-chip__label">Folder</span>
            <span className="ctx-chip__detail">{folderLabel || "Not set"}</span>
          </button>
        </div>
      </div>

      {!status?.canRecord && status && !busy && (
        <div className="recorder-disabled-reason">
          {status.blockingReason || "Complete required setup to enable recording."}
        </div>
      )}

      {error && <div className="field-error">{error}</div>}
    </section>
  );
}

function providerName(provider: Settings["transcriptionProvider"]): string {
  switch (provider) {
    case "gemini":
      return "Gemini";
    case "openai":
      return "OpenAI";
    case "deepgram":
      return "Deepgram";
  }
}

function modelName(settings: Settings): string {
  switch (settings.transcriptionProvider) {
    case "gemini":
      return settings.geminiModel;
    case "openai":
      return settings.openaiModel;
    case "deepgram":
      return settings.deepgramModel;
  }
}

function phaseLabel(phase: Phase): string {
  switch (phase) {
    case "starting":
      return "Starting";
    case "stopping":
      return "Stopping";
    case "saving":
      return "Saving";
    case "transcribing":
      return "Transcribing";
    case "complete":
      return "Complete";
    case "failed":
      return "Failed";
    case "idle":
    case "recording":
      return "Press R to record";
  }
}

function shortPath(path: string): string {
  if (!path) return "";
  const home = path.replace(/^\/Users\/[^/]+/, "~");
  if (home.length <= 24) return home;
  const parts = home.split("/");
  return parts.length > 2 ? `…/${parts.slice(-2).join("/")}` : home;
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const mm = String(m).padStart(2, "0");
  const ss = String(s).padStart(2, "0");
  if (h > 0) return `${h}:${mm}:${ss}`;
  return `${mm}:${ss}`;
}
