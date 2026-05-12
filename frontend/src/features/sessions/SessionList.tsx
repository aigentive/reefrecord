import { useState } from "react";
import { Archive, RefreshCw, Trash2, Upload } from "lucide-react";
import type { SessionSummary } from "../../api/types";
import {
  clearSessionAudio,
  deleteSession,
  syncSession,
  transcribeSession,
} from "../../api/bridge";

type Props = {
  sessions: SessionSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onSessionUpdated: (session: SessionSummary) => void;
  onSessionRemoved: (id: string) => void;
  githubSyncEnabled: boolean;
};

export function SessionList({
  sessions,
  selectedId,
  onSelect,
  onSessionUpdated,
  onSessionRemoved,
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
          onSessionRemoved={onSessionRemoved}
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
  onSessionRemoved: (id: string) => void;
  githubSyncEnabled: boolean;
};

function SessionRow({
  session,
  selected,
  onSelect,
  onSessionUpdated,
  onSessionRemoved,
  githubSyncEnabled,
}: RowProps) {
  const [retrying, setRetrying] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [busyAction, setBusyAction] = useState<null | "delete" | "clear-audio">(
    null
  );

  const transStatus = session.transcriptionStatus;
  const syncStatus = session.syncStatus;
  const transcriptBusy = transStatus === "transcribing" || retrying;
  const hasAudio = !!session.audioPath;
  const canRetranscribe =
    hasAudio &&
    (transStatus === "pending" ||
      transStatus === "failed" ||
      transStatus === "complete" ||
      transStatus === "not_started");
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
        transcriptionPromptTokens: undefined,
        transcriptionOutputTokens: undefined,
        transcriptionTotalTokens: undefined,
        transcriptionCostUsd: undefined,
        transcriptionModel: undefined,
      });
      const r = await transcribeSession(session.id);
      onSessionUpdated(r);
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

  async function doDelete(e: React.MouseEvent) {
    e.stopPropagation();
    const ok = window.confirm(
      `Delete ${session.id}?\n\nRemoves the audio file, transcript, and metadata. This cannot be undone.`
    );
    if (!ok) return;
    setBusyAction("delete");
    try {
      await deleteSession(session.id);
      onSessionRemoved(session.id);
    } catch (err) {
      window.alert(`Delete failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }

  async function doClearAudio(e: React.MouseEvent) {
    e.stopPropagation();
    if (!hasAudio) return;
    const ok = window.confirm(
      `Clear the audio file from ${session.id}?\n\nKeeps the transcript. You won't be able to retranscribe afterwards.`
    );
    if (!ok) return;
    setBusyAction("clear-audio");
    try {
      const next = await clearSessionAudio(session.id);
      onSessionUpdated(next);
    } catch (err) {
      window.alert(`Clear audio failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
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
      {session.transcriptPreview && (
        <div className="session-row-preview">{session.transcriptPreview}</div>
      )}
      <div className="session-row-meta">
        <span className="session-row-quant">
          {formatSeconds(session.durationSeconds)}
          {session.audioPath && (
            <>
              <span className="session-row-dot" aria-hidden>·</span>
              <span>{session.audioFormat.toUpperCase()}</span>
            </>
          )}
          {providerModel(session) && (
            <>
              <span className="session-row-dot" aria-hidden>·</span>
              <span>{providerModel(session)}</span>
            </>
          )}
          {typeof session.transcriptionCostUsd === "number" && (
            <>
              <span className="session-row-dot" aria-hidden>·</span>
              <span
                title={
                  session.transcriptionUsage
                    ? formatUsageTitle(session)
                    : session.transcriptionTotalTokens
                    ? `${formatTokens(session.transcriptionPromptTokens ?? 0)} in · ${formatTokens(
                        session.transcriptionOutputTokens ?? 0
                      )} out${modelSuffix(session)}`
                    : providerModel(session) || undefined
                }
                className="session-row-cost"
              >
                {formatUsd(session.transcriptionCostUsd)}
              </span>
            </>
          )}
        </span>
        <span className="session-row-tag" data-state={transStatus}>
          {labelTrans(transStatus, transcriptBusy)}
        </span>
        <span className="session-row-tag" data-state={syncStatus}>
          {labelSync(syncStatus, syncing)}
        </span>
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
              ? "Retranscribe with selected parser"
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
        <button
          type="button"
          className="btn btn-icon"
          aria-label="Clear audio"
          title={
            hasAudio
              ? "Clear audio (keeps transcript)"
              : "Audio already cleared"
          }
          disabled={!hasAudio || busyAction === "clear-audio"}
          onClick={doClearAudio}
        >
          <Archive size={13} />
        </button>
        <button
          type="button"
          className="btn btn-icon btn-danger"
          aria-label="Delete session"
          title="Delete session (audio + transcript + metadata)"
          disabled={busyAction === "delete"}
          onClick={doDelete}
        >
          <Trash2 size={13} />
        </button>
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

function providerModel(session: SessionSummary): string {
  const provider = session.transcriptionProvider
    ? providerName(session.transcriptionProvider)
    : "";
  const model = session.transcriptionModel ?? "";
  return [provider, model].filter(Boolean).join(" · ");
}

function modelSuffix(session: SessionSummary): string {
  const text = providerModel(session);
  return text ? ` · ${text}` : "";
}

function providerName(provider: NonNullable<SessionSummary["transcriptionProvider"]>): string {
  switch (provider) {
    case "gemini":
      return "Gemini";
    case "openai":
      return "OpenAI";
    case "deepgram":
      return "Deepgram";
  }
}

function formatUsageTitle(session: SessionSummary): string | undefined {
  const usage = session.transcriptionUsage;
  if (!usage) return providerModel(session) || undefined;
  switch (usage.kind) {
    case "tokens":
      return `${formatTokens(usage.promptTokens)} in · ${formatTokens(
        usage.outputTokens
      )} out · ${formatTokens(usage.totalTokens)} total${modelSuffix(session)}`;
    case "duration":
      return `${formatSeconds(Math.round(usage.seconds))}${modelSuffix(session)}`;
    case "deepgram":
      return `${
        typeof usage.durationSeconds === "number"
          ? formatSeconds(Math.round(usage.durationSeconds))
          : "Deepgram"
      }${modelSuffix(session)}`;
    case "unknown":
      return providerModel(session) || undefined;
  }
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
