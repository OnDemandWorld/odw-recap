//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::error::{RecapError, Result};
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use uuid::Uuid;

/// Maximum accepted upload size (2 GiB) to bound memory/disk usage.
const MAX_UPLOAD_BYTES: usize = 2 * 1024 * 1024 * 1024;

const SUPPORTED_EXTENSIONS: &[&str] = &["m4a", "wav", "mp3", "ogg", "webm", "flac"];

pub struct HttpUploadServer {
    port: u16,
    inbox_path: PathBuf,
}

impl HttpUploadServer {
    pub fn new(port: u16, inbox_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&inbox_path)?;
        Ok(Self { port, inbox_path })
    }

    /// Start the upload server.
    ///
    /// The server binds to the loopback interface only: it exists to receive
    /// transfers from the user's own devices and has no authentication, so it
    /// must never be exposed on all interfaces.
    pub async fn start(self) -> Result<()> {
        let app = Router::new()
            .route("/upload/audio", post(upload_audio))
            .with_state(self.inbox_path.clone());

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            RecapError::AudioInput(format!("Failed to bind HTTP server: {}", e))
        })?;

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("HTTP upload server error: {}", e);
            }
        });

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

async fn upload_audio(
    inbox_path: State<PathBuf>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_data: Option<Vec<u8>> = None;
    let mut ext = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" {
            continue;
        }

        let filename = field.file_name().unwrap_or("recording.m4a").to_string();
        let candidate_ext = PathBuf::from(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !SUPPORTED_EXTENSIONS.contains(&candidate_ext.as_str()) {
            return (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Json(json!({
                    "status": "error",
                    "message": format!(
                        "Unsupported file type '.{}'. Supported: {}",
                        candidate_ext,
                        SUPPORTED_EXTENSIONS.join(", ")
                    )
                })),
            );
        }

        match field.bytes().await {
            Ok(bytes) if bytes.len() <= MAX_UPLOAD_BYTES => {
                file_data = Some(bytes.to_vec());
                ext = candidate_ext;
            }
            Ok(_) => {
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(json!({
                        "status": "error",
                        "message": "Uploaded file is too large"
                    })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "error",
                        "message": format!("Failed to read upload: {}", e)
                    })),
                );
            }
        }
    }

    let Some(data) = file_data else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": "No file provided (expected multipart field 'file')"
            })),
        );
    };

    // Use only the validated extension; the upload is stored under a random
    // UUID name so the client-supplied filename never touches the filesystem.
    let uuid = Uuid::new_v4();
    let path = inbox_path.join(format!("{}.{}", uuid, ext));
    if let Err(e) = tokio::fs::write(&path, data).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to save file: {}", e)
            })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "uploaded",
            "message": "Audio uploaded successfully",
            "path": path.to_string_lossy().to_string()
        })),
    )
}
