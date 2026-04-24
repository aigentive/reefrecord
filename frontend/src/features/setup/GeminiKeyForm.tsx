import { useState } from "react";
import { Eye, EyeOff, Check, Trash2 } from "lucide-react";
import type { AppStatus } from "../../api/types";
import {
  deleteGeminiKey,
  saveGeminiKey,
  validateGeminiKey,
} from "../../api/bridge";

type Props = {
  status: AppStatus | null;
  onChanged: () => Promise<void> | void;
};

export function GeminiKeyForm({ status, onChanged }: Props) {
  const [key, setKey] = useState("");
  const [reveal, setReveal] = useState(false);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [msg, setMsg] = useState<{ kind: "success" | "error" | "info"; text: string } | null>(null);

  const hasKey = status?.gemini.state === "ready" || status?.gemini.state === "warning";
  const canValidate = hasKey || key.trim().length > 0;

  async function doSave() {
    if (!key.trim()) {
      setMsg({ kind: "error", text: "Paste a key first." });
      return;
    }
    setSaving(true);
    setMsg(null);
    try {
      const result = await saveGeminiKey(key.trim());
      if (result.state === "ready") {
        setMsg({ kind: "success", text: result.detail || "Key saved and validated." });
      } else {
        setMsg({ kind: "error", text: result.detail || "Key saved but validation failed." });
      }
      setKey("");
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    } finally {
      setSaving(false);
    }
  }

  async function doValidate() {
    setValidating(true);
    setMsg(null);
    try {
      const typed = key.trim();
      const result = await validateGeminiKey(typed || undefined);
      setMsg({
        kind: result.state === "ready" ? "success" : "error",
        text: result.detail || "Validation complete.",
      });
      // Refresh app status only when we validated the stored key; a typed-but-
      // unsaved check doesn't change persisted state.
      if (!typed) await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    } finally {
      setValidating(false);
    }
  }

  async function doDelete() {
    const ok = window.confirm(
      "Remove the saved Gemini API key from the keychain? You can paste it again later."
    );
    if (!ok) return;
    setMsg(null);
    try {
      await deleteGeminiKey();
      setMsg({ kind: "info", text: "Key removed." });
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    }
  }

  return (
    <div className="inline-setup-body">
      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        Stored locally as an AES-256-GCM encrypted file under the app config
        dir. Key is derived from your machine identifier. Never written to
        .env, logs, or settings JSON.
      </p>

      <div className="field">
        <label className="field-label" htmlFor="gemini-key">
          {hasKey ? "Replace key" : "Paste key"}
        </label>
        <div className="row">
          <input
            id="gemini-key"
            className="input"
            type={reveal ? "text" : "password"}
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="AIzaSy…"
            autoComplete="off"
            spellCheck={false}
          />
          <button
            type="button"
            className="btn btn-icon"
            aria-label={reveal ? "Hide key" : "Reveal key"}
            onClick={() => setReveal((p) => !p)}
          >
            {reveal ? <EyeOff size={14} /> : <Eye size={14} />}
          </button>
        </div>
      </div>

      <div className="row">
        <button
          type="button"
          className="btn btn-primary"
          disabled={saving || !key}
          onClick={doSave}
        >
          {saving ? "Saving…" : "Save key"}
        </button>
        <button
          type="button"
          className="btn"
          disabled={validating || !canValidate}
          onClick={doValidate}
          title={
            key.trim()
              ? "Validate the typed key without saving"
              : hasKey
              ? "Validate the saved key"
              : "Paste a key first"
          }
        >
          <Check size={14} />
          {validating ? "Validating…" : "Validate"}
        </button>
        <div className="spacer" />
        {hasKey && (
          <button type="button" className="btn btn-danger" onClick={doDelete}>
            <Trash2 size={14} />
            Remove
          </button>
        )}
      </div>

      {msg && (
        <div
          className={
            msg.kind === "success"
              ? "field-success"
              : msg.kind === "error"
              ? "field-error"
              : "field-hint"
          }
        >
          {msg.text}
        </div>
      )}

      {hasKey && !msg && status?.gemini.detail && (
        <div className="field-hint">{status.gemini.detail}</div>
      )}
    </div>
  );
}
