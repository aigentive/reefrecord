import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Settings as SettingsIcon } from "lucide-react";
import { RecorderPanel } from "./features/recorder/RecorderPanel";
import { SessionList } from "./features/sessions/SessionList";
import { TranscriptDrawer } from "./features/sessions/TranscriptDrawer";
import {
  SettingsSheet,
  type SettingsSection,
} from "./features/settings/SettingsSheet";
import type { AppStatus, SessionSummary, Settings } from "./api/types";
import {
  getAppStatus,
  getSettings,
  listSessions,
} from "./api/bridge";

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
    null
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] =
    useState<SettingsSection>("setup");
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

  const setupOpenedRef = useRef(false);
  useEffect(() => {
    if (!status) return;
    if (!status.canRecord && !settingsOpen && !setupOpenedRef.current) {
      setupOpenedRef.current = true;
      setSettingsSection("setup");
      setSettingsOpen(true);
    }
  }, [status, settingsOpen]);

  useEffect(() => {
    if (sessions.length === 0) {
      setSelectedSessionId(null);
      return;
    }
    if (
      !selectedSessionId ||
      !sessions.some((session) => session.id === selectedSessionId)
    ) {
      setSelectedSessionId(sessions[0]?.id ?? null);
    }
  }, [sessions, selectedSessionId]);

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

  const openSettings = useCallback((section: SettingsSection = "setup") => {
    setSettingsSection(section);
    setSettingsOpen(true);
  }, []);

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
            onClick={() => openSettings("setup")}
          >
            <SettingsIcon size={16} />
          </button>
        </div>
      </header>

      <main className="app-main">
        <section className="app-main-left">
          <RecorderPanel
            status={status}
            settings={settings}
            onCompleted={handleSessionCompleted}
            onSessionUpdated={handleSessionUpdated}
            onRefreshStatus={refreshStatus}
            onOpenSettings={openSettings}
          />

          {selectedSession ? (
            <TranscriptDrawer
              session={selectedSession}
              onSessionUpdated={handleSessionUpdated}
            />
          ) : (
            <section className="transcript" aria-label="Transcript">
              <div className="transcript__body">
                <div className="empty">No sessions yet. Record to create one.</div>
              </div>
            </section>
          )}
        </section>

        <div className="app-main-right">
          <SessionList
            sessions={sessions}
            selectedId={selectedSessionId}
            onSelect={setSelectedSessionId}
            onSessionUpdated={handleSessionUpdated}
            onSessionRemoved={handleSessionRemoved}
            githubSyncEnabled={settings?.githubSyncEnabled ?? false}
          />
        </div>
      </main>

      {error && (
        <div
          role="alert"
          className="app-toast app-toast--error"
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
          status={status}
          initialSection={settingsSection}
          onClose={() => setSettingsOpen(false)}
          onSaved={async (next) => {
            setSettings(next);
            await refreshStatus();
          }}
          onChanged={async () => {
            await refreshStatus();
            await refreshSettings();
          }}
        />
      )}
    </div>
  );
}
