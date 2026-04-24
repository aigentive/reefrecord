# Phase 5: Polish And Release

## Objective

Make the MVP feel finished: clean UX, resilient states, tests, and a local build.

## UX Polish

Recorder:

- Stable record/stop button size.
- Elapsed timer never shifts layout.
- Clear disabled reasons.
- Inline progress for transcription and sync.

Setup:

- First-run setup opens only missing required sections.
- Optional GitHub sync stays collapsed by default.
- Warnings are actionable.

Sessions:

- Latest session is visually selected after completion.
- Transcript drawer supports copy and reveal file.
- Failed transcription and failed sync are visibly retryable.

Settings:

- Dirty state.
- Save success.
- Validation messages near fields.
- Destructive actions use confirmation.

## Visual QA Checklist

- 1440x900 desktop.
- 1280x720 desktop.
- Narrow 900px width.
- Light theme only for MVP.
- Text does not overflow buttons or chips.
- Orange accent is used for primary action and focus, not every surface.
- Error/warning/success colors are distinct from orange.

## Accessibility

- Keyboard reachable setup chips.
- Visible focus states.
- Buttons have accessible names.
- Status changes are reflected in text, not color only.
- Record/stop has clear state labels.

## Logging

Add Rust logging:

- app startup
- settings load/save
- device detection
- recording start/stop
- transcription start/finish/failure
- sync start/finish/failure

Rules:

- Never log Gemini API keys.
- Never log credentialed remote URLs.
- Keep user-facing error IDs in logs for support.

## Packaging

Minimum:

- `npm run build`
- `npm run tauri build`
- unsigned macOS local build

Later:

- App icon set.
- Signed and notarized macOS build.
- Updater.

## Test Matrix

Automated:

- Rust unit tests.
- Frontend unit tests.
- Typecheck.
- Production build.

Manual:

- First launch with no settings.
- Save Gemini key.
- Select sessions folder.
- Open microphone settings.
- Mic-only recording.
- BlackHole warning when missing.
- BlackHole recording when available.
- Gemini transcription.
- GitHub sync disabled.
- GitHub sync enabled.
- GitHub sync failure and retry.

## Definition Of Done

- App can be built locally.
- A clean checkout can run the desktop app after dependency install.
- Required setup is possible from the UI.
- A recording produces WAV and transcript files.
- GitHub sync works when configured.
- Failure states are understandable and retryable.
- README is updated with app setup and dev commands.

