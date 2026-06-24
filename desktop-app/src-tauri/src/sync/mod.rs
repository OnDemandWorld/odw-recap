pub mod client;
pub mod types;

use crate::error::Result;
use std::sync::Arc;
use std::sync::Mutex;

pub use client::SyncClient;

/// Sync engine for managing data synchronization with the team server and Vault/Loop
pub struct SyncEngine {
    client: Arc<Mutex<SyncClient>>,
}

impl SyncEngine {
    pub fn new(server_url: String, api_token: Option<String>) -> Self {
        Self {
            client: Arc::new(Mutex::new(SyncClient::new(server_url, api_token))),
        }
    }

    /// Sync a meeting to the team server
    pub fn sync_meeting(&self, meeting_id: &str) -> Result<()> {
        let client = self.client.lock().map_err(|_| {
            crate::error::RecapError::Sync("Failed to lock sync client".to_string())
        })?;
        client.sync_meeting(meeting_id)
    }

    /// Queue a meeting for sync
    pub fn queue_meeting(&self, meeting_id: &str) -> Result<()> {
        // In a real implementation, this would add to a local queue
        // For now, just sync directly
        self.sync_meeting(meeting_id)
    }

    /// Check sync status
    pub fn get_sync_status(&self, meeting_id: &str) -> Result<String> {
        let client = self.client.lock().map_err(|_| {
            crate::error::RecapError::Sync("Failed to lock sync client".to_string())
        })?;
        client.get_sync_status(meeting_id)
    }
}
