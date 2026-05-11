import { X } from "lucide-react";
import type { AppStatus, Settings } from "../../api/types";
import type { SetupPanelKey } from "../../App";
import { TranscriptionProviderPanel } from "./TranscriptionProviderPanel";
import { FolderPicker } from "./FolderPicker";
import { MicPanel } from "./MicPanel";
import { SystemAudioPanel } from "./SystemAudioPanel";
import { GithubSyncPanel } from "./GithubSyncPanel";

type Props = {
  panel: Exclude<SetupPanelKey, null>;
  status: AppStatus | null;
  settings: Settings | null;
  onClose: () => void;
  onChanged: () => Promise<void> | void;
};

const TITLES: Record<Exclude<SetupPanelKey, null>, string> = {
  parser: "Parser",
  folder: "Sessions Folder",
  mic: "Microphone",
  systemAudio: "System Audio",
  github: "GitHub Sync",
};

export function InlineSetup({
  panel,
  status,
  settings,
  onClose,
  onChanged,
}: Props) {
  return (
    <div className="panel inline-setup">
      <div className="inline-setup-title">
        <h3>{TITLES[panel]}</h3>
        <button
          type="button"
          className="btn btn-icon"
          aria-label="Close setup panel"
          onClick={onClose}
        >
          <X size={14} />
        </button>
      </div>

      {panel === "parser" && (
        <TranscriptionProviderPanel
          status={status}
          settings={settings}
          onChanged={onChanged}
        />
      )}
      {panel === "folder" && (
        <FolderPicker settings={settings} onChanged={onChanged} />
      )}
      {panel === "mic" && <MicPanel status={status} onChanged={onChanged} />}
      {panel === "systemAudio" && (
        <SystemAudioPanel status={status} onChanged={onChanged} />
      )}
      {panel === "github" && (
        <GithubSyncPanel settings={settings} onChanged={onChanged} />
      )}
    </div>
  );
}
