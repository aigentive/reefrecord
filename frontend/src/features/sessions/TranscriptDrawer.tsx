import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Copy, ExternalLink, RefreshCw, Upload } from "lucide-react";
import type { SessionSummary } from "../../api/types";
import {
  readTranscript,
  revealPath,
  syncSession,
  transcribeSession,
} from "../../api/bridge";

type Props = {
  session: SessionSummary;
  onSessionUpdated: (s: SessionSummary) => void;
};

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

export function TranscriptDrawer({ session, onSessionUpdated }: Props) {
  const [text, setText] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [retrying, setRetrying] = useState(false);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setText(null);
    setMsg(null);
    if (session.transcriptionStatus === "complete") {
      setLoading(true);
      readTranscript(session.id)
        .then((t) => {
          if (!cancelled) setText(t);
        })
        .catch((e) => {
          if (!cancelled) setMsg(String(e));
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }
    return () => {
      cancelled = true;
    };
  }, [session.id, session.transcriptionStatus]);

  async function retry() {
    setRetrying(true);
    setMsg(null);
    try {
      const r = await transcribeSession(session.id);
      onSessionUpdated(r);
    } catch (e) {
      setMsg(String(e));
      onSessionUpdated({
        ...session,
        transcriptionStatus: "failed",
        transcriptionError: String(e),
      });
    } finally {
      setRetrying(false);
    }
  }

  async function doSync() {
    setSyncing(true);
    setMsg(null);
    try {
      const r = await syncSession(session.id);
      onSessionUpdated({
        ...session,
        syncStatus: r.status,
        syncError: r.status === "failed" ? r.message : undefined,
      });
    } catch (e) {
      setMsg(String(e));
      onSessionUpdated({
        ...session,
        syncStatus: "failed",
        syncError: String(e),
      });
    } finally {
      setSyncing(false);
    }
  }

  async function copy() {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setMsg("Copied.");
    } catch (e) {
      setMsg(String(e));
    }
  }

  return (
    <section className="transcript" aria-label={`Transcript for ${session.id}`}>
      <header className="transcript__head">
        <div className="transcript__title-row">
          <h2 className="transcript__title">Transcript</h2>
          <span className="transcript__id">{session.id}</span>
        </div>
        <div className="transcript__meta">
          <span className="num">{formatDuration(session.durationSeconds)}</span>
          {typeof session.transcriptionCostUsd === "number" && (
            <>
              <span className="sep" aria-hidden />
              <span className="num">{formatUsd(session.transcriptionCostUsd)}</span>
            </>
          )}
          {providerModel(session) && (
            <>
              <span className="sep" aria-hidden />
              <span className="mono muted">{providerModel(session)}</span>
            </>
          )}
          {session.transcriptionUsage && (
            <>
              <span className="sep" aria-hidden />
              <span>{formatUsage(session)}</span>
            </>
          )}
        </div>
        <div className="transcript__actions">
          {session.transcriptionStatus === "complete" && text && (
            <button type="button" className="btn btn-primary" onClick={copy}>
              <Copy size={14} />
              Copy transcript
            </button>
          )}
          <button
            type="button"
            className="btn"
            disabled={!session.audioPath}
            title={session.audioPath ? undefined : "Audio has been cleared"}
            onClick={() =>
              session.audioPath &&
              revealPath(session.audioPath).catch((e) => setMsg(String(e)))
            }
          >
            <ExternalLink size={14} />
            Reveal {session.audioFormat.toUpperCase()}
          </button>
          {session.transcriptPath && (
            <button
              type="button"
              className="btn"
              onClick={() =>
                revealPath(session.transcriptPath!).catch((e) => setMsg(String(e)))
              }
            >
              <ExternalLink size={14} />
              Reveal transcript
            </button>
          )}
          {session.transcriptionStatus === "failed" && (
            <button
              type="button"
              className="btn"
              onClick={retry}
              disabled={retrying}
            >
              <RefreshCw size={14} />
              {retrying ? "Retrying…" : "Retry transcription"}
            </button>
          )}
          {(session.syncStatus === "failed" ||
            session.syncStatus === "not_enabled" ||
            session.syncStatus === "skipped") && (
            <button
              type="button"
              className="btn"
              onClick={doSync}
              disabled={syncing}
            >
              <Upload size={14} />
              {syncing ? "Syncing…" : session.syncStatus === "failed" ? "Retry sync" : "Sync now"}
            </button>
          )}
        </div>
      </header>

      <div className="transcript__messages">
        {session.transcriptionStatus === "failed" && session.transcriptionError && (
          <div className="field-error">{session.transcriptionError}</div>
        )}
        {session.syncStatus === "failed" && session.syncError && (
          <div className="field-error">Sync: {session.syncError}</div>
        )}
        {msg && <div className="field-hint">{msg}</div>}
      </div>

      {session.transcriptionStatus === "complete" ? (
        <div className="transcript__body">
          {loading ? (
            <div className="empty">Loading…</div>
          ) : text ? (
            renderTranscript(text)
          ) : (
            <div className="empty">Transcript file is empty.</div>
          )}
        </div>
      ) : session.transcriptionStatus === "transcribing" ? (
        <div className="transcript__body">
          <div className="empty">Transcribing…</div>
        </div>
      ) : session.transcriptionStatus === "failed" ? (
        <div className="transcript__body">
          <p className="t-paragraph muted">
            Transcription failed. Audio is still saved for retry.
          </p>
          <button
            type="button"
            className="btn"
            onClick={retry}
            disabled={retrying}
          >
            <RefreshCw size={14} />
            {retrying ? "Retrying…" : "Retry transcription"}
          </button>
        </div>
      ) : (
        <div className="transcript__body">
          <div className="empty">Transcript appears here once recording stops.</div>
        </div>
      )}
    </section>
  );
}

