use std::path::{Path, PathBuf};

use hound::WavReader;

use crate::audio::format::AudioFormat;
use crate::audio::writer::{read_flac_i32, write_flac_i32};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct ChunkPolicy {
    pub max_seconds: Option<u64>,
    pub max_bytes: Option<u64>,
    pub preserve_existing_short_buffer: bool,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub path: PathBuf,
    pub offset_seconds: u64,
    pub format: AudioFormat,
}

pub fn split_audio_with_policy(
    audio_path: &Path,
    format: AudioFormat,
    policy: ChunkPolicy,
) -> AppResult<Vec<AudioChunk>> {
    match format {
        AudioFormat::Wav => split_wav_with_policy(audio_path, policy).map(|chunks| {
            chunks
                .into_iter()
                .map(|(path, offset_seconds)| AudioChunk {
                    path,
                    offset_seconds,
                    format,
                })
                .collect()
        }),
        AudioFormat::Flac => split_flac_with_policy(audio_path, policy),
    }
}

pub fn split_wav_with_policy(
    wav_path: &Path,
    policy: ChunkPolicy,
) -> AppResult<Vec<(PathBuf, u64)>> {
    let reader =
        WavReader::open(wav_path).map_err(|e| AppError::Audio(format!("cannot read wav: {e}")))?;
    let spec = reader.spec();
    let total_samples = reader.len() as u64;
    let duration_seconds = total_samples / spec.channels as u64 / spec.sample_rate as u64;
    let file_size = std::fs::metadata(wav_path)?.len();
    let bytes_per_second = bytes_per_second(spec);

    let mut chunk_seconds = policy.max_seconds.unwrap_or(duration_seconds.max(1)).max(1);
    if let Some(max_bytes) = policy.max_bytes {
        let seconds_by_bytes = (max_bytes / bytes_per_second.max(1)).max(1);
        chunk_seconds = chunk_seconds.min(seconds_by_bytes);
    }

    let duration_fits = if policy.preserve_existing_short_buffer {
        duration_seconds <= chunk_seconds + 60
    } else {
        duration_seconds <= chunk_seconds
    };
    let bytes_fit = policy.max_bytes.map(|m| file_size <= m).unwrap_or(true);
    if duration_fits && bytes_fit {
        return Ok(vec![(wav_path.to_path_buf(), 0)]);
    }

    drop(reader);

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

fn split_flac_with_policy(flac_path: &Path, policy: ChunkPolicy) -> AppResult<Vec<AudioChunk>> {
    let reader = claxon::FlacReader::open(flac_path)
        .map_err(|e| AppError::Audio(format!("cannot read flac: {e}")))?;
    let info = reader.streaminfo();
    let total_interchannel_samples = match info.samples {
        Some(samples) => samples,
        None => {
            drop(reader);
            let (samples, spec) = read_flac_i32(flac_path)?;
            samples.len() as u64 / spec.channels.max(1) as u64
        }
    };
    let duration_seconds = total_interchannel_samples / info.sample_rate.max(1) as u64;
    let file_size = std::fs::metadata(flac_path)?.len();
    let encoded_bytes_per_second = if duration_seconds == 0 {
        file_size.max(1)
    } else {
        (file_size / duration_seconds).max(1)
    };

    let mut chunk_seconds = policy.max_seconds.unwrap_or(duration_seconds.max(1)).max(1);
    if let Some(max_bytes) = policy.max_bytes {
        let seconds_by_bytes = (max_bytes / encoded_bytes_per_second).max(1);
        chunk_seconds = chunk_seconds.min(seconds_by_bytes);
    }

    let duration_fits = if policy.preserve_existing_short_buffer {
        duration_seconds <= chunk_seconds + 60
    } else {
        duration_seconds <= chunk_seconds
    };
    let bytes_fit = policy.max_bytes.map(|m| file_size <= m).unwrap_or(true);
    if duration_fits && bytes_fit {
        return Ok(vec![AudioChunk {
            path: flac_path.to_path_buf(),
            offset_seconds: 0,
            format: AudioFormat::Flac,
        }]);
    }

    let (samples, spec) = read_flac_i32(flac_path)?;
    let samples_per_chunk =
        (chunk_seconds as usize) * spec.sample_rate as usize * spec.channels as usize;
    let mut chunks = Vec::new();
    let mut idx = 0usize;
    let mut offset_seconds = 0u64;
    while idx * samples_per_chunk < samples.len() {
        let start = idx * samples_per_chunk;
        let end = (start + samples_per_chunk).min(samples.len());
        let slice = &samples[start..end];
        let chunk_path = flac_path.with_file_name(format!(
            "{}_chunk{}.flac",
            flac_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("chunk"),
            idx
        ));
        write_flac_i32(
            &chunk_path,
            slice,
            spec.channels,
            spec.bits_per_sample,
            spec.sample_rate,
        )?;
        chunks.push(AudioChunk {
            path: chunk_path,
            offset_seconds,
            format: AudioFormat::Flac,
        });
        offset_seconds += chunk_seconds;
        idx += 1;
    }
    Ok(chunks)
}

pub fn audio_duration_seconds(path: &Path, format: AudioFormat) -> AppResult<f64> {
    match format {
        AudioFormat::Wav => wav_duration_seconds(path),
        AudioFormat::Flac => flac_duration_seconds(path),
    }
}

pub fn wav_duration_seconds(path: &Path) -> AppResult<f64> {
    let reader =
        WavReader::open(path).map_err(|e| AppError::Audio(format!("cannot read wav: {e}")))?;
    let spec = reader.spec();
    Ok(reader.len() as f64 / spec.channels as f64 / spec.sample_rate as f64)
}

fn flac_duration_seconds(path: &Path) -> AppResult<f64> {
    let reader = claxon::FlacReader::open(path)
        .map_err(|e| AppError::Audio(format!("cannot read flac: {e}")))?;
    let info = reader.streaminfo();
    if let Some(samples) = info.samples {
        return Ok(samples as f64 / info.sample_rate as f64);
    }
    drop(reader);
    let (samples, spec) = read_flac_i32(path)?;
    Ok(samples.len() as f64 / spec.channels as f64 / spec.sample_rate as f64)
}

fn bytes_per_second(spec: hound::WavSpec) -> u64 {
    spec.sample_rate as u64 * spec.channels as u64 * (spec.bits_per_sample as u64 / 8).max(1)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::writer::write_flac_i32;

    #[test]
    fn split_audio_chunks_flac_and_preserves_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("long.flac");
        let samples = vec![0i32; 8_000 * 3];
        write_flac_i32(&path, &samples, 1, 16, 8_000).unwrap();

        let chunks = split_audio_with_policy(
            &path,
            AudioFormat::Flac,
            ChunkPolicy {
                max_seconds: Some(1),
                max_bytes: None,
                preserve_existing_short_buffer: false,
            },
        )
        .unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.offset_seconds)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(chunks.iter().all(|chunk| chunk.format == AudioFormat::Flac));
        for chunk in chunks {
            if chunk.path != path {
                std::fs::remove_file(chunk.path).unwrap();
            }
        }
    }
}
