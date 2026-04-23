import {
  Mic,
  Volume2,
  KeyRound,
  FolderOpen,
  Github,
  Check,
  AlertTriangle,
  X,
  Clock,
  MinusCircle,
} from "lucide-react";
import type { AppStatus, ProviderStatus, ReadinessState } from "../../api/types";
import type { SetupPanelKey } from "../../App";

type Props = {
  status: AppStatus | null;
  openPanel: SetupPanelKey;
  onToggle: (key: Exclude<SetupPanelKey, null>) => void;
};

type ChipConfig = {
  key: Exclude<SetupPanelKey, null>;
  label: string;
  icon: React.ReactNode;
  pick: (s: AppStatus) => ProviderStatus;
  optional?: boolean;
};

const CHIPS: ChipConfig[] = [
  { key: "mic", label: "Mic", icon: <Mic size={12} />, pick: (s) => s.mic },
  {
    key: "systemAudio",
    label: "System Audio",
    icon: <Volume2 size={12} />,
    pick: (s) => s.systemAudio,
    optional: true,
  },
  {
    key: "gemini",
    label: "Gemini",
    icon: <KeyRound size={12} />,
    pick: (s) => s.gemini,
  },
  {
    key: "folder",
    label: "Folder",
    icon: <FolderOpen size={12} />,
    pick: (s) => s.folder,
  },
  {
    key: "github",
    label: "GitHub",
    icon: <Github size={12} />,
    pick: (s) => s.github,
    optional: true,
  },
];

function stateIcon(state: ReadinessState) {
  switch (state) {
    case "ready":
      return <Check size={12} />;
    case "warning":
      return <AlertTriangle size={12} />;
    case "missing":
    case "denied":
      return <X size={12} />;
    case "checking":
      return <Clock size={12} />;
    case "optional":
      return <MinusCircle size={12} />;
  }
}

export function SetupRail({ status, openPanel, onToggle }: Props) {
  return (
    <div className="setup-rail" role="toolbar" aria-label="Setup readiness">
      {CHIPS.map((chip) => {
        const ps: ProviderStatus = status
          ? chip.pick(status)
          : { state: "checking" };
        const state: ReadinessState =
          chip.optional && ps.state === "missing" ? "optional" : ps.state;
        return (
          <button
            key={chip.key}
            type="button"
            className="chip"
            data-state={state}
            data-selected={openPanel === chip.key}
            aria-pressed={openPanel === chip.key}
            onClick={() => onToggle(chip.key)}
            title={ps.detail || chip.label}
          >
            {chip.icon}
            <span>{chip.label}</span>
            <span aria-hidden>{stateIcon(state)}</span>
          </button>
        );
      })}
    </div>
  );
}
