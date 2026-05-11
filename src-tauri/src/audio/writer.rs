use std::path::Path;

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
