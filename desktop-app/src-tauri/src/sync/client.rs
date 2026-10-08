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
    if s.len() > 300 {
        format!("{}…", &s[..300])
    } else {
        s.to_string()
    }
}
