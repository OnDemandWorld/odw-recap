use crate::error::{RecapError, Result};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::storage::StorageManager;

/// Extensions the upload endpoint accepts (mirrors AudioInputManager's list —
/// R-2: arbitrary extensions previously landed unchecked in the inbox).
const SUPPORTED_EXTENSIONS: &[&str] = &["m4a", "wav", "mp3", "ogg", "webm", "flac"];

/// Maximum accepted upload size (500 MiB). axum's Multipart defaults to a
/// 2 MiB body limit — real recordings (an hour of m4a is ~30 MiB) were all
/// rejected before this was raised.
const MAX_UPLOAD_BYTES: usize = 500 * 1024 * 1024;

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
    storage: Arc<Mutex<StorageManager>>,
    stt_provider: String,
    llm_provider: String,
}

pub struct HttpUploadServer {
    port: u16,
    inbox_path: PathBuf,
    token: String,
    storage: Arc<Mutex<StorageManager>>,
    stt_provider: String,
    llm_provider: String,
}

impl HttpUploadServer {
    pub fn new(
        port: u16,
        inbox_path: PathBuf,
        storage: Arc<Mutex<StorageManager>>,
        stt_provider: String,
        llm_provider: String,
    ) -> Result<Self> {
        std::fs::create_dir_all(&inbox_path)?;
        Ok(Self {
            port,
            inbox_path,
            token: Uuid::new_v4().to_string(),
            storage,
            stt_provider,
            llm_provider,
        })
    }

    /// Bind and serve until the task is aborted. Runs on the caller's async
    /// runtime (tauri::async_runtime::spawn) — failures are returned, never
    /// unwrapped, so a busy port surfaces as an error instead of a panic.
    /// Serve on an already-bound listener. Audit #9: the caller binds the
    /// socket synchronously so a busy port surfaces as an error from
    /// start_http_server instead of being logged inside a detached task while
    /// the UI shows a URL for a server that never started.
    pub async fn run_with_listener(self, listener: tokio::net::TcpListener) -> Result<()> {
        let state = UploadState {
            inbox_path: self.inbox_path,
            token: self.token,
            storage: self.storage,
            stt_provider: self.stt_provider,
            llm_provider: self.llm_provider,
        };
        let app = Router::new()
            // GET serves a minimal upload page (R-2): the USER-GUIDE flow is
            // "open this URL on your phone" — a bare 405 broke the documented
            // first step.
            .route("/upload/audio", get(upload_page).post(upload_audio))
            .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
            .with_state(state);

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
    // Audit #6: authorize BEFORE buffering the file body (up to 500 MiB).
    // Previously the whole multipart was buffered and only then checked, so an
    // unauthenticated LAN device could exhaust memory with concurrent large
    // bodies. Query/header token is checked up front; the browser form posts
    // the token as its first field, so we validate it on arrival and refuse to
    // read any file bytes until authorized.
    let mut pre_authorized = authorized(&query, &headers, &state.token);

    let mut file_data: Option<Result<Vec<u8>>> = None;
    let mut ext = "m4a".to_string();

    // Multipart errors are surfaced (previously .ok() swallowed them into a
    // misleading "No file provided").
    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            return Json(json!({
                "status": "error",
                "message": format!("Malformed multipart upload: {}", e)
            }));
        }
    } {
        let name = field.name().unwrap_or("").to_string();
        if name == "token" {
            if let Ok(value) = field.text().await {
                if value == state.token {
                    pre_authorized = true;
                } else if !pre_authorized {
                    return Json(json!({
                        "status": "error",
                        "message": "Invalid or missing upload token"
                    }));
                }
            }
            continue;
        }
        if name == "file" {
            if !pre_authorized {
                // Reject before allocating the body — no unauthenticated buffering.
                return Json(json!({
                    "status": "error",
                    "message": "Invalid or missing upload token"
                }));
            }
            let filename = field.file_name().unwrap_or("recording.m4a").to_string();
            ext = PathBuf::from(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("m4a")
                .to_ascii_lowercase();
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|e| RecapError::AudioInput(format!("Failed to read upload: {}", e))),
            );
        }
    }

    if !pre_authorized {
        return Json(json!({
            "status": "error",
            "message": "Invalid or missing upload token"
        }));
    }

    let data = match file_data {
        Some(Ok(data)) => data,
        Some(Err(e)) => {
            return Json(json!({
                "status": "error",
                "message": format!("{}", e)
            }));
        }
        None => {
            return Json(json!({
                "status": "error",
                "message": "No file provided"
            }));
        }
    };

    // R-2: reject unsupported extensions up front.
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Json(json!({
            "status": "error",
            "message": format!(
                "Unsupported file type '.{}'. Supported: {}",
                ext,
                SUPPORTED_EXTENSIONS.join(", ")
            )
        }));
    }

    let uuid = Uuid::new_v4();
    let path = state.inbox_path.join(format!("{}.{}", uuid, ext));
    if let Err(e) = tokio::fs::write(&path, &data).await {
        return Json(json!({
            "status": "error",
            "message": format!("Failed to save file: {}", e)
        }));
    }

    // R-2: create the REAL library meeting so the upload shows up in Recap and
    // gets processed — previously the file landed in the inbox with a fake
    // meeting id that existed nowhere.
    let title = "Phone upload".to_string();
    let meeting_result = crate::audio_input::create_meeting_for_inbox_file(
        &state.storage,
        &path,
        &title,
        &ext,
        crate::storage::types::AudioSource::MobileUpload,
        &state.stt_provider,
        &state.llm_provider,
    );
    match meeting_result {
        Ok(meeting_id) => Json(json!({
            "status": "uploaded",
            "meeting_id": meeting_id,
            "message": "Uploaded — the meeting now appears in your Recap library.",
            "path": path.to_string_lossy().to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("File saved but library registration failed: {}", e)
        })),
    }
}

