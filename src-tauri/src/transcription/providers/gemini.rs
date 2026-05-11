use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::gemini::client::{
    build_file_audio_part, build_inline_audio_part, GeminiClient, GeminiUsage,
};
use crate::gemini::GEMINI_AUDIO_INLINE_LIMIT;
use crate::settings::Settings;
use crate::transcription::chunking::{split_wav_with_policy, ChunkPolicy};
use crate::transcription::prompt::build_gemini_prompt;
use crate::transcription::types::{
    TranscriptionOutcome, TranscriptionProvider, TranscriptionUsage,
};

pub async fn validate_key(key: &str, settings: &Settings) -> AppResult<String> {
    let client = GeminiClient::new(key.to_string())?;
    client.validate(&settings.gemini_model).await
}

pub async fn transcribe(
    key: String,
    settings: &Settings,
    wav_path: PathBuf,
) -> AppResult<TranscriptionOutcome> {
    let client = GeminiClient::new(key)?;
    let chunk_seconds = settings.chunk_minutes.max(1) as u64 * 60;
    let chunks = split_wav_with_policy(
        &wav_path,
        ChunkPolicy {
            max_seconds: Some(chunk_seconds),
            max_bytes: None,
            preserve_existing_short_buffer: true,
        },
    )?;
    tracing::info!(
        wav = %wav_path.display(),
        chunks = chunks.len(),
        provider = "gemini",
        "starting transcription"
    );

    let mut collected_text = Vec::<String>::new();
    let mut total_usage = GeminiUsage::default();
    let mut last_model = settings.gemini_model.clone();
    for (idx, (chunk_path, offset_s)) in chunks.iter().enumerate() {
        tracing::info!(
            chunk = idx + 1,
            total = chunks.len(),
            offset_s,
            provider = "gemini",
            "transcribing chunk"
        );
        let prompt = build_gemini_prompt(
            *offset_s,
            &settings.language_hint,
            settings.include_speaker_labels,
            settings.include_timestamps,
        );
        let (text, usage, model_used) = transcribe_chunk(
            &client,
            chunk_path,
            &prompt,
            &settings.gemini_model,
            &settings.gemini_fallback_model,
        )
        .await?;
        collected_text.push(text);
        total_usage = total_usage.add(usage);
        last_model = model_used;
        if chunk_path != &wav_path {
            let _ = std::fs::remove_file(chunk_path);
        }
    }

    if collected_text.is_empty() {
        return Err(AppError::Gemini("no transcript produced".into()));
    }
    Ok(TranscriptionOutcome {
        text: collected_text.join("\n\n"),
        provider: TranscriptionProvider::Gemini,
        usage: TranscriptionUsage::Tokens {
            prompt_tokens: total_usage.prompt_tokens,
            output_tokens: total_usage.output_tokens,
            total_tokens: total_usage.total_tokens,
            audio_tokens: None,
            text_tokens: None,
        },
        model_used: last_model,
    })
}

async fn transcribe_chunk(
    client: &GeminiClient,
    chunk_path: &Path,
    prompt: &str,
    primary: &str,
    fallback: &str,
) -> AppResult<(String, GeminiUsage, String)> {
    let file_size = std::fs::metadata(chunk_path)?.len();
    let audio_part = if file_size > GEMINI_AUDIO_INLINE_LIMIT {
        let uri = client.upload_audio_file(chunk_path).await?;
        build_file_audio_part(uri)
    } else {
        build_inline_audio_part(chunk_path)?
    };

    match client
        .generate_transcript(primary, prompt, audio_part.clone())
        .await
    {
        Ok((text, usage)) => Ok((text, usage, primary.to_string())),
        Err(e) => {
            tracing::warn!("primary model {primary} failed: {e}. Trying fallback {fallback}.");
            let (text, usage) = client
                .generate_transcript(fallback, prompt, audio_part)
                .await?;
            Ok((text, usage, fallback.to_string()))
        }
    }
}
