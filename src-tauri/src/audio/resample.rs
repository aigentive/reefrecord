use super::TARGET_RATE;

/// Downmix a multi-channel interleaved i16 buffer into mono by averaging.
pub fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    let ch = channels as usize;
    let mut out = Vec::with_capacity(samples.len() / ch);
    for frame in samples.chunks_exact(ch) {
        let sum: i32 = frame.iter().map(|s| *s as i32).sum();
        let avg = sum / ch as i32;
        out.push(avg.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
    }
    out
}

/// Simple linear resampler from `src_rate` to `TARGET_RATE`. Input is mono i16.
pub fn resample_to_target(mono: &[i16], src_rate: u32) -> Vec<i16> {
    if src_rate == TARGET_RATE || mono.is_empty() {
        return mono.to_vec();
    }
    if src_rate == 0 {
        return Vec::new();
    }
    let ratio = TARGET_RATE as f64 / src_rate as f64;
    let out_len = ((mono.len() as f64) * ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = src_pos - idx as f64;
        let a = *mono.get(idx).unwrap_or(&0) as f64;
        let b = *mono.get(idx + 1).unwrap_or(&(a as i16)) as f64;
        let v = a + (b - a) * frac;
        out.push(v.clamp(i16::MIN as f64, i16::MAX as f64) as i16);
    }
    out
}

/// Convert f32 [-1.0, 1.0] samples to i16.
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// Convert u16 samples to i16 (centered at 0x8000).
pub fn u16_to_i16(samples: &[u16]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s as i32 - 0x8000) as i16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_complete_frames_and_ignores_tail() {
        let out = downmix_to_mono(&[10, 30, -10, -30, 100], 2);

        assert_eq!(out, vec![20, -20]);
    }

    #[test]
    fn downmix_returns_input_for_mono_or_invalid_channel_count() {
        assert_eq!(downmix_to_mono(&[1, 2, 3], 1), vec![1, 2, 3]);
        assert_eq!(downmix_to_mono(&[1, 2, 3], 0), vec![1, 2, 3]);
    }

    #[test]
    fn resample_returns_input_for_target_rate_or_empty_input() {
        assert_eq!(resample_to_target(&[1, 2, 3], TARGET_RATE), vec![1, 2, 3]);
        assert_eq!(resample_to_target(&[], 48_000), Vec::<i16>::new());
    }

    #[test]
    fn resample_handles_zero_rate_and_interpolates() {
        assert_eq!(resample_to_target(&[1, 2, 3], 0), Vec::<i16>::new());

        let out = resample_to_target(&[0, 1000], TARGET_RATE / 2);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], 0);
        assert_eq!(out[1], 500);
        assert_eq!(out[2], 1000);
        assert_eq!(out[3], 1000);
    }

    #[test]
    fn sample_conversions_clamp_and_center() {
        assert_eq!(
            f32_to_i16(&[-2.0, -1.0, 0.0, 0.5, 2.0]),
            vec![i16::MIN + 1, i16::MIN + 1, 0, 16_383, i16::MAX,]
        );
        assert_eq!(
            u16_to_i16(&[0, 0x8000, 0xffff]),
            vec![i16::MIN, 0, i16::MAX,]
        );
    }
}
