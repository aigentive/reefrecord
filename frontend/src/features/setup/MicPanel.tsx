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

export function MicPanel({ status, onChanged }: Props) {
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

  const micDevices = devices.filter((d) => d.inputChannels > 0 && !d.isBlackhole);
  const detail = status?.mic.detail;

  return (
    <div className="inline-setup-body">
      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        Choose which microphone to use, or leave it on default.
      </p>
      {status?.mic.state === "denied" && (
        <div className="field-error">
          Microphone access is denied. Open System Settings to grant it.
        </div>
      )}
      {detail && status?.mic.state !== "denied" && (
        <div className="field-hint">{detail}</div>
      )}

      <div className="field">
        <label className="field-label">Input devices</label>
        {micDevices.length === 0 ? (
          <div className="field-hint">
            {loading ? "Scanning devices…" : "No input devices detected."}
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
            {micDevices.map((d) => (
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
                <span
                  className="dot"
                  data-state={d.isDefault ? "ready" : "checking"}
                  aria-hidden
                />
                <span style={{ flex: 1 }}>{d.name}</span>
                <span className="muted">{d.inputChannels}ch</span>
                {d.isDefault && (
                  <span className="muted" style={{ fontSize: 10, textTransform: "uppercase", letterSpacing: "0.05em" }}>default</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

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
          onClick={() => openPermissionsSettings("microphone").catch((e) => setErr(String(e)))}
        >
          <ExternalLink size={14} />
          Open microphone settings
        </button>
      </div>
      {err && <div className="field-error">{err}</div>}
    </div>
  );
}
