import { useState } from "react";
import { RefreshCw, Upload } from "lucide-react";
import type { SessionSummary } from "../../api/types";
import { syncSession, transcribeSession } from "../../api/bridge";

type Props = {
  sessions: SessionSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onSessionUpdated: (session: SessionSummary) => void;
  githubSyncEnabled: boolean;
};

export function SessionList({
  sessions,
  selectedId,
  onSelect,
  onSessionUpdated,
  githubSyncEnabled,
}: Props) {
  if (sessions.length === 0) {
    return <div className="empty">No sessions yet. Record to create one.</div>;
  }
  return (
    <div className="session-list" role="list">
      {sessions.map((s) => (
        <SessionRow
          key={s.id}
          session={s}
          selected={s.id === selectedId}
          onSelect={onSelect}
          onSessionUpdated={onSessionUpdated}
          githubSyncEnabled={githubSyncEnabled}
        />
      ))}
    </div>
  );
}

type RowProps = {
  session: SessionSummary;
  selected: boolean;
  onSelect: (id: string) => void;
  onSessionUpdated: (session: SessionSummary) => void;
  githubSyncEnabled: boolean;
};

function SessionRow({
  session,
  selected,
  onSelect,
  onSessionUpdated,
  githubSyncEnabled,
}: RowProps) {
  const [retrying, setRetrying] = useState(false);
  const [syncing, setSyncing] = useState(false);

  const transStatus = session.transcriptionStatus;
  const syncStatus = session.syncStatus;
  const transcriptBusy = transStatus === "transcribing" || retrying;
  const canRetranscribe =
    transStatus === "pending" ||
    transStatus === "failed" ||
    transStatus === "complete" ||
    transStatus === "not_started";
  const canSync = githubSyncEnabled && !syncing;

  async function doRetranscribe(e: React.MouseEvent) {
    e.stopPropagation();
    if (!canRetranscribe) return;
    setRetrying(true);
    try {
      onSessionUpdated({
        ...session,
        transcriptionStatus: "transcribing",
        transcriptionError: undefined,
      });
      const r = await transcribeSession(session.id);
      onSessionUpdated({
        ...session,
        transcriptPath: r.transcriptPath,
        transcriptionStatus: r.status,
        transcriptionError: undefined,
      });
    } catch (err) {
      onSessionUpdated({
        ...session,
        transcriptionStatus: "failed",
        transcriptionError: String(err),
      });
    } finally {
      setRetrying(false);
    }
  }

  async function doSync(e: React.MouseEvent) {
    e.stopPropagation();
    if (!canSync) return;
    setSyncing(true);
    try {
      const r = await syncSession(session.id);
      onSessionUpdated({
        ...session,
        syncStatus: r.status,
        syncError: r.status === "failed" ? r.message : undefined,
      });
    } catch (err) {
      onSessionUpdated({
        ...session,
        syncStatus: "failed",
        syncError: String(err),
      });
    } finally {
      setSyncing(false);
    }
  }

  return (
    <div
      role="listitem"
      className="session-row"
      data-selected={selected}
      tabIndex={0}
      onClick={() => onSelect(session.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect(session.id);
        }
      }}
    >
      <div className="session-row-head">
        <span className="session-row-id">{session.id}</span>
        <span className="session-row-time">{formatDate(session.startedAt)}</span>
      </div>
      <div className="session-row-meta">
        <span>{formatSeconds(session.durationSeconds)}</span>
        <span className="session-row-tag" data-state={transStatus}>
          {labelTrans(transStatus, transcriptBusy)}
        </span>
        <span className="session-row-tag" data-state={syncStatus}>
          {labelSync(syncStatus, syncing)}
        </span>
        {typeof session.transcriptionCostUsd === "number" && (
          <span
            className="session-row-cost"
            title={
              session.transcriptionTotalTokens
                ? `${formatTokens(
                    session.transcriptionPromptTokens ?? 0
                  )} in · ${formatTokens(
                    session.transcriptionOutputTokens ?? 0
                  )} out${
                    session.transcriptionModel
                      ? ` · ${session.transcriptionModel}`
                      : ""
                  }`
                : undefined
            }
          >
            {formatUsd(session.transcriptionCostUsd)}
          </span>
        )}
        <div className="spacer" />
        <button
          type="button"
          className="btn btn-icon"
          aria-label={
            transStatus === "complete"
              ? "Retranscribe"
              : transStatus === "failed"
              ? "Retry transcription"
              : "Transcribe"
          }
          title={
            transStatus === "complete"
              ? "Retranscribe (re-run Gemini with current settings)"
              : transStatus === "failed"
              ? "Retry transcription"
              : "Transcribe"
          }
          disabled={transcriptBusy || !canRetranscribe}
          onClick={doRetranscribe}
        >
          <RefreshCw size={13} className={transcriptBusy ? "spin" : undefined} />
        </button>
        {githubSyncEnabled && (
          <button
            type="button"
            className="btn btn-icon"
            aria-label={
              syncStatus === "failed"
                ? "Retry sync"
                : syncStatus === "synced"
                ? "Re-sync"
                : "Sync now"
            }
            title={
              syncStatus === "failed"
                ? "Retry sync"
                : syncStatus === "synced"
                ? "Re-sync"
                : "Sync now"
            }
            disabled={syncing || !canSync}
            onClick={doSync}
          >
            <Upload size={13} />
          </button>
        )}
      </div>
    </div>
  );
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      month: "short",
      day: "2-digit",
    });
  } catch {
    return iso;
  }
}

function formatSeconds(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}m ${String(s).padStart(2, "0")}s`;
}

function formatUsd(n: number): string {
  if (n < 0.01) return `$${n.toFixed(4)}`;
  if (n < 1) return `$${n.toFixed(3)}`;
  return `$${n.toFixed(2)}`;
}

function formatTokens(n: number): string {
  if (n < 1000) return `${n} tok`;
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k tok`;
  return `${(n / 1_000_000).toFixed(2)}M tok`;
}

function labelTrans(
  s: SessionSummary["transcriptionStatus"],
  busy: boolean
): string {
  if (busy) return "transcribing";
  switch (s) {
    case "not_started":
      return "no transcript";
    case "pending":
      return "pending";
    case "transcribing":
      return "transcribing";
    case "complete":
      return "transcript";
    case "failed":
      return "transcript failed";
  }
}

function labelSync(s: SessionSummary["syncStatus"], busy: boolean): string {
  if (busy) return "syncing";
  switch (s) {
    case "not_enabled":
      return "sync off";
    case "queued":
      return "queued";
    case "syncing":
      return "syncing";
    case "synced":
      return "synced";
    case "skipped":
      return "skipped";
    case "failed":
      return "sync failed";
  }
}
