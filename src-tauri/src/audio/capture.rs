use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};

use crate::error::{AppError, AppResult};

use super::resample::{downmix_to_mono, f32_to_i16, resample_to_target, u16_to_i16};

/// A cpal input stream running on a dedicated OS thread. The public handle is
/// Send + Sync; the raw `Stream` is owned by the worker thread, matching cpal's
/// macOS `!Send` constraint.
pub struct InputCapture {
    device_name: String,
    buffer: Arc<Mutex<Vec<i16>>>,
    stop_flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl InputCapture {
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Signal the worker to stop, join it, and return the collected samples.
    pub fn stop(mut self) -> Vec<i16> {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        let mut guard = self.buffer.lock().unwrap();
        std::mem::take(&mut *guard)
    }
}

impl Drop for InputCapture {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

pub fn resolve_input_device(selector: Option<&str>, require_blackhole: bool) -> AppResult<Device> {
    let host = cpal::default_host();
    let devices: Vec<Device> = host
        .input_devices()
        .map_err(|e| AppError::Audio(e.to_string()))?
        .collect();
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

/// Start a capture. Moves device resolution into the worker thread since cpal
/// `Device` on macOS is not Send.
pub fn start_capture(
    selector: Option<String>,
    require_blackhole: bool,
    label: &'static str,
) -> AppResult<InputCapture> {
    // Resolve upfront only to surface a friendly name and fail fast if the
    // device is missing; re-resolve on the worker thread where Stream lives.
    let preview = resolve_input_device(selector.as_deref(), require_blackhole)?;
    let device_name = preview.name().unwrap_or_else(|_| label.to_string());
    drop(preview);

    let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let stop_flag = Arc::new(AtomicBool::new(false));

    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

    let thread_buffer = buffer.clone();
    let thread_stop = stop_flag.clone();
    let thread_selector = selector.clone();

    let handle = std::thread::Builder::new()
        .name(format!("reef-capture-{label}"))
        .spawn(move || {
            let device = match resolve_input_device(thread_selector.as_deref(), require_blackhole) {
                Ok(d) => d,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };

            let stream = match build_stream(device, thread_buffer) {
                Ok(s) => s,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = ready_tx.send(Err(format!("cannot start stream: {e}")));
                return;
            }
            let _ = ready_tx.send(Ok(()));

            while !thread_stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(100));
            }
            drop(stream);
        })
        .map_err(|e| AppError::Audio(format!("cannot spawn capture thread: {e}")))?;

    match ready_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(InputCapture {
            device_name,
            buffer,
            stop_flag,
            thread: Some(handle),
        }),
        Ok(Err(msg)) => {
            let _ = handle.join();
            Err(AppError::Audio(msg))
        }
        Err(RecvTimeoutError::Timeout) => {
            stop_flag.store(true, Ordering::SeqCst);
            let _ = handle.join();
            Err(AppError::Audio(
                "timed out waiting for audio stream to start".into(),
            ))
        }
        Err(RecvTimeoutError::Disconnected) => {
            let _ = handle.join();
            Err(AppError::Audio("capture thread exited".into()))
        }
    }
}

fn build_stream(device: Device, buffer: Arc<Mutex<Vec<i16>>>) -> Result<cpal::Stream, String> {
    let supported = device
        .default_input_config()
        .map_err(|e| format!("no default input config: {e}"))?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();
    let src_rate = supported.sample_rate().0;
    let config: StreamConfig = supported.into();
    let device_label = device.name().unwrap_or_else(|_| "input".into());

    let err_fn = {
        let label = device_label.clone();
        move |err| tracing::error!("cpal stream error on {label}: {err}")
    };

    let push = move |mono_i16: Vec<i16>| {
        let resampled = resample_to_target(&mono_i16, src_rate);
        if resampled.is_empty() {
            return;
        }
        let mut guard = buffer.lock().unwrap();
        guard.extend_from_slice(&resampled);
    };

    let stream = match sample_format {
        SampleFormat::F32 => {
            let push = push.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        let i16s = f32_to_i16(data);
                        let mono = downmix_to_mono(&i16s, channels);
                        push(mono);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("build f32 stream failed: {e}"))?
        }
        SampleFormat::I16 => {
            let push = push.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let mono = downmix_to_mono(data, channels);
                        push(mono);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("build i16 stream failed: {e}"))?
        }
        SampleFormat::U16 => {
            let push = push.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        let i16s = u16_to_i16(data);
                        let mono = downmix_to_mono(&i16s, channels);
                        push(mono);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("build u16 stream failed: {e}"))?
        }
        other => {
            return Err(format!("unsupported sample format: {other:?}"));
        }
    };

    Ok(stream)
}
