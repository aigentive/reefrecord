# Phase 1: Tauri React Shell

## Objective

Create the app scaffold, baseline architecture, and first usable UI shell.

## Inputs

- Official Tauri v2 create-project docs.
- Official Vite docs.
- `../ralphx` layout pattern: `frontend/` plus `src-tauri/`.

## Tasks

1. Scaffold app:
   - Use Tauri v2.
   - Use React + TypeScript.
   - Use Vite.
   - Keep existing `recorder.py` as a behavior reference during the port.
2. Add core frontend dependencies:
   - `@tauri-apps/api`
   - `lucide-react`
   - `zod`
   - optional `@tanstack/react-query` if command state becomes repetitive
3. Add Rust dependencies:
   - `tauri`
   - `serde`, `serde_json`
   - `tokio`
   - `thiserror`
   - `dirs`
   - `tracing`, `tracing-subscriber`
4. Configure Tauri:
   - `productName`: `Reef Recorder`
   - `identifier`: `com.aigentive.reefrecord`
   - frontend dev command and dist path
   - one main window
5. Configure capabilities:
   - `core:default`
   - `dialog:default`
   - `opener:default`
   - no broad `shell` permission in MVP
   - no broad frontend `fs` permission in MVP
6. Create Rust command registry:
   - `commands/mod.rs`
   - `commands/status_commands.rs`
   - `commands/settings_commands.rs`
   - `commands/session_commands.rs`
7. Create frontend app shell:
   - `App.tsx`
   - `features/setup/SetupRail.tsx`
   - `features/recorder/RecorderPanel.tsx`
   - `features/sessions/SessionList.tsx`
   - `features/settings/SettingsSheet.tsx`

## First UI State

The first screen should render:

- App title and status.
- Setup chips.
- Disabled record button when required setup is missing.
- Empty session list.
- Settings button.

## Design Tasks

- Add global CSS variables for the orange minimal palette.
- Add focus-visible styles for keyboard users.
- Add compact button, icon button, input, toggle, chip, panel, and sheet primitives.
- Use 8px max border radius for panels and controls.

## Commands Stubbed In This Phase

- `get_app_status`
- `get_settings`
- `save_settings`

These can return placeholder values until later phases fill them.

## Tests

- Frontend renders shell.
- Setup chips render required/missing states.
- Rust command serialization test for `AppStatus` and `Settings`.

## Acceptance Criteria

- `npm run tauri dev` opens the app window.
- The UI shell is polished enough to judge layout and visual direction.
- Settings can be opened and closed.
- No recording, Gemini, or GitHub sync is required in this phase.

