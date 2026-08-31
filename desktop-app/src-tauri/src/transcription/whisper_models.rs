//! Whisper model registry, path resolution, and resumable downloads.
//!
//! Models are the GGML weights published by the whisper.cpp project on
//! Hugging Face. They are stored under `<data_dir>/models/` and downloaded
//! on demand with resume support.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{RecapError, Result};

const MODEL_URL_PREFIX: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-";

/// A known whisper.cpp model.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WhisperModelSpec {
    pub name: &'static str,
    /// Approximate download size in bytes (for display only).
    pub size_bytes: u64,
    pub description: &'static str,
}

/// Recommended order for the UI: fastest/smallest first.
pub const WHISPER_MODELS: &[WhisperModelSpec] = &[
    WhisperModelSpec {
        name: "tiny",
        size_bytes: 75_000_000,
        description: "Fastest, lowest accuracy (~1 GB RAM)",
    },
    WhisperModelSpec {
        name: "base",
        size_bytes: 142_000_000,
        description: "Good balance for short recordings (~1 GB RAM)",
    },
    WhisperModelSpec {
        name: "small",
        size_bytes: 466_000_000,
        description: "Solid accuracy on laptops (~2 GB RAM)",
    },
    WhisperModelSpec {
        name: "medium",
        size_bytes: 1_500_000_000,
        description: "High accuracy, slower (~5 GB RAM)",
    },
    WhisperModelSpec {
        name: "large-v3-turbo",
        size_bytes: 1_600_000_000,
        description: "Recommended: near-large accuracy, much faster (~6 GB RAM)",
    },
    WhisperModelSpec {
        name: "large-v3",
        size_bytes: 3_100_000_000,
        description: "Best accuracy, slowest (~10 GB RAM)",
    },
];

/// Model status as surfaced to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct WhisperModelInfo {
    pub name: String,
    pub size_bytes: u64,
    pub description: String,
    pub installed: bool,
}

pub fn spec_for(name: &str) -> Option<&'static WhisperModelSpec> {
    WHISPER_MODELS.iter().find(|m| m.name == name)
}

/// Directory where whisper models live.
pub fn models_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models")
}

/// Path of a model file, whether or not it has been downloaded.
pub fn model_file(data_dir: &Path, name: &str) -> PathBuf {
    models_dir(data_dir).join(format!("ggml-{}.bin", name))
}

/// Default model for new installations.
pub const DEFAULT_MODEL: &str = "base";

/// List every known model with its installed status.
pub fn list_models(data_dir: &Path) -> Vec<WhisperModelInfo> {
    WHISPER_MODELS
        .iter()
        .map(|spec| WhisperModelInfo {
            name: spec.name.to_string(),
            size_bytes: spec.size_bytes,
            description: spec.description.to_string(),
            installed: model_file(data_dir, spec.name).exists(),
        })
        .collect()
}

/// Download a model with resume support. `on_progress` receives
/// `(downloaded_bytes, total_bytes_estimate)` after each chunk; totals are
/// approximate because registries do not always report exact sizes.
pub async fn download_model<F>(data_dir: &Path, name: &str, on_progress: F) -> Result<PathBuf>
where
    F: Fn(u64, u64) + Send + 'static,
{
    let spec = spec_for(name).ok_or_else(|| {
        RecapError::Transcription(format!("Unknown whisper model: '{}'", name))
    })?;

    std::fs::create_dir_all(models_dir(data_dir))?;
    let dest = model_file(data_dir, name);
    if dest.exists() {
        return Ok(dest); // already downloaded
    }

    let part = dest.with_extension("bin.part");
    let already = if part.exists() {
        std::fs::metadata(&part)?.len()
    } else {
        0
    };

    let url = format!("{}{}.bin", MODEL_URL_PREFIX, spec.name);
    let client = reqwest::Client::new();
    let mut request = client.get(&url);
    if already > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={}-", already));
    }

    let response = request.send().await.map_err(|e| {
        RecapError::Transcription(format!("Model download failed: {}", e))
    })?;

    let status = response.status();
    let mut downloaded: u64 = 0;
    let mut options = std::fs::OpenOptions::new().write(true).create(true).to_owned();

    if status == reqwest::StatusCode::PARTIAL_CONTENT {
        downloaded = already;
        options.append(true);
    } else if status.is_success() {
        // Server ignored Range (or fresh download): start over.
        options.truncate(true);
    } else {
        return Err(RecapError::Transcription(format!(
            "Model download failed with HTTP {}",
            status
        )));
    }

    let total_estimate = response
        .content_length()
        .map(|len| len + downloaded)
        .unwrap_or(spec.size_bytes);

    {
        use std::io::Write;
        let mut file = options.open(&part).map_err(|e| {
            RecapError::Transcription(format!("Cannot open model file: {}", e))
        })?;

        let mut stream = response;
        while let Some(chunk) = stream.chunk().await.map_err(|e| {
            RecapError::Transcription(format!("Model download interrupted: {}", e))
        })? {
            file.write_all(&chunk).map_err(|e| {
                RecapError::Transcription(format!("Failed to write model file: {}", e))
            })?;
            downloaded += chunk.len() as u64;
            on_progress(downloaded, total_estimate);
        }
        file.flush()?;
    }

    std::fs::rename(&part, &dest)?;
    on_progress(downloaded, downloaded);
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_paths_and_listing() {
        let dir = tempfile::TempDir::new().unwrap();
        let data_dir = dir.path();

        assert_eq!(
            model_file(data_dir, "base"),
            data_dir.join("models").join("ggml-base.bin")
        );

        let listing = list_models(data_dir);
        assert_eq!(listing.len(), WHISPER_MODELS.len());
        assert!(listing.iter().all(|m| !m.installed));

        // Install a fake model file and check status flips.
        std::fs::create_dir_all(models_dir(data_dir)).unwrap();
        std::fs::write(model_file(data_dir, "base"), b"fake").unwrap();
        let listing = list_models(data_dir);
        let base = listing.iter().find(|m| m.name == "base").unwrap();
        assert!(base.installed);
    }

    #[test]
    fn test_spec_lookup() {
        assert!(spec_for("base").is_some());
        assert!(spec_for("large-v3-turbo").is_some());
        assert!(spec_for("nonexistent").is_none());
        assert_eq!(DEFAULT_MODEL, "base");
    }
}
