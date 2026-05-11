import { useEffect, useRef, useState } from "react";
import { Circle, Square } from "lucide-react";
import type { AppStatus, SessionSummary, Settings } from "../../api/types";
import {
  startRecording,
  stopRecording,
  transcribeSession,
} from "../../api/bridge";

type Props = {
  status: AppStatus | null;
  settings: Settings | null;
  onCompleted: (session: SessionSummary) => void;
  onSessionUpdated: (session: SessionSummary) => void;
  onRefreshStatus: () => Promise<void> | void;
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
}: Props) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const [error, setError] = useState<string | null>(null);
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
    setPhase("stopping");
    try {
      const summary = await stopRecording(sessionId);
      setPhase("saving");
      onCompleted(summary);
      setPhase("transcribing");
      try {
        const result = await transcribeSession(sessionId);
        onSessionUpdated(result);
      } catch (e) {
        onSessionUpdated({
          ...summary,
          transcriptionStatus: "failed",
          transcriptionError: String(e),
        });
      }
      setPhase("complete");
      setSessionId(null);
      setElapsed(0);
      startRef.current = null;
    } catch (e) {
      setError(String(e));
      setPhase("failed");
    }
  }

  const disabled = !status?.canRecord || phase === "starting" || phase === "stopping" || phase === "saving";
  const recording = phase === "recording";
  const busy = phase === "starting" || phase === "stopping" || phase === "saving" || phase === "transcribing";

  const label = recording ? "Stop" : "Record";

  const selectedMic = status?.mic.detail;
  const selectedSys = status?.systemAudio.detail;
  const providerLabel = settings
    ? providerName(settings.transcriptionProvider)
    : "selected parser";

  return (
    <div className="recorder-panel">
      <div className="recorder-timer" aria-live="polite">
        {formatDuration(elapsed)}
      </div>

      <button
        type="button"
        className="record-btn"
        data-state={recording ? "recording" : "idle"}
        aria-pressed={recording}
        aria-label={label}
        disabled={disabled}
        onClick={recording ? doStop : doStart}
      >
        {recording ? <Square size={24} fill="#fff" /> : <Circle size={28} fill="#fff" />}
      </button>

      <div className="recorder-devices">
        <div>
          <span className="muted">Mic:</span> {selectedMic || "—"}
        </div>
        <div>
          <span className="muted">System:</span>{" "}
          {settings?.captureSystemAudio ? selectedSys || "—" : "off"}
        </div>
      </div>

      {phase === "transcribing" && (
        <div className="field-hint">Transcribing with {providerLabel}...</div>
      )}

      {!status?.canRecord && status && !busy && (
        <div className="recorder-disabled-reason">
          {status.blockingReason || "Complete required setup to enable recording."}
        </div>
      )}

      {error && <div className="field-error">{error}</div>}
    </div>
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

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const mm = String(m).padStart(2, "0");
  const ss = String(s).padStart(2, "0");
  if (h > 0) return `${h}:${mm}:${ss}`;
  return `${mm}:${ss}`;
}
