use std::path::{Path, PathBuf};

use hound::WavReader;

use crate::error::{AppError, AppResult};
use crate::gemini::client::{
    build_file_audio_part, build_inline_audio_part, GeminiClient, GeminiUsage,
};
use crate::gemini::GEMINI_AUDIO_INLINE_LIMIT;

pub struct TranscriptionOutcome {
    pub text: String,
    pub usage: GeminiUsage,
    pub model_used: String,
}

pub struct TranscriptionJob {
    pub wav_path: PathBuf,
    pub primary_model: String,
    pub fallback_model: String,
    pub chunk_minutes: u32,
    pub language_hint: String,
    pub include_speaker_labels: bool,
    pub include_timestamps: bool,
}

pub async fn transcribe(
    client: &GeminiClient,
    job: TranscriptionJob,
) -> AppResult<TranscriptionOutcome> {
    let chunks = split_wav_if_needed(&job.wav_path, job.chunk_minutes)?;
    tracing::info!(
        wav = %job.wav_path.display(),
        chunks = chunks.len(),
        chunk_minutes = job.chunk_minutes,
        "starting transcription"
    );
    let mut collected_text = Vec::<String>::new();
    let mut total_usage = GeminiUsage::default();
    let mut last_model = job.primary_model.clone();
    for (idx, (chunk_path, offset_s)) in chunks.iter().enumerate() {
        tracing::info!(
            chunk = idx + 1,
            total = chunks.len(),
            offset_s,
            "transcribing chunk"
        );
        let prompt = build_prompt(
            *offset_s,
            &job.language_hint,
            job.include_speaker_labels,
            job.include_timestamps,
        );
        let (text, usage, model_used) = transcribe_chunk(
            client,
            chunk_path,
            &prompt,
            &job.primary_model,
            &job.fallback_model,
        )
        .await?;
        collected_text.push(text);
        total_usage = total_usage.add(usage);
        last_model = model_used;
        // Clean up chunk file if split.
        if chunk_path != &job.wav_path {
            let _ = std::fs::remove_file(chunk_path);
        }
    }

    if collected_text.is_empty() {
        return Err(AppError::Gemini("no transcript produced".into()));
    }
    Ok(TranscriptionOutcome {
        text: collected_text.join("\n\n"),
        usage: total_usage,
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

pub fn build_prompt(
    offset_seconds: u64,
    language_hint: &str,
    include_speaker_labels: bool,
    include_timestamps: bool,
) -> String {
    let mut out = String::new();
    if offset_seconds > 0 && include_timestamps {
        let mm = offset_seconds / 60;
        let ss = offset_seconds % 60;
        out.push_str(&format!(
            "This audio chunk starts at {mm:02}:{ss:02} in the full recording. Adjust your timestamps accordingly.\n"
        ));
    }
    out.push_str("Transcribe this audio verbatim. Do not summarize or paraphrase.\n");
    if !language_hint.trim().is_empty() {
        out.push_str(&format!(
            "The audio is primarily in {language_hint}; preserve the original language as spoken.\n"
        ));
    }
    match (include_speaker_labels, include_timestamps) {
        (true, true) => {
            out.push_str(
                "Identify and label different speakers as [Speaker 1], [Speaker 2], etc.\n",
            );
            out.push_str(
                "Include timestamps in [MM:SS] format at each speaker change.\n",
            );
        }
        (true, false) => {
            out.push_str(
                "Identify and label different speakers as [Speaker 1], [Speaker 2], etc.\n",
            );
            out.push_str("Do not include timestamps.\n");
        }
        (false, true) => {
            out.push_str("Do not label speakers.\n");
            out.push_str(
                "Include timestamps in [MM:SS] format at natural pauses or roughly every minute.\n",
            );
        }
        (false, false) => {
            out.push_str(
                "Do not label speakers. Do not include timestamps. Produce clean flowing text only.\n",
            );
        }
    }
    out.push_str("Output only the transcription, nothing else.");
    out
}

/// Returns list of (chunk_path, offset_seconds). If splitting is not required,
/// returns a single-entry vec with the original path and offset 0.
pub fn split_wav_if_needed(
    wav_path: &Path,
    chunk_minutes: u32,
) -> AppResult<Vec<(PathBuf, u64)>> {
    let reader = WavReader::open(wav_path)
        .map_err(|e| AppError::Audio(format!("cannot read wav: {e}")))?;
    let spec = reader.spec();
    let total_samples = reader.len() as u64;
    let duration_seconds = total_samples / spec.channels as u64 / spec.sample_rate as u64;
    let chunk_seconds = (chunk_minutes as u64).max(1) * 60;
    // Only split if meaningfully longer than chunk size (match python behavior: +60s buffer).
    if duration_seconds <= chunk_seconds + 60 {
        return Ok(vec![(wav_path.to_path_buf(), 0)]);
    }

    drop(reader);

    // Read full samples then re-chunk by sample count.
    let mut reader = WavReader::open(wav_path)
        .map_err(|e| AppError::Audio(format!("cannot reopen wav: {e}")))?;
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Audio(format!("read samples: {e}")))?;

    let samples_per_chunk =
        (chunk_seconds as usize) * spec.sample_rate as usize * spec.channels as usize;
    let mut chunks = Vec::new();
    let mut idx = 0usize;
    let mut offset_seconds = 0u64;
    while idx * samples_per_chunk < samples.len() {
        let start = idx * samples_per_chunk;
        let end = (start + samples_per_chunk).min(samples.len());
        let slice = &samples[start..end];
        let chunk_path = wav_path.with_file_name(format!(
            "{}_chunk{}.wav",
            wav_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("chunk"),
            idx
        ));
        write_chunk_wav(&chunk_path, slice, spec)?;
        chunks.push((chunk_path, offset_seconds));
        offset_seconds += chunk_seconds;
        idx += 1;
    }
    Ok(chunks)
}

fn write_chunk_wav(path: &Path, samples: &[i16], spec: hound::WavSpec) -> AppResult<()> {
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| AppError::Audio(format!("chunk write: {e}")))?;
    for &s in samples {
        writer
            .write_sample(s)
            .map_err(|e| AppError::Audio(format!("chunk sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| AppError::Audio(format!("chunk finalize: {e}")))?;
    Ok(())
}
