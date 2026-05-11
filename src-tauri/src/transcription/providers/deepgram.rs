use std::path::PathBuf;
use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::transcription::chunking::{split_wav_with_policy, ChunkPolicy};
use crate::transcription::prompt::format_timestamp;
use crate::transcription::types::{
    TranscriptionOutcome, TranscriptionProvider, TranscriptionUsage,
};

pub async fn validate_key(key: &str, _settings: &Settings) -> AppResult<String> {
    let client = http_client()?;
    let resp = client
        .get("https://api.deepgram.com/v1/auth/token")
        .header("Authorization", format!("Token {key}"))
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::Http(format!(
            "Deepgram validation failed ({status}): {}",
            truncate(&text, 200)
        )));
    }
    Ok("Deepgram key valid.".to_string())
}

pub async fn transcribe(
    key: String,
    settings: &Settings,
    wav_path: PathBuf,
) -> AppResult<TranscriptionOutcome> {
    let client = http_client()?;
    let chunk_minutes = settings.chunk_minutes.min(9).max(1) as u64;
    let chunks = split_wav_with_policy(
        &wav_path,
        ChunkPolicy {
            max_seconds: Some(chunk_minutes * 60),
            max_bytes: None,
            preserve_existing_short_buffer: false,
        },
    )?;
    tracing::info!(
        wav = %wav_path.display(),
        chunks = chunks.len(),
        provider = "deepgram",
        "starting transcription"
    );

    let mut collected_text = Vec::<String>::new();
    let mut total_usage = TranscriptionUsage::Unknown;
    for (idx, (chunk_path, offset_s)) in chunks.iter().enumerate() {
        tracing::info!(
            chunk = idx + 1,
            total = chunks.len(),
            offset_s,
            provider = "deepgram",
            "transcribing chunk"
        );
        let response =
            transcribe_chunk(&client, &key, settings, chunk_path.clone(), *offset_s).await?;
        collected_text.push(response.text);
        total_usage = total_usage.add(response.usage);
        if chunk_path != &wav_path {
            let _ = std::fs::remove_file(chunk_path);
        }
    }

    if collected_text.is_empty() {
        return Err(AppError::Http("Deepgram returned no transcript.".into()));
    }
    Ok(TranscriptionOutcome {
        text: collected_text.join("\n\n"),
        provider: TranscriptionProvider::Deepgram,
        model_used: settings.deepgram_model.clone(),
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
    let mut req = client
        .post("https://api.deepgram.com/v1/listen")
        .header("Authorization", format!("Token {key}"))
        .header("Content-Type", "audio/wav")
        .query(&[
            ("model", settings.deepgram_model.as_str()),
            (
                "smart_format",
                if settings.deepgram_smart_format {
                    "true"
                } else {
                    "false"
                },
            ),
            (
                "diarize",
                if settings.include_speaker_labels && settings.deepgram_diarize {
                    "true"
                } else {
                    "false"
                },
            ),
            (
                "utterances",
                if (settings.include_timestamps || settings.include_speaker_labels)
                    && settings.deepgram_utterances
                {
                    "true"
                } else {
                    "false"
                },
            ),
        ]);
    if !settings.language_hint.trim().is_empty() {
        if let Some(language) = deepgram_language_hint(&settings.language_hint) {
            req = req.query(&[("language", language)]);
        }
    }
    let resp = req.body(bytes).send().await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::Http(format!(
            "Deepgram transcription failed ({status}): {}",
            truncate(&text, 300)
        )));
    }
    let parsed: DeepgramResponse = serde_json::from_str(&text).map_err(|e| {
        AppError::Http(format!(
            "Deepgram returned invalid JSON: {e}: {}",
            truncate(&text, 200)
        ))
    })?;
    let rendered = render_response(&parsed, offset_seconds as f64, settings);
    Ok(ChunkOutcome {
        text: rendered,
        usage: TranscriptionUsage::Deepgram {
            request_id: parsed.metadata.as_ref().and_then(|m| m.request_id.clone()),
            duration_seconds: parsed.metadata.as_ref().and_then(|m| m.duration),
            confidence: parsed
                .results
                .as_ref()
                .and_then(|r| r.channels.first())
                .and_then(|c| c.alternatives.first())
                .and_then(|a| a.confidence),
        },
    })
}

