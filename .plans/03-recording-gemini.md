# Phase 3: Recording And Gemini

## Objective

Port the current `recorder.py` behavior to Rust and produce local WAV plus transcript files.

## Current Behavior To Preserve

- Record microphone.
- Optionally record BlackHole/system audio.
- Mix mic and system streams into mono.
- Save `session_<timestamp>.wav`.
- Transcribe with Gemini.
- Save `session_<timestamp>_gemini.txt`.
- Split long recordings into chunks.
- Preserve Romanian and English as spoken.
- Include timestamps and speaker labels.

## Audio Implementation

Use Rust-native audio capture:

- `cpal` for input device enumeration and capture.
- `hound` for WAV writing.
- Add a resampling strategy if selected devices cannot capture at 16 kHz.
- Mix two streams with int16 clamping, matching current Python behavior.

Session naming:

```text
session_YYYYMMDD_HHMMSS.wav
session_YYYYMMDD_HHMMSS_gemini.txt
```

Session metadata:

```json
{
  "id": "session_20260424_120000",
  "startedAt": "2026-04-24T12:00:00Z",
  "durationSeconds": 423,
  "wavPath": "...",
  "transcriptPath": "...",
  "micDeviceName": "...",
  "systemDeviceName": "...",
  "transcriptionStatus": "complete",
  "syncStatus": "not_enabled"
}
```

## Recording State Machine

States:

- `idle`
- `starting`
- `recording`
- `stopping`
- `saving`
- `transcribing`
- `complete`
- `failed`

Events:

- elapsed time tick
- input device unavailable
- write failure
- transcription started
- transcription progress by chunk
- transcription complete
- transcription failed

## Gemini Implementation

Use Rust backend REST calls with the saved key.

Model strategy:

- Primary default: `gemini-3-flash-preview`, matching current Gemini audio docs.
- Fallback: `gemini-2.5-flash`, matching the existing Python script.
- Settings must allow user override.

Audio payload strategy:

- For small chunks, inline audio data can be sent to `generateContent`.
- For larger chunks or requests above the documented size threshold, use Gemini Files API resumable upload.

Prompt:

```text
Transcribe this audio verbatim. Do not summarize or paraphrase.
The conversation is primarily in Romanian but may include English words or sentences; preserve the original language as spoken.
Identify and label different speakers as [Speaker 1], [Speaker 2], etc.
Include timestamps in MM:SS format at each speaker change.
Output only the transcription, nothing else.
```

For chunked transcription, include an offset hint so timestamps remain aligned.

## UI Work

Recorder panel:

- Record/stop button.
- Elapsed timer.
- Input meters if feasible; otherwise show device names and capture status.
- Transcription progress after stop.

Session list:

- WAV saved.
- Transcript pending/complete/error.
- Reveal files.
- Retry transcription.

## Error Handling

User-facing errors:

- Microphone permission denied.
- Device disappeared.
- Could not create output folder.
- Disk write failed.
- Gemini key missing.
- Gemini validation failed.
- Gemini rate limit or transient unavailable.
- Gemini response had no transcript.

Retry policy:

- Retry transient Gemini 5xx responses with short backoff.
- Do not retry invalid key or permission errors.

## Tests

Rust:

- Audio mixing clamps correctly.
- Session filenames are stable.
- Chunk splitting preserves offsets.
- Gemini request payload generation.
- Gemini response parsing.
- Transcription fallback model behavior.

Frontend:

- Recording state transitions.
- Stop button disabled while saving.
- Retry transcription from failed state.

Manual QA:

- Mic-only recording.
- BlackHole recording when available.
- Long recording chunking with a small test chunk size.

## Acceptance Criteria

- App records a mic-only WAV and writes it to the selected folder.
- App records mic plus BlackHole when available.
- App transcribes saved audio with Gemini.
- App saves transcript next to WAV.
- User can recover from transcription failure without losing the WAV.

