//! LLM provider implementations.
//!
//! Provider structs are constructed through `LLMRouter::with_all_providers`
//! (see `crate::summarization`), which keys them by the names used in the
//! desktop UI settings.

pub mod anthropic;
pub mod aws_bedrock;
pub mod azure;
pub mod google;
pub mod llama_local;
pub mod ollama;
pub mod openai;
