//! STT provider implementations.
//!
//! Provider structs are constructed through `STTRouter::with_all_providers`
//! (see `crate::transcription`), which keys them by the names used in the
//! desktop UI settings.

pub mod assemblyai;
pub mod aws_transcribe;
pub mod azure_speech;
pub mod deepgram;
pub mod google_stt;
pub mod openai_whisper;
pub mod whisper_local;
