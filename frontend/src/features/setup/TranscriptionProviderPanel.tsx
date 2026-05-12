import { useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, Check, Eye, EyeOff, KeyRound, Trash2 } from "lucide-react";
import type {
  AppStatus,
  ProviderStatus,
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
  const [editProvider, setEditProvider] = useState<TranscriptionProvider>(
    settings?.transcriptionProvider ?? "gemini"
  );
  const [key, setKey] = useState("");
  const [model, setModel] = useState("");
  const [reveal, setReveal] = useState(false);
  const [replacingKey, setReplacingKey] = useState(false);
  const [savingActive, setSavingActive] = useState(false);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [msg, setMsg] = useState<{
    kind: "success" | "error" | "info";
    text: string;
  } | null>(null);
  const initializedFromSettings = useRef(false);

  useEffect(() => {
    if (!settings || initializedFromSettings.current) return;
    setEditProvider(settings.transcriptionProvider);
    initializedFromSettings.current = true;
  }, [settings]);

  useEffect(() => {
    if (!settings) return;
    setModel(modelFor(settings, editProvider));
  }, [settings, editProvider]);

  useEffect(() => {
    setKey("");
    setReveal(false);
    setReplacingKey(false);
    setMsg(null);
  }, [editProvider]);

  const activeProvider = settings?.transcriptionProvider ?? "gemini";
  const activeProviderStatus = status?.providers?.[activeProvider];
  const providerStatus = status?.providers?.[editProvider];
  const modelOptions = modelOptionsWithCurrent(
    modelOptionsForProvider(editProvider),
    model
  );
  const hasKey = providerHasSavedKey(providerStatus);
  const showKeyInput = !hasKey || replacingKey;
  const canValidate = hasKey || key.trim().length > 0;

  const warning = useMemo(() => {
    if (!settings) return null;
    if (
      editProvider === "openai" &&
      model === "whisper-1" &&
      settings.includeSpeakerLabels
    ) {
      return "whisper-1 will not label speakers. Use gpt-4o-transcribe-diarize for speaker-aware output.";
    }
    return null;
  }, [settings, editProvider, model]);

  function chooseProvider(next: TranscriptionProvider) {
    setEditProvider(next);
  }

  async function saveActiveProvider(next: TranscriptionProvider) {
    setMsg(null);
    if (!settings || next === settings.transcriptionProvider) return;
    setSavingActive(true);
    try {
      await saveSettings({ transcriptionProvider: next });
      setEditProvider(next);
      const nextStatus = status?.providers?.[next];
      setMsg({
        kind: providerHasSavedKey(nextStatus) ? "success" : "info",
        text: providerHasSavedKey(nextStatus)
          ? `${LABELS[next]} is now the active parser.`
          : `${LABELS[next]} is now active. Add its API key before recording.`,
      });
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    } finally {
      setSavingActive(false);
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
      await saveSettings(modelInput(editProvider, trimmed));
      setMsg({ kind: "success", text: `${LABELS[editProvider]} model saved.` });
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
      const result = await saveTranscriptionKey(editProvider, key.trim());
      setMsg({
        kind: result.state === "ready" ? "success" : "error",
        text: result.detail || "Key saved.",
      });
      setKey("");
      setReveal(false);
      setReplacingKey(false);
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
      const result = await validateTranscriptionKey(editProvider, typed || undefined);
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
      `Remove the saved ${LABELS[editProvider]} API key? You can paste it again later.`
    );
    if (!ok) return;
    setMsg(null);
    try {
      await deleteTranscriptionKey(editProvider);
      setKey("");
      setReveal(false);
      setReplacingKey(false);
      setMsg({ kind: "info", text: "Key removed." });
      await onChanged();
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    }
  }

  return (
    <div className="inline-setup-body">
      <div className="field">
        <label className="field-label" htmlFor="active-parser">
          Active parser
        </label>
        <div className="row">
          <select
            id="active-parser"
            className="input"
            value={activeProvider}
            disabled={!settings || savingActive}
            onChange={(e) =>
              saveActiveProvider(e.target.value as TranscriptionProvider)
            }
          >
            {PROVIDERS.map((p) => (
              <option key={p} value={p}>
                {LABELS[p]}
              </option>
            ))}
          </select>
          <span
            className="parser-status-pill"
            data-state={activeProviderStatus?.state ?? "checking"}
          >
            <span
              className="dot"
              data-state={activeProviderStatus?.state ?? "checking"}
            />
            {activeProviderStatus
              ? providerStatusLabel(activeProviderStatus)
              : "Checking"}
          </span>
        </div>
        <div className="field-hint">
          New recordings use {LABELS[activeProvider]} unless you change this.
        </div>
      </div>

      <div
        className="parser-provider-list"
        role="tablist"
        aria-label="Provider key and model settings"
      >
        {PROVIDERS.map((p) => (
          <button
            key={p}
            type="button"
            className="parser-provider-option"
            data-selected={editProvider === p}
            data-active={activeProvider === p}
            onClick={() => chooseProvider(p)}
          >
            <span className="parser-provider-main">
              <span>{LABELS[p]}</span>
              {activeProvider === p && (
                <span className="parser-status-pill" data-state="ready">
                  Active
                </span>
              )}
            </span>
            <span className="parser-provider-sub">
              <span
                className="dot"
                data-state={status?.providers?.[p]?.state ?? "checking"}
              />
              {providerStatusLabel(status?.providers?.[p])}
            </span>
          </button>
        ))}
      </div>

      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        API keys are stored in the OS credential store. Keys are never written
        to settings, logs, transcripts, or session metadata.
      </p>

      <div className="parser-editor-head">
        <div>
          <div className="field-label">{LABELS[editProvider]} settings</div>
          <div className="field-hint">
            {editProvider === activeProvider
              ? "This provider is currently used for recording."
              : `Editing ${LABELS[editProvider]} does not change the active parser.`}
          </div>
        </div>
        {editProvider !== activeProvider && (
          <button
            type="button"
            className="btn"
            disabled={savingActive || !settings}
            onClick={() => saveActiveProvider(editProvider)}
          >
            Use as active
          </button>
        )}
      </div>

      <div className="field">
        <label className="field-label" htmlFor="parser-model">
          {LABELS[editProvider]} model
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
        <div className="field-label">{LABELS[editProvider]} API key</div>

        {hasKey && !showKeyInput && (
          <div className="stored-secret">
            <div className="stored-secret-main">
              <div className="stored-secret-title">
                {providerStatus?.state === "warning" ? (
                  <AlertTriangle size={14} />
                ) : (
                  <KeyRound size={14} />
                )}
                <span>Key saved</span>
              </div>
              <div className="field-hint">
                {providerStatus?.detail || "Stored in the OS credential store."}
              </div>
            </div>
            <div className="row stored-secret-actions">
              <button
                type="button"
                className="btn"
                onClick={() => setReplacingKey(true)}
              >
                Replace
              </button>
              <button
                type="button"
                className="btn"
                disabled={validating}
                onClick={doValidate}
              >
                <Check size={14} />
                {validating ? "Validating..." : "Validate"}
              </button>
              <button type="button" className="btn btn-danger" onClick={doDelete}>
                <Trash2 size={14} />
                Remove
              </button>
            </div>
          </div>
        )}

        {showKeyInput && (
          <>
            <div className="row">
              <input
                id="parser-key"
                className="input"
                type={reveal ? "text" : "password"}
                value={key}
                onChange={(e) => setKey(e.target.value)}
                placeholder={
                  hasKey
                    ? "Paste a replacement key"
                    : keyPlaceholder(editProvider)
                }
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
            {hasKey && (
              <div className="field-hint">
                Leave this blank to keep the saved key.
              </div>
            )}
          </>
        )}
      </div>

      {showKeyInput && (
        <div className="row">
          <button
            type="button"
            className="btn btn-primary"
            disabled={saving || !key.trim()}
            onClick={doSaveKey}
          >
            {saving ? "Saving..." : hasKey ? "Save replacement" : "Save key"}
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
            <button
              type="button"
              className="btn"
              onClick={() => {
                setReplacingKey(false);
                setKey("");
                setReveal(false);
              }}
            >
              Cancel
            </button>
          )}
        </div>
      )}

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
    </div>
  );
}

function providerHasSavedKey(status?: ProviderStatus): boolean {
  return status?.state === "ready" || status?.state === "warning";
}

function providerStatusLabel(status?: ProviderStatus): string {
  if (!status) return "Checking";
  switch (status.state) {
    case "ready":
      return "Key saved";
    case "warning":
      return "Key saved, warning";
    case "missing":
      return "No key";
    case "denied":
      return "Key issue";
    case "checking":
      return "Checking";
    case "optional":
      return "Optional";
  }
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
