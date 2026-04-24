import { useEffect, useState } from "react";
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
      onSessionUpdated({
        ...session,
        transcriptPath: r.transcriptPath,
        transcriptionStatus: r.status,
        transcriptionError: undefined,
      });
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
    <>
      <div className="row">
        <button
          type="button"
          className="btn"
          disabled={!session.wavPath}
          title={session.wavPath ? undefined : "WAV has been cleared"}
          onClick={() =>
            session.wavPath &&
            revealPath(session.wavPath).catch((e) => setMsg(String(e)))
          }
        >
          <ExternalLink size={14} />
          Reveal WAV
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
        <div className="spacer" />
        {session.transcriptionStatus === "failed" && (
          <button
            type="button"
            className="btn btn-primary"
            onClick={retry}
            disabled={retrying}
          >
            <RefreshCw size={14} />
            {retrying ? "Retrying…" : "Retry transcription"}
          </button>
        )}
        {session.transcriptionStatus === "complete" && text && (
          <button type="button" className="btn" onClick={copy}>
            <Copy size={14} />
            Copy
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

      {session.transcriptionStatus === "complete" &&
        typeof session.transcriptionCostUsd === "number" && (
          <div
            className="field-hint"
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontVariantNumeric: "tabular-nums",
            }}
          >
            {formatUsd(session.transcriptionCostUsd)}
            {session.transcriptionTotalTokens
              ? ` · ${formatTokens(session.transcriptionPromptTokens ?? 0)} in · ${formatTokens(
                  session.transcriptionOutputTokens ?? 0
                )} out · ${formatTokens(session.transcriptionTotalTokens)} total`
              : ""}
            {session.transcriptionModel ? ` · ${session.transcriptionModel}` : ""}
          </div>
        )}

      {session.transcriptionStatus === "failed" && session.transcriptionError && (
        <div className="field-error">{session.transcriptionError}</div>
      )}
      {session.syncStatus === "failed" && session.syncError && (
        <div className="field-error">Sync: {session.syncError}</div>
      )}
      {msg && <div className="field-hint">{msg}</div>}

      {session.transcriptionStatus === "complete" ? (
        <div className="transcript-body">
          {loading ? "Loading…" : text || "Transcript file is empty."}
        </div>
      ) : session.transcriptionStatus === "transcribing" ? (
        <div className="empty">Transcribing…</div>
      ) : session.transcriptionStatus === "failed" ? (
        <div className="empty">Transcription failed. WAV is still saved.</div>
      ) : (
        <div className="empty">No transcript yet.</div>
      )}
    </>
  );
}
