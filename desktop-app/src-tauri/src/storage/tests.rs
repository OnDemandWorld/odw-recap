#[cfg(test)]
mod tests {
    use crate::storage::types::{AudioSource, Meeting, MeetingStatus, SyncStatus, TranscriptSegment};
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

    #[test]
    fn test_transcript_summary_action_items_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();
        let meeting = create_test_meeting();
        let id = meeting.id;
        storage.create_meeting(&meeting).unwrap();

        // Transcript segments: save, read back in order, replace.
        let segments = vec![
            TranscriptSegment {
                id: 0,
                meeting_id: id,
                start_ms: 0,
                end_ms: 2000,
                text: "Good morning everyone.".to_string(),
                speaker_id: Some("speaker_0".to_string()),
                confidence: 0.95,
                is_final: true,
                version: 1,
            },
            TranscriptSegment {
                id: 0,
                meeting_id: id,
                start_ms: 2000,
                end_ms: 5000,
                text: "Let's start with the roadmap.".to_string(),
                speaker_id: None,
                confidence: 0.9,
                is_final: true,
                version: 1,
            },
        ];
        storage.save_transcript_segments(id, &segments).unwrap();
        let loaded = storage.get_transcript_segments(id).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].text, "Good morning everyone.");
        assert_eq!(loaded[1].start_ms, 2000);
        assert!(loaded[0].is_final);

        // Replacing must not duplicate.
        storage.save_transcript_segments(id, &segments[..1]).unwrap();
        let loaded = storage.get_transcript_segments(id).unwrap();
        assert_eq!(loaded.len(), 1);

        // Summary: latest wins.
        storage.save_summary(id, "First draft", "rule_based", Some("rule_based")).unwrap();
        storage.save_summary(id, "Better draft", "openai", Some("gpt-4o-mini")).unwrap();
        let summary = storage.get_latest_summary(id).unwrap().unwrap();
        assert_eq!(summary.content, "Better draft");
        assert_eq!(summary.generation_mode, "openai");

        // Action items + decisions replace cleanly.
        storage.save_action_items(id, &["Send recap".to_string(), "Book room".to_string()]).unwrap();
        storage.save_decisions(id, &["Ship in Q3".to_string()]).unwrap();
        let items = storage.get_action_items(id).unwrap();
        let decisions = storage.get_decisions(id).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(decisions, vec!["Ship in Q3".to_string()]);
        storage.save_action_items(id, &[]).unwrap();
        assert!(storage.get_action_items(id).unwrap().is_empty());
    }

    #[test]
    fn test_delete_meeting() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();
        let meeting = create_test_meeting();
        let id = meeting.id;
        storage.create_meeting(&meeting).unwrap();
        storage.save_transcript_segments(id, &[]).unwrap();

        storage.delete_meeting(id).unwrap();
        assert!(storage.get_meeting(id).unwrap().is_none());
    }

    #[test]
    fn test_salt_persisted_for_new_installs() {
        // Fresh install: a random salt file must be created, and two fresh
        // installs with the same passphrase must derive different keys (i.e.
        // not silently share the historical zero salt).
        let temp_a = TempDir::new().unwrap();
        let temp_b = TempDir::new().unwrap();
        let storage_a = StorageManager::new(temp_a.path().to_path_buf(), "pass").unwrap();
        let storage_b = StorageManager::new(temp_b.path().to_path_buf(), "pass").unwrap();

        assert!(temp_a.path().join("salt.bin").exists());

        storage_a.save_api_key("openai", "sk-one").unwrap();
        storage_b.save_api_key("openai", "sk-two").unwrap();
        assert_eq!(storage_a.get_api_key("openai").unwrap(), Some("sk-one".to_string()));
        assert_eq!(storage_b.get_api_key("openai").unwrap(), Some("sk-two".to_string()));

        // Reopening the same install (same salt file) still decrypts.
        drop(storage_a);
        let reopened = StorageManager::new(temp_a.path().to_path_buf(), "pass").unwrap();
        assert_eq!(reopened.get_api_key("openai").unwrap(), Some("sk-one".to_string()));
    }

    #[test]
    fn test_list_prompt_templates() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf(), "test-passphrase").unwrap();
        storage.save_prompt_template("alpha", "content-a", None, false).unwrap();
        storage.save_prompt_template("beta", "content-b", Some("team_meeting"), true).unwrap();

        let templates = storage.list_prompt_templates().unwrap();
        assert_eq!(templates.len(), 2);
        let names: Vec<&str> = templates.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
        let beta = templates.iter().find(|(n, _, _)| n == "beta").unwrap();
        assert!(beta.2); // is_custom
    }
}
