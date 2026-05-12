import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  Check,
  DollarSign,
  FolderOpen,
  Github,
  HardDrive,
  Mic,
  Settings2,
  Type,
  X,
} from "lucide-react";
import type {
  AppStatus,
  ReadinessState,
  Settings,
  SettingsInput,
} from "../../api/types";
import { saveSettings } from "../../api/bridge";
import { FolderPicker } from "../setup/FolderPicker";
import { TranscriptionProviderPanel } from "../setup/TranscriptionProviderPanel";

export type SettingsSection =
  | "setup"
  | "audio"
  | "transcription"
  | "storage"
  | "sync"
  | "cost";

type Props = {
  settings: Settings;
  status: AppStatus | null;
  initialSection: SettingsSection;
  onClose: () => void;
  onSaved: (next: Settings) => void;
  onChanged: () => Promise<void> | void;
};

type StepState = "done" | "missing" | "optional";

type SetupStep = {
  section: SettingsSection;
  label: string;
  detail: string;
  required: boolean;
  state: StepState;
};

const NAV: Array<{
  id: SettingsSection;
  label: string;
  icon: ReactNode;
}> = [
  { id: "setup", label: "Get started", icon: <Check size={16} /> },
  { id: "audio", label: "Audio", icon: <Mic size={16} /> },
  { id: "transcription", label: "Transcription", icon: <Type size={16} /> },
  { id: "storage", label: "Storage", icon: <HardDrive size={16} /> },
  { id: "sync", label: "GitHub sync", icon: <Github size={16} /> },
  { id: "cost", label: "Cost tracking", icon: <DollarSign size={16} /> },
];

