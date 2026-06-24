use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum RecapError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Audio input error: {0}")]
    AudioInput(String),

    #[error("Transcription error: {0}")]
    Transcription(String),

    #[error("Summarization error: {0}")]
    Summarization(String),

    #[error("Provider error ({provider}): {message}")]
    Provider { provider: String, message: String },

    #[error("IO error: {0}")]
    Io(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for RecapError {
    fn from(e: std::io::Error) -> Self {
        RecapError::Io(e.to_string())
    }
}

impl From<rusqlite::Error> for RecapError {
    fn from(e: rusqlite::Error) -> Self {
        RecapError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for RecapError {
    fn from(e: serde_json::Error) -> Self {
        RecapError::Json(e.to_string())
    }
}

impl From<anyhow::Error> for RecapError {
    fn from(e: anyhow::Error) -> Self {
        RecapError::Unknown(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RecapError>;
