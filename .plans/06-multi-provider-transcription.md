# Phase 6: Multi-Provider Transcription

## Objective

Allow the saved WAV to be transcribed by one selected parser:

- Gemini, using the existing Gemini `generateContent` pipeline.
- OpenAI, defaulting to `whisper-1`, with optional OpenAI transcription models.
- Deepgram, using a user-provided Deepgram API key and pre-recorded `/listen`.

This is still post-recording transcription only. Do not add real-time transcription in this phase.

## Researched API Baseline

Gemini:

- The current Gemini audio guide uses `gemini-3-flash-preview` for audio understanding and transcription examples.
- Gemini accepts uploaded audio files through the Files API, or inline audio for requests under the documented 20 MB total request limit.
- Gemini supports `audio/wav`.
- Gemini docs explicitly keep real-time transcription out of this API path; use the existing post-stop flow.
- Source: https://ai.google.dev/gemini-api/docs/audio

OpenAI:

- Use `POST https://api.openai.com/v1/audio/transcriptions` with multipart form data and `Authorization: Bearer`.
- Supported transcription models include `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`, and `gpt-4o-transcribe-diarize`.
- File uploads are limited to 25 MB. Supported input types include `wav`.
- `whisper-1` supports `verbose_json`, `srt`, `vtt`, and timestamp granularities; `timestamp_granularities[]` is only supported for `whisper-1`.
- `gpt-4o-transcribe-diarize` is the OpenAI path for speaker-aware segments and requires `response_format=diarized_json` to receive speaker annotations.
- Source: https://developers.openai.com/api/docs/guides/speech-to-text
- API reference: https://developers.openai.com/api/reference/resources/audio

Deepgram:

- Use `POST https://api.deepgram.com/v1/listen` with `Authorization: Token <DEEPGRAM_API_KEY>`.
- For local WAV upload, send `Content-Type: audio/wav` and the file bytes.
- Use `model=nova-3&smart_format=true` by default. `smart_format` includes punctuation and paragraphs where supported.
- Use `diarize=true` for speaker labels and `utterances=true` when we need segment-level speaker/timestamp output.
- Validate a key with `GET https://api.deepgram.com/v1/auth/token`.
- Deepgram does not store transcripts; save the API response-derived transcript immediately.
- Source: https://developers.deepgram.com/docs/pre-recorded-audio
- Auth source: https://developers.deepgram.com/docs/authenticating
- Feature sources: https://developers.deepgram.com/docs/smart-format, https://developers.deepgram.com/docs/diarization, https://developers.deepgram.com/docs/utterances

## Product Decision

The app has one active transcription provider at a time. The selected provider is required for recording readiness, matching the existing "record then transcribe" product flow. Non-selected provider keys are optional and do not block recording.

Do not build compare mode in this phase. A session has one active transcript. Re-transcribing with a different provider replaces the active transcript metadata and writes a new provider-named transcript file.

## Settings Contract

Add provider-neutral settings while keeping provider-specific options explicit:

```json
{
  "transcriptionProvider": "gemini",
  "geminiModel": "gemini-3-flash-preview",
  "geminiFallbackModel": "gemini-2.5-flash",
  "openaiModel": "whisper-1",
  "openaiFallbackModel": "",
  "deepgramModel": "nova-3",
  "deepgramSmartFormat": true,
  "deepgramDiarize": true,
  "deepgramUtterances": true,
  "chunkMinutes": 15,
  "languageHint": "Romanian with possible English",
  "includeSpeakerLabels": true,
  "includeTimestamps": true
}
```

Cost settings should become provider-aware and optional:

```json
{
  "geminiInputCostPerMillionUsd": 1.0,
  "geminiOutputCostPerMillionUsd": 3.0,
  "openaiCostPerMinuteUsd": 0.006,
  "openaiInputCostPerMillionUsd": 0.0,
  "openaiOutputCostPerMillionUsd": 0.0,
  "deepgramCostPerHourUsd": 0.0
}
```

Only `openaiCostPerMinuteUsd` gets a non-zero default because `whisper-1` is duration-priced in OpenAI's model docs. Token-priced OpenAI models and Deepgram rates change by plan/model; leave those at `0.0` unless the user edits them.

## Secret Storage

