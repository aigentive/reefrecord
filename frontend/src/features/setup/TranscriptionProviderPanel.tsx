import { useEffect, useMemo, useState } from "react";
import { Check, Eye, EyeOff, Trash2 } from "lucide-react";
import type {
  AppStatus,
  Settings,
  TranscriptionProvider,
} from "../../api/types";
import {
  deleteTranscriptionKey,
  saveSettings,
  saveTranscriptionKey,
  validateTranscriptionKey,
} from "../../api/bridge";
import {
  modelOptionsForProvider,
  modelOptionsWithCurrent,
} from "./modelOptions";

type Props = {
  status: AppStatus | null;
  settings: Settings | null;
  onChanged: () => Promise<void> | void;
};

const PROVIDERS: TranscriptionProvider[] = ["gemini", "openai", "deepgram"];

const LABELS: Record<TranscriptionProvider, string> = {
  gemini: "Gemini",
  openai: "OpenAI",
  deepgram: "Deepgram",
};

export function TranscriptionProviderPanel({
  status,
  settings,
  onChanged,
}: Props) {
  const [provider, setProvider] = useState<TranscriptionProvider>(
    settings?.transcriptionProvider ?? "gemini"
  );
  const [key, setKey] = useState("");
  const [model, setModel] = useState("");
  const [reveal, setReveal] = useState(false);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [msg, setMsg] = useState<{
    kind: "success" | "error" | "info";
    text: string;
  } | null>(null);

  useEffect(() => {
    if (!settings) return;
    setProvider(settings.transcriptionProvider);
  }, [settings]);

  useEffect(() => {
    if (!settings) return;
    setModel(modelFor(settings, provider));
    setKey("");
    setMsg(null);
  }, [settings, provider]);

  const providerStatus = status?.providers?.[provider];
  const modelOptions = modelOptionsWithCurrent(
    modelOptionsForProvider(provider),
    model
  );
  const hasKey =
    providerStatus?.state === "ready" || providerStatus?.state === "warning";
  const canValidate = hasKey || key.trim().length > 0;

  const warning = useMemo(() => {
    if (!settings) return null;
    if (
      provider === "openai" &&
      model === "whisper-1" &&
      settings.includeSpeakerLabels
    ) {
      return "whisper-1 will not label speakers. Use gpt-4o-transcribe-diarize for speaker-aware output.";
    }
    return null;
  }, [settings, provider, model]);

  async function chooseProvider(next: TranscriptionProvider) {
    setProvider(next);
    setMsg(null);
    if (!settings || next === settings.transcriptionProvider) return;
    try {
      await saveSettings({ transcriptionProvider: next });
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    }
  }

  async function saveModel() {
    if (!settings) return;
    const trimmed = model.trim();
    if (!trimmed) {
      setMsg({ kind: "error", text: "Model is required." });
      return;
    }
    try {
      await saveSettings(modelInput(provider, trimmed));
      setMsg({ kind: "success", text: "Parser settings saved." });
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    }
  }

  async function doSaveKey() {
    if (!key.trim()) {
      setMsg({ kind: "error", text: "Paste a key first." });
      return;
    }
    setSaving(true);
    setMsg(null);
    try {
      const result = await saveTranscriptionKey(provider, key.trim());
      setMsg({
        kind: result.state === "ready" ? "success" : "error",
        text: result.detail || "Key saved.",
      });
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
      const result = await validateTranscriptionKey(provider, typed || undefined);
      setMsg({
        kind: result.state === "ready" ? "success" : "error",
        text: result.detail || "Validation complete.",
      });
      if (!typed) await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    } finally {
      setValidating(false);
    }
  }

  async function doDelete() {
    const ok = window.confirm(
      `Remove the saved ${LABELS[provider]} API key? You can paste it again later.`
    );
    if (!ok) return;
    setMsg(null);
    try {
      await deleteTranscriptionKey(provider);
      setMsg({ kind: "info", text: "Key removed." });
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    }
  }

  return (
    <div className="inline-setup-body">
      <div className="segmented" role="tablist" aria-label="Transcription parser">
        {PROVIDERS.map((p) => (
          <button
            key={p}
            type="button"
            className="btn"
            data-selected={provider === p}
            onClick={() => chooseProvider(p)}
          >
            {LABELS[p]}
          </button>
        ))}
      </div>

      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        API keys are stored in the OS credential store. Keys are never written
        to settings, logs, transcripts, or session metadata.
      </p>

      <div className="field">
        <label className="field-label" htmlFor="parser-model">
          Model
        </label>
        <div className="row">
          <select
            id="parser-model"
            className="input"
            value={model}
            onChange={(e) => setModel(e.target.value)}
          >
            {modelOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <button type="button" className="btn" onClick={saveModel}>
            Save
          </button>
        </div>
        {warning && <div className="field-hint">{warning}</div>}
      </div>

      <div className="field">
        <label className="field-label" htmlFor="parser-key">
          {hasKey ? `Replace ${LABELS[provider]} key` : `Paste ${LABELS[provider]} key`}
        </label>
        <div className="row">
          <input
            id="parser-key"
            className="input"
            type={reveal ? "text" : "password"}
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder={keyPlaceholder(provider)}
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
          disabled={saving || !key.trim()}
          onClick={doSaveKey}
        >
          {saving ? "Saving..." : "Save key"}
        </button>
        <button
          type="button"
          className="btn"
          disabled={validating || !canValidate}
          onClick={doValidate}
        >
          <Check size={14} />
          {validating ? "Validating..." : "Validate"}
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

      {hasKey && !msg && providerStatus?.detail && (
        <div className="field-hint">{providerStatus.detail}</div>
      )}
    </div>
  );
}

function modelFor(settings: Settings, provider: TranscriptionProvider): string {
  switch (provider) {
    case "gemini":
      return settings.geminiModel;
    case "openai":
      return settings.openaiModel;
    case "deepgram":
      return settings.deepgramModel;
  }
}

function modelInput(
  provider: TranscriptionProvider,
  model: string
): Partial<Settings> {
  switch (provider) {
    case "gemini":
      return { geminiModel: model };
    case "openai":
      return { openaiModel: model };
    case "deepgram":
      return { deepgramModel: model };
  }
}

function keyPlaceholder(provider: TranscriptionProvider): string {
  switch (provider) {
    case "gemini":
      return "AIza...";
    case "openai":
      return "sk-...";
    case "deepgram":
      return "Deepgram API key";
  }
}
