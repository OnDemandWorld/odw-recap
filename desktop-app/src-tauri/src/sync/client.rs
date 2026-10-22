use crate::error::{RecapError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client for syncing data with the team server.
///
/// The desktop app authenticates via `POST /auth/login` (JWT) and pushes a
/// full meeting payload to `POST /sync`; the team-server forwards to Vault
/// (knowledge) and Loop (action-item workflows). See INTEGRATION_CONTRACT §3.
#[derive(Clone)]
pub struct SyncClient {
    server_url: String,
    api_token: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncRequest {
    meeting_id: String,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    meeting_data: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transcript_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary_data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub status: String,
    #[serde(default)]
    pub meeting_id: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
}

fn clean_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

impl SyncClient {
    pub fn new(server_url: String, api_token: Option<String>) -> Self {
        Self {
            server_url: clean_base_url(&server_url),
            api_token,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.api_token = token;
    }

    pub fn has_token(&self) -> bool {
        self.api_token.is_some()
    }

    /// Authenticate against the team server and return the JWT.
    pub async fn login(&self, email: &str, password: &str) -> Result<String> {
        let resp = self
            .client
            .post(format!("{}/auth/login", self.server_url))
            .json(&serde_json::json!({
                "email": email,
                "password": password,
            }))
            .send()
            .await
            .map_err(|e| RecapError::Sync(format!("login request failed: {}", e)))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| RecapError::Sync(format!("login response read failed: {}", e)))?;
        if !status.is_success() {
            return Err(RecapError::Sync(format!(
                "login failed (HTTP {}): {}",
                status,
                truncate(&body)
            )));
        }
        let parsed: LoginResponse = serde_json::from_str(&body)
            .map_err(|e| RecapError::Sync(format!("login response parse failed: {}", e)))?;
        Ok(parsed.token)
    }

    /// Push a full meeting payload to the team server (`POST /sync`).
    pub async fn sync_meeting(
        &self,
        meeting_id: &str,
        meeting_data: HashMap<String, String>,
        transcript_data: serde_json::Value,
        summary_data: serde_json::Value,
    ) -> Result<SyncResponse> {
        let token = self
            .api_token
            .clone()
            .ok_or_else(|| RecapError::Sync("not logged in to team server".to_string()))?;

        // R-5: upsert the meeting row first — POST /meetings with a client
        // id makes /sync reference a real meeting (409 on the server for an
        // existing id is fine). Without this the meeting never showed up in
        // the team-server library and admin stats.
        let mut meeting_body = serde_json::Map::new();
        for (k, v) in &meeting_data {
            meeting_body.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        meeting_body.insert(
            "id".to_string(),
            serde_json::Value::String(meeting_id.to_string()),
        );
        if !meeting_body
            .get("title")
            .map(|v| !v.as_str().unwrap_or("").trim().is_empty())
            .unwrap_or(false)
        {
            meeting_body.insert(
                "title".to_string(),
                serde_json::Value::String("Synced meeting".to_string()),
            );
        }
        let upsert = self
            .client
            .post(format!("{}/meetings/", self.server_url))
            .bearer_auth(&token)
            .json(&serde_json::Value::Object(meeting_body))
            .send()
            .await
            .map_err(|e| RecapError::Sync(format!("meeting upsert request failed: {}", e)))?;
        // 2xx = created, 409 = already exists (team-server semantics) — both
        // let the /sync below reference a real row. Other statuses: the server
        // side of /sync falls back to a NULL-meeting queue entry, so surface
        // the failure instead of silently proceeding.
        let upsert_status = upsert.status();
        if !upsert_status.is_success() && upsert_status.as_u16() != 409 {
            let body = upsert.text().await.unwrap_or_default();
            return Err(RecapError::Sync(format!(
                "meeting upsert failed (HTTP {}): {}",
                upsert_status,
                truncate(&body)
            )));
        }

        let req = SyncRequest {
            meeting_id: meeting_id.to_string(),
            action: "full_sync".to_string(),
            meeting_data: Some(meeting_data),
            transcript_data: Some(transcript_data),
            summary_data: Some(summary_data),
        };

        let resp = self
            .client
            .post(format!("{}/sync", self.server_url))
            .bearer_auth(token)
            .json(&req)
            .send()
            .await
            .map_err(|e| RecapError::Sync(format!("sync request failed: {}", e)))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| RecapError::Sync(format!("sync response read failed: {}", e)))?;
        if !status.is_success() {
            return Err(RecapError::Sync(format!(
                "sync failed (HTTP {}): {}",
                status,
                truncate(&body)
            )));
        }
        serde_json::from_str(&body)
            .map_err(|e| RecapError::Sync(format!("sync response parse failed: {}", e)))
    }
}

fn truncate(s: &str) -> String {
    // R-8: char-boundary-safe truncation — a raw byte slice panicked whenever
    // a multibyte (e.g. Chinese) error body crossed the 300-byte cut.
    if s.len() > 300 {
        let mut end = 300;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    } else {
        s.to_string()
    }
}
