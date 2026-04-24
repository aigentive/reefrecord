# Deepgram Tauri App Plan

Last checked against official docs: 2026-04-24.

This folder defines the minimal phased plan for turning the current Python audio recorder into a small Rust + Tauri + React desktop app with a progressive setup flow, polished orange minimal UI, local session storage, Gemini transcription, and GitHub sync.

## MVP Outcome

The app opens directly to a recorder dashboard, shows setup readiness at a glance, helps the user configure missing pieces, records microphone plus optional BlackHole system audio, transcribes the saved WAV with Gemini, stores files in the selected local folder, and optionally syncs each completed session to a configured GitHub repository.

## Phase Files

- [00-product-spec.md](00-product-spec.md): user goals, MVP scope, non-goals, architecture, data model, design system.
- [01-tauri-react-shell.md](01-tauri-react-shell.md): scaffold, dependencies, project layout, commands, baseline UI.
- [02-settings-permissions.md](02-settings-permissions.md): settings, secrets, permissions, folder picker, device readiness.
- [03-recording-gemini.md](03-recording-gemini.md): Rust audio capture, WAV output, Gemini transcription pipeline.
- [04-github-sync.md](04-github-sync.md): Git repo configuration, commit/push workflow, sync status.
- [05-polish-release.md](05-polish-release.md): UX polish, tests, packaging, release readiness.

## Reference Inputs

Current repo behavior:

- `recorder.py` records mic plus optional BlackHole system audio.
- `GEMINI_API_KEY`, `GH_SESSIONS_REPO`, `GH_SESSIONS_FOLDER`, `SYSTEM_AUDIO_DEVICE`, and `MIC_DEVICE` are configured through `.env`.
- Sessions are saved under `sessions/` as `.wav` plus `_gemini.txt`.
- GitHub sync clones, copies files, tracks WAV files with Git LFS, commits, and pushes.

RalphX patterns worth borrowing, not copying wholesale:

- `frontend/` plus `src-tauri/` separation.
- Tauri v2 `src-tauri/capabilities/default.json` for explicit plugin permissions.
- Rust command modules under `src-tauri/src/commands`.
- Thin frontend API wrappers around Tauri commands.
- Tests around command/service behavior.

## Official Docs Checked

- Tauri create project: https://v2.tauri.app/start/create-project/
- Tauri capabilities: https://v2.tauri.app/reference/acl/capability/
- Tauri dialog plugin: https://v2.tauri.app/plugin/dialog/
- Tauri opener plugin: https://v2.tauri.app/plugin/opener/
- Tauri filesystem guidance: https://v2.tauri.app/plugin/file-system/
- Tauri store plugin: https://v2.tauri.app/plugin/store/
- Tauri stronghold plugin: https://v2.tauri.app/plugin/stronghold/
- Vite guide: https://vite.dev/guide/
- React app guidance: https://react.dev/learn/creating-a-react-app
- Gemini audio docs: https://ai.google.dev/gemini-api/docs/audio
- Gemini text generation docs: https://ai.google.dev/gemini-api/docs/text-generation
- GitHub OAuth/device flow docs: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps

## MVP Decisions

- Build macOS-first because the existing system-audio workflow depends on BlackHole and macOS Audio MIDI Setup.
- Port recorder behavior to Rust instead of shipping a Python runtime.
- Keep the first release single-window and local-first.
- Use Git CLI for sync in MVP, relying on the user's existing SSH key or credential manager.
- Do not implement GitHub OAuth in MVP; keep that as a later convenience phase.
- Keep secrets out of `.env` and non-secret settings files.

