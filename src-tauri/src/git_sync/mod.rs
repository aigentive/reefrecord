use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::services::sessions::SessionSummary;
use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncStatus {
    pub git_installed: bool,
    pub git_lfs_installed: bool,
    pub repo_url_valid: bool,
    pub target_folder_safe: bool,
    pub message: Option<String>,
}

pub fn validate(settings: &Settings) -> GitSyncStatus {
    let git_installed = which::which("git").is_ok();
    let git_lfs_installed = which::which("git-lfs").is_ok()
        || Command::new("git")
            .args(["lfs", "version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

    let repo_url_valid = !settings.github_repo_url.trim().is_empty();
    let target_folder_safe = is_safe_relative_path(&settings.github_target_folder);

    let mut msgs = Vec::<String>::new();
    if !git_installed {
        msgs.push("git is not installed or not on PATH".into());
    }
    if settings.git_lfs_enabled && !git_lfs_installed {
        msgs.push("git-lfs is not installed".into());
    }
    if settings.github_sync_enabled && !repo_url_valid {
        msgs.push("repository URL is empty".into());
    }
    if !target_folder_safe {
        msgs.push("target folder is not a safe relative path".into());
    }

    GitSyncStatus {
        git_installed,
        git_lfs_installed,
        repo_url_valid,
        target_folder_safe,
        message: if msgs.is_empty() { None } else { Some(msgs.join("; ")) },
    }
}

pub fn is_safe_relative_path(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return false;
    }
    if trimmed.contains("..") {
        return false;
    }
    if trimmed.contains(':') && !cfg!(windows) {
        return false;
    }
    true
}

pub enum SyncOutcome {
    Synced,
    Skipped,
}

pub fn push_session(settings: &Settings, session: &SessionSummary) -> AppResult<SyncOutcome> {
    if !settings.github_sync_enabled {
        return Err(AppError::Invalid("GitHub sync is disabled.".into()));
    }
    if settings.github_repo_url.trim().is_empty() {
        return Err(AppError::Invalid("GitHub repository URL is empty.".into()));
    }
    if !is_safe_relative_path(&settings.github_target_folder) {
        return Err(AppError::Invalid("GitHub target folder is not safe.".into()));
    }

    let validation = validate(settings);
    if !validation.git_installed {
        return Err(AppError::Git("git is not installed".into()));
    }
    if settings.git_lfs_enabled && !validation.git_lfs_installed {
        return Err(AppError::Git("git-lfs is not installed".into()));
    }

    let tmp = tempfile::Builder::new()
        .prefix("reefrecord_sync_")
        .tempdir()
        .map_err(|e| AppError::Git(format!("cannot create tempdir: {e}")))?;
    let tmp_path = tmp.path().to_path_buf();

    run_git(&tmp_path, &["clone", "--depth", "1", &settings.github_repo_url, "."], true)?;

    if settings.git_lfs_enabled {
        run_git(&tmp_path, &["lfs", "install"], false)?;
        run_git(&tmp_path, &["lfs", "track", "*.wav"], false)?;
    }

    let target = tmp_path.join(&settings.github_target_folder);
    std::fs::create_dir_all(&target)
        .map_err(|e| AppError::Git(format!("cannot create target folder: {e}")))?;

    let wav_src = Path::new(&session.wav_path);
    if wav_src.exists() {
        let dst = target.join(
            wav_src
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("session.wav")),
        );
        std::fs::copy(wav_src, &dst).map_err(|e| AppError::Git(format!("copy wav: {e}")))?;
    }
    if let Some(tp) = &session.transcript_path {
        let src = Path::new(tp);
        if src.exists() {
            let dst = target.join(src.file_name().unwrap_or_else(|| std::ffi::OsStr::new("transcript.txt")));
            std::fs::copy(src, &dst).map_err(|e| AppError::Git(format!("copy transcript: {e}")))?;
        }
    }
    let meta_path = session.metadata_path();
    if meta_path.exists() {
        let dst = target.join(meta_path.file_name().unwrap());
        std::fs::copy(&meta_path, &dst).map_err(|e| AppError::Git(format!("copy metadata: {e}")))?;
    }

    if settings.git_lfs_enabled {
        let gitattr = tmp_path.join(".gitattributes");
        if gitattr.exists() {
            run_git(&tmp_path, &["add", ".gitattributes"], false)?;
        }
    }
    run_git(&tmp_path, &["add", "."], false)?;

    let status = run_git_capture(&tmp_path, &["status", "--porcelain"], false)?;
    if status.stdout_text.trim().is_empty() {
        return Ok(SyncOutcome::Skipped);
    }

    ensure_commit_identity(&tmp_path)?;

    let commit_message = format!("session {}", session.id.trim_start_matches("session_"));
    run_git(&tmp_path, &["commit", "-m", &commit_message], false)?;
    run_git(&tmp_path, &["push"], true)?;

    Ok(SyncOutcome::Synced)
}