fn render_response(parsed: &DeepgramResponse, offset_seconds: f64, settings: &Settings) -> String {
    if let Some(utterances) = &parsed.results.as_ref().and_then(|r| r.utterances.as_ref()) {
        let lines = utterances
            .iter()
            .filter_map(|u| {
                let text = u.transcript.trim();
                if text.is_empty() {
                    return None;
                }
                let mut prefix = String::new();
                if settings.include_timestamps {
                    prefix.push_str(&format!(
                        "[{}] ",
                        format_timestamp(offset_seconds + u.start)
                    ));
                }
                if settings.include_speaker_labels {
                    if let Some(speaker) = u.speaker {
                        prefix.push_str(&format!("[Speaker {}]: ", speaker + 1));
                    }
                }
                Some(format!("{prefix}{text}"))
            })
            .collect::<Vec<_>>();
        if !lines.is_empty() {
            return lines.join("\n");
        }
    }

    let Some(alternative) = parsed
        .results
        .as_ref()
        .and_then(|r| r.channels.first())
        .and_then(|c| c.alternatives.first())
    else {
        return String::new();
    };

    if settings.include_speaker_labels {
        let grouped = render_words_by_speaker(alternative, offset_seconds, settings);
        if !grouped.is_empty() {
            return grouped;
        }
    }

    alternative
        .paragraphs
        .as_ref()
        .and_then(|p| p.transcript.as_ref())
        .or(alternative.transcript.as_ref())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn render_words_by_speaker(
    alternative: &DeepgramAlternative,
    offset_seconds: f64,
    settings: &Settings,
) -> String {
    let Some(words) = &alternative.words else {
        return String::new();
    };
    let mut lines = Vec::new();
    let mut current_speaker: Option<u64> = None;
    let mut current_start = 0.0;
    let mut current_words: Vec<String> = Vec::new();

    for word in words {
        let speaker = word.speaker;
        if current_speaker.is_some() && speaker != current_speaker {
            push_word_group(
                &mut lines,
                current_speaker,
                current_start,
                &current_words,
                offset_seconds,
                settings,
            );
            current_words.clear();
        }
        if current_words.is_empty() {
            current_start = word.start.unwrap_or(0.0);
        }
        current_speaker = speaker;
        current_words.push(
            word.punctuated_word
                .clone()
                .unwrap_or_else(|| word.word.clone()),
        );
    }
    push_word_group(
        &mut lines,
        current_speaker,
        current_start,
        &current_words,
        offset_seconds,
        settings,
    );
    lines.join("\n")
}

fn push_word_group(
    lines: &mut Vec<String>,
    speaker: Option<u64>,
    start: f64,
    words: &[String],
    offset_seconds: f64,
    settings: &Settings,
) {
    if words.is_empty() {
        return;
    }
    let mut prefix = String::new();
    if settings.include_timestamps {
        prefix.push_str(&format!("[{}] ", format_timestamp(offset_seconds + start)));
    }
    if settings.include_speaker_labels {
        if let Some(speaker) = speaker {
            prefix.push_str(&format!("[Speaker {}]: ", speaker + 1));
        }
    }
    lines.push(format!("{prefix}{}", words.join(" ")));
}

fn deepgram_language_hint(language_hint: &str) -> Option<&'static str> {
    let lower = language_hint.to_ascii_lowercase();
    if lower.contains("romanian") {
        Some("ro")
    } else if lower.contains("english") {
        Some("en")
    } else {
        None
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

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    metadata: Option<DeepgramMetadata>,
    results: Option<DeepgramResults>,
}

#[derive(Debug, Deserialize)]
struct DeepgramMetadata {
    #[serde(rename = "request_id")]
    request_id: Option<String>,
    duration: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DeepgramResults {
    #[serde(default)]
    channels: Vec<DeepgramChannel>,
    utterances: Option<Vec<DeepgramUtterance>>,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    #[serde(default)]
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: Option<String>,
    confidence: Option<f64>,
    paragraphs: Option<DeepgramParagraphs>,
    words: Option<Vec<DeepgramWord>>,
}

#[derive(Debug, Deserialize)]
struct DeepgramParagraphs {
    transcript: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepgramUtterance {
    start: f64,
    speaker: Option<u64>,
    transcript: String,
}

#[derive(Debug, Deserialize)]
struct DeepgramWord {
    word: String,
    #[serde(rename = "punctuated_word")]
    punctuated_word: Option<String>,
    start: Option<f64>,
    speaker: Option<u64>,
}
