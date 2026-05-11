# Reef Recorder

Desktop app that records microphone (and optionally system audio via BlackHole), saves a WAV, transcribes with Gemini, and optionally syncs sessions to a GitHub repository — all configurable from a single window.

Built with [Tauri v2](https://tauri.app/), React + TypeScript, and Rust. No more `.env` editing or terminal.

## Features

- **Setup in the app** — paste your Gemini API key, pick a local sessions folder, enable GitHub sync. No `.env`.
- **Mic-only or mic + system audio** — system audio requires [BlackHole](https://github.com/ExistentialAudio/BlackHole) as a loopback input; without it you get mic-only.
- **Gemini transcription** — runs after the recording stops. Handles long recordings by splitting into chunks, tries a fallback model on failure.
- **GitHub sync** — uses your local `git` + `git-lfs` installs. Sessions go into a configured subfolder of the target repo.
- **Safe secret storage** — the Gemini API key lives in the macOS Keychain, never in plain files or logs.

## Prerequisites

- **macOS 13+** (system-audio capture is macOS-only in this MVP).
- **Node.js 20+** and **npm**.
- **Rust** (`rustup` toolchain). Stable channel.
- **Tauri v2 prerequisites** — follow [https://tauri.app/start/prerequisites/](https://tauri.app/start/prerequisites/).
- **git** and, if you enable WAV sync, **git-lfs**: `brew install git-lfs`.
- Optional: **BlackHole 2ch** — `brew install --cask blackhole-2ch` (reboot after install).

## Install

```bash
git clone <this-repo> reef-recorder
cd reef-recorder
cd frontend && npm install
```

## Run (dev)

```bash
cd frontend
npx tauri dev
```

The first launch opens the app window. The setup rail shows readiness chips for Mic, System Audio, Gemini, Folder, and GitHub. Missing required items auto-open their setup panel.

## Build (release)

```bash
cd /path/to/reef-recorder
./frontend/node_modules/.bin/tauri build --no-bundle
```

Produces an unsigned macOS binary at `src-tauri/target/release/reef-recorder`.

For a signed / notarized DMG, configure `tauri.conf.json` → `bundle.macOS.signingIdentity` and run `tauri build` without `--no-bundle`.

## Tests

```bash
cd src-tauri && cargo test
cd frontend && npm run build
cd frontend && npm run test:e2e
```

The real Gemini integration test is opt-in because it uses the saved app key
and makes a live API request:

```bash
npm run test:gemini
```

## Configuration

| Setting | Where | Default |
|---|---|---|
| Gemini API key | Setup rail → **Gemini** → Save key | keychain only |
| Sessions folder | Setup rail → **Folder** or Settings → Audio | prompted on first launch |
| Microphone device | Settings → Audio → Microphone selector | system default |
| System audio device | Settings → Audio → System audio selector | first BlackHole match |
| Gemini model | Settings → Gemini → Model | `gemini-3-flash-preview` |
| Fallback model | Settings → Gemini → Fallback model | `gemini-2.5-flash` |
| Chunk size | Settings → Gemini → Chunk minutes | 15 |
| Language hint | Settings → Gemini → Language hint | `Romanian with possible English` |
| GitHub sync | Setup rail → **GitHub** or Settings → GitHub sync | off |
| Repo URL | Settings → GitHub sync → Repository URL | — |
| Target folder | Settings → GitHub sync → Target folder | `sessions` |
| Use Git LFS | Settings → GitHub sync → Use Git LFS for WAV files | on |

Non-secret settings are stored in the OS app config dir:

```
~/Library/Application Support/com.aigentive.reefrecord/settings.json
```

## File layout

```
frontend/             Vite + React + TS (UI).
src-tauri/            Tauri v2 Rust backend.
.plans/               Phased product spec.
recorder.py           Python reference (retained for parity checks — not run).
CLAUDE.md             Dev guide (Claude Code).
AGENTS.md             Dev guide (cross-provider).
```

See `CLAUDE.md` for conventions, Rust `Send/Sync` gotchas, and how commands flow between the frontend bridge (`frontend/src/api/bridge.ts`) and the Rust command modules (`src-tauri/src/commands/`).

## Troubleshooting

- **"BlackHole not detected — recording will be mic-only"** — install it via `brew install --cask blackhole-2ch`, reboot, and set macOS output to a Multi-Output Device containing BlackHole + your speakers.
- **"BlackHole driver is installed but CoreAudio has not loaded it yet"** — reboot macOS.
- **Microphone chip stays "denied"** — macOS → System Settings → Privacy & Security → Microphone → enable for Reef Recorder.
- **GitHub sync fails with "authentication failed"** — your local `git` credentials aren't set up. Try an SSH remote (`git@github.com:...`) or configure the credential manager.
- **Gemini validation "warning"** — the key is stored but the `/models` call returned a non-success. The recording path will still try the primary model, then fall back.

## Spec

`.plans/00-product-spec.md` through `.plans/05-polish-release.md` contain the frozen product spec. Read them before contributing feature changes.

## License

Apache License 2.0. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
