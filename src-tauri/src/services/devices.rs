use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use crate::error::{AppError, AppResult};

pub const BLACKHOLE_FRAGMENTS: &[&str] = &["blackhole", "black hole"];

#[cfg(target_os = "macos")]
const HAL_PLUGIN_DIR: &str = "/Library/Audio/Plug-Ins/HAL";
#[cfg(target_os = "macos")]
const BLACKHOLE_DRIVER_BUNDLES: &[&str] = &[
    "BlackHole2ch.driver",
    "BlackHole16ch.driver",
    "BlackHole64ch.driver",
];

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
    devices
        .iter()
        .find(|d| d.is_blackhole && d.input_channels > 0)
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
    devices
        .iter()
        .find(|d| d.name.to_lowercase().contains(&lower))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, channels: u32, is_default: bool) -> AudioDevice {
        AudioDevice {
            id: name.to_string(),
            name: name.to_string(),
            input_channels: channels,
            is_default,
            is_blackhole: name_is_blackhole(name),
        }
    }

    #[test]
    fn detects_blackhole_names_case_insensitively() {
        assert!(name_is_blackhole("BlackHole 2ch"));
        assert!(name_is_blackhole("Black Hole 16ch"));
        assert!(name_is_blackhole("my BLACKHOLE aggregate"));
        assert!(!name_is_blackhole("Studio Microphone"));
    }

    #[test]
    fn finds_blackhole_only_when_it_has_input_channels() {
        let devices = vec![
            device("BlackHole muted", 0, false),
            device("Studio Mic", 1, true),
            device("BlackHole 2ch", 2, false),
        ];

        assert_eq!(
            find_blackhole_device(&devices).map(|d| d.name.as_str()),
            Some("BlackHole 2ch")
        );
    }

    #[test]
    fn resolves_selector_by_index_or_name_fragment() {
        let devices = vec![
            device("Built-in Mic", 1, true),
            device("USB Studio Mic", 2, false),
        ];

        assert_eq!(
            resolve_device_selector(&devices, Some("1")).map(|d| d.name.as_str()),
            Some("USB Studio Mic")
        );
        assert_eq!(
            resolve_device_selector(&devices, Some("studio")).map(|d| d.name.as_str()),
            Some("USB Studio Mic")
        );
        assert!(resolve_device_selector(&devices, Some("")).is_none());
        assert!(resolve_device_selector(&devices, None).is_none());
        assert!(resolve_device_selector(&devices, Some("9")).is_none());
    }
}
