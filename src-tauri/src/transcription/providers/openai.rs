use std::path::PathBuf;
use std::time::Duration;

use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::transcription::chunking::{split_wav_with_policy, wav_duration_seconds, ChunkPolicy};
use crate::transcription::prompt::{build_openai_prompt, format_timestamp};
use crate::transcription::types::{
    TranscriptionOutcome, TranscriptionProvider, TranscriptionUsage,
};

const OPENAI_UPLOAD_LIMIT_BYTES: u64 = 24 * 1024 * 1024;

pub async fn validate_key(key: &str, settings: &Settings) -> AppResult<String> {
    let client = http_client()?;
    let resp = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::Http(format!(
            "OpenAI validation failed ({status}): {}",
            truncate(&text, 200)
        )));
    }
    let parsed: ModelsResponse =
        serde_json::from_str(&text).unwrap_or(ModelsResponse { data: Vec::new() });
    let found = parsed.data.iter().any(|m| m.id == settings.openai_model);
    if found {
        Ok(format!(
            "OpenAI key valid. Model {} available.",
            settings.openai_model
        ))
    } else {
        Err(AppError::Invalid(format!(
            "OpenAI key valid, but model {} was not returned by /v1/models.",
            settings.openai_model
        )))
    }
}

pub async fn transcribe(
    key: String,
    settings: &Settings,
    wav_path: PathBuf,
) -> AppResult<TranscriptionOutcome> {
    let client = http_client()?;
    let chunk_seconds = settings.chunk_minutes.max(1) as u64 * 60;
    let chunks = split_wav_with_policy(
        &wav_path,
        ChunkPolicy {
            max_seconds: Some(chunk_seconds),
            max_bytes: Some(OPENAI_UPLOAD_LIMIT_BYTES),
            preserve_existing_short_buffer: true,
        },
    )?;
    tracing::info!(
        wav = %wav_path.display(),
        chunks = chunks.len(),
        provider = "openai",
        "starting transcription"
    );

    let mut collected_text = Vec::<String>::new();
    let mut total_usage = TranscriptionUsage::Unknown;
    let mut model_used = settings.openai_model.clone();
    for (idx, (chunk_path, offset_s)) in chunks.iter().enumerate() {
        tracing::info!(
            chunk = idx + 1,
            total = chunks.len(),
            offset_s,
            provider = "openai",
            "transcribing chunk"
        );
        let response =
            transcribe_chunk(&client, &key, settings, chunk_path.clone(), *offset_s).await?;
        model_used = settings.openai_model.clone();
        collected_text.push(response.text);
        total_usage = total_usage.add(response.usage);
        if chunk_path != &wav_path {
            let _ = std::fs::remove_file(chunk_path);
        }
    }

    if collected_text.is_empty() {
        return Err(AppError::Http("OpenAI returned no transcript.".into()));
    }
    Ok(TranscriptionOutcome {
        text: collected_text.join("\n\n"),
        provider: TranscriptionProvider::Openai,
        model_used,
        usage: total_usage,
    })
}

async fn transcribe_chunk(
    client: &Client,
    key: &str,
    settings: &Settings,
    chunk_path: PathBuf,
    offset_seconds: u64,
) -> AppResult<ChunkOutcome> {
    let bytes = tokio::fs::read(&chunk_path).await?;
    let file_name = chunk_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();
    let file_part = Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("audio/wav")
        .map_err(|e| AppError::Http(format!("OpenAI multipart MIME error: {e}")))?;
    let mut form = Form::new()
        .part("file", file_part)
        .text("model", settings.openai_model.clone())
        .text("prompt", build_openai_prompt(&settings.language_hint));

    let diarized =
        settings.openai_model == "gpt-4o-transcribe-diarize" && settings.include_speaker_labels;
    let verbose_whisper = settings.openai_model == "whisper-1" && settings.include_timestamps;
    if diarized {
        form = form
            .text("response_format", "diarized_json")
            .text("chunking_strategy", "auto");
    } else if verbose_whisper {
        form = form
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "segment");
    } else {
        form = form.text("response_format", "json");
    }

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::Http(format!(
            "OpenAI transcription failed ({status}): {}",
            truncate(&text, 300)
        )));
    }

    let parsed: OpenAiTranscriptionResponse = serde_json::from_str(&text).map_err(|e| {
        AppError::Http(format!(
            "OpenAI returned invalid JSON: {e}: {}",
            truncate(&text, 200)
        ))
    })?;
    let duration = wav_duration_seconds(&chunk_path).unwrap_or(0.0);
    let rendered = render_response(parsed, offset_seconds as f64);
    Ok(ChunkOutcome {
        text: rendered.text,
        usage: rendered
            .usage
            .unwrap_or(TranscriptionUsage::Duration { seconds: duration }),
    })
}

fn render_response(response: OpenAiTranscriptionResponse, offset_seconds: f64) -> Rendered {
    let usage = response.usage.and_then(|u| u.into_usage());
    if let Some(segments) = response.segments {
        let lines = segments
            .into_iter()
            .filter_map(|s| {
                let text = s.text.trim();
                if text.is_empty() {
                    return None;
                }
                let ts = format_timestamp(offset_seconds + s.start.unwrap_or(0.0));
                let speaker = s.speaker.or(s.speaker_label);
                Some(match speaker {
                    Some(speaker) => format!("[{ts}] [{speaker}]: {text}"),
                    None => format!("[{ts}] {text}"),
                })
            })
            .collect::<Vec<_>>();
        if !lines.is_empty() {
            return Rendered {
                text: lines.join("\n"),
                usage,
            };
        }
    }
    Rendered {
        text: response.text.unwrap_or_default().trim().to_string(),
        usage,
    }
}

fn http_client() -> AppResult<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(600))
        .build()?)
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}...", &s[..n])
    }
}

struct ChunkOutcome {
    text: String,
    usage: TranscriptionUsage,
}

struct Rendered {
    text: String,
    usage: Option<TranscriptionUsage>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiTranscriptionResponse {
    text: Option<String>,
    segments: Option<Vec<OpenAiSegment>>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiSegment {
    text: String,
    #[serde(default)]
    start: Option<f64>,
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default, rename = "speaker_label")]
    speaker_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    seconds: Option<f64>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default, rename = "input_tokens")]
    input_tokens: Option<u64>,
    #[serde(default, rename = "output_tokens")]
    output_tokens: Option<u64>,
    #[serde(default, rename = "total_tokens")]
    total_tokens: Option<u64>,
    #[serde(default, rename = "audio_tokens")]
    audio_tokens: Option<u64>,
    #[serde(default, rename = "text_tokens")]
    text_tokens: Option<u64>,
}

impl OpenAiUsage {
    fn into_usage(self) -> Option<TranscriptionUsage> {
        if self.input_tokens.is_some()
            || self.output_tokens.is_some()
            || self.total_tokens.is_some()
        {
            let input = self.input_tokens.unwrap_or(0);
            let output = self.output_tokens.unwrap_or(0);
            return Some(TranscriptionUsage::Tokens {
                prompt_tokens: input,
                output_tokens: output,
                total_tokens: self.total_tokens.unwrap_or(input + output),
                audio_tokens: self.audio_tokens,
                text_tokens: self.text_tokens,
            });
        }
        self.seconds
            .or(self.duration)
            .map(|seconds| TranscriptionUsage::Duration { seconds })
    }
}
