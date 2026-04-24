# AGENTS.md

This file is the cross-provider development guide for Reef Recorder. It mirrors `CLAUDE.md` for agents that follow the OpenAI Codex / AGENTS.md convention.

See [CLAUDE.md](./CLAUDE.md) for the full guide. The rules below are the minimum an agent must follow before writing code here.

## Non-Negotiables

1. **Do not commit emitted JS/DTS from `frontend/src/`.** `tsconfig.json` has `"noEmit": true`; keep it that way. Build script is `tsc --noEmit && vite build`.
2. **Do not log or persist the Gemini API key.** It lives in the macOS Keychain only.
3. **Do not broaden `.gitignore` line `/sessions/`.** That pattern is intentionally anchored so it does not ignore `frontend/src/features/sessions/`.
4. **Tauri `State<T>` requires `T: Send + Sync + 'static`.** On macOS, `cpal::Stream` is `!Send` — own streams on a dedicated thread (`audio/capture.rs` is the reference pattern).
5. **No backwards-compat shims.** This is a greenfield project; delete code rather than deprecate it.
6. **Testing:** UI/UX Playwright or manual QA only. Do not add Rust or frontend unit-test harnesses unless the user explicitly asks.

## Commands

```bash
# Frontend
cd frontend && npm install
cd frontend && npm run typecheck
cd frontend && npm run build
cd frontend && npx tauri dev          # run the desktop app in dev mode

# Rust
cd src-tauri && cargo check
cd src-tauri && cargo build

# Full release binary (no installer)
cd /path/to/deepgram && ./frontend/node_modules/.bin/tauri build --no-bundle
```

## Running the app headlessly from agents

Do not attempt `tauri dev` from an automation step — it's interactive and long-lived. Use `cargo check` + `npm run build` + `tauri build --no-bundle` for CI-style verification.

## Editing rules

- **Rust:** prefer `?` over `unwrap()`. Use `AppResult<T>`. Use `#[tauri::command]` on `async fn` that take `State<'_, AppState>`. Keep command bodies thin; push logic into `services/` or `gemini/` or `git_sync/`.
- **TypeScript:** strict mode. `type` aliases over `interface` for DTOs. Props objects typed inline. Never use `any`. Use `noUncheckedIndexedAccess`.
- **CSS:** global variables in `frontend/src/styles/global.css`. Never hardcode hex colors outside that file. Palette + type come from the Teal Design System (teal-500 accent, paper-0 canvas, Inter + JetBrains Mono + Instrument Serif).

## Layout quick-ref

| Concern | File |
|---|---|
| App shell | `frontend/src/App.tsx` |
| Setup chips | `frontend/src/features/setup/SetupRail.tsx` |
| Inline setup panels | `frontend/src/features/setup/*.tsx` |
| Record button / phase machine (client) | `frontend/src/features/recorder/RecorderPanel.tsx` |
| Session list & transcript | `frontend/src/features/sessions/*.tsx` |
| Settings drawer | `frontend/src/features/settings/SettingsSheet.tsx` |
| Tauri bridge | `frontend/src/api/bridge.ts`, `frontend/src/api/types.ts` |
| Tauri entry | `src-tauri/src/lib.rs` |
| Commands | `src-tauri/src/commands/` |
| Settings store | `src-tauri/src/settings/mod.rs` |
| Keychain secrets | `src-tauri/src/services/secrets.rs` |
| Audio capture | `src-tauri/src/audio/capture.rs` |
| WAV writer + mix | `src-tauri/src/audio/writer.rs` |
| Resampler | `src-tauri/src/audio/resample.rs` |
| Gemini REST client | `src-tauri/src/gemini/client.rs` |
| Transcription pipeline | `src-tauri/src/gemini/transcription.rs` |
| Git sync | `src-tauri/src/git_sync/mod.rs` |
| Device enumeration | `src-tauri/src/services/devices.rs` |
| Session store (JSON metadata) | `src-tauri/src/services/sessions.rs` |
| Recording service | `src-tauri/src/services/recording.rs` |

## Spec

`.plans/00-05*.md` are the frozen product spec. Read them before making feature changes.