function providerModel(session: SessionSummary): string {
  const provider = session.transcriptionProvider
    ? providerName(session.transcriptionProvider)
    : "";
  const model = session.transcriptionModel ?? "";
  return [provider, model].filter(Boolean).join(" · ");
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

function formatUsage(session: SessionSummary): string {
  const usage = session.transcriptionUsage;
  if (!usage) return "";
  switch (usage.kind) {
    case "tokens":
      return `${formatTokens(usage.promptTokens)} in · ${formatTokens(
        usage.outputTokens
      )} out · ${formatTokens(usage.totalTokens)} total`;
    case "duration":
      return `${Math.round(usage.seconds)} sec`;
    case "deepgram":
      return typeof usage.durationSeconds === "number"
        ? `${Math.round(usage.durationSeconds)} sec`
        : "Deepgram";
    case "unknown":
      return "";
  }
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return `${minutes}m ${String(rest).padStart(2, "0")}s`;
}

function renderTranscript(text: string): ReactNode {
  const paragraphs = text
    .split(/\n{2,}/)
    .map((block) => block.trim())
    .filter(Boolean);
  return paragraphs.map((paragraph, index) => {
    const parsed = parseTranscriptParagraph(paragraph);
    return (
      <p key={`${parsed.timestamp ?? "p"}-${index}`} className="t-paragraph">
        {parsed.timestamp && (
          <a href={`#${parsed.timestamp}`} className="t-anchor">
            [{parsed.timestamp}]
          </a>
        )}
        {parsed.speaker && <span className="t-speaker">{parsed.speaker}</span>}
        {parsed.text}
      </p>
    );
  });
}

function parseTranscriptParagraph(paragraph: string): {
  timestamp: string | null;
  speaker: string | null;
  text: string;
} {
  const normalized = paragraph.replace(/\s+/g, " ").trim();
  const timestampMatch = normalized.match(/^\[([0-9:]+)\]\s*(.*)$/);
  const withoutTimestamp = timestampMatch?.[2] ?? normalized;
  const speakerMatch = withoutTimestamp.match(/^\[?([A-Za-z ]+\s*\d*)\]?:\s*(.*)$/);
  const rawSpeaker = speakerMatch?.[1]?.trim() ?? null;
  const speaker =
    rawSpeaker && /^speaker\s*\d*$/i.test(rawSpeaker)
      ? rawSpeaker.toUpperCase()
      : null;
  return {
    timestamp: timestampMatch?.[1] ?? null,
    speaker,
    text: speaker ? speakerMatch?.[2]?.trim() ?? "" : withoutTimestamp,
  };
}
