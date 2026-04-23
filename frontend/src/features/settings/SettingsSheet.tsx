import { useState } from "react";
import { X } from "lucide-react";
import type { Settings } from "../../api/types";
import { saveSettings } from "../../api/bridge";

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
        if (e.target === e.currentTarget && !dirty) onClose();
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
            <h3 style={{ margin: 0, fontSize: 14 }}>Gemini</h3>

            <div className="field">
              <label className="field-label" htmlFor="s-model">Model</label>
              <input
                id="s-model"
                className="input"
                value={draft.geminiModel}
                onChange={(e) => set("geminiModel", e.target.value)}
              />
            </div>

            <div className="field">
              <label className="field-label" htmlFor="s-fallback">Fallback model</label>
              <input
                id="s-fallback"
                className="input"
                value={draft.geminiFallbackModel}
                onChange={(e) => set("geminiFallbackModel", e.target.value)}
              />
            </div>

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
