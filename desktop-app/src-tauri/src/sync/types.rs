//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Sync status for a meeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    NotSynced,
    Pending,
    Syncing,
    Synced,
    Failed,
}

/// Sync queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueItem {
    pub id: i64,
    pub meeting_id: String,
    pub action: String,
    pub payload: String,
    pub status: String,
    pub retry_count: i32,
    pub created_at: i64,
    pub processed_at: Option<i64>,
}

/// Mobile companion app sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncRequest {
    pub device_id: String,
    pub meeting_id: Option<String>,
    pub audio_file_url: Option<String>,
}

/// iOS Shortcuts integration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOSShortcutsRequest {
    pub shortcut_name: String,
    pub meeting_id: String,
    pub params: std::collections::HashMap<String, String>,
}
