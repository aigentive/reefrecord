pub mod chunking;
pub mod prompt;
pub mod providers;
pub mod types;

use std::path::PathBuf;

use crate::audio::format::AudioFormat;
use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::transcription::types::{
    TranscriptionOutcome, TranscriptionProvider, TranscriptionUsage,
};

pub const OPENAI_WHISPER_MODEL: &str = "whisper-1";
pub const OPENAI_DIARIZE_MODEL: &str = "gpt-4o-transcribe-diarize";

pub use types::TranscriptionProvider as Provider;

pub async fn validate_key(
    provider: TranscriptionProvider,
    key: &str,
    settings: &Settings,
) -> AppResult<String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::Invalid(format!(
            "{} key is empty.",
            provider.label()
        )));
    }
    match provider {
        TranscriptionProvider::Gemini => providers::gemini::validate_key(key, settings).await,
        TranscriptionProvider::Openai => providers::openai::validate_key(key, settings).await,
        TranscriptionProvider::Deepgram => providers::deepgram::validate_key(key, settings).await,
    }
}

pub async fn transcribe(
    provider: TranscriptionProvider,
    key: String,
    settings: &Settings,
    audio_path: PathBuf,
    audio_format: AudioFormat,
) -> AppResult<TranscriptionOutcome> {
    match provider {
        TranscriptionProvider::Gemini => {
            providers::gemini::transcribe(key, settings, audio_path, audio_format).await
        }
        TranscriptionProvider::Openai => {
            providers::openai::transcribe(key, settings, audio_path, audio_format).await
        }
        TranscriptionProvider::Deepgram => {
            providers::deepgram::transcribe(key, settings, audio_path, audio_format).await
        }
    }
}

pub fn estimate_cost(settings: &Settings, outcome: &TranscriptionOutcome) -> f64 {
    match (&outcome.provider, &outcome.usage) {
        (
            TranscriptionProvider::Gemini,
            TranscriptionUsage::Tokens {
                prompt_tokens,
                output_tokens,
                ..
            },
        ) => {
            (*prompt_tokens as f64 * settings.gemini_input_cost_per_million_usd
                + *output_tokens as f64 * settings.gemini_output_cost_per_million_usd)
                / 1_000_000.0
        }
        (TranscriptionProvider::Openai, TranscriptionUsage::Duration { seconds }) => {
            (*seconds / 60.0) * settings.openai_cost_per_minute_usd
        }
        (
            TranscriptionProvider::Openai,
            TranscriptionUsage::Tokens {
                prompt_tokens,
                output_tokens,
                ..
            },
        ) => {
            (*prompt_tokens as f64 * settings.openai_input_cost_per_million_usd
                + *output_tokens as f64 * settings.openai_output_cost_per_million_usd)
                / 1_000_000.0
        }
        (
            TranscriptionProvider::Deepgram,
            TranscriptionUsage::Deepgram {
                duration_seconds: Some(seconds),
                ..
            },
        ) => (*seconds / 3600.0) * settings.deepgram_cost_per_hour_usd,
        _ => 0.0,
    }
}

pub fn capability_warning(settings: &Settings) -> Option<String> {
    match settings.transcription_provider {
        TranscriptionProvider::Openai
            if settings.include_speaker_labels && settings.openai_model == OPENAI_WHISPER_MODEL =>
        {
            Some("OpenAI whisper-1 does not provide speaker labels. Use gpt-4o-transcribe-diarize for speaker-aware output.".into())
        }
        TranscriptionProvider::Openai
            if settings.include_timestamps && settings.openai_model != OPENAI_WHISPER_MODEL =>
        {
            Some("OpenAI timestamp granularity is available for whisper-1; other OpenAI transcription models may return plain text unless using diarized JSON.".into())
        }
        _ => None,
    }
}
