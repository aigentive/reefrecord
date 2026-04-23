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
/// Shorter buffer is padded with silence.
pub fn mix_mono_i16(a: &[i16], b: &[i16]) -> Vec<i16> {
    let len = a.len().max(b.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let av = *a.get(i).unwrap_or(&0) as i32;
        let bv = *b.get(i).unwrap_or(&0) as i32;
        out.push((av + bv).clamp(i16::MIN as i32, i16::MAX as i32) as i16);
    }
    out
}
