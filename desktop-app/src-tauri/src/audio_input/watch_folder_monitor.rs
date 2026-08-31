//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::error::{RecapError, Result};
use notify::Watcher;
use std::path::PathBuf;
use std::time::Duration;

pub struct WatchFolderMonitor {
    inbox_path: PathBuf,
}

impl WatchFolderMonitor {
    pub fn new(inbox_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&inbox_path)?;
        Ok(Self { inbox_path })
    }

    pub fn watch_with_callback<F>(&self, callback: F) -> Result<notify::RecommendedWatcher>
    where
        F: FnMut(PathBuf) + Send + 'static,
    {
        let inbox_path = self.inbox_path.clone();
        let callback = std::sync::Arc::new(std::sync::Mutex::new(callback));

        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<notify::Event, notify::Error>| {
            let Ok(event) = res else { return };
            // Handle both newly created files and files moved into the folder.
            let is_relevant = matches!(
                event.kind,
                notify::EventKind::Create(notify::event::CreateKind::File)
                    | notify::EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::To
                    ))
            );
            if !is_relevant {
                return;
            }

            for path in event.paths {
                if !path.starts_with(&inbox_path) {
                    continue;
                }
                // The stability check sleeps for several seconds; run it off the
                // notify dispatch thread so other events keep flowing.
                let callback = callback.clone();
                std::thread::spawn(move || {
                    if Self::wait_for_file_stable(&path) {
                        if let Ok(mut cb) = callback.lock() {
                            cb(path);
                        }
                    }
                });
            }
        }).map_err(|e| RecapError::AudioInput(e.to_string()))?;

        watcher
            .watch(&self.inbox_path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| RecapError::AudioInput(e.to_string()))?;

        Ok(watcher)
    }

    fn wait_for_file_stable(path: &std::path::Path) -> bool {
        let mut last_size = 0u64;
        let mut stable_count = 0u32;

        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(500));

            if let Ok(metadata) = std::fs::metadata(path) {
                let size = metadata.len();
                if size == last_size && size > 0 {
                    stable_count += 1;
                    if stable_count >= 3 {
                        return true;
                    }
                } else {
                    stable_count = 0;
                }
                last_size = size;
            } else {
                return false;
            }
        }

        false
    }
}
