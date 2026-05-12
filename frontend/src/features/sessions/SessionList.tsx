import { useMemo, useState } from "react";
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

const FILTERS = ["All", "Pending", "Failed", "Synced"] as const;
type Filter = (typeof FILTERS)[number];
type Bucket = "today" | "earlier";

export function SessionList({
  sessions,
  selectedId,
  onSelect,
  onSessionUpdated,
  onSessionRemoved,
  githubSyncEnabled,
}: Props) {
  const [filter, setFilter] = useState<Filter>("All");
  const visible = useMemo(
    () => sessions.filter((session) => matchesFilter(session, filter)),
    [sessions, filter]
  );

  return (
    <aside className="sessions" aria-label="Sessions">
      <div className="sessions__head">
        <div className="sessions__title-row">
          <h2 className="sessions__title">Sessions</h2>
          <span className="sessions__count">{sessions.length} total</span>
        </div>
      </div>
      <div className="sessions__filter-row" role="tablist" aria-label="Filter sessions">
        {FILTERS.map((item) => (
          <button
            key={item}
            type="button"
            className="filter-chip"
            role="tab"
            aria-selected={filter === item}
            onClick={() => setFilter(item)}
          >
            {item}
          </button>
        ))}
      </div>

      <div className="sessions__list" role="listbox" aria-label="Recorded sessions">
        {sessions.length === 0 ? (
          <div className="empty">No sessions yet. Record to create one.</div>
        ) : visible.length === 0 ? (
          <div className="empty">No sessions match this filter.</div>
        ) : (
          (["today", "earlier"] as const).map((bucket) => {
            const rows = visible.filter((session) => bucketOf(session.startedAt) === bucket);
            if (rows.length === 0) return null;
            return (
              <div
                key={bucket}
                className="sessions__group"
                data-muted={bucket !== "today"}
              >
                <div className="sessions__group-label">
                  {bucket === "today" ? "Today" : "Earlier"}
                </div>
                {rows.map((session) => (
                  <SessionRow
                    key={session.id}
                    session={session}
                    selected={session.id === selectedId}
                    onSelect={onSelect}
                    onSessionUpdated={onSessionUpdated}
                    onSessionRemoved={onSessionRemoved}
                    githubSyncEnabled={githubSyncEnabled}
                  />
                ))}
              </div>
            );
          })
        )}
      </div>
    </aside>
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
      role="option"
      className="session-row"
      aria-selected={selected}
      tabIndex={0}
      onClick={() => onSelect(session.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect(session.id);
        }
      }}
    >
      <span className="session-row__title">{formatTime(session.startedAt)}</span>
      {bucketOf(session.startedAt) === "earlier" && (
        <span className="session-row__date">{formatShortDate(session.startedAt)}</span>
      )}
      <div className="session-row__meta">
        <span className="num">{formatSeconds(session.durationSeconds)}</span>
        {typeof session.transcriptionCostUsd === "number" && (
          <>
            <span className="sep" aria-hidden />
            <span className="num" title={formatUsageTitle(session)}>
              {formatUsd(session.transcriptionCostUsd)}
            </span>
          </>
        )}
        {session.transcriptionStatus !== "complete" && (
          <>
            <span className="sep" aria-hidden />
            <span className="session-row-tag" data-state={transStatus}>
              {labelTrans(transStatus, transcriptBusy)}
            </span>
          </>
        )}
        {session.syncStatus === "synced" || session.syncStatus === "failed" ? (
          <>
            <span className="sep" aria-hidden />
            <span className="session-row-tag" data-state={syncStatus}>
              {labelSync(syncStatus, syncing)}
            </span>
          </>
        ) : null}
      </div>
      <div className="session-row__actions" onClick={(e) => e.stopPropagation()}>
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

function matchesFilter(session: SessionSummary, filter: Filter): boolean {
  if (filter === "All") return true;
  if (filter === "Pending") {
    return (
      session.transcriptionStatus === "pending" ||
      session.transcriptionStatus === "transcribing" ||
      session.transcriptionStatus === "not_started"
    );
  }
  if (filter === "Failed") {
    return session.transcriptionStatus === "failed" || session.syncStatus === "failed";
  }
  return session.syncStatus === "synced";
}

function bucketOf(iso: string): Bucket {
  const date = new Date(iso);
  const now = new Date();
  const startToday = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate()
  ).getTime();
  return date.getTime() >= startToday ? "today" : "earlier";
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function formatShortDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString([], {
      month: "short",
      day: "numeric",
    });
  } catch {
    return "";
  }
}

function formatSeconds(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
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
