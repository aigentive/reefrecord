import type { TranscriptionProvider } from "../../api/types";

export type ModelOption = {
  value: string;
  label: string;
};

export const GEMINI_MODEL_OPTIONS: ModelOption[] = [
  { value: "gemini-3-flash-preview", label: "Gemini 3 Flash Preview" },
  { value: "gemini-2.5-flash", label: "Gemini 2.5 Flash" },
];

export const OPENAI_MODEL_OPTIONS: ModelOption[] = [
  { value: "whisper-1", label: "Whisper" },
  { value: "gpt-4o-transcribe", label: "GPT-4o Transcribe" },
  { value: "gpt-4o-mini-transcribe", label: "GPT-4o mini Transcribe" },
  {
    value: "gpt-4o-transcribe-diarize",
    label: "GPT-4o Transcribe Diarize",
  },
];

export const OPENAI_FALLBACK_MODEL_OPTIONS: ModelOption[] = [
  { value: "", label: "None" },
  ...OPENAI_MODEL_OPTIONS,
];

export const DEEPGRAM_MODEL_OPTIONS: ModelOption[] = [
  { value: "nova-3", label: "Nova-3" },
  { value: "nova-3-general", label: "Nova-3 General" },
  { value: "nova-3-medical", label: "Nova-3 Medical" },
  { value: "nova-2", label: "Nova-2" },
  { value: "nova-2-general", label: "Nova-2 General" },
  { value: "nova-2-meeting", label: "Nova-2 Meeting" },
  { value: "nova-2-phonecall", label: "Nova-2 Phone Call" },
  { value: "nova-2-finance", label: "Nova-2 Finance" },
  { value: "nova-2-video", label: "Nova-2 Video" },
  { value: "nova", label: "Nova" },
  { value: "enhanced", label: "Enhanced" },
  { value: "base", label: "Base" },
  { value: "whisper", label: "Whisper Cloud" },
  { value: "whisper-large", label: "Whisper Cloud Large" },
];

export function modelOptionsForProvider(
  provider: TranscriptionProvider
): ModelOption[] {
  switch (provider) {
    case "gemini":
      return GEMINI_MODEL_OPTIONS;
    case "openai":
      return OPENAI_MODEL_OPTIONS;
    case "deepgram":
      return DEEPGRAM_MODEL_OPTIONS;
  }
}

export function modelOptionsWithCurrent(
  options: ModelOption[],
  current: string
): ModelOption[] {
  if (!current || options.some((option) => option.value === current)) {
    return options;
  }
  return [{ value: current, label: `Current: ${current}` }, ...options];
}
