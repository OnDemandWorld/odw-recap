use crate::error::{RecapError, Result};
use axum::{
    extract::Multipart,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct HttpUploadServer {
    port: u16,
    inbox_path: PathBuf,
}

impl HttpUploadServer {
    pub fn new(port: u16, inbox_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&inbox_path)?;
        Ok(Self { port, inbox_path })
    }

    pub async fn start(self) -> Result<()> {
        let app = Router::new().route("/upload/audio", post(upload_audio)).with_state(self.inbox_path.clone());

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            RecapError::AudioInput(format!("Failed to bind HTTP server: {}", e))
        })?;

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

async fn upload_audio(
    inbox_path: axum::extract::State<PathBuf>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_data = None;
    let mut ext = "m4a".to_string();

    while let Some(field) = multipart.next_field().await.ok().flatten() {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field
                .file_name()
                .unwrap_or("recording.m4a")
                .to_string();
            ext = PathBuf::from(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("m4a")
                .to_string();

            let bytes = field.bytes().await.ok();
            file_data = bytes;
        }
    }

    if let Some(data) = file_data {
        let uuid = Uuid::new_v4();
        let path = inbox_path.join(format!("{}.{}", uuid, ext));
        if let Err(e) = tokio::fs::write(&path, data).await {
            return Json(json!({
                "status": "error",
                "message": format!("Failed to save file: {}", e)
            }));
        }

        return Json(json!({
            "status": "uploaded",
            "meeting_id": uuid.to_string(),
            "message": "Audio uploaded successfully",
            "path": path.to_string_lossy().to_string()
        }));
    }

    Json(json!({
        "status": "error",
        "message": "No file provided"
    }))
}
