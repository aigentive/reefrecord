import { useEffect, useState } from "react";
import { X } from "lucide-react";
import type { Settings } from "../../api/types";
import { saveSettings } from "../../api/bridge";
import {
  DEEPGRAM_MODEL_OPTIONS,
  GEMINI_MODEL_OPTIONS,
  OPENAI_FALLBACK_MODEL_OPTIONS,
  OPENAI_MODEL_OPTIONS,
  modelOptionsWithCurrent,
} from "../setup/modelOptions";

type Props = {
  settings: Settings;
  onClose: () => void;
  onSaved: (next: Settings) => void;
};

export function SettingsSheet({ settings, onClose, onSaved }: Props) {
  const [draft, setDraft] = useState<Settings>(settings);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);

  async function save() {
    setSaving(true);
    setErr(null);
    setSaved(false);
    try {
      const next = await saveSettings(draft);
      onSaved(next);
      setSaved(true);
    } catch (e) {
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  }

  // Auto-dismiss after a successful save so the user gets visible confirmation
  // before the sheet closes itself.
  useEffect(() => {
    if (!saved || err) return;
    const t = window.setTimeout(() => {
      onClose();
    }, 1800);
    return () => window.clearTimeout(t);
  }, [saved, err, onClose]);

  function set<K extends keyof Settings>(k: K, v: Settings[K]) {
    setDraft((d) => ({ ...d, [k]: v }));
    setSaved(false);
  }

  return (
    <div
      className="sheet-backdrop"
      role="dialog"
      aria-modal="true"
      onClick={(e) => {
        if (e.target !== e.currentTarget) return;
        if (!dirty) {
          onClose();
          return;
        }
        const confirmed = window.confirm(
          "Discard unsaved changes?"
        );
        if (confirmed) onClose();
      }}
    >
      <div className="sheet">
        <div className="sheet-head">
          <h2 className="sheet-title">Settings</h2>
          <button
            type="button"
            className="btn btn-icon"
            aria-label="Close"
            onClick={onClose}
          >
            <X size={14} />
          </button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          <section style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <h3 style={{ margin: 0, fontSize: 14 }}>Audio</h3>

            <label className="toggle">
              <input
                type="checkbox"
                checked={draft.captureSystemAudio}
                onChange={(e) => set("captureSystemAudio", e.target.checked)}
              />
              <span>Capture system audio when BlackHole is available</span>
            </label>

            <div className="field">
              <label className="field-label" htmlFor="s-mic">Microphone selector</label>
              <input
                id="s-mic"
                className="input"
                value={draft.micDeviceSelector ?? ""}
                onChange={(e) => set("micDeviceSelector", e.target.value || null)}
                placeholder="Leave blank for default"
              />
              <div className="field-hint">Exact name fragment or numeric index.</div>
            </div>

            <div className="field">
              <label className="field-label" htmlFor="s-sys">System audio selector</label>
              <input
                id="s-sys"
                className="input"
                value={draft.systemAudioDeviceSelector ?? ""}
                onChange={(e) => set("systemAudioDeviceSelector", e.target.value || null)}
                placeholder="blackhole"
              />
            </div>
          </section>

          <section style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <h3 style={{ margin: 0, fontSize: 14 }}>Transcription</h3>

            <div className="field">
              <label className="field-label" htmlFor="s-active-parser">
                Active parser
              </label>
              <select
                id="s-active-parser"
                className="input"
                value={draft.transcriptionProvider}
                onChange={(e) =>
                  set(
                    "transcriptionProvider",
                    e.target.value as Settings["transcriptionProvider"]
                  )
                }
              >
                {(["gemini", "openai", "deepgram"] as const).map((provider) => (
                  <option key={provider} value={provider}>
                    {providerLabel(provider)}
                  </option>
                ))}
              </select>
              <div className="field-hint">
                Recording and default retranscription use this provider.
              </div>
            </div>

            {draft.transcriptionProvider === "gemini" && (
              <>
                <div className="field">
                  <label className="field-label" htmlFor="s-gemini-model">
                    Gemini model
                  </label>
                  <ModelSelect
                    id="s-gemini-model"
                    value={draft.geminiModel}
                    options={GEMINI_MODEL_OPTIONS}
                    onChange={(value) => set("geminiModel", value)}
                  />
                </div>

                <div className="field">
                  <label className="field-label" htmlFor="s-gemini-fallback">
                    Gemini fallback
                  </label>
                  <ModelSelect
                    id="s-gemini-fallback"
                    value={draft.geminiFallbackModel}
                    options={GEMINI_MODEL_OPTIONS}
                    onChange={(value) => set("geminiFallbackModel", value)}
                  />
                </div>
              </>
            )}


            {draft.transcriptionProvider === "openai" && (
              <>
                <div className="field">
                  <label className="field-label" htmlFor="s-openai-model">
                    OpenAI model
                  </label>
                  <ModelSelect
                    id="s-openai-model"
                    value={draft.openaiModel}
                    options={OPENAI_MODEL_OPTIONS}
                    onChange={(value) => set("openaiModel", value)}
                  />
                </div>

                <div className="field">
                  <label className="field-label" htmlFor="s-openai-fallback">
                    OpenAI fallback
                  </label>
                  <ModelSelect
                    id="s-openai-fallback"
                    value={draft.openaiFallbackModel}
                    options={OPENAI_FALLBACK_MODEL_OPTIONS}
                    onChange={(value) => set("openaiFallbackModel", value)}
                  />
                </div>
              </>
            )}

            {draft.transcriptionProvider === "deepgram" && (
              <>
                <div className="field">
                  <label className="field-label" htmlFor="s-deepgram-model">
                    Deepgram model
                  </label>
                  <ModelSelect
                    id="s-deepgram-model"
                    value={draft.deepgramModel}
                    options={DEEPGRAM_MODEL_OPTIONS}
                    onChange={(value) => set("deepgramModel", value)}
                  />
                </div>

                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={draft.deepgramSmartFormat}
                    onChange={(e) => set("deepgramSmartFormat", e.target.checked)}
                  />
                  <span>Smart format</span>
                </label>

                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={draft.deepgramDiarize}
                    onChange={(e) => set("deepgramDiarize", e.target.checked)}
                  />
                  <span>Diarization</span>
                </label>

                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={draft.deepgramUtterances}
                    onChange={(e) => set("deepgramUtterances", e.target.checked)}
                  />
                  <span>Utterances</span>
                </label>
              </>
            )}

            <div className="field">
              <label className="field-label" htmlFor="s-chunk">Chunk minutes</label>
              <input
                id="s-chunk"
                className="input"
                type="number"
                min={1}
                max={60}
                value={draft.chunkMinutes}
                onChange={(e) => set("chunkMinutes", Number(e.target.value) || 15)}
              />
            </div>

            <div className="field">
              <label className="field-label" htmlFor="s-lang">Language hint</label>
              <input
                id="s-lang"
                className="input"
                value={draft.languageHint}
                onChange={(e) => set("languageHint", e.target.value)}
              />
            </div>

            <label className="toggle">
              <input
                type="checkbox"
                checked={draft.includeSpeakerLabels}
                onChange={(e) => set("includeSpeakerLabels", e.target.checked)}
              />
              <span>Label speakers as [Speaker 1], [Speaker 2]</span>
            </label>
            <div className="field-hint" style={{ marginLeft: 44, marginTop: -6 }}>
              Turn off when recording yourself solo.
            </div>

            <label className="toggle">
              <input
                type="checkbox"
                checked={draft.includeTimestamps}
                onChange={(e) => set("includeTimestamps", e.target.checked)}
              />
              <span>Include [MM:SS] timestamps</span>
            </label>
            <div className="field-hint" style={{ marginLeft: 44, marginTop: -6 }}>
              Turn off for a clean flowing transcript.
            </div>

            <div className="cost-grid">
              <NumberField
                id="s-gemini-in-cost"
                label="Gemini input $/1M"
                value={draft.geminiInputCostPerMillionUsd}
                onChange={(value) => set("geminiInputCostPerMillionUsd", value)}
              />
              <NumberField
                id="s-gemini-out-cost"
                label="Gemini output $/1M"
                value={draft.geminiOutputCostPerMillionUsd}
                onChange={(value) => set("geminiOutputCostPerMillionUsd", value)}
              />
              <NumberField
                id="s-openai-minute-cost"
                label="OpenAI $/minute"
                value={draft.openaiCostPerMinuteUsd}
                step="0.001"
                onChange={(value) => set("openaiCostPerMinuteUsd", value)}
              />
              <NumberField
                id="s-deepgram-hour-cost"
                label="Deepgram $/hour"
                value={draft.deepgramCostPerHourUsd}
                step="0.001"
                onChange={(value) => set("deepgramCostPerHourUsd", value)}
              />
            </div>
          </section>

          <section style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <h3 style={{ margin: 0, fontSize: 14 }}>GitHub sync</h3>

            <label className="toggle">
              <input
                type="checkbox"
                checked={draft.githubSyncEnabled}
                onChange={(e) => set("githubSyncEnabled", e.target.checked)}
              />
              <span>Enable GitHub sync</span>
            </label>

            <div className="field">
              <label className="field-label" htmlFor="s-repo">Repository URL</label>
              <input
                id="s-repo"
                className="input"
                value={draft.githubRepoUrl}
                onChange={(e) => set("githubRepoUrl", e.target.value)}
                placeholder="git@github.com:org/repo.git"
                disabled={!draft.githubSyncEnabled}
              />
            </div>

            <div className="field">
              <label className="field-label" htmlFor="s-target">Target folder</label>
              <input
                id="s-target"
                className="input"
                value={draft.githubTargetFolder}
                onChange={(e) => set("githubTargetFolder", e.target.value)}
                disabled={!draft.githubSyncEnabled}
              />
            </div>

            <label className="toggle">
              <input
                type="checkbox"
                checked={draft.gitLfsEnabled}
                onChange={(e) => set("gitLfsEnabled", e.target.checked)}
                disabled={!draft.githubSyncEnabled}
              />
              <span>Use Git LFS for WAV files</span>
            </label>
          </section>
        </div>

        <div className="row" style={{ marginTop: 8 }}>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            {dirty ? "Discard" : "Close"}
          </button>
          <div className="spacer" />
          {saved && !dirty && <span className="field-success">Saved.</span>}
          <button
            type="button"
            className="btn btn-primary"
            onClick={save}
            disabled={saving || !dirty}
          >
            {saving ? "Saving…" : "Save changes"}
          </button>
        </div>

        {err && <div className="field-error">{err}</div>}
      </div>
    </div>
  );
}

function NumberField({
  id,
  label,
  value,
  step = "0.01",
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  step?: string;
  onChange: (value: number) => void;
}) {
  return (
    <div className="field">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className="input"
        type="number"
        step={step}
        min={0}
        value={value}
        onChange={(e) => onChange(Number(e.target.value) || 0)}
      />
    </div>
  );
}

function ModelSelect({
  id,
  value,
  options,
  onChange,
}: {
  id: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (value: string) => void;
}) {
  return (
    <select
      id={id}
      className="input"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {modelOptionsWithCurrent(options, value).map((option) => (
        <option key={option.value || "none"} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

function providerLabel(provider: Settings["transcriptionProvider"]): string {
  switch (provider) {
    case "gemini":
      return "Gemini";
    case "openai":
      return "OpenAI";
    case "deepgram":
      return "Deepgram";
  }
}
