# Reef Recorder — Development Guide

A Tauri v2 + React + TypeScript desktop app that records audio (mic + optional system via BlackHole), saves WAV, transcribes with Gemini, and optionally pushes sessions to a GitHub repo via local git + git-lfs.

## Repo Layout

```
frontend/            Vite + React + TS. Owns UI state, forms, setup flows.
  src/api/           Typed bridge to Rust (invoke + DTO types).
  src/features/      Feature-scoped panels (setup, recorder, sessions, settings).
  src/styles/        Global CSS with the orange minimal palette.
src-tauri/           Tauri v2 host (Rust).
  src/commands/      #[tauri::command] entry points (status, settings, session).
  src/services/      Device listing, permissions, secrets, recording, session store.
  src/audio/         cpal capture (threaded), resampling, WAV writing.
  src/gemini/        REST client + transcription pipeline.
  src/git_sync/      git + git-lfs subprocess sync.
  src/settings/      Non-secret JSON settings store (camelCase on disk).
  capabilities/      Tauri ACL (core, dialog, opener).
.plans/              Phased implementation spec. Authoritative.
.progress.md         Local tracker (gitignored).
```

## Run + Build

- `cd frontend && npm install` — install frontend deps
- `cd frontend && npm run typecheck` — type-check only (uses `tsc --noEmit`)
- `cd frontend && npm run build` — type-check + vite production build
- `cd src-tauri && cargo check` — fast compile check
- `cd src-tauri && /path/to/tauri build --no-bundle` — full release build (no installer)
- `cd frontend && npx tauri dev` — dev run (opens window, hot reload)

Important: `beforeBuildCommand` in `tauri.conf.json` is run from the **project root** (the directory containing `src-tauri/`), not from `src-tauri/`. Relative paths must be relative to the root.

## Conventions

### Rust

- All Tauri state types must be `Send + Sync + 'static`. `cpal::Stream` on macOS is `!Send` — own it on a dedicated worker thread and expose only a `JoinHandle` + `AtomicBool` stop flag to the outside. See `src-tauri/src/audio/capture.rs`.
- Errors: `AppError` with `thiserror`. `Serialize` implemented as `to_string()` so Tauri propagates a clean string to JS. Never leak credentials or API keys in error messages.
- Settings: single `SettingsStore` (sync `RwLock`). All mutation via `update(SettingsInput)` which persists atomically via temp-file-rename. Serde config is `#[serde(rename_all = "camelCase")]` to match the JS bridge.
- Audio samples are i16 mono @ 16 kHz end-to-end. Resample only when the device sample rate differs (see `audio/resample.rs`).
- Logging: `tracing::info!/warn!/error!`. Never log Gemini keys or credentialed git URLs. `git_sync::redact_url` strips `user:token@` segments.

### React / TypeScript

- No global state libs in MVP. Local `useState` in `App.tsx` owns settings, status, sessions. Features receive props and report changes via callbacks.
- `noEmit` is on in `tsconfig.json`; never commit emitted `.js`/`.d.ts` from `src/`.
- API bridge: every `invoke` is wrapped in `src/api/bridge.ts`. Types live in `src/api/types.ts` and match Rust DTOs exactly (camelCase).
- Readiness states: `ready | missing | denied | warning | checking | optional`. Warning is non-blocking; missing/denied block recording.
- Lucide icons only. No emoji.

### UI

- Orange accent (`--accent: #F97316`) is the primary-action color. Never paint whole surfaces in orange.
- Panels are flat cards; no nested cards. Border radius max 8px.
- Status changes must also be reflected in text, not color alone (a11y).
- Record/stop button is a fixed 112px circle. Timer never shifts layout.

## Secrets

- Gemini API key is stored only in the macOS Keychain via the `keyring` crate under service `com.aigentive.reefrecord`, account `gemini_api_key`. It is never written to `settings.json`, logs, or transcript files.
- No `.env` is read or written by the app.
- If a future `github_token` is added, use the same pattern.

## Sessions

- Session files live in the user-chosen `sessionsDir`. File names: `session_YYYYMMDD_HHMMSS.wav`, `session_YYYYMMDD_HHMMSS_gemini.txt`, `session_YYYYMMDD_HHMMSS.json` (metadata).
- `SessionSummary.metadata_path()` derives the JSON path from the WAV path.
- Metadata is written atomically (`.json.tmp` rename).

## Gemini

- Primary model (default): `gemini-3-flash-preview`. Fallback: `gemini-2.5-flash`. Configurable in settings.
- Inline audio for chunks under 20 MB; Files API (resumable upload) for larger ones.
- Retry transient 5xx/429 with backoff up to 3 attempts. Do not retry invalid key (4xx auth) errors.
- Prompt enforces speaker labels `[Speaker 1]`, timestamps `MM:SS`, and language preservation (Romanian + English).
- Chunk splitting: only split if duration exceeds `chunkMinutes + 60s` buffer, matching `recorder.py` behavior.

## Git Sync

- Shell-outs to local `git` and `git-lfs`. No OAuth/REST writes.
- Safe target folder = non-empty, no leading `/`, no `..`.
- Always `git clone --depth 1` to a tempdir, copy session files, `git add`, commit with message `session YYYYMMDD_HHMMSS`, `git push`.
- If `git status --porcelain` is empty after add → `SyncOutcome::Skipped`.
- If `user.name` / `user.email` are unset, locally configure fallback identity (Reef Recorder / reef-recorder@local).

## Testing Policy

- Per user instruction: **UI/UX Playwright or manual QA only**. No Rust/Vitest/Jest unit tests in this project.
- Verification strategy: `cargo check`, `npm run typecheck`, `tauri build --no-bundle`, then run the app and exercise flows in the window.

## Gotchas

- `frontend/tsconfig.json` must keep `"noEmit": true` — `tsc -b` otherwise scatters `.js`/`.d.ts` into `src/`.
- The root `.gitignore` rule for `/sessions/` is anchored — don't re-broaden it to `sessions/`, which would ignore `frontend/src/features/sessions/`.
- Tauri CLI v2.10.x looks for `tauri.conf.json` by walking parent dirs; run from `src-tauri/` or `/Users/.../deepgram/` with the CLI binary path.
- The old `recorder.py` + `.env` + `keys` files stay on disk for parity reference only. Do not import from them.

## Progress Tracking

`.progress.md` (gitignored) is the running checklist. `.plans/00-05*.md` are the immutable spec.
