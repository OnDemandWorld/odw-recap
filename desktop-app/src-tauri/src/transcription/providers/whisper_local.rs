//! Local on-device transcription via whisper.cpp (through whisper-rs).
//!
//! The model file (GGML weights) is resolved from `model_path` and loaded
//! lazily; loaded contexts are cached process-wide so repeated transcriptions
//! don't pay the load cost again. Transcription runs on a blocking thread so
//! the async runtime stays responsive.

use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::audio_decode;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Local Whisper provider using whisper.cpp
pub struct WhisperLocalProvider {
    model_path: Option<String>,
}

impl WhisperLocalProvider {
    pub fn new(model_path: Option<String>) -> Self {
        Self { model_path }
    }
}

/// Process-wide cache of loaded whisper contexts, keyed by model path.
fn context_cache() -> &'static Mutex<HashMap<PathBuf, Arc<WhisperContext>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<WhisperContext>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load_context(model_path: &Path) -> Result<Arc<WhisperContext>> {
    if let Some(cache) = context_cache().lock().ok() {
        if let Some(ctx) = cache.get(model_path) {
            return Ok(ctx.clone());
        }
    }

    // Load outside the lock — this can take several seconds.
    let path_str = model_path.to_str().ok_or_else(|| {
        RecapError::Transcription("Model path is not valid UTF-8".to_string())
    })?;
    let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
        .map_err(|e| RecapError::Transcription(format!("Failed to load whisper model: {}", e)))?;
    let ctx = Arc::new(ctx);

    if let Ok(mut cache) = context_cache().lock() {
        cache.insert(model_path.to_path_buf(), ctx.clone());
    }
    Ok(ctx)
}

/// Run whisper.cpp on 16 kHz mono samples. Returns `(start_ms, end_ms, text, confidence)`
/// per segment.
fn run_whisper(
    model_path: PathBuf,
    samples: Vec<f32>,
    language: Option<String>,
) -> Result<Vec<(i64, i64, String, f32)>> {
    let ctx = load_context(&model_path)?;
    let mut state = ctx
        .create_state()
        .map_err(|e| RecapError::Transcription(format!("Failed to create whisper state: {}", e)))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(true);

    // "auto" (or unset) lets whisper.cpp detect the language.
    match language.as_deref() {
        None | Some("") | Some("auto") => params.set_language(Some("auto")),
        Some(lang) => params.set_language(Some(lang)),
    }

    state
        .full(params, &samples)
        .map_err(|e| RecapError::Transcription(format!("Whisper transcription failed: {}", e)))?;

    let n_segments = state.full_n_segments();
    let mut segments = Vec::with_capacity(n_segments as usize);

    for i in 0..n_segments {
        let seg = match state.get_segment(i) {
            Some(s) => s,
            None => continue,
        };

        let text = seg.to_str_lossy().unwrap_or_default().trim().to_string();
        if text.is_empty() {
            continue;
        }

        // whisper.cpp timestamps are centiseconds.
        let start_ms = seg.start_timestamp() * 10;
        let end_ms = seg.end_timestamp() * 10;
        let confidence = seg.no_speech_probability(); // 0 = speech, 1 = silence
        segments.push((start_ms, end_ms, text, confidence));
    }

    Ok(segments)
}

#[async_trait]
impl STTProvider for WhisperLocalProvider {
    fn name(&self) -> &str {
        "whisper_local"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self
            .model_path
            .as_ref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false))
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        let model_path = self.model_path.as_ref().ok_or_else(|| {
            RecapError::Transcription(
                "No local whisper model configured. Choose one in Settings.".to_string(),
            )
        })?;
        let model_path = PathBuf::from(model_path);
        if !model_path.exists() {
            return Err(RecapError::Transcription(
                "Whisper model is not downloaded yet. Open Settings → Local Whisper \
                 Model and download it first."
                    .to_string(),
            ));
        }

        // Decode and resample to whisper.cpp's required 16 kHz mono format.
        let samples = audio_decode::decode_for_whisper(audio_path)?;
        let duration_seconds = samples.len() as f64
            / audio_decode::WHISPER_SAMPLE_RATE as f64;

        let language = config.language.clone();
        let raw_segments = tokio::task::spawn_blocking(move || {
            run_whisper(model_path, samples, language)
        })
        .await
        .map_err(|e| RecapError::Transcription(format!("Transcription task failed: {}", e)))??;

        let segments = raw_segments
            .into_iter()
            .enumerate()
            .map(|(i, (start_ms, end_ms, text, confidence))| TranscriptSegment {
                id: (i + 1) as i64,
                meeting_id: Uuid::nil(), // set by the caller
                speaker_id: None,
                start_ms,
                end_ms,
                text,
                confidence,
                is_final: true,
                version: 1,
            })
            .collect();

        Ok(TranscriptionResult {
            segments,
            language: config.language.unwrap_or_else(|| "auto".to_string()),
            duration_seconds,
        })
    }

    fn supported_models(&self) -> Vec<String> {
        crate::transcription::whisper_models::WHISPER_MODELS
            .iter()
            .map(|m| m.name.to_string())
            .collect()
    }
}
