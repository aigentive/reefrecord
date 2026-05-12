use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Flac,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self::Wav
    }
}

impl AudioFormat {
    pub fn from_path(path: &Path) -> AppResult<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("wav") => Ok(Self::Wav),
            Some(ext) if ext.eq_ignore_ascii_case("flac") => Ok(Self::Flac),
            _ => Err(AppError::Audio(format!(
                "unsupported audio format for {}",
                path.display()
            ))),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Wav => "WAV",
            Self::Flac => "FLAC",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Wav => "audio/wav",
            Self::Flac => "audio/flac",
        }
    }
}
