//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::audio_input::{AudioInputManager, ImportedFile};
use crate::error::{RecapError, Result};
use std::path::PathBuf;

pub struct FileImporter;

impl FileImporter {
    pub fn import_from_path(manager: &AudioInputManager, path: PathBuf) -> Result<ImportedFile> {
        manager.import_file(&path)
    }

    /// Import several files (e.g. from a drag-and-drop). Files that fail to
    /// import are reported in the error without discarding the ones that
    /// already succeeded on disk.
    pub fn import_from_drag_drop(
        manager: &AudioInputManager,
        dropped_files: Vec<PathBuf>,
    ) -> Result<Vec<ImportedFile>> {
        let mut imported = Vec::new();
        for file in dropped_files {
            match manager.import_file(&file) {
                Ok(imported_file) => imported.push(imported_file),
                Err(e) => {
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
