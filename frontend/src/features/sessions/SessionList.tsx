import type { SessionSummary } from "../../api/types";

type Props = {
  sessions: SessionSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

export function SessionList({ sessions, selectedId, onSelect }: Props) {
  if (sessions.length === 0) {
    return <div className="empty">No sessions yet. Record to create one.</div>;
  }
  return (
    <div className="session-list" role="list">
      {sessions.map((s) => (
        <button
          key={s.id}
          type="button"
          role="listitem"
          className="session-row"
          data-selected={selectedId === s.id}
          onClick={() => onSelect(s.id)}
          aria-pressed={selectedId === s.id}
          style={{ textAlign: "left" }}
        >
          <div className="session-row-head">
            <span className="session-row-id">{s.id}</span>
            <span className="session-row-time">{formatDate(s.startedAt)}</span>
          </div>
          <div className="session-row-meta">
            <span>{formatSeconds(s.durationSeconds)}</span>
            <span className="session-row-tag" data-state={mapTrans(s.transcriptionStatus)}>
              {labelTrans(s.transcriptionStatus)}
            </span>
            <span className="session-row-tag" data-state={mapSync(s.syncStatus)}>
              {labelSync(s.syncStatus)}
            </span>
          </div>
        </button>
      ))}
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

function mapTrans(s: SessionSummary["transcriptionStatus"]): string {
  return s;
}

function labelTrans(s: SessionSummary["transcriptionStatus"]): string {
  switch (s) {
    case "not_started": return "no transcript";
    case "pending": return "pending";
    case "transcribing": return "transcribing";
    case "complete": return "transcript";
    case "failed": return "transcript failed";
  }
}

function mapSync(s: SessionSummary["syncStatus"]): string {
  return s;
}

function labelSync(s: SessionSummary["syncStatus"]): string {
  switch (s) {
    case "not_enabled": return "sync off";
    case "queued": return "queued";
    case "syncing": return "syncing";
    case "synced": return "synced";
    case "skipped": return "skipped";
    case "failed": return "sync failed";
  }
}
