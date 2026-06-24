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

    pub fn watch_with_callback<F>(&self, mut callback: F) -> Result<notify::RecommendedWatcher>
    where
        F: FnMut(PathBuf) + Send + 'static,
    {
        let inbox_path = self.inbox_path.clone();

        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if let notify::EventKind::Create(_) = event.kind {
                    for path in event.paths {
                        let inbox = inbox_path.clone();
                        if path.starts_with(&inbox) {
                            if Self::wait_for_file_stable(&path) {
                                callback(path);
                            }
                        }
                    }
                }
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
