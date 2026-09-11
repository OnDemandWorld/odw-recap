use crate::error::{RecapError, Result};
use axum::{
    extract::{Multipart, Query, State},
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use uuid::Uuid;

/// Upload server shared secret. It is embedded in the URL shown to the user
/// (e.g. `http://192.168.1.5:8765/upload/audio?token=...`), so a phone can
/// upload without an app-side login — but random per server start, and the
/// endpoint rejects requests that do not present it. This keeps the LAN
/// upload feature from being an open write-on-my-disk endpoint for every
/// device on the network.
#[derive(Clone)]
struct UploadState {
    inbox_path: PathBuf,
    token: String,
}

pub struct HttpUploadServer {
    port: u16,
    inbox_path: PathBuf,
    token: String,
}

impl HttpUploadServer {
    pub fn new(port: u16, inbox_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&inbox_path)?;
        Ok(Self {
            port,
            inbox_path,
            token: Uuid::new_v4().to_string(),
        })
    }

    /// Bind and serve until the task is aborted. Runs on the caller's async
    /// runtime (tauri::async_runtime::spawn) — failures are returned, never
    /// unwrapped, so a busy port surfaces as an error instead of a panic.
    pub async fn run(self) -> Result<()> {
        let state = UploadState {
            inbox_path: self.inbox_path,
            token: self.token,
        };
        let app = Router::new()
            .route("/upload/audio", post(upload_audio))
            .with_state(state);

        // 0.0.0.0 is intentional: the whole feature is "upload from another
        // device on the local network". Access is gated by the token.
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| RecapError::AudioInput(format!("Failed to bind HTTP server: {}", e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| RecapError::AudioInput(format!("HTTP server failed: {}", e)))?;
        Ok(())
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Best-effort LAN address of this machine for display in the UI.
    fn lan_ip(&self) -> String {
        // Opening a UDP "connection" does not send packets; it just makes the
        // OS pick the outbound interface for a default route.
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    return addr.ip().to_string();
                }
            }
        }
        "127.0.0.1".to_string()
    }

    /// Full upload URL including the token, ready to paste into a phone.
    pub fn upload_url(&self) -> String {
        format!(
            "http://{}:{}/upload/audio?token={}",
            self.lan_ip(),
            self.port,
            self.token
        )
    }
}

/// Authorize the request: token must be present either as the `token` query
/// parameter or the `X-Upload-Token` header.
fn authorized(query: &HashMap<String, String>, headers: &axum::http::HeaderMap, token: &str) -> bool {
    if query.get("token").map(String::as_str) == Some(token) {
        return true;
    }
    headers
        .get("X-Upload-Token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == token)
        .unwrap_or(false)
}

async fn upload_audio(
    State(state): State<UploadState>,
    Query(query): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if !authorized(&query, &headers, &state.token) {
        return Json(json!({
            "status": "error",
            "message": "Invalid or missing upload token"
        }));
    }

    let mut file_data = None;
    let mut ext = "m4a".to_string();

    while let Some(field) = multipart.next_field().await.ok().flatten() {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("recording.m4a").to_string();
            ext = PathBuf::from(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("m4a")
                .to_ascii_lowercase();
            file_data = field.bytes().await.ok();
        }
    }

    if let Some(data) = file_data {
        let uuid = Uuid::new_v4();
        let path = state.inbox_path.join(format!("{}.{}", uuid, ext));
        if let Err(e) = tokio::fs::write(&path, &data).await {
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
