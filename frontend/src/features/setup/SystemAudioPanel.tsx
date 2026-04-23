import { useEffect, useState } from "react";
import { ExternalLink, RefreshCw } from "lucide-react";
import type { AppStatus, AudioDevice } from "../../api/types";
import {
  listAudioDevices,
  openPermissionsSettings,
} from "../../api/bridge";

type Props = {
  status: AppStatus | null;
  onChanged: () => Promise<void> | void;
};

export function SystemAudioPanel({ status, onChanged }: Props) {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      const list = await listAudioDevices();
      setDevices(list);
      await onChanged();
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();

  }, []);

  const blackhole = devices.filter((d) => d.isBlackhole);

  return (
    <div className="inline-setup-body">
      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        Optional. Enables mixing system audio into the recording by reading a
        BlackHole virtual device. Missing BlackHole is a warning, not a
        blocker.
      </p>
      {status?.systemAudio.detail && (
        <div
          className={
            status.systemAudio.state === "ready"
              ? "field-success"
              : status.systemAudio.state === "warning"
              ? "field-hint"
              : "field-hint"
          }
        >
          {status.systemAudio.detail}
        </div>
      )}

      <div className="field">
        <label className="field-label">Detected BlackHole devices</label>
        {blackhole.length === 0 ? (
          <div className="field-hint">
            {loading ? "Scanning devices…" : "None detected."}
          </div>
        ) : (
          <ul
            style={{
              listStyle: "none",
              margin: 0,
              padding: 0,
              display: "flex",
              flexDirection: "column",
              gap: 4,
            }}
          >
            {blackhole.map((d) => (
              <li
                key={d.id}
                style={{
                  padding: "6px 10px",
                  borderRadius: "var(--radius)",
                  border: "1px solid var(--border)",
                  fontSize: 12,
                  display: "flex",
                  gap: 8,
                  alignItems: "center",
                }}
              >
                <span className="dot" data-state="ready" aria-hidden />
                <span style={{ flex: 1 }}>{d.name}</span>
                <span className="muted">{d.inputChannels}ch</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <details style={{ fontSize: 12 }}>
        <summary style={{ cursor: "pointer", color: "var(--muted)" }}>
          Setup checklist
        </summary>
        <ol style={{ marginTop: 8, marginBottom: 0, paddingLeft: 18 }}>
          <li>Install BlackHole 2ch: <code>brew install --cask blackhole-2ch</code></li>
          <li>Reboot so CoreAudio loads the driver.</li>
          <li>Open Audio MIDI Setup and create a Multi-Output Device with BlackHole + your speakers.</li>
          <li>Set macOS output to that Multi-Output Device while recording.</li>
          <li>Refresh this panel.</li>
        </ol>
      </details>

      <div className="row">
        <button
          type="button"
          className="btn"
          onClick={refresh}
          disabled={loading}
        >
          <RefreshCw size={14} />
          {loading ? "Refreshing…" : "Refresh"}
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => openPermissionsSettings("soundSettings").catch((e) => setErr(String(e)))}
        >
          <ExternalLink size={14} />
          Open sound settings
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => openPermissionsSettings("audioMidiSetup").catch((e) => setErr(String(e)))}
        >
          <ExternalLink size={14} />
          Audio MIDI Setup
        </button>
      </div>
      {err && <div className="field-error">{err}</div>}
    </div>
  );
}
