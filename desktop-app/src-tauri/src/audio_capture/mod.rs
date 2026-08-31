//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

pub mod recorder;
pub mod system_audio;

use crate::error::Result;
use std::path::PathBuf;

pub use recorder::{AudioRecorder, RecordingConfig};

/// Audio capture manager for coordinating recording sources
pub struct AudioCaptureManager {
    recorder: AudioRecorder,
}

impl AudioCaptureManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            recorder: AudioRecorder::new()?,
        })
    }

    /// Start recording from microphone
    pub fn start_microphone_recording(&mut self, output_path: PathBuf, config: RecordingConfig) -> Result<()> {
        self.recorder.start_recording(output_path, config)
    }

    /// Stop recording
    pub fn stop_recording(&mut self) -> Result<PathBuf> {
        self.recorder.stop_recording()
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }

    /// List available audio input devices
    pub fn list_input_devices(&self) -> Result<Vec<String>> {
        self.recorder.list_input_devices()
    }
}