export function SettingsSheet({
  settings,
  status,
  initialSection,
  onClose,
  onSaved,
  onChanged,
}: Props) {
  const [draft, setDraft] = useState<Settings>(settings);
  const [section, setSection] = useState<SettingsSection>(initialSection);
  const [dirtyKeys, setDirtyKeys] = useState<Set<keyof Settings>>(
    () => new Set()
  );
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setDraft((current) => mergeDirtySettings(settings, current, dirtyKeys));
  }, [settings, dirtyKeys]);

  useEffect(() => {
    setSection(initialSection);
  }, [initialSection]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const dirty = dirtyKeys.size > 0;
  const steps = useMemo(() => buildSetupSteps(status, draft), [status, draft]);
  const requiredDone = steps.filter((step) => step.required && step.state === "done").length;
  const requiredTotal = steps.filter((step) => step.required).length;
  const allRequiredDone = requiredDone === requiredTotal;

  async function save() {
    setSaving(true);
    setErr(null);
    setSaved(false);
    try {
      const next = await saveSettings(pickDirtySettings(draft, dirtyKeys));
      setDirtyKeys(new Set());
      onSaved(next);
      setSaved(true);
    } catch (e) {
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  }

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    setDraft((current) => ({ ...current, [key]: value }));
    setDirtyKeys((current) => {
      const next = new Set(current);
      if (Object.is(value, settings[key])) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
    setSaved(false);
  }

  function closeWithDirtyCheck() {
    if (!dirty || window.confirm("Discard unsaved changes?")) onClose();
  }

  return (
    <div
      className="sheet-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
      onClick={(event) => {
        if (event.target === event.currentTarget) closeWithDirtyCheck();
      }}
    >
      <div className="settings-modal">
        <button
          type="button"
          className="settings-modal__close"
          aria-label="Close settings"
          onClick={closeWithDirtyCheck}
        >
          <X size={16} />
        </button>

        <nav className="settings-modal__nav" aria-label="Settings sections">
          <h2 id="settings-title" className="settings-modal__title">
            Settings
          </h2>
          {NAV.map((item) => (
            <button
              key={item.id}
              type="button"
              className="settings-modal__nav-btn"
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => setSection(item.id)}
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <div className="settings-modal__pane">
          {section !== "setup" && (
            <button
              type="button"
              className="settings-modal__back"
              onClick={() => setSection("setup")}
            >
              Back to Get started
            </button>
          )}

          {section === "setup" && (
            <>
              <PaneHead
                title="Get started"
                sub="Required items must be ready before recording. Optional items can be tuned later."
              />
              <ol className="setup-list">
                {steps.map((step, index) => (
                  <li
                    key={`${step.section}-${step.label}`}
                    className="setup-step"
                    data-state={step.state}
                  >
                    <span className="setup-step__num">{index + 1}</span>
                    <span className="setup-step__icon" aria-hidden>
                      {iconForSection(step.section)}
                    </span>
                    <div className="setup-step__body">
                      <div className="setup-step__title">
                        {step.label}
                        {!step.required && (
                          <span className="setup-step__tag">Optional</span>
                        )}
                      </div>
                      <div className="setup-step__detail">{step.detail}</div>
                    </div>
                    <span className="setup-step__badge" data-state={step.state}>
                      {step.state === "done"
                        ? "Done"
                        : step.state === "missing"
                        ? "Needs attention"
                        : "Skipped"}
                    </span>
                    <button
                      type="button"
                      className="btn"
                      onClick={() => setSection(step.section)}
                    >
                      {step.state === "done" ? "Edit" : "Set up"}
                    </button>
                  </li>
                ))}
              </ol>
              <div className="setup-footer">
                <span className="setup-footer__summary">
                  <Check size={14} />
                  {requiredDone} of {requiredTotal} required steps done
                </span>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={!allRequiredDone}
                  onClick={onClose}
                >
                  Start recording
                </button>
              </div>
            </>
          )}

          {section === "audio" && (
            <>
              <PaneHead
                title="Audio"
                sub="Capture source selection and raw archive behavior."
              />
              <label className="toggle toggle-row">
                <input
                  type="checkbox"
                  checked={draft.captureSystemAudio}
                  onChange={(event) => set("captureSystemAudio", event.target.checked)}
                />
                <span>
                  <span className="toggle-row__title">
                    Capture system audio when BlackHole is available
                  </span>
                  <span className="toggle-row__hint">
                    Mixed with the selected microphone into one recording.
                  </span>
                </span>
              </label>
              <div className="field-grid-2">
                <TextField
                  id="s-mic"
                  label="Microphone selector"
                  value={draft.micDeviceSelector ?? ""}
                  placeholder="Leave blank for default"
                  hint="Exact name fragment or numeric index."
                  onChange={(value) => set("micDeviceSelector", value || null)}
                />
                <TextField
                  id="s-sys"
                  label="System audio selector"
                  value={draft.systemAudioDeviceSelector ?? ""}
                  placeholder="blackhole"
                  onChange={(value) =>
                    set("systemAudioDeviceSelector", value || null)
                  }
                />
              </div>
            </>
          )}

          {section === "transcription" && (
            <>
              <PaneHead
                title="Transcription"
                sub="Provider keys, active parser, model selection, and transcript formatting."
              />
              <TranscriptionProviderPanel
                status={status}
                settings={settings}
                onChanged={onChanged}
              />
              <hr className="divider" />
              <div className="field-grid-2">
                <NumberField
                  id="s-chunk"
                  label="Chunk minutes"
                  min={1}
                  max={60}
                  value={draft.chunkMinutes}
                  onChange={(value) => set("chunkMinutes", value || 15)}
                />
                <TextField
                  id="s-lang"
                  label="Language hint"
                  value={draft.languageHint}
                  onChange={(value) => set("languageHint", value)}
                />
              </div>
              <label className="toggle toggle-row">
                <input
                  type="checkbox"
                  checked={draft.includeSpeakerLabels}
                  onChange={(event) =>
                    set("includeSpeakerLabels", event.target.checked)
                  }
                />
                <span>
                  <span className="toggle-row__title">
                    Label speakers as [Speaker 1], [Speaker 2]
                  </span>
                  <span className="toggle-row__hint">
                    Turn off when recording yourself solo.
                  </span>
                </span>
              </label>
              <label className="toggle toggle-row">
                <input
                  type="checkbox"
                  checked={draft.includeTimestamps}
                  onChange={(event) => set("includeTimestamps", event.target.checked)}
                />
                <span>
                  <span className="toggle-row__title">Include timestamps</span>
                  <span className="toggle-row__hint">
                    Adds [MM:SS] markers for skimmable review.
                  </span>
                </span>
              </label>
              {draft.transcriptionProvider === "deepgram" && (
                <div className="settings-card-grid">
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={draft.deepgramSmartFormat}
                      onChange={(event) =>
                        set("deepgramSmartFormat", event.target.checked)
                      }
                    />
                    <span>Smart format</span>
                  </label>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={draft.deepgramDiarize}
                      onChange={(event) =>
                        set("deepgramDiarize", event.target.checked)
                      }
                    />
                    <span>Diarization</span>
                  </label>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={draft.deepgramUtterances}
                      onChange={(event) =>
                        set("deepgramUtterances", event.target.checked)
                      }
                    />
                    <span>Utterances</span>
                  </label>
                </div>
              )}
            </>
          )}

          {section === "storage" && (
            <>
              <PaneHead
                title="Storage"
                sub="Session folder and durable audio format for future retranscription."
              />
              <FolderPicker settings={settings} onChanged={onChanged} />
              <div className="field">
                <label className="field-label" htmlFor="s-audio-storage">
                  Audio archive format
                </label>
                <select
                  id="s-audio-storage"
                  className="input"
                  value={draft.audioStorageFormat}
                  onChange={(event) =>
                    set(
                      "audioStorageFormat",
                      event.target.value as Settings["audioStorageFormat"]
                    )
                  }
                >
                  <option value="flac">FLAC archive (recommended)</option>
                  <option value="wav">WAV archive (current behavior)</option>
                </select>
                <div className="field-hint">
                  Applied after successful transcription; recording still captures WAV first.
                </div>
              </div>
            </>
          )}

          {section === "sync" && (
            <>
              <PaneHead
                title="GitHub sync"
                sub="Optional off-device backup for audio, transcript, and metadata files."
              />
              <label className="toggle toggle-row">
                <input
                  type="checkbox"
                  checked={draft.githubSyncEnabled}
                  onChange={(event) => set("githubSyncEnabled", event.target.checked)}
                />
                <span>
                  <span className="toggle-row__title">Enable GitHub sync</span>
                  <span className="toggle-row__hint">
                    Recording works normally when this is off.
                  </span>
                </span>
              </label>
              <TextField
                id="s-repo"
                label="Repository URL"
                value={draft.githubRepoUrl}
                placeholder="git@github.com:org/repo.git"
                disabled={!draft.githubSyncEnabled}
                onChange={(value) => set("githubRepoUrl", value)}
              />
              <TextField
                id="s-target"
                label="Target folder"
                value={draft.githubTargetFolder}
                disabled={!draft.githubSyncEnabled}
                onChange={(value) => set("githubTargetFolder", value)}
              />
              <label className="toggle toggle-row">
                <input
                  type="checkbox"
                  checked={draft.gitLfsEnabled}
                  disabled={!draft.githubSyncEnabled}
                  onChange={(event) => set("gitLfsEnabled", event.target.checked)}
                />
                <span>
                  <span className="toggle-row__title">Use Git LFS for audio files</span>
                </span>
              </label>
            </>
          )}

          {section === "cost" && (
            <>
              <PaneHead
                title="Cost tracking"
                sub="Per-session estimates are computed from these provider rates."
              />
              <div className="cost-grid">
                <NumberField
                  id="s-gemini-in-cost"
                  label="Gemini input $/1M"
                  value={draft.geminiInputCostPerMillionUsd}
                  step="0.01"
                  onChange={(value) => set("geminiInputCostPerMillionUsd", value)}
                />
                <NumberField
                  id="s-gemini-out-cost"
                  label="Gemini output $/1M"
                  value={draft.geminiOutputCostPerMillionUsd}
                  step="0.01"
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
            </>
          )}

          <div className="settings-modal__footer">
            {saved && !dirty && <span className="field-success">Saved.</span>}
            {err && <span className="field-error">{err}</span>}
            <div className="spacer" />
            <button type="button" className="btn" onClick={closeWithDirtyCheck}>
              {dirty ? "Discard" : "Close"}
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={save}
              disabled={saving || !dirty}
            >
              {saving ? "Saving…" : "Save changes"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function PaneHead({ title, sub }: { title: string; sub: string }) {
  return (
    <div className="settings-modal__pane-head">
      <h3 className="settings-modal__pane-title">{title}</h3>
      <p className="settings-modal__pane-sub">{sub}</p>
    </div>
  );
}

function TextField({
  id,
  label,
  value,
  placeholder,
  hint,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  placeholder?: string;
  hint?: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  return (
    <div className="field">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className="input"
        value={value}
        placeholder={placeholder}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      />
      {hint && <div className="field-hint">{hint}</div>}
    </div>
  );
}

function NumberField({
  id,
  label,
  value,
  min = 0,
  max,
  step = "0.01",
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  min?: number;
  max?: number;
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
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || 0)}
      />
    </div>
  );
}

function mergeDirtySettings(
  base: Settings,
  current: Settings,
  dirtyKeys: Set<keyof Settings>
): Settings {
  if (dirtyKeys.size === 0) return base;
  const next = { ...base };
  const writable = next as Record<keyof Settings, Settings[keyof Settings]>;
  for (const key of dirtyKeys) {
    writable[key] = current[key];
  }
  return next;
}

function pickDirtySettings(
  draft: Settings,
  dirtyKeys: Set<keyof Settings>
): SettingsInput {
  const input: SettingsInput = {};
  const writable = input as Record<keyof Settings, Settings[keyof Settings]>;
  for (const key of dirtyKeys) {
    writable[key] = draft[key];
  }
  return input;
}

function buildSetupSteps(
  status: AppStatus | null,
  settings: Settings
): SetupStep[] {
  return [
    {
      section: "audio",
      label: "Pick your microphone",
      detail: status?.mic.detail || "Default input device",
      required: true,
      state: readyState(status?.mic.state),
    },
    {
      section: "audio",
      label: "Enable system audio capture",
      detail: settings.captureSystemAudio
        ? status?.systemAudio.detail || "BlackHole selector enabled"
        : "Off",
      required: false,
      state: settings.captureSystemAudio && status?.systemAudio.state === "ready" ? "done" : "optional",
    },
    {
      section: "transcription",
      label: "Choose a transcription provider",
      detail: `${providerLabel(settings.transcriptionProvider)} · ${activeModel(settings)}`,
      required: true,
      state: readyState(status?.transcription.state),
    },
    {
      section: "storage",
      label: "Set the sessions folder",
      detail: settings.sessionsDir || "No folder selected",
      required: true,
      state: readyState(status?.folder.state),
    },
    {
      section: "sync",
      label: "Connect GitHub for backup",
      detail: settings.githubSyncEnabled
        ? status?.github.detail || settings.githubRepoUrl || "Enabled"
        : "Optional. Recording works without this.",
      required: false,
      state: settings.githubSyncEnabled ? readyState(status?.github.state) : "optional",
    },
  ];
}

function readyState(state?: ReadinessState): StepState {
  return state === "ready" || state === "warning" || state === "optional"
    ? "done"
    : "missing";
}

function iconForSection(section: SettingsSection) {
  switch (section) {
    case "audio":
      return <Mic size={18} />;
    case "transcription":
      return <Type size={18} />;
    case "storage":
      return <FolderOpen size={18} />;
    case "sync":
      return <Github size={18} />;
    case "cost":
      return <DollarSign size={18} />;
    case "setup":
      return <Settings2 size={18} />;
  }
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

function activeModel(settings: Settings): string {
  switch (settings.transcriptionProvider) {
    case "gemini":
      return settings.geminiModel;
    case "openai":
      return settings.openaiModel;
    case "deepgram":
      return settings.deepgramModel;
  }
}