struct CaptureOutput {
    stdout_text: String,
}

fn run_git(cwd: &PathBuf, args: &[&str], allow_long: bool) -> AppResult<()> {
    let _ = run_git_capture(cwd, args, allow_long)?;
    Ok(())
}

fn run_git_capture(cwd: &PathBuf, args: &[&str], allow_long: bool) -> AppResult<CaptureOutput> {
    let mut cmd = Command::new("git");
    cmd.current_dir(cwd);
    cmd.args(args);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_CLONE_PROTECTION_ACTIVE", "false");
    let output = cmd
        .output()
        .map_err(|e| AppError::Git(format!("git {:?}: {e}", args)))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let cleaned = redact_url(&format!("{}{}", stderr, stdout));
        let short = if cleaned.len() > 400 {
            format!("{}…", &cleaned[..400])
        } else {
            cleaned
        };
        // Map common failures to clearer errors.
        let msg = if short.contains("could not read Username") || short.contains("Authentication failed") {
            "authentication failed — check git credentials".to_string()
        } else if short.contains("Repository not found") || short.contains("does not exist") {
            "repository not found or no access".to_string()
        } else if short.contains("Please tell me who you are") {
            "git user.name / user.email not configured".to_string()
        } else if short.contains("rejected") && short.contains("fetch first") {
            "push rejected — remote has new commits".to_string()
        } else if short.contains("Could not resolve host") {
            "network unavailable".to_string()
        } else {
            short
        };
        let _ = allow_long;
        return Err(AppError::Git(msg));
    }
    Ok(CaptureOutput {
        stdout_text: String::from_utf8_lossy(&output.stdout).to_string(),
    })
}

fn ensure_commit_identity(cwd: &PathBuf) -> AppResult<()> {
    // If repo/global identity is missing, set a safe local default so commit can proceed.
    let name = Command::new("git")
        .current_dir(cwd)
        .args(["config", "user.name"])
        .output();
    let email = Command::new("git")
        .current_dir(cwd)
        .args(["config", "user.email"])
        .output();
    let name_ok = matches!(&name, Ok(o) if o.status.success() && !o.stdout.is_empty());
    let email_ok = matches!(&email, Ok(o) if o.status.success() && !o.stdout.is_empty());
    if !name_ok {
        run_git(cwd, &["config", "user.name", "Reef Recorder"], false)?;
    }
    if !email_ok {
        run_git(cwd, &["config", "user.email", "reef-recorder@local"], false)?;
    }
    Ok(())
}

fn redact_url(text: &str) -> String {
    // Remove credentials from URLs of form https://user:token@host/...
    let re = regex_like_strip_credentials(text);
    re
}

fn regex_like_strip_credentials(text: &str) -> String {
    // Hand-rolled since we don't pull `regex` for a single pattern.
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find("://") {
        out.push_str(&rest[..idx + 3]);
        let after = &rest[idx + 3..];
        if let Some(at_rel) = after.find('@') {
            // ensure no whitespace before '@' (i.e. same token)
            let up_to_at = &after[..at_rel];
            let next_slash = up_to_at.find('/').unwrap_or(up_to_at.len());
            if next_slash > at_rel {
                out.push_str("***@");
                rest = &after[at_rel + 1..];
                continue;
            }
        }
        rest = after;
    }
    out.push_str(rest);
    out
}