Provider API keys must be stored by Rust only. The frontend may hold a key only while the user is typing before save.

Replace the Gemini-only secret API with provider-key commands:

- `save_transcription_key(provider, key) -> ProviderStatus`
- `has_transcription_key(provider) -> bool`
- `delete_transcription_key(provider) -> ()`
- `validate_transcription_key(provider, key?) -> ProviderStatus`

Use macOS Keychain accounts under service `com.aigentive.reefrecord`:

- `gemini_api_key`
- `openai_api_key`
- `deepgram_api_key`

Do not log keys. Do not write keys to `settings.json`, transcript files, metadata files, `.env`, request URLs, or error text.

## Backend Architecture

Add a provider-neutral transcription module:

```text
src-tauri/src/transcription/
  mod.rs
  types.rs
  prompt.rs
  chunking.rs
  providers/
    gemini.rs
    openai.rs
    deepgram.rs
```

Keep provider clients thin:

- Gemini may reuse `src-tauri/src/gemini/client.rs`.
- Add `src-tauri/src/openai/client.rs` only if separating HTTP details reads cleaner.
- Add `src-tauri/src/deepgram/client.rs` only if separating HTTP details reads cleaner.

Core types:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionProvider {
    Gemini,
    Openai,
    Deepgram,
}

pub struct TranscriptionJob {
    pub wav_path: PathBuf,
    pub provider: TranscriptionProvider,
    pub model: String,
    pub fallback_model: Option<String>,
    pub chunk_minutes: u32,
    pub language_hint: String,
    pub include_speaker_labels: bool,
    pub include_timestamps: bool,
}

pub struct TranscriptionOutcome {
    pub text: String,
    pub provider: TranscriptionProvider,
    pub model_used: String,
    pub usage: TranscriptionUsage,
}

pub enum TranscriptionUsage {
    Tokens {
        prompt_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
        audio_tokens: Option<u64>,
        text_tokens: Option<u64>,
    },
    Duration {
        seconds: f64,
    },
    Deepgram {
        request_id: Option<String>,
        duration_seconds: Option<f64>,
        confidence: Option<f64>,
    },
    Unknown,
}
```

The provider-neutral `transcribe(job, settings, secrets)` dispatches to exactly one provider and returns normalized text plus usage.

## Provider Implementation Details

Gemini:

- Preserve existing prompt-builder behavior.
- Preserve primary/fallback model behavior.
- Use inline `audio/wav` for chunks under 20 MB and Files API upload above that.
- Continue using `usageMetadata` for prompt/output/total token accounting.

OpenAI:

- Build a multipart request with `reqwest::multipart`.
- Endpoint: `https://api.openai.com/v1/audio/transcriptions`.
- Headers: `Authorization: Bearer <key>`.
- Form fields:
  - `file`: WAV bytes, filename, MIME `audio/wav`.
  - `model`: `settings.openaiModel`.
  - `prompt`: provider-neutral transcription prompt, shortened to context/punctuation guidance.
  - `response_format`: use `verbose_json` when model is `whisper-1` and timestamps are enabled; otherwise use `json`.
  - `timestamp_granularities[]=segment` when model is `whisper-1` and timestamps are enabled.
  - `response_format=diarized_json` and `chunking_strategy=auto` only when model is `gpt-4o-transcribe-diarize` and speaker labels are enabled.
- Enforce OpenAI's 25 MB upload limit in chunking. Use a 24 MB safety threshold, independent of `chunkMinutes`.
- If `includeSpeakerLabels` is true but model is `whisper-1`, show a provider capability warning in status/settings and produce timestamped text without speaker labels.
- Parse:
  - `json`: `text` plus optional `usage`.
  - `verbose_json`: render `segments` as `[MM:SS] text`.
  - `diarized_json`: render `segments` as `[MM:SS] [Speaker X]: text`.
- Validation: call `GET https://api.openai.com/v1/models` or retrieve the selected model with the bearer token. A 200 validates the key; missing selected model returns warning.

Deepgram:

- Endpoint: `https://api.deepgram.com/v1/listen`.
- Headers:
  - `Authorization: Token <key>`
  - `Content-Type: audio/wav`
