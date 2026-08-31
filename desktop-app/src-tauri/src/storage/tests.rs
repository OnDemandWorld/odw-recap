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
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();
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
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

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
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        storage.save_api_key("openai", "sk-test123").unwrap();
        let retrieved = storage.get_api_key("openai").unwrap();

        assert_eq!(retrieved, Some("sk-test123".to_string()));
    }

    #[test]
    fn test_prompt_template_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

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

    #[test]
    fn test_transcript_segments_roundtrip_and_fts_search() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        let meeting = create_test_meeting();
        storage.create_meeting(&meeting).unwrap();

        storage
            .insert_transcript_segment(&crate::storage::types::TranscriptSegment {
                id: 0,
                meeting_id: meeting.id,
                start_ms: 0,
                end_ms: 2500,
                text: "We need to finalize the quarterly budget".to_string(),
                speaker_id: None,
                confidence: 0.9,
                is_final: true,
                version: 1,
            })
            .unwrap();
        storage
            .insert_transcript_segment(&crate::storage::types::TranscriptSegment {
                id: 0,
                meeting_id: meeting.id,
                start_ms: 2500,
                end_ms: 5000,
                text: "Agreed, let us schedule a follow-up".to_string(),
                speaker_id: Some("speaker-1".to_string()),
                confidence: 0.8,
                is_final: true,
                version: 1,
            })
            .unwrap();

        let segments = storage.get_transcript_segments(meeting.id).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].meeting_id, meeting.id);
        assert_eq!(segments[1].speaker_id.as_deref(), Some("speaker-1"));

        // The FTS index is kept in sync via triggers; search must find text.
        let results = storage.search_transcripts("budget", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, meeting.id);
        assert!(results[0].1.contains("quarterly budget"));

        // Porter stemming: "finalizes" should match "finalize".
        let stemmed = storage.search_transcripts("finalizes", 10).unwrap();
        assert_eq!(stemmed.len(), 1);
    }

    #[test]
    fn test_summary_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        let meeting = create_test_meeting();
        storage.create_meeting(&meeting).unwrap();

        storage
            .save_summary(&crate::storage::types::Summary {
                id: 0,
                meeting_id: meeting.id,
                content: "Short summary of the meeting.".to_string(),
                generation_mode: "rule_based".to_string(),
                model_used: None,
                created_at: 1,
                updated_at: 1,
                user_edited: false,
            })
            .unwrap();

        let summary = storage.get_summary(meeting.id).unwrap().unwrap();
        assert_eq!(summary.content, "Short summary of the meeting.");
        assert_eq!(summary.generation_mode, "rule_based");

        // Saving again replaces the previous summary.
        storage
            .save_summary(&crate::storage::types::Summary {
                id: 0,
                meeting_id: meeting.id,
                content: "Updated summary".to_string(),
                generation_mode: "llm".to_string(),
                model_used: Some("test-model".to_string()),
                created_at: 2,
                updated_at: 2,
                user_edited: false,
            })
            .unwrap();
        let summary = storage.get_summary(meeting.id).unwrap().unwrap();
        assert_eq!(summary.content, "Updated summary");
    }

    #[test]
    fn test_salt_persists_across_reopen() {
        // Regression test: the KDF salt must be persisted, otherwise data
        // encrypted in one session cannot be decrypted in the next.
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        {
            let storage = StorageManager::open_or_initialize(data_dir.clone(), "test-passphrase").unwrap();
            storage.save_api_key("openai", "sk-secret-123").unwrap();
        }

        // Reopen with the same passphrase: the API key must still decrypt.
        let storage = StorageManager::open_or_initialize(data_dir.clone(), "test-passphrase").unwrap();
        assert_eq!(
            storage.get_api_key("openai").unwrap(),
            Some("sk-secret-123".to_string())
        );
        drop(storage);

        // A wrong passphrase must be rejected at unlock time.
        let wrong = StorageManager::open_or_initialize(data_dir, "wrong-passphrase");
        assert!(wrong.is_err());
    }

    #[test]
    fn test_vault_initialize_open_and_wrong_passphrase() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        assert!(!crate::storage::vault_initialized(&data_dir));

        // Opening before initialization must fail.
        assert!(StorageManager::open(data_dir.clone(), "whatever").is_err());

        // Initialize, store something, drop.
        {
            let storage = StorageManager::initialize(data_dir.clone(), "hunter2hunter2").unwrap();
            storage.save_api_key("openai", "sk-abc").unwrap();
        }
        assert!(crate::storage::vault_initialized(&data_dir));

        // Initializing again must fail.
        assert!(StorageManager::initialize(data_dir.clone(), "another-pass").is_err());

        // Correct passphrase opens; wrong passphrase is rejected.
        let storage = StorageManager::open(data_dir.clone(), "hunter2hunter2").unwrap();
        assert_eq!(storage.get_api_key("openai").unwrap(), Some("sk-abc".to_string()));
        drop(storage);
        assert!(StorageManager::open(data_dir, "wrong-passphrase").is_err());
    }

    #[test]
    fn test_change_passphrase_reencrypts_data() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        {
            let storage = StorageManager::initialize(data_dir.clone(), "old-passphrase").unwrap();
            storage.save_api_key("openai", "sk-1").unwrap();
            storage.save_api_key("anthropic", "sk-ant-2").unwrap();

            // Short passphrases are rejected.
            assert!(storage.change_passphrase("short").is_err());

            storage.change_passphrase("new-passphrase").unwrap();

            // Data is still readable within the same session.
            assert_eq!(storage.get_api_key("openai").unwrap(), Some("sk-1".to_string()));
        }

        // Old passphrase no longer opens the vault.
        assert!(StorageManager::open(data_dir.clone(), "old-passphrase").is_err());

        // New passphrase opens it and all keys decrypt.
        let storage = StorageManager::open(data_dir, "new-passphrase").unwrap();
        assert_eq!(storage.get_api_key("openai").unwrap(), Some("sk-1".to_string()));
        assert_eq!(
            storage.get_api_key("anthropic").unwrap(),
            Some("sk-ant-2".to_string())
        );
    }

    #[test]
    fn test_verify_passphrase() {
        let temp_dir = TempDir::new().unwrap();
        let storage =
            StorageManager::initialize(temp_dir.path().to_path_buf(), "some-passphrase").unwrap();
        assert!(storage.get_encryption().verify_passphrase("some-passphrase").unwrap());
        assert!(!storage.get_encryption().verify_passphrase("not-the-passphrase").unwrap());
    }

    #[test]
    fn test_list_prompt_templates() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::open_or_initialize(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();

        storage
            .save_prompt_template("Template A", "Content A {{var}}", None, false)
            .unwrap();
        storage
            .save_prompt_template("Template B", "Content B", Some("standup"), true)
            .unwrap();

        let templates = storage.list_prompt_templates().unwrap();
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].0, "Template A");
        assert_eq!(templates[1].2, Some("standup".to_string()));
        assert!(templates[1].3);
    }
}
