pub mod whisper_local;
pub mod openai_whisper;
pub mod assemblyai;
pub mod deepgram;
pub mod aws_transcribe;
pub mod azure_speech;
pub mod google_stt;

pub use whisper_local::WhisperLocalProvider;
pub use openai_whisper::OpenAIWhisperProvider;
pub use assemblyai::AssemblyAIProvider;
pub use deepgram::DeepgramProvider;
pub use aws_transcribe::AWSTranscribeProvider;
pub use azure_speech::AzureSpeechProvider;
pub use google_stt::GoogleSTTProvider;
