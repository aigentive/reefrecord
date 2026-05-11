use std::path::Path;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};

pub struct GeminiClient {
    api_key: String,
    http: Client,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AudioPart {
    Inline {
        #[serde(rename = "inline_data")]
        inline_data: InlineData,
    },
    File {
        #[serde(rename = "file_data")]
        file_data: FileData,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineData {
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileData {
    pub mime_type: String,
    pub file_uri: String,
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    #[allow(dead_code)]
    prompt_feedback: Option<Value>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: u64,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: u64,
    #[serde(rename = "totalTokenCount", default)]
    total_token_count: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GeminiUsage {
    pub prompt_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl GeminiUsage {
    pub fn add(self, other: GeminiUsage) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + other.prompt_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Option<Vec<ContentPart>>,
}

#[derive(Debug, Deserialize)]
struct ContentPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileStartResponse {
    file: FileInfo,
}

#[derive(Debug, Deserialize)]
struct FileInfo {
    uri: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct FileStatusResponse {
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    uri: Option<String>,
}

impl GeminiClient {
    pub fn new(api_key: String) -> AppResult<Self> {
        if api_key.trim().is_empty() {
            return Err(AppError::Invalid("Gemini API key is empty.".into()));
        }
        let http = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;
        Ok(Self { api_key, http })
    }

    /// Quick validation: list models and look for the configured model.
    pub async fn validate(&self, model: &str) -> AppResult<String> {
        let url = "https://generativelanguage.googleapis.com/v1beta/models?pageSize=200";
        let resp = self
            .http
            .get(url)
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::Gemini(format!(
                "validation failed ({}): {}",
                status,
                truncate(&text, 200)
            )));
        }
        // Accept any 200; optionally verify the model exists.
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if let Some(models) = v.get("models").and_then(|m| m.as_array()) {
            let found = models
                .iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                .any(|n| n.contains(model));
            if found {
                return Ok(format!("Key valid. Model {model} available."));
            }
            return Ok(format!(
                "Key valid. Model {model} not in list — fallback will be used if needed."
            ));
        }
        Ok("Key valid.".to_string())
    }

    pub async fn upload_audio_file(&self, path: &Path) -> AppResult<String> {
        let file_size = std::fs::metadata(path)?.len();
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav")
            .to_string();

        let start_url = "https://generativelanguage.googleapis.com/upload/v1beta/files";
        let start_resp = self
            .http
            .post(start_url)
            .header("x-goog-api-key", &self.api_key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", file_size.to_string())
            .header("X-Goog-Upload-Header-Content-Type", "audio/wav")
            .header("Content-Type", "application/json")
            .json(&json!({ "file": { "display_name": display_name } }))
            .send()
            .await?;
        if !start_resp.status().is_success() {
            let s = start_resp.status();
            let body = start_resp.text().await.unwrap_or_default();
            return Err(AppError::Gemini(format!(
                "upload start failed ({s}): {}",
                truncate(&body, 200)
            )));
        }
        let upload_url = start_resp
            .headers()
            .get("X-Goog-Upload-URL")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Gemini("missing X-Goog-Upload-URL header".into()))?
            .to_string();

        let bytes = tokio::fs::read(path).await?;
        let upload_resp = self
            .http
            .post(&upload_url)
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .header("Content-Length", bytes.len().to_string())
            .body(bytes)
            .send()
            .await?;
        if !upload_resp.status().is_success() {
            let s = upload_resp.status();
            let body = upload_resp.text().await.unwrap_or_default();
            return Err(AppError::Gemini(format!(
                "upload failed ({s}): {}",
                truncate(&body, 200)
            )));
        }
        let start: FileStartResponse = upload_resp.json().await?;
        let file_name = start.file.name.clone();
        let file_uri = start.file.uri.clone();

        for _ in 0..60 {
            let check_url = format!(
                "https://generativelanguage.googleapis.com/v1beta/{}",
                file_name
            );
            let resp = self
                .http
                .get(&check_url)
                .header("x-goog-api-key", &self.api_key)
                .send()
                .await?;
            let status: FileStatusResponse = resp.json().await.unwrap_or(FileStatusResponse {
                state: None,
                uri: None,
            });
            match status.state.as_deref() {
                Some("ACTIVE") => return Ok(status.uri.unwrap_or(file_uri)),
                Some("FAILED") => return Err(AppError::Gemini("file processing failed".into())),
                _ => tokio::time::sleep(Duration::from_secs(2)).await,
            }
        }
        Err(AppError::Gemini("file processing timed out".into()))
    }

    pub async fn generate_transcript(
        &self,
        model: &str,
        prompt: &str,
        audio_part: AudioPart,
    ) -> AppResult<(String, GeminiUsage)> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            model
        );
        let audio_value = serde_json::to_value(&audio_part)
            .map_err(|e| AppError::Gemini(format!("serialize audio part: {e}")))?;
        let parts = vec![json!({ "text": prompt }), audio_value];
        let payload = json!({
            "contents": [{ "parts": parts }],
            "generationConfig": {
                "temperature": 0.0,
                "topP": 1.0,
                "candidateCount": 1
            }
        });

        let mut attempts: u32 = 0;
        loop {
            attempts += 1;
            let resp = self
                .http
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .json(&payload)
                .send()
                .await;
            let (status, body_text) = match resp {
                Ok(r) => (r.status(), r.text().await.unwrap_or_default()),
                Err(e) => {
                    return Err(AppError::Gemini(format!("request failed: {e}")));
                }
            };

            if status.as_u16() == 503 || status.as_u16() == 502 || status.as_u16() == 504 {
                if attempts < 3 {
                    let wait = 10 * attempts as u64;
                    tracing::warn!("gemini {} — retrying in {wait}s", status);
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    continue;
                }
                return Err(AppError::Gemini(format!(
                    "service unavailable after retries ({status})"
                )));
            }
            if status.as_u16() == 429 {
                if attempts < 3 {
                    let wait = 15 * attempts as u64;
                    tracing::warn!("gemini 429 — retrying in {wait}s");
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    continue;
                }
                return Err(AppError::Gemini("rate limited".into()));
            }
            if !status.is_success() {
                return Err(AppError::Gemini(format!(
                    "generateContent {}: {}",
                    status,
                    truncate(&body_text, 300)
                )));
            }

            let parsed: GenerateContentResponse =
                serde_json::from_str(&body_text).map_err(|e| {
                    AppError::Gemini(format!(
                        "invalid response json: {e}: {}",
                        truncate(&body_text, 200)
                    ))
                })?;
            let usage = parsed
                .usage_metadata
                .clone()
                .map(|u| GeminiUsage {
                    prompt_tokens: u.prompt_token_count,
                    output_tokens: u.candidates_token_count,
                    total_tokens: u.total_token_count,
                })
                .unwrap_or_default();
            let text = parsed
                .candidates
                .and_then(|mut c| c.drain(..).next())
                .and_then(|c| c.content)
                .and_then(|c| c.parts)
                .and_then(|mut p| p.drain(..).next())
                .and_then(|p| p.text);
            match text {
                Some(t) if !t.trim().is_empty() => return Ok((t, usage)),
                _ => return Err(AppError::Gemini("empty transcript returned".into())),
            }
        }
    }
}

pub fn build_inline_audio_part(path: &Path) -> AppResult<AudioPart> {
    let bytes = std::fs::read(path)?;
    let encoded = B64_STANDARD.encode(bytes);
    Ok(AudioPart::Inline {
        inline_data: InlineData {
            mime_type: "audio/wav".into(),
            data: encoded,
        },
    })
}

pub fn build_file_audio_part(file_uri: String) -> AudioPart {
    AudioPart::File {
        file_data: FileData {
            mime_type: "audio/wav".into(),
            file_uri,
        },
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_keys() {
        assert!(GeminiClient::new("".into()).is_err());
        assert!(GeminiClient::new("   ".into()).is_err());
        assert!(GeminiClient::new("AIzaSy_test".into()).is_ok());
    }

    #[test]
    fn usage_add_sums_each_counter() {
        let a = GeminiUsage {
            prompt_tokens: 1,
            output_tokens: 2,
            total_tokens: 3,
        };
        let b = GeminiUsage {
            prompt_tokens: 10,
            output_tokens: 20,
            total_tokens: 30,
        };

        let out = a.add(b);

        assert_eq!(out.prompt_tokens, 11);
        assert_eq!(out.output_tokens, 22);
        assert_eq!(out.total_tokens, 33);
    }

    #[test]
    fn audio_parts_serialize_to_gemini_payload_shape() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("sample.wav");
        std::fs::write(&wav, [0u8, 1, 2, 3]).unwrap();

        let inline = build_inline_audio_part(&wav).unwrap();
        let inline_json = serde_json::to_value(inline).unwrap();
        assert_eq!(
            inline_json["inline_data"]["mime_type"].as_str(),
            Some("audio/wav")
        );
        assert_eq!(
            inline_json["inline_data"]["data"].as_str(),
            Some("AAECAw==")
        );

        let file = build_file_audio_part("files/abc".into());
        let file_json = serde_json::to_value(file).unwrap();
        assert_eq!(
            file_json["file_data"]["mime_type"].as_str(),
            Some("audio/wav")
        );
        assert_eq!(
            file_json["file_data"]["file_uri"].as_str(),
            Some("files/abc")
        );
    }

    #[test]
    fn truncate_preserves_short_text_and_shortens_long_text() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdef", 3), "abc…");
    }
}
