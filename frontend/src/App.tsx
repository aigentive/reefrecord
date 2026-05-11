import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Settings as SettingsIcon } from "lucide-react";
import { SetupRail } from "./features/setup/SetupRail";
import { InlineSetup } from "./features/setup/InlineSetup";
import { RecorderPanel } from "./features/recorder/RecorderPanel";
import { SessionList } from "./features/sessions/SessionList";
import { TranscriptDrawer } from "./features/sessions/TranscriptDrawer";
import { SettingsSheet } from "./features/settings/SettingsSheet";
import { Archive, Trash2, X } from "lucide-react";
import type { AppStatus, SessionSummary, Settings } from "./api/types";
import {
  clearAllWavs,
  deleteAllSessions,
  getAppStatus,
  getSettings,
  listSessions,
  selectSessionsFolder,
} from "./api/bridge";

export type SetupPanelKey =
  | "parser"
  | "folder"
  | "mic"
  | "systemAudio"
  | "github"
  | null;

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
    null
  );
  const [openPanel, setOpenPanel] = useState<SetupPanelKey>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const s = await getAppStatus();
      setStatus(s);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const refreshSettings = useCallback(async () => {
    try {
      const s = await getSettings();
      setSettings(s);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const refreshSessions = useCallback(async () => {
    try {
      const s = await listSessions();
      setSessions(s);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refreshStatus();
    refreshSettings();
    refreshSessions();
  }, [refreshStatus, refreshSettings, refreshSessions]);

  // Progressive setup: if required items are missing, auto-open the first one.
  // Folder is also auto-picked natively on first run when missing — matches
  // the spec's "open native folder picker" behavior.
  const folderAutoPickedRef = useRef(false);
  useEffect(() => {
    if (!status) return;
    if (
      !folderAutoPickedRef.current &&
      status.folder.state === "missing"
    ) {
      folderAutoPickedRef.current = true;
      selectSessionsFolder()
        .then(async () => {
          await refreshStatus();
          await refreshSettings();
        })
        .catch(() => {
          setOpenPanel("folder");
        });
      return;
    }
    if (openPanel !== null) return;
    if (
      status.transcription.state !== "ready" &&
      status.transcription.state !== "warning"
    ) {
      setOpenPanel("parser");
      return;
    }
    if (status.folder.state !== "ready") {
      setOpenPanel("folder");
      return;
    }
    if (status.mic.state === "denied" || status.mic.state === "missing") {
      setOpenPanel("mic");
    }
  }, [status, openPanel, refreshStatus, refreshSettings]);

  const selectedSession = useMemo(
    () => sessions.find((s) => s.id === selectedSessionId) ?? null,
    [sessions, selectedSessionId]
  );

  const overallLabel = useMemo(() => {
    if (!status) return "Loading";
    if (status.canRecord) return "Ready to record";
    return status.blockingReason || "Setup required";
  }, [status]);

  const overallState = !status
    ? "checking"
    : status.canRecord
    ? "ready"
    : "missing";

  const handleSessionCompleted = useCallback(
    (session: SessionSummary) => {
      setSessions((prev) => {
        const next = prev.filter((s) => s.id !== session.id);
        return [session, ...next];
      });
      setSelectedSessionId(session.id);
    },
    []
  );

  const handleSessionUpdated = useCallback((updated: SessionSummary) => {
    setSessions((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
  }, []);

  const handleSessionRemoved = useCallback((id: string) => {
    setSessions((prev) => prev.filter((s) => s.id !== id));
    setSelectedSessionId((prev) => (prev === id ? null : prev));
  }, []);

  async function doDeleteAllSessions() {
    if (sessions.length === 0) return;
    const ok = window.confirm(
      `Delete all ${sessions.length} sessions?\n\nRemoves every WAV, transcript, and metadata file. This cannot be undone.`
    );
    if (!ok) return;
    try {
      await deleteAllSessions();
      setSessions([]);
      setSelectedSessionId(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function doClearAllWavs() {
    const withWav = sessions.filter((s) => s.wavPath).length;
    if (withWav === 0) return;
    const ok = window.confirm(
      `Clear WAV from ${withWav} session${withWav === 1 ? "" : "s"}?\n\nKeeps transcripts and metadata. Retranscription won't be possible after this.`
    );
    if (!ok) return;
    try {
      await clearAllWavs();
      await refreshSessions();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="app">
      <header className="app-top">
        <div className="app-title">
          <img
            src="/logo.png"
            alt=""
            className="app-title-mark"
            aria-hidden
          />
          Reef Recorder
        </div>
        <div className="row">
          <span className="chip" data-state={overallState}>
            <span className="dot" data-state={overallState} />
            {overallLabel}
          </span>
          <button
            type="button"
            className="btn btn-icon"
            aria-label="Open settings"
            onClick={() => setSettingsOpen(true)}
          >
            <SettingsIcon size={16} />
          </button>
        </div>
      </header>

      <main className="app-main">
        <section className="app-main-left">
          <div className="panel">
            <h2 className="panel-title">Setup</h2>
            <SetupRail
              status={status}
              openPanel={openPanel}
              onToggle={(key) =>
                setOpenPanel((prev) => (prev === key ? null : key))
              }
            />
          </div>

          {openPanel && (
            <InlineSetup
              panel={openPanel}
              status={status}
              settings={settings}
              onClose={() => setOpenPanel(null)}
              onChanged={async () => {
                await refreshStatus();
                await refreshSettings();
              }}
            />
          )}

          <div className="panel">
            <h2 className="panel-title">Recorder</h2>
            <RecorderPanel
              status={status}
              settings={settings}
              onCompleted={handleSessionCompleted}
              onSessionUpdated={handleSessionUpdated}
              onRefreshStatus={refreshStatus}
            />
          </div>

          {selectedSession && (
            <>
              <div
                className="transcript-scrim"
                onClick={() => setSelectedSessionId(null)}
                aria-hidden
              />
              <div
                className="panel transcript-drawer"
                role="dialog"
                aria-label={`Transcript for ${selectedSession.id}`}
              >
                <div className="inline-setup-title">
                  <h3 style={{ margin: 0, fontSize: 14 }}>
                    Transcript — {selectedSession.id}
                  </h3>
                  <button
                    type="button"
                    className="btn btn-icon transcript-drawer-close"
                    aria-label="Close transcript"
                    onClick={() => setSelectedSessionId(null)}
                  >
                    <X size={14} />
                  </button>
                </div>
                <TranscriptDrawer
                  session={selectedSession}
                  onSessionUpdated={handleSessionUpdated}
                />
              </div>
            </>
          )}
        </section>

        <aside className="app-main-right">
          <div
            className="panel panel-list"
            style={{ flex: 1, minHeight: 0 }}
          >
            <div className="panel-list-head">
              <h3>Sessions</h3>
              <div className="row" style={{ gap: 2 }}>
                <span
                  className="muted"
                  style={{ fontSize: 12, marginRight: 4 }}
                >
                  {sessions.length} total
                </span>
                <button
                  type="button"
                  className="btn btn-icon"
                  aria-label="Clear all WAVs"
                  title="Clear WAV for every session (keeps transcripts)"
                  disabled={sessions.every((s) => !s.wavPath)}
                  onClick={doClearAllWavs}
                >
                  <Archive size={13} />
                </button>
                <button
                  type="button"
                  className="btn btn-icon btn-danger"
                  aria-label="Delete all sessions"
                  title="Delete all sessions (WAV + transcript + metadata)"
                  disabled={sessions.length === 0}
                  onClick={doDeleteAllSessions}
                >
                  <Trash2 size={13} />
                </button>
              </div>
            </div>
            <SessionList
              sessions={sessions}
              selectedId={selectedSessionId}
              onSelect={setSelectedSessionId}
              onSessionUpdated={handleSessionUpdated}
              onSessionRemoved={handleSessionRemoved}
              githubSyncEnabled={settings?.githubSyncEnabled ?? false}
            />
          </div>
        </aside>
      </main>

      {error && (
        <div
          role="alert"
          style={{
            position: "fixed",
            bottom: 16,
            left: 16,
            right: 16,
            maxWidth: 520,
            margin: "0 auto",
            background: "var(--surface)",
            border: "1px solid var(--error)",
            color: "var(--error)",
            padding: "10px 14px",
            borderRadius: "var(--radius)",
            fontSize: 13,
          }}
        >
          <div className="row">
            <span>{error}</span>
            <div className="spacer" />
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setError(null)}
            >
              Dismiss
            </button>
          </div>
        </div>
      )}

      {settingsOpen && settings && (
        <SettingsSheet
          settings={settings}
          onClose={() => setSettingsOpen(false)}
          onSaved={async (next) => {
            setSettings(next);
            await refreshStatus();
          }}
        />
      )}
    </div>
  );
}
