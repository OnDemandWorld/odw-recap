pub mod client;
pub mod types;

use crate::error::{RecapError, Result};
use std::sync::Arc;
use std::sync::Mutex;

pub use client::SyncClient;

/// Sync engine for managing data synchronization with the team server and Vault/Loop.
///
/// The engine owns the authenticated client (JWT stored after
/// `login_team_server`); the Tauri command layer gathers the meeting payload
/// from storage and hands it to `sync_meeting`.
pub struct SyncEngine {
    client: Arc<Mutex<SyncClient>>,
}

impl SyncEngine {
    pub fn new(server_url: String, api_token: Option<String>) -> Self {
        Self {
            client: Arc::new(Mutex::new(SyncClient::new(server_url, api_token))),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, SyncClient>> {
        self.client
            .lock()
            .map_err(|_| RecapError::Sync("Failed to lock sync client".to_string()))
    }

    /// Point the engine at a (different) team server, keeping or clearing
    /// the session token.
    pub fn reconfigure(&self, server_url: String, token: Option<String>) -> Result<()> {
        let mut client = self.lock()?;
        *client = SyncClient::new(server_url, token);
        Ok(())
    }

    /// Authenticate and store the returned JWT for subsequent syncs.
    pub async fn login(&self, email: &str, password: &str) -> Result<String> {
        // Clone a client snapshot (reqwest::Client is an Arc handle) so the
        // MutexGuard never spans an await — guards are !Send.
        // R-9: allow re-login with an existing (possibly EXPIRED) token —
        // previously users hit "already logged in — log out first" after the
        // 24h JWT expiry with no way to know why syncs were failing.
        let snapshot = self.lock()?.clone();
        let token = snapshot.login(email, password).await?;
        self.lock()?.set_token(Some(token.clone()));
        Ok(token)
    }

    pub fn logout(&self) -> Result<()> {
        self.lock()?.set_token(None);
        Ok(())
    }

    pub fn is_logged_in(&self) -> Result<bool> {
        Ok(self.lock()?.has_token())
    }

    /// Push a full meeting payload to the team server.
    pub async fn sync_meeting(
        &self,
        meeting_id: &str,
        meeting_data: std::collections::HashMap<String, String>,
        transcript_data: serde_json::Value,
        summary_data: serde_json::Value,
    ) -> Result<client::SyncResponse> {
        let snapshot = self.lock()?.clone();
        let result = snapshot
            .sync_meeting(meeting_id, meeting_data, transcript_data, summary_data)
            .await;
        // R-9: a 401 means the JWT expired — drop it so the UI forces a fresh
        // login instead of retrying a dead token forever (previously the stale
        // token also BLOCKED re-login with "log out first").
        if let Err(RecapError::Sync(msg)) = &result {
            if msg.contains("HTTP 401") {
                let _ = self.lock().map(|mut inner| inner.set_token(None));
            }
        }
        result
    }
}
