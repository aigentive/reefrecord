use std::path::Path;

use flacenc::component::BitRepr;
use flacenc::error::Verify;
use hound::{SampleFormat, WavSpec, WavWriter};

use crate::error::{AppError, AppResult};

use super::{TARGET_CHANNELS, TARGET_RATE};

pub fn write_wav_mono_i16(path: &Path, samples: &[i16]) -> AppResult<()> {
    let spec = WavSpec {
        channels: TARGET_CHANNELS,
        sample_rate: TARGET_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| AppError::Audio(format!("cannot create wav: {e}")))?;
    for &s in samples {
        writer
            .write_sample(s)
            .map_err(|e| AppError::Audio(format!("cannot write sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| AppError::Audio(format!("cannot finalize wav: {e}")))?;
    Ok(())
}

pub fn transcode_wav_to_flac(wav_path: &Path) -> AppResult<std::path::PathBuf> {
    let (samples, spec) = read_wav_i32(wav_path)?;
    let flac_path = wav_path.with_extension("flac");
    let tmp_path = flac_path.with_extension("flac.tmp");
    write_flac_i32(
        &tmp_path,
        &samples,
        spec.channels,
        spec.bits_per_sample,
        spec.sample_rate,
    )?;
    std::fs::rename(&tmp_path, &flac_path)?;
    std::fs::remove_file(wav_path)?;
    Ok(flac_path)
}

pub fn transcode_flac_to_wav(flac_path: &Path) -> AppResult<std::path::PathBuf> {
    let (samples, spec) = read_flac_i32(flac_path)?;
    let wav_path = flac_path.with_extension("wav");
    let tmp_path = wav_path.with_extension("wav.tmp");
    write_wav_i32(&tmp_path, &samples, spec)?;
    std::fs::rename(&tmp_path, &wav_path)?;
    std::fs::remove_file(flac_path)?;
    Ok(wav_path)
}

pub fn write_flac_i32(
    path: &Path,
    samples: &[i32],
    channels: u16,
    bits_per_sample: u16,
    sample_rate: u32,
) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| AppError::Audio(format!("FLAC encoder config: {e:?}")))?;
    let channels = channels.max(1);
    let min_sample_count = 16 * channels as usize;
    let padded_samples;
    let source_samples = if samples.len() < min_sample_count {
        padded_samples = {
            let mut next = samples.to_vec();
            next.resize(min_sample_count, 0);
            next
        };
        padded_samples.as_slice()
    } else {
        samples
    };
    let source = flacenc::source::MemSource::from_samples(
        source_samples,
        channels as usize,
        bits_per_sample as usize,
        sample_rate as usize,
    );
    let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| AppError::Audio(format!("FLAC encode: {e}")))?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    flac_stream
        .write(&mut sink)
        .map_err(|e| AppError::Audio(format!("FLAC write: {e}")))?;
    std::fs::write(path, sink.as_slice())?;
    Ok(())
}

pub fn read_flac_i32(path: &Path) -> AppResult<(Vec<i32>, WavSpec)> {
    let mut reader = claxon::FlacReader::open(path)
        .map_err(|e| AppError::Audio(format!("cannot read flac: {e}")))?;
    let info = reader.streaminfo();
    let samples = reader
        .samples()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Audio(format!("read flac samples: {e}")))?;
    Ok((
        samples,
        WavSpec {
            channels: info.channels as u16,
            sample_rate: info.sample_rate,
            bits_per_sample: info.bits_per_sample as u16,
            sample_format: SampleFormat::Int,
        },
    ))
}

pub fn read_wav_i32(path: &Path) -> AppResult<(Vec<i32>, WavSpec)> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| AppError::Audio(format!("cannot read wav: {e}")))?;
    let spec = reader.spec();
    if spec.sample_format != SampleFormat::Int {
        return Err(AppError::Audio(
            "only integer PCM WAV files are supported".into(),
        ));
    }
    let samples = match spec.bits_per_sample {
        8 => reader
            .samples::<i8>()
            .map(|sample| sample.map(i32::from))
            .collect::<Result<Vec<_>, _>>(),
        16 => reader
            .samples::<i16>()
            .map(|sample| sample.map(i32::from))
            .collect::<Result<Vec<_>, _>>(),
        24 | 32 => reader.samples::<i32>().collect::<Result<Vec<_>, _>>(),
        other => {
            return Err(AppError::Audio(format!(
                "unsupported WAV bit depth: {other}"
            )))
        }
    }
    .map_err(|e| AppError::Audio(format!("read wav samples: {e}")))?;
    Ok((samples, spec))
}

