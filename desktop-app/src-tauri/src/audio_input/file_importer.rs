use crate::audio_input::AudioInputManager;
use crate::error::{RecapError, Result};
use std::path::PathBuf;

pub struct FileImporter;

impl FileImporter {
    pub fn import_from_path(manager: &AudioInputManager, path: PathBuf) -> Result<PathBuf> {
        manager.import_file(&path)
    }

    pub fn import_from_drag_drop(
        manager: &AudioInputManager,
        dropped_files: Vec<PathBuf>,
    ) -> Result<Vec<PathBuf>> {
        let mut imported = Vec::new();
        for file in dropped_files {
            match manager.import_file(&file) {
                Ok(path) => imported.push(path),
                Err(e) => {
                    // Audit #31: roll back files already copied so a single bad
                    // file in a multi-file drop does not leave orphan audio (with
                    // no meeting rows) in the inbox.
                    for copied in &imported {
                        let _ = std::fs::remove_file(copied);
                    }
                    return Err(RecapError::AudioInput(format!(
                        "Failed to import {}: {}",
                        file.display(),
                        e
                    )));
                }
            }
        }
        Ok(imported)
    }
}