/// Minimal upload page for phones (R-2): the documented flow starts by
/// OPENING the upload URL in a browser.
async fn upload_page(
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = query.get("token").cloned().unwrap_or_default();
    Html(upload_page_html(&token))
}

/// Escape the five HTML-significant characters so attacker-controlled text
/// (the `token` query parameter is reflected into an attribute) cannot break
/// out of its context. Audit #1: the raw token was previously interpolated
/// unescaped, giving unauthenticated reflected XSS on the LAN-bound service.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

/// Pure renderer for the upload page, separated from the axum handler so the
/// escaping can be unit-tested. The token is HTML-escaped before interpolation.
fn upload_page_html(token: &str) -> String {
    let token = html_escape(token);
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>ODW Recap — Upload recording</title>
<style>body{{font-family:system-ui;margin:2rem auto;max-width:28rem;padding:0 1rem}}
button{{width:100%;padding:.9rem;font-size:1rem;margin-top:1rem}}</style></head>
<body><h2>Upload a recording</h2>
<p>Select an audio file (m4a, wav, mp3, ogg, webm, flac). It will appear as a new meeting in ODW Recap.</p>
<form method="post" action="/upload/audio" enctype="multipart/form-data">
<input type="hidden" name="token" value="{token}">
<input type="file" name="file" accept=".m4a,.wav,.mp3,.ogg,.webm,.flac" required style="width:100%;padding:.6rem">
<button type="submit">Upload</button>
</form></body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression for audit #1: a malicious token must not be able to inject
    // markup/script into the reflected upload page.
    #[test]
    fn upload_page_escapes_hostile_token() {
        let evil = "\"><script>alert(1)</script>";
        let html = upload_page_html(evil);
        assert!(!html.contains("<script>alert(1)</script>"), "raw script leaked: {}", html);
        assert!(!html.contains("\"><script>"), "attribute breakout leaked");
        assert!(html.contains("&quot;&gt;&lt;script&gt;"), "token not escaped: {}", html);
    }

    #[test]
    fn html_escape_covers_all_special_chars() {
        assert_eq!(html_escape("&<>\"'"), "&amp;&lt;&gt;&quot;&#x27;");
        assert_eq!(html_escape("plain-token-123"), "plain-token-123");
    }
}
