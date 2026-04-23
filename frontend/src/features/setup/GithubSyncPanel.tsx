import { useEffect, useState } from "react";
import type { GitSyncStatus, Settings } from "../../api/types";
import {
  saveSettings,
  validateGitSyncSettings,
} from "../../api/bridge";

type Props = {
  settings: Settings | null;
  onChanged: () => Promise<void> | void;
};

export function GithubSyncPanel({ settings, onChanged }: Props) {
  const [enabled, setEnabled] = useState(settings?.githubSyncEnabled ?? false);
  const [repoUrl, setRepoUrl] = useState(settings?.githubRepoUrl ?? "");
  const [targetFolder, setTargetFolder] = useState(settings?.githubTargetFolder ?? "sessions");
  const [lfs, setLfs] = useState(settings?.gitLfsEnabled ?? true);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [msg, setMsg] = useState<{ kind: "success" | "error" | "info"; text: string } | null>(null);
  const [gitStatus, setGitStatus] = useState<GitSyncStatus | null>(null);

  useEffect(() => {
    if (!settings) return;
    setEnabled(settings.githubSyncEnabled);
    setRepoUrl(settings.githubRepoUrl);
    setTargetFolder(settings.githubTargetFolder);
    setLfs(settings.gitLfsEnabled);
  }, [settings]);

  useEffect(() => {
    validateGitSyncSettings().then(setGitStatus).catch(() => null);
  }, []);

  async function doSave() {
    setSaving(true);
    setMsg(null);
    try {
      await saveSettings({
        githubSyncEnabled: enabled,
        githubRepoUrl: repoUrl.trim(),
        githubTargetFolder: targetFolder.trim() || "sessions",
        gitLfsEnabled: lfs,
      });
      setMsg({ kind: "success", text: "Saved." });
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
      const next = await validateGitSyncSettings();
      setGitStatus(next);
      if (next.gitInstalled && (next.gitLfsInstalled || !lfs) && next.repoUrlValid && next.targetFolderSafe) {
        setMsg({ kind: "success", text: "Local tooling is ready." });
      } else {
        setMsg({ kind: "error", text: next.message || "One or more checks failed." });
      }
    } catch (e) {
      setMsg({ kind: "error", text: String(e) });
    } finally {
      setValidating(false);
    }
  }

  return (
    <div className="inline-setup-body">
      <p className="muted" style={{ margin: 0, fontSize: 12 }}>
        Optional. Sync completed sessions to a GitHub repo using local git and
        git-lfs. Uses your existing credentials.
      </p>

      <label className="toggle">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
        />
        <span>Enable GitHub sync</span>
      </label>

      <div className="field">
        <label className="field-label" htmlFor="gh-repo">Repository URL</label>
        <input
          id="gh-repo"
          className="input"
          value={repoUrl}
          placeholder="git@github.com:org/repo.git"
          onChange={(e) => setRepoUrl(e.target.value)}
          disabled={!enabled}
        />
      </div>

      <div className="field">
        <label className="field-label" htmlFor="gh-folder">Target folder</label>
        <input
          id="gh-folder"
          className="input"
          value={targetFolder}
          placeholder="sessions"
          onChange={(e) => setTargetFolder(e.target.value)}
          disabled={!enabled}
        />
      </div>

      <label className="toggle">
        <input
          type="checkbox"
          checked={lfs}
          onChange={(e) => setLfs(e.target.checked)}
          disabled={!enabled}
        />
        <span>Use Git LFS for WAV files</span>
      </label>

      <div className="row">
        <button
          type="button"
          className="btn btn-primary"
          onClick={doSave}
          disabled={saving}
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button
          type="button"
          className="btn"
          onClick={doValidate}
          disabled={validating}
        >
          {validating ? "Checking…" : "Test sync tools"}
        </button>
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

      {gitStatus && (
        <ul style={{ marginTop: 0, fontSize: 12, color: "var(--muted)" }}>
          <li>git installed: {gitStatus.gitInstalled ? "yes" : "no"}</li>
          <li>git-lfs installed: {gitStatus.gitLfsInstalled ? "yes" : "no"}</li>
          <li>repo URL set: {gitStatus.repoUrlValid ? "yes" : "no"}</li>
          <li>target folder safe: {gitStatus.targetFolderSafe ? "yes" : "no"}</li>
        </ul>
      )}
    </div>
  );
}