- Query:
  - `model=settings.deepgramModel`
  - `smart_format=settings.deepgramSmartFormat`
  - `diarize=settings.includeSpeakerLabels && settings.deepgramDiarize`
  - `utterances=(settings.includeTimestamps || settings.includeSpeakerLabels) && settings.deepgramUtterances`
- Prefer response utterances when present, because they already carry speaker and timestamp boundaries.
- Fallback response path: `results.channels[0].alternatives[0].paragraphs.transcript`, then `results.channels[0].alternatives[0].transcript`.
- For words-only diarization fallback, group consecutive words by `speaker` and render speaker/timestamp lines.
- Validation: call `GET https://api.deepgram.com/v1/auth/token`.
- Do not chunk for upload size. Use a provider cap of 9 minutes per chunk for Nova/Base/Enhanced to reduce timeout risk; leave the existing `chunkMinutes` value as the user-facing maximum.

## Chunking Rules

Replace `split_wav_if_needed(wav_path, chunk_minutes)` with a provider-aware splitter:

```rust
pub struct ChunkPolicy {
    pub max_seconds: Option<u64>,
    pub max_bytes: Option<u64>,
    pub preserve_existing_short_buffer: bool,
}
```

Policies:

- Gemini: `max_seconds = chunkMinutes * 60`, `max_bytes = None`, inline/upload chosen later.
- OpenAI: `max_seconds = chunkMinutes * 60`, `max_bytes = 24 * 1024 * 1024`.
- Deepgram: `max_seconds = min(chunkMinutes, 9) * 60`, `max_bytes = None`.

Offsets must be passed into every provider renderer so timestamps remain global.

## Session Metadata

Replace Gemini-specific metadata with provider-neutral fields:

```json
{
  "transcriptionProvider": "openai",
  "transcriptionModel": "whisper-1",
  "transcriptionUsage": {
    "kind": "duration",
    "seconds": 183.2
  },
  "transcriptionCostUsd": 0.0183
}
```

Keep `transcriptionStatus`, `transcriptionError`, `transcriptPath`, and `transcriptPreview`.

Transcript file naming:

```text
session_YYYYMMDD_HHMMSS_gemini.txt
session_YYYYMMDD_HHMMSS_openai.txt
session_YYYYMMDD_HHMMSS_deepgram.txt
```

When a session is re-transcribed with a different provider, update `transcriptPath` to the new provider file. If the previous transcript path is inside the same sessions directory and starts with the same session id, remove it to avoid orphaned active transcripts.

## Tauri Command Changes

Settings commands:

- Remove Gemini-specific frontend usage.
- Add provider-key commands listed above.
- `get_app_status()` returns:

```ts
type AppStatus = {
  mic: ProviderStatus;
  systemAudio: ProviderStatus;
  transcription: ProviderStatus;
  providers: Record<TranscriptionProvider, ProviderStatus>;
  folder: ProviderStatus;
  github: ProviderStatus;
  git: ProviderStatus;
  gitLfs: ProviderStatus;
  canRecord: boolean;
  blockingReason?: string;
};
```

Session command:

```rust
pub async fn transcribe_session(
    state: State<'_, AppState>,
    session_id: String,
    provider: Option<TranscriptionProvider>,
) -> AppResult<SessionSummary>
```

If `provider` is `None`, use `settings.transcription_provider`.

## UI/UX Handoff

Setup rail:

- Rename the `Gemini` chip to `Parser`.
- The chip shows the selected provider state and detail:
  - `Gemini ready`
  - `OpenAI key missing`
  - `Deepgram validation failed`
- Clicking it opens a transcription setup panel.

Inline setup panel:

- Replace `GeminiKeyForm.tsx` with `TranscriptionProviderPanel.tsx`.
- Top control: segmented provider selector: `Gemini`, `OpenAI`, `Deepgram`.
- For the selected provider:
  - Key field with reveal/hide, save, validate, remove.
  - Model field/select.
  - Provider capability warning if the selected model cannot honor speaker labels or timestamps.
- Saving the provider selector updates `settings.transcriptionProvider`.

Settings sheet:

- Rename the `Gemini` section to `Transcription`.
- Show common controls once: chunk minutes, language hint, speaker labels, timestamps.
- Show provider-specific model and cost fields under tabs or segmented controls.
- Do not show all three API key values. Only show saved/missing/validated state.

