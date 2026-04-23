use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};

use crate::error::{AppError, AppResult};

use super::resample::{downmix_to_mono, f32_to_i16, resample_to_target, u16_to_i16};
use super::TARGET_RATE;

/// Handle to a running cpal input stream that pushes mono @ 16 kHz i16 samples
/// into `buffer`.
pub struct InputCapture {
    _stream: Stream,
    pub device_name: String,
    pub buffer: Arc<Mutex<Vec<i16>>>,
}

impl InputCapture {
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn take_samples(&self) -> Vec<i16> {
        let mut guard = self.buffer.lock().unwrap();
        let out = std::mem::take(&mut *guard);
        out
    }

    pub fn stop(self) -> Vec<i16> {
        drop(self._stream);
        let mut guard = self.buffer.lock().unwrap();
        std::mem::take(&mut *guard)
    }
}

pub fn resolve_input_device(selector: Option<&str>, require_blackhole: bool) -> AppResult<Device> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| AppError::Audio(e.to_string()))?
        .collect::<Vec<_>>();
    if let Some(sel) = selector {
        let sel_trim = sel.trim();
        if !sel_trim.is_empty() {
            if let Ok(idx) = sel_trim.parse::<usize>() {
                if let Some(dev) = devices.into_iter().nth(idx) {
                    return Ok(dev);
                }
                return Err(AppError::Audio(format!("no input device at index {idx}")));
            }
            let lower = sel_trim.to_lowercase();
            if let Some(dev) = devices
                .into_iter()
                .find(|d| d.name().map(|n| n.to_lowercase().contains(&lower)).unwrap_or(false))
            {
                return Ok(dev);
            }
            return Err(AppError::Audio(format!("no input device matches {sel_trim:?}")));
        }
    }
    if require_blackhole {
        for d in devices {
            if let Ok(name) = d.name() {
                let lower = name.to_lowercase();
                if lower.contains("blackhole") || lower.contains("black hole") {
                    return Ok(d);
                }
            }
        }
        return Err(AppError::Audio("BlackHole input device not found.".into()));
    }
    host.default_input_device()
        .ok_or_else(|| AppError::Audio("No default input device available.".into()))
}

pub fn start_capture(device: Device, label: &str) -> AppResult<InputCapture> {
    let device_name = device.name().unwrap_or_else(|_| label.to_string());
    let supported = device
        .default_input_config()
        .map_err(|e| AppError::Audio(format!("no default input config for {device_name}: {e}")))?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();
    let src_rate = supported.sample_rate().0;
    let config: StreamConfig = supported.into();

    let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

    let err_label = device_name.clone();
    let err_fn = move |err| tracing::error!("cpal stream error on {err_label}: {err}");

    let push_chunk = {
        let buffer = buffer.clone();
        move |mono_i16: Vec<i16>| {
            let resampled = resample_to_target(&mono_i16, src_rate);
            if resampled.is_empty() {
                return;
            }
            let mut guard = buffer.lock().unwrap();
            guard.extend_from_slice(&resampled);
            let _ = TARGET_RATE;
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => {
            let cb_push = push_chunk.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        let i16s = f32_to_i16(data);
                        let mono = downmix_to_mono(&i16s, channels);
                        cb_push(mono);
                    },
                    err_fn.clone(),
                    None,
                )
                .map_err(|e| AppError::Audio(format!("build f32 stream failed: {e}")))?
        }
        SampleFormat::I16 => {
            let cb_push = push_chunk.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let mono = downmix_to_mono(data, channels);
                        cb_push(mono);
                    },
                    err_fn.clone(),
                    None,
                )
                .map_err(|e| AppError::Audio(format!("build i16 stream failed: {e}")))?
        }
        SampleFormat::U16 => {
            let cb_push = push_chunk.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        let i16s = u16_to_i16(data);
                        let mono = downmix_to_mono(&i16s, channels);
                        cb_push(mono);
                    },
                    err_fn.clone(),
                    None,
                )
                .map_err(|e| AppError::Audio(format!("build u16 stream failed: {e}")))?
        }
        other => {
            return Err(AppError::Audio(format!("unsupported sample format: {other:?}")));
        }
    };

    stream
        .play()
        .map_err(|e| AppError::Audio(format!("cannot start stream: {e}")))?;

    Ok(InputCapture {
        _stream: stream,
        device_name,
        buffer,
    })
}
