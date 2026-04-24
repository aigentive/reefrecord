use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use crate::error::{AppError, AppResult};

pub const BLACKHOLE_FRAGMENTS: &[&str] = &["blackhole", "black hole"];

#[cfg(target_os = "macos")]
const HAL_PLUGIN_DIR: &str = "/Library/Audio/Plug-Ins/HAL";
#[cfg(target_os = "macos")]
const BLACKHOLE_DRIVER_BUNDLES: &[&str] =
    &["BlackHole2ch.driver", "BlackHole16ch.driver", "BlackHole64ch.driver"];

/// Returns true if a BlackHole `.driver` bundle is installed on disk even when
/// CoreAudio hasn't loaded it yet. Matches the hint shown by `recorder.py`:
/// "BlackHole is installed on disk, but CoreAudio has not loaded it yet — reboot".
#[cfg(target_os = "macos")]
pub fn blackhole_driver_installed() -> bool {
    BLACKHOLE_DRIVER_BUNDLES
        .iter()
        .any(|bundle| std::path::Path::new(HAL_PLUGIN_DIR).join(bundle).exists())
}

#[cfg(not(target_os = "macos"))]
pub fn blackhole_driver_installed() -> bool {
    false
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub input_channels: u32,
    pub is_default: bool,
    pub is_blackhole: bool,
}

pub fn list_input_devices() -> AppResult<Vec<AudioDevice>> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    let devices = host
        .input_devices()
        .map_err(|e| AppError::Audio(e.to_string()))?;
    let mut out = Vec::new();
    for dev in devices {
        let name = match dev.name() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let channels = dev
            .default_input_config()
            .map(|c| c.channels() as u32)
            .unwrap_or(0);
        let is_blackhole = name_is_blackhole(&name);
        let is_default = !default_id.is_empty() && name == default_id;
        out.push(AudioDevice {
            id: name.clone(),
            name,
            input_channels: channels,
            is_default,
            is_blackhole,
        });
    }
    out.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

pub fn name_is_blackhole(name: &str) -> bool {
    let lower = name.to_lowercase();
    BLACKHOLE_FRAGMENTS.iter().any(|f| lower.contains(f))
}

pub fn find_blackhole_device(devices: &[AudioDevice]) -> Option<&AudioDevice> {
    devices.iter().find(|d| d.is_blackhole && d.input_channels > 0)
}

/// Resolve a device by numeric index (position in list) or case-insensitive name fragment.
pub fn resolve_device_selector<'a>(
    devices: &'a [AudioDevice],
    selector: Option<&str>,
) -> Option<&'a AudioDevice> {
    let sel = selector?.trim();
    if sel.is_empty() {
        return None;
    }
    if let Ok(idx) = sel.parse::<usize>() {
        return devices.get(idx);
    }
    let lower = sel.to_lowercase();
    devices.iter().find(|d| d.name.to_lowercase().contains(&lower))
}
