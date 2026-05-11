use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionProvider {
    #[default]
    Gemini,
    Openai,
    Deepgram,
}

impl TranscriptionProvider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Gemini => "Gemini",
            Self::Openai => "OpenAI",
            Self::Deepgram => "Deepgram",
        }
    }

    pub fn key_account(self) -> &'static str {
        match self {
            Self::Gemini => "gemini_api_key",
            Self::Openai => "openai_api_key",
            Self::Deepgram => "deepgram_api_key",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Gemini, Self::Openai, Self::Deepgram]
    }
}

impl fmt::Display for TranscriptionProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Gemini => "gemini",
            Self::Openai => "openai",
            Self::Deepgram => "deepgram",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TranscriptionUsage {
    Tokens {
        prompt_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
        audio_tokens: Option<u64>,
        text_tokens: Option<u64>,
    },
    Duration {
        seconds: f64,
    },
    Deepgram {
        request_id: Option<String>,
        duration_seconds: Option<f64>,
        confidence: Option<f64>,
    },
    Unknown,
}

impl TranscriptionUsage {
    pub fn add(self, other: Self) -> Self {
        match (self, other) {
            (
                Self::Tokens {
                    prompt_tokens,
                    output_tokens,
                    total_tokens,
                    audio_tokens,
                    text_tokens,
                },
                Self::Tokens {
                    prompt_tokens: b_prompt,
                    output_tokens: b_output,
                    total_tokens: b_total,
                    audio_tokens: b_audio,
                    text_tokens: b_text,
                },
            ) => Self::Tokens {
                prompt_tokens: prompt_tokens + b_prompt,
                output_tokens: output_tokens + b_output,
                total_tokens: total_tokens + b_total,
                audio_tokens: sum_options(audio_tokens, b_audio),
                text_tokens: sum_options(text_tokens, b_text),
            },
            (Self::Duration { seconds }, Self::Duration { seconds: b }) => Self::Duration {
                seconds: seconds + b,
            },
            (
                Self::Deepgram {
                    request_id: _,
                    duration_seconds,
                    confidence,
                },
                Self::Deepgram {
                    request_id,
                    duration_seconds: b_duration,
                    confidence: b_confidence,
                },
            ) => Self::Deepgram {
                request_id,
                duration_seconds: sum_f64_options(duration_seconds, b_duration),
                confidence: average_options(confidence, b_confidence),
            },
            (Self::Unknown, other) => other,
            (left, Self::Unknown) => left,
            (left, _) => left,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionOutcome {
    pub text: String,
    pub provider: TranscriptionProvider,
    pub model_used: String,
    pub usage: TranscriptionUsage,
}

fn sum_options(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn sum_f64_options(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn average_options(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a + b) / 2.0),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}