Recorder panel:

- Change `Transcribing with Gemini...` to `Transcribing with {providerLabel}...`.
- The disabled reason should say `selected parser key`, not `Gemini key`.

Session list and transcript drawer:

- Show provider and model in the cost/usage line.
- Re-transcribe uses the current selected provider by default.
- Tooltip/title should say `Retranscribe with selected parser`.

## Exact File Handoff

Rust:

- `src-tauri/Cargo.toml`: add `keyring` if Keychain is not already available; no OpenAI or Deepgram SDK is needed because `reqwest` already has JSON and multipart.
- `src-tauri/src/lib.rs`: register provider-key commands; add provider validation cache to app state.
- `src-tauri/src/settings/mod.rs`: add settings contract fields and provider enum.
- `src-tauri/src/services/secrets.rs`: replace Gemini-only functions with provider-key functions.
- `src-tauri/src/commands/status_commands.rs`: compute selected transcription provider readiness.
- `src-tauri/src/commands/settings_commands.rs`: expose provider-key save/validate/delete.
- `src-tauri/src/commands/session_commands.rs`: dispatch to provider-neutral transcription service and write provider-named transcript file.
- `src-tauri/src/gemini/transcription.rs`: move generic prompt/chunking out or make this file Gemini-only.
- Add `src-tauri/src/transcription/**`, `src-tauri/src/openai/**`, and `src-tauri/src/deepgram/**` as needed.

Frontend:

- `frontend/src/api/types.ts`: add `TranscriptionProvider`, provider settings, provider statuses, provider-neutral usage.
- `frontend/src/api/bridge.ts`: add provider-key invoke wrappers; update `transcribeSession`.
- `frontend/src/App.tsx`: open parser panel when active provider is missing.
- `frontend/src/features/setup/SetupRail.tsx`: rename chip and use `status.transcription`.
- `frontend/src/features/setup/InlineSetup.tsx`: route parser panel.
- Replace `frontend/src/features/setup/GeminiKeyForm.tsx` with `TranscriptionProviderPanel.tsx`.
- `frontend/src/features/settings/SettingsSheet.tsx`: restructure transcription section.
- `frontend/src/features/recorder/RecorderPanel.tsx`: dynamic provider label.
- `frontend/src/features/sessions/SessionList.tsx`: dynamic provider/model/cost display.
- `frontend/src/features/sessions/TranscriptDrawer.tsx`: dynamic provider/model/cost display.
- `frontend/src/styles/global.css`: update class names only where needed; keep Teal Design System variables.

## Verification

Do not add new Rust or frontend unit-test harnesses in this phase unless explicitly requested.

Run:

```bash
cd src-tauri && cargo check
cd frontend && npm run typecheck
cd frontend && npm run build
./frontend/node_modules/.bin/tauri build --no-bundle
```

Manual QA:

- First launch with no provider keys: Parser chip is missing, record is blocked with selected parser key reason.
- Save and validate Gemini key; record/transcribe still works.
- Switch to OpenAI, save key, validate key, record a short WAV, transcript writes `_openai.txt`.
- OpenAI `whisper-1` with timestamps produces timestamped transcript and no speaker labels warning.
- Switch to Deepgram, save key, validate key, record a short WAV, transcript writes `_deepgram.txt`.
- Deepgram with diarization enabled renders speaker/timestamp lines when the response includes utterances or speaker-tagged words.
- Clear WAV still keeps transcript; delete session removes WAV, current transcript, and metadata.
- Git sync includes the active provider transcript and metadata.

## Acceptance Criteria

- User can choose Gemini, OpenAI, or Deepgram as the active audio parser.
- User can store, validate, replace, and remove API keys for all three providers without keys leaving Rust persistence.
- Recording readiness depends only on the selected parser's saved key plus existing mic/folder requirements.
- Existing WAV recording path is unchanged.
- Transcription writes one active transcript, records provider/model/usage metadata, and displays provider/model in the UI.
- OpenAI upload chunking never exceeds the 25 MB API limit.
- Deepgram uses `Authorization: Token`, never query-string credentials.
- No API key is logged, persisted in settings, persisted in metadata, or shown after save.
