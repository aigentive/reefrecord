# Phase 4: GitHub Sync

## Objective

Sync completed sessions to a configured GitHub repository with clear status and retry behavior.

## MVP Sync Decision

Use local `git` and `git-lfs` commands from Rust, not GitHub OAuth or REST writes.

Why:

- Matches current `recorder.py` behavior.
- Works with SSH remotes and existing credential managers.
- Avoids registering an OAuth app before the product shape is settled.
- Keeps the app small.

OAuth/device flow can be added later for a guided "Connect GitHub" button. GitHub docs support device flow, but it still requires an OAuth app client ID and polling rules.

## Settings

- `githubSyncEnabled: boolean`
- `githubRepoUrl: string`
- `githubTargetFolder: string`
- `gitLfsEnabled: boolean`

## Required Checks

- `git` is installed.
- If `gitLfsEnabled`, `git lfs` is installed.
- Repo URL is not empty when sync is enabled.
- Target folder is relative and safe.
- Session files exist before sync.

## Sync Algorithm

1. Create temp directory.
2. Clone repo with depth 1.
3. Run `git lfs install` if enabled.
4. Run `git lfs track "*.wav"` if enabled.
5. Create target folder.
6. Copy WAV, transcript, and metadata JSON.
7. Add `.gitattributes` if LFS is enabled.
8. Add session files.
9. If no changes, mark `skipped`.
10. Commit with message `session YYYYMMDD_HHMMSS`.
11. Push.
12. Remove temp directory.

## Rust Command Surface

- `validate_git_sync_settings() -> GitSyncStatus`
- `sync_session(session_id: SessionId) -> SyncResult`
- `retry_failed_sync(session_id: SessionId) -> SyncResult`

## UI

Settings:

- Toggle: Enable GitHub Sync.
- Input: Repository URL.
- Input: Target folder.
- Toggle: Use Git LFS for WAV files.
- Button: Test Sync.

Session row:

- `Not enabled`
- `Queued`
- `Syncing`
- `Synced`
- `Skipped`
- `Failed`

Actions:

- Retry sync.
- Open repo URL.
- Copy error details.

## Error Handling

Make these clear:

- Git is missing.
- Git LFS is missing.
- Clone failed.
- Authentication failed.
- No write access.
- Commit failed due missing git user config.
- Push rejected.
- Network unavailable.

Do not log secrets or full credentialed URLs.

## Tests

Rust:

- Safe target-folder validation.
- Command construction does not expose secrets.
- Sync no-op when files already exist.
- Failure mapping from process status to user message.

Integration:

- Local bare git repo test.
- Git LFS disabled path.

Manual QA:

- SSH repo sync.
- HTTPS repo with existing credential manager.
- Push failure shows retryable error.

## Acceptance Criteria

- User can enable sync from settings.
- App validates local tooling before first sync.
- Completed sessions can be pushed to a GitHub repository.
- Failed sync does not affect local session files.
- Failed sync can be retried.

