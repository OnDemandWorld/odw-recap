use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    pub id: Uuid,
    pub title: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub duration_seconds: Option<i32>,
    pub status: MeetingStatus,
    pub audio_file_path: PathBuf,
    pub audio_format: String,
    pub audio_size_bytes: i64,
    pub model_used: String,
    pub tags: Vec<String>,
    pub folder_id: Option<Uuid>,
    pub created_at: i64,
    pub updated_at: i64,
    pub sync_status: SyncStatus,
    pub meeting_type: Option<String>,
    pub location: Option<String>,
    pub participants: Vec<String>,
    pub language: String,
    pub topic: Option<String>,
    pub audio_source: AudioSource,
    pub stt_provider: String,
    pub llm_provider: String,
    pub prompt_template_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeetingStatus {
    Recording,
    Processing,
    Completed,
    Failed,
    Archived,
    Deleted,
}

impl fmt::Display for MeetingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MeetingStatus::Recording => "recording",
            MeetingStatus::Processing => "processing",
            MeetingStatus::Completed => "completed",
            MeetingStatus::Failed => "failed",
            MeetingStatus::Archived => "archived",
            MeetingStatus::Deleted => "deleted",
        };
        f.write_str(s)
    }
}

impl From<String> for MeetingStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "recording" => MeetingStatus::Recording,
            "processing" => MeetingStatus::Processing,
            "completed" => MeetingStatus::Completed,
            "failed" => MeetingStatus::Failed,
            "archived" => MeetingStatus::Archived,
            _ => MeetingStatus::Deleted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    NotSynced,
    Syncing,
    Synced,
    Failed,
    PermanentlyFailed,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SyncStatus::NotSynced => "not_synced",
            SyncStatus::Syncing => "syncing",
            SyncStatus::Synced => "synced",
            SyncStatus::Failed => "failed",
            SyncStatus::PermanentlyFailed => "permanently_failed",
        };
        f.write_str(s)
    }
}

impl From<String> for SyncStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "not_synced" => SyncStatus::NotSynced,
            "syncing" => SyncStatus::Syncing,
            "synced" => SyncStatus::Synced,
            "failed" => SyncStatus::Failed,
            _ => SyncStatus::PermanentlyFailed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioSource {
    SystemCapture,
    FileImport,
    MobileUpload,
    WatchFolder,
}

impl fmt::Display for AudioSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AudioSource::SystemCapture => "system_capture",
            AudioSource::FileImport => "file_import",
            AudioSource::MobileUpload => "mobile_upload",
            AudioSource::WatchFolder => "watch_folder",
        };
        f.write_str(s)
    }
}

impl From<String> for AudioSource {
    fn from(s: String) -> Self {
        match s.as_str() {
            "file_import" => AudioSource::FileImport,
            "mobile_upload" => AudioSource::MobileUpload,
            "watch_folder" => AudioSource::WatchFolder,
            _ => AudioSource::SystemCapture,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: i64,
    pub meeting_id: Uuid,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub speaker_id: Option<String>,
    pub confidence: f32,
    pub is_final: bool,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Part of the data model; produced once action-item/decision extraction and
/// diarization land (see IMPROVEMENT_PLAN.md).
#[allow(dead_code)]
pub struct Speaker {
    pub id: String,
    pub meeting_id: Uuid,
    pub label: String,
    pub display_name: Option<String>,
    pub is_local_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: i64,
    pub meeting_id: Uuid,
    pub content: String,
    pub generation_mode: String,
    pub model_used: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub user_edited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Part of the data model; produced once action-item/decision extraction and
/// diarization land (see IMPROVEMENT_PLAN.md).
#[allow(dead_code)]
pub struct ActionItem {
    pub id: Uuid,
    pub meeting_id: Uuid,
    pub description: String,
    pub assignee: Option<String>,
    pub deadline: Option<String>,
    pub source_segment_id: Option<i64>,
    pub status: String,
    pub loop_task_id: Option<String>,
    pub sync_status: SyncStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Part of the data model; produced once action-item/decision extraction and
/// diarization land (see IMPROVEMENT_PLAN.md).
#[allow(dead_code)]
pub struct Decision {
    pub id: Uuid,
    pub meeting_id: Uuid,
    pub description: String,
    pub context: Option<String>,
    pub participants: Vec<String>,
    pub source_segment_ids: Vec<i64>,
    pub vault_entry_id: Option<String>,
    pub sync_status: SyncStatus,
    pub created_at: i64,
}
