#[cfg(test)]
mod tests {
    use crate::storage::types::{AudioSource, Meeting, MeetingStatus, SyncStatus};
    use crate::storage::StorageManager;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_meeting() -> Meeting {
        Meeting {
            id: Uuid::new_v4(),
            title: Some("Test Meeting".to_string()),
            started_at: 1719136800000,
            ended_at: None,
            duration_seconds: None,
            status: MeetingStatus::Completed,
            audio_file_path: PathBuf::from("audio/test.opus"),
            audio_format: "opus".to_string(),
            audio_size_bytes: 0,
            model_used: "whisper-medium".to_string(),
            tags: vec!["test".to_string()],
            folder_id: None,
            created_at: 1719136800000,
            updated_at: 1719136800000,
            sync_status: SyncStatus::NotSynced,
            meeting_type: Some("team_meeting".to_string()),
            location: Some("Remote".to_string()),
            participants: vec!["Alice".to_string(), "Bob".to_string()],
            language: "en".to_string(),
            topic: Some("Testing".to_string()),
            audio_source: AudioSource::SystemCapture,
            stt_provider: "whisper_local".to_string(),
            llm_provider: "llama_local".to_string(),
            prompt_template_used: Some("standard_meeting_summary".to_string()),
        }
    }

    #[test]
    fn test_create_and_get_meeting() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();
        let meeting = create_test_meeting();
        let id = meeting.id;

        storage.create_meeting(&meeting).unwrap();
        let retrieved = storage.get_meeting(id).unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, Some("Test Meeting".to_string()));
        assert_eq!(retrieved.language, "en".to_string());
        assert_eq!(retrieved.participants.len(), 2);
    }

    #[test]
    fn test_list_meetings() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        for _ in 0..3 {
            let meeting = create_test_meeting();
            storage.create_meeting(&meeting).unwrap();
        }

        let meetings = storage.list_meetings(10, 0).unwrap();
        assert_eq!(meetings.len(), 3);
    }

    #[test]
    fn test_api_key_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        storage.save_api_key("openai", "sk-test123").unwrap();
        let retrieved = storage.get_api_key("openai").unwrap();

        assert_eq!(retrieved, Some("sk-test123".to_string()));
    }

    #[test]
    fn test_prompt_template_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        storage.save_prompt_template(
            "standard_meeting_summary",
            "Summarize this meeting: {{transcript}}",
            None,
            false,
        ).unwrap();

        let template = storage.get_prompt_template("standard_meeting_summary").unwrap();
        assert!(template.is_some());
        let (content, _, _) = template.unwrap();
        assert_eq!(content, "Summarize this meeting: {{transcript}}");
    }
}
