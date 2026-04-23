import { useState } from "react";
import { FolderOpen, ExternalLink } from "lucide-react";
import type { Settings } from "../../api/types";
import {
  revealSessionsFolder,
  selectSessionsFolder,
} from "../../api/bridge";

type Props = {
  settings: Settings | null;
  onChanged: () => Promise<void> | void;
};

export function FolderPicker({ settings, onChanged }: Props) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function doPick() {
    setBusy(true);
    setErr(null);
    try {
      const next = await selectSessionsFolder();
      if (next) {
        await onChanged();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function doReveal() {
    setErr(null);
    try {
      await revealSessionsFolder();
    } catch (e) {
      setErr(String(e));
    }
  }

  return (
    <div className="inline-setup-body">
      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        Where WAV and transcript files are saved.
      </p>
      <div className="field">
        <label className="field-label">Current folder</label>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: settings?.sessionsDir ? "var(--text)" : "var(--muted)",
            padding: "8px 12px",
            border: "1px solid var(--border)",
            borderRadius: "var(--radius)",
            background: "var(--bg)",
            wordBreak: "break-all",
          }}
        >
          {settings?.sessionsDir || "No folder selected yet."}
        </div>
      </div>

      <div className="row">
        <button
          type="button"
          className="btn btn-primary"
          onClick={doPick}
          disabled={busy}
        >
          <FolderOpen size={14} />
          {busy ? "Opening…" : "Choose folder"}
        </button>
        {settings?.sessionsDir && (
          <button type="button" className="btn" onClick={doReveal}>
            <ExternalLink size={14} />
            Reveal
          </button>
        )}
      </div>
      {err && <div className="field-error">{err}</div>}
    </div>
  );
}