pub fn write_wav_i32(path: &Path, samples: &[i32], spec: WavSpec) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut writer =
        WavWriter::create(path, spec).map_err(|e| AppError::Audio(format!("wav write: {e}")))?;
    match spec.bits_per_sample {
        8 => {
            for &sample in samples {
                writer
                    .write_sample(sample.clamp(i8::MIN as i32, i8::MAX as i32) as i8)
                    .map_err(|e| AppError::Audio(format!("wav sample: {e}")))?;
            }
        }
        16 => {
            for &sample in samples {
                writer
                    .write_sample(sample.clamp(i16::MIN as i32, i16::MAX as i32) as i16)
                    .map_err(|e| AppError::Audio(format!("wav sample: {e}")))?;
            }
        }
        24 | 32 => {
            for &sample in samples {
                writer
                    .write_sample(sample)
                    .map_err(|e| AppError::Audio(format!("wav sample: {e}")))?;
            }
        }
        other => {
            return Err(AppError::Audio(format!(
                "unsupported WAV bit depth: {other}"
            )))
        }
    }
    writer
        .finalize()
        .map_err(|e| AppError::Audio(format!("wav finalize: {e}")))?;
    Ok(())
}

/// Mix two mono int16 buffers sample-by-sample with int clamping.
/// Matches the Python reference (`recorder.py::mix_audio`) which truncates to
/// the shorter buffer — this keeps the two streams aligned and drops the tail
/// of whichever side captured more samples.
pub fn mix_mono_i16(a: &[i16], b: &[i16]) -> Vec<i16> {
    let len = a.len().min(b.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let av = a[i] as i32;
        let bv = b[i] as i32;
        out.push((av + bv).clamp(i16::MIN as i32, i16::MAX as i32) as i16);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_truncates_to_shorter_buffer_and_clamps() {
        let out = mix_mono_i16(&[20_000, -20_000, 1, 99], &[20_000, -20_000, -4]);

        assert_eq!(out, vec![i16::MAX, i16::MIN, -3]);
    }

    #[test]
    fn writes_flac_that_decodes_to_original_samples() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.flac");
        let samples = (0..32).map(|n| n - 16).collect::<Vec<i32>>();

        write_flac_i32(&path, &samples, 1, 16, 16_000).unwrap();
        let (samples, spec) = read_flac_i32(&path).unwrap();

        assert_eq!(samples, (0..32).map(|n| n - 16).collect::<Vec<i32>>());
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_rate, 16_000);
    }

    #[test]
    fn transcodes_wav_to_flac_and_removes_source() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("sample.wav");
        let samples = (0..32).map(|n| n as i16 - 16).collect::<Vec<i16>>();
        write_wav_mono_i16(&wav, &samples).unwrap();

        let flac = transcode_wav_to_flac(&wav).unwrap();

        assert_eq!(flac.extension().and_then(|s| s.to_str()), Some("flac"));
        assert!(!wav.exists());
        let (samples, _) = read_flac_i32(&flac).unwrap();
        assert_eq!(samples, (0..32).map(|n| n - 16).collect::<Vec<i32>>());
    }

    #[test]
    fn transcodes_flac_to_wav_and_removes_source() {
        let dir = tempfile::tempdir().unwrap();
        let flac = dir.path().join("sample.flac");
        let samples = (0..32).map(|n| n - 16).collect::<Vec<i32>>();
        write_flac_i32(&flac, &samples, 1, 16, 16_000).unwrap();

        let wav = transcode_flac_to_wav(&flac).unwrap();

        assert_eq!(wav.extension().and_then(|s| s.to_str()), Some("wav"));
        assert!(!flac.exists());
        let (samples, spec) = read_wav_i32(&wav).unwrap();
        assert_eq!(samples, (0..32).map(|n| n - 16).collect::<Vec<i32>>());
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.bits_per_sample, 16);
    }

    #[test]
    fn writes_mono_i16_wav_with_target_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("session.wav");

        write_wav_mono_i16(&path, &[0, 123, -456]).unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        let samples = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(spec.channels, TARGET_CHANNELS);
        assert_eq!(spec.sample_rate, TARGET_RATE);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(samples, vec![0, 123, -456]);
    }
}
