# Technical Specification Document: Recap

**Product:** Recap by ODW.ai
**Version:** 1.0
**Date:** June 23, 2026
**Status:** Draft
**Author:** Engineering

---

## 1. System Overview

### 1.1 Purpose

Recap is a sovereign, on-device meeting intelligence system. It captures audio from any meeting platform (Zoom, Teams, Jitsi, Google Meet, in-person), transcribes conversations locally using open-source speech models (Whisper-family), generates structured outputs (summaries, action items, decisions), and syncs results into the ODW.ai suite (Vault for knowledge, Loop for workflows). No meeting data ever leaves the user's device or self-hosted infrastructure.

### 1.2 Architecture Components → Services/Modules

| SAD Component | TSD Module | Deployment Unit |
|---|---|---|
| Audio Capture Engine | `audio-capture` | In-process (Tauri backend) |
| Transcription Pipeline | `transcription` | In-process (Tauri backend, whisper.cpp native lib) |
| Summarization Engine | `summarization` | In-process (Tauri backend, llama.cpp FFI or rule-based) |
| Local Storage Manager | `storage` | In-process (SQLite + encrypted filesystem) |
| Sync Engine | `sync` | In-process (HTTP client to team server) |
| Desktop Application Shell | `desktop-shell` | Tauri application (Rust backend + webview frontend) |
| Team Server | `team-server` | Docker container (Go HTTP server + PostgreSQL + Redis) |
| Model Registry & Manager | `model-manager` | In-process (HTTP client for downloads) |

### 1.3 System Boundaries

**Included:**
- System audio capture (macOS, Windows, Linux)
- Audio file import (drag-and-drop, file browser)
- Watch folder with automatic processing (monitoring ~/RecapData/inbox/)
- Local HTTP upload server for smartphone audio transfer (LAN only)
- Local transcription via whisper.cpp
- Multiple STT providers (OpenAI Whisper API, AssemblyAI, Deepgram, AWS Transcribe, Azure Speech, Google Speech-to-Text)
- Multiple LLM providers (OpenAI GPT-4, Anthropic Claude, Google Gemini, AWS Bedrock, Azure OpenAI)
- Speaker diarization
- Structured output generation (summary, action items, decisions, tags)
- Prompt template management and editing (pre-built templates + user customization)
- Meeting metadata entry (type, location, participants, language, topic)
- Local encrypted storage with full-text search
- Optional team server for RBAC, audit, sync coordination
- Vault/Loop integration via team server proxy
- Model management (download, version, switch)
- API key management for cloud providers (encrypted storage)

**Excluded:**
- Mandatory cloud processing (cloud providers are optional, local remains default)
- Meeting platform bots (no Zoom/Teams/Meet bots)
- Video recording/capture
- Real-time translation
- Mobile apps with local processing (companion app uploads only, no local transcription)
- Telephony/SIP capture
- Public API for third-party integrations
- Custom model fine-tuning UI

---

## 2. Service & Module Breakdown

### 2.1 Audio Capture Module (`audio-capture`)

**Responsibility:** Capture system audio output and microphone input from the host OS; merge into a unified audio stream with speaker channel separation. Also handles audio file imports, watch folder monitoring, and HTTP upload server.

**Inputs:**
- OS audio subsystem output (system audio via virtual device or loopback API)
- Microphone device input (optional, user-configured)
- File imports (drag-and-drop, file browser)
- Watch folder events (new audio files in ~/RecapData/inbox/)
- HTTP uploads (POST /upload/audio from smartphone companion app)

**Outputs:**
- Raw PCM audio stream (16kHz, 16-bit mono per channel) → Transcription Pipeline
- Buffered WAV/Opus file → Local Storage Manager (every 5 seconds)
- Imported audio file path → Transcription Pipeline (batch mode)

**Internal Sub-components:**
- `PlatformAdapter` trait: abstracts OS-specific audio APIs
  - `CoreAudioAdapter` (macOS: CoreAudio + BlackHole virtual device)
  - `WASAPIAdapter` (Windows: WASAPI loopback)
  - `PulseAudioAdapter` (Linux: PulseAudio/PipeWire monitor source)
- `AudioMixer`: combines system audio + mic into stereo stream (system=left, mic=right)
- `BufferedWriter`: flushes audio chunks to disk every 5 seconds
- `FileImporter` (NEW): handles drag-and-drop and file browser imports
  ```rust
  pub struct FileImporter {
      supported_formats: Vec<AudioFormat>, // M4A, WAV, MP3, OGG, WebM, FLAC
  }
  impl FileImporter {
      pub fn import_from_path(path: PathBuf) -> Result<AudioFile>;
      pub fn import_from_drag_drop(dropped_files: Vec<PathBuf>) -> Result<Vec<AudioFile>>;
  }
  ```
- `WatchFolderMonitor` (NEW): watches inbox folder using `notify` crate
  ```rust
  use notify::{Watcher, RecursiveMode, watcher};
  pub struct WatchFolderMonitor {
      watcher: notify::RecommendedWatcher,
      inbox_path: PathBuf,
      debounce_ms: u64,
  }
  impl WatchFolderMonitor {
      pub fn start(path: PathBuf) -> Result<Self>;
      pub fn on_new_file<F: Fn(AudioFile) + Send + 'static>(&self, callback: F);
  }
  ```
- `HttpUploadServer` (NEW): embedded Axum HTTP server for LAN uploads
  ```rust
  use axum::{Router, routing::post, extract::Multipart};
  pub struct HttpUploadServer {
      port: u16,
      inbox_path: PathBuf,
  }
  impl HttpUploadServer {
      pub async fn start(&self) -> Result<()>;
      // POST /upload/audio → saves to inbox_path, returns UUID
  }
  ```

**Dependencies:** None (leaf module)

**Deployment:** In-process within Tauri backend

**Key Constraints:**
- Must not introduce audible latency or degrade meeting audio quality
- Must handle audio device hot-plug/unplug gracefully
- Buffer flush interval: 5 seconds (max data loss window on crash)
- Audio format validation: reject unsupported formats with clear error
- Watch folder debouncing: wait for file write completion before processing
- HTTP upload server: LAN only (bind to 0.0.0.0 or specific interface, no external exposure)

---

### 2.2 Transcription Module (`transcription`)

**Responsibility:** Convert audio streams into timestamped, speaker-labeled text. Operates in real-time streaming mode (during recording) and batch re-processing mode (after recording ends).

**Inputs:**
- Streaming mode: PCM audio chunks (16kHz, 16-bit mono) from Audio Capture
- Batch mode: Complete audio file path (Opus or WAV)

**Outputs:**
- `Transcript` struct: ordered `TranscriptSegment[]` with timestamps, speaker labels, confidence scores, text

**Internal Sub-components:**
- `TranscriptionBackend` trait (plugin interface):
  ```rust
  trait TranscriptionBackend {
      fn load_model(path: &str) -> Result<()>;
      fn transcribe_streaming(&mut self, audio_chunk: &[u8]) -> Result<TranscriptSegment>;
      fn transcribe_batch(&mut self, audio_file: &str) -> Result<Transcript>;
      fn diarize(&self, audio_file: &str) -> Result<Vec<SpeakerSegment>>;
      fn get_model_info(&self) -> ModelInfo;
  }
  ```
- `WhisperCppBackend`: primary implementation using whisper.cpp C++ bindings
- `FasterWhisperBackend`: alternative implementation (future)
- `DiarizationEngine`: speaker identification via pyannote-audio or neMo (loaded as native lib)
- `StreamingBuffer`: ring buffer for real-time audio chunks with backpressure signaling

**STT Provider Router (NEW):**
- `STTProvider` trait (unified interface for local and cloud providers):
  ```rust
  #[async_trait]
  pub trait STTProvider: Send + Sync {
      async fn transcribe(&self, audio: AudioInput, language: Language) -> Result<Transcript>;
      fn supports_streaming(&self) -> bool;
      fn estimate_cost(&self, duration_seconds: u64) -> Money;
  }
  ```
- `STTRouter`: routes requests to selected provider with fallback logic
  ```rust
  pub struct STTRouter {
      providers: HashMap<STTProviderType, Box<dyn STTProvider>>,
      primary: STTProviderType,
      fallback: Option<STTProviderType>,
  }
  impl STTRouter {
      pub async fn transcribe(&self, audio: AudioInput, language: Language) -> Result<Transcript> {
          match self.providers[&self.primary].transcribe(audio.clone(), language).await {
              Ok(transcript) => Ok(transcript),
              Err(e) => {
                  if let Some(fallback) = &self.fallback {
                      self.providers[fallback].transcribe(audio, language).await
                  } else {
                      Err(e)
                  }
              }
          }
      }
  }
  ```
- Cloud provider implementations (NEW):
  - `OpenAIWhisperAPIBackend`: OpenAI Whisper API client (using `reqwest` + multipart upload)
  - `AssemblyAIBackend`: AssemblyAI API client
  - `DeepgramBackend`: Deepgram API client (supports real-time streaming)
  - `AWSTranscribeBackend`: AWS Transcribe client (using `aws-sdk-transcribe` crate)
  - `AzureSpeechBackend`: Azure Speech Services client
  - `GoogleSpeechBackend`: Google Speech-to-Text client

**Dependencies:**
- `audio-capture` (audio input)
- `model-manager` (model weights)

**Deployment:** In-process within Tauri backend

**Key Constraints:**
- Streaming latency: ≤2 seconds end-to-end
- Batch processing: ≤10 minutes for 60-minute meeting on M2 Mac, 16GB RAM
- Model tiers: small (~75MB), medium (~1.5GB), large (~3GB)

---

### 2.3 Summarization Module (`summarization`)

**Responsibility:** Generate structured outputs from transcripts — meeting summary, action items, decisions, and topic tags.

**Inputs:**
- Complete `Transcript` (post batch processing)
- Optional user-configured prompt template
- Optional custom output template

**Outputs:**
- `Summary` (markdown string)
- `ActionItem[]` (description, assignee, deadline, source_segment_id)
- `Decision[]` (description, context, participants)
- `TopicTag[]` (string labels)

**Internal Sub-components:**
- `RuleBasedExtractor`: pattern matching + heuristic extraction (no LLM required)
  - Action phrase detection ("I'll...", "Action item:", "TODO:", "follow up")
  - Temporal expression parsing for deadlines ("by Friday", "next week", "end of month")
  - Decision phrase detection ("We decided", "Let's go with", "Agreed that")
  - Topic extraction via TF-IDF or keyword frequency
- `LLMSummarizer`: local LLM-powered extraction via llama.cpp or Ollama
  - Standardized prompt/response interface
  - Model-agnostic (any compatible local model)
  - Grounding: all claims reference transcript segments (no hallucination)
- `OutputFormatter`: applies user-configured templates to raw extraction results

**LLM Provider Router (NEW):**
- `LLMProvider` trait (unified interface for local and cloud providers):
  ```rust
  #[async_trait]
  pub trait LLMProvider: Send + Sync {
      async fn complete(&self, prompt: String) -> Result<String>;
      async fn complete_structured<T: DeserializeOwned>(&self, prompt: String, schema: JsonSchema) -> Result<T>;
      fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> Money;
  }
  ```
- `LLMRouter`: routes requests to selected provider
  ```rust
  pub struct LLMRouter {
      providers: HashMap<LLMProviderType, Box<dyn LLMProvider>>,
      primary: LLMProviderType,
  }
  impl LLMRouter {
      pub async fn summarize(&self, transcript: Transcript, prompt: String) -> Result<MeetingSummary>;
  }
  ```
- Cloud provider implementations (NEW):
  - `OpenAIBackend`: OpenAI GPT-4/3.5 client (using `async-openai` crate)
  - `AnthropicBackend`: Anthropic Claude client (using `anthropic` crate)
  - `GoogleGeminiBackend`: Google Gemini client (using `google-generativeai` crate)
  - `AWSBedrockBackend`: AWS Bedrock client (hosts Claude, Llama models)
  - `AzureOpenAIBackend`: Azure OpenAI client

**Prompt Manager (NEW):**
- `PromptManager`: loads, versions, and applies prompt templates
  ```rust
  pub struct PromptManager {
      templates_dir: PathBuf, // ~/RecapData/prompts/
      current_version: u32,
  }
  impl PromptManager {
      pub fn load_template(&self, name: &str) -> Result<PromptTemplate>;
      pub fn save_custom(&self, name: &str, content: String) -> Result<()>;
      pub fn list_templates(&self) -> Vec<PromptTemplate>;
      pub fn render(&self, template: &PromptTemplate, context: MeetingContext) -> String;
  }
  ```
- Template variables: `{{transcript}}`, `{{meeting_type}}`, `{{participants}}`, `{{language}}`
- Pre-built templates (in `prompt_templates/` directory):
  - `standard_meeting_summary.txt`
  - `client_call_action_items.txt`
  - `team_meeting_blockers.txt`
  - `interview_evaluation.txt`
  - `presentation_qa.txt`
- User-edited templates saved with incremented version number
- Meeting-type-specific prompt selection based on metadata

**Dependencies:**
- `transcription` (transcript input)
- `model-manager` (LLM weights for LLM mode)

**Deployment:** In-process within Tauri backend

**Key Constraints:**
- Rule-based mode: zero network calls, zero external model downloads
- LLM mode: all summary claims must reference transcript segments
- Fallback: if LLM fails to load, automatically degrades to rule-based mode

---

### 2.4 Storage Module (`storage`)

**Responsibility:** Persist all meeting data (audio, transcripts, summaries, metadata) to local filesystem with encryption at rest. Manage storage lifecycle, retention policies, and data organization.

**Inputs:**
- Meeting artifacts from upstream pipeline (audio, transcripts, summaries, action items, decisions)
- User commands (save, delete, export, search)

**Outputs:**
- Encrypted files on disk (audio as Opus, transcripts/summaries as encrypted blobs)
- SQLite database (metadata, FTS5 search index, application state)
- Search query results
- Export files (Markdown, PDF, JSON, plain text)

**Internal Sub-components:**
- `SQLiteManager`: database connection, migrations, transactions
  - FTS5 virtual table for full-text search
  - WAL mode for crash recovery
- `EncryptionManager`: AES-256-GCM encryption/decryption
  - Key derivation: Argon2id from user passphrase + per-installation salt
  - Per-file DEKs encrypted by master key (key hierarchy)
- `FileStore`: manages encrypted audio files on disk
- `ExportEngine`: converts meeting data to export formats
- `RetentionEnforcer`: applies retention policies (delete after N days)

**Dependencies:** None (leaf module for data persistence)

**Deployment:** In-process within Tauri backend

**Key Constraints:**
- Support ≥10,000 hours of meetings per installation
- Search across full corpus: ≤1 second response time
- Data directory: configurable, default `~/RecapData`
- Atomic writes: transcript + summary + action items persisted in single SQLite transaction

---

### 2.5 Sync Module (`sync`)

**Responsibility:** Coordinate data flow between Recap and ODW.ai suite modules (Vault and Loop). Evaluate sync rules, transform data, manage sync state, handle retries.

**Inputs:**
- Meeting metadata and structured outputs from Storage
- Sync rule configuration
- Vault/Loop API responses

**Outputs:**
- Synced entries in Vault (knowledge base entries linked to source meetings)
- Tasks/workflows created in Loop
- Sync status updates reflected in UI

**Internal Sub-components:**
- `SyncRuleEvaluator`: determines which meetings/items to sync based on rules
- `VaultSyncClient`: transforms summaries/decisions → Vault-compatible entries, POSTs to Vault API
- `LoopSyncClient`: transforms action items → Loop tasks, POSTs to Loop API
- `RetryQueue`: SQLite-backed queue for failed syncs with exponential backoff
- `CircuitBreaker`: opens after 5 consecutive failures within 10 minutes; half-open probe after 15 minutes

**Dependencies:**
- `storage` (reads meeting data)
- Team Server (proxy to Vault/Loop APIs)

**Deployment:** In-process within Tauri backend

**Key Constraints:**
- Sync is opportunistic and non-blocking (never blocks capture-transcribe-summarize pipeline)
- Retry: exponential backoff (1min → 2min → 4min → ... → max 1hr), max 30 days
- Queue TTL: 30 days (after which marked permanently failed, admin notified)
- Circuit breaker: 5 failures in 10 min → open for 15 min → half-open probe

---

### 2.6 Desktop Shell Module (`desktop-shell`)

**Responsibility:** Provide user interface, system tray/menu bar integration, hotkey handling, onboarding flow, settings management, and orchestrate all internal components.

**Inputs:**
- User interactions (UI clicks, hotkeys, settings changes)
- System events (audio device changes, OS notifications)

**Outputs:**
- Visual transcription display (real-time)
- Meeting library (browse, search, filter)
- Settings panels
- Sync status indicators
- System tray/menu bar presence

**Internal Sub-components:**
- `TauriApp`: Rust backend orchestrating all modules
- `WebViewFrontend`: HTML/CSS/JS rendered in OS webview
  - Pages: Onboarding, Recording, Meeting Library, Meeting Detail, Settings, Admin (team mode), Import, ProviderSettings, PromptEditor (NEW)
- `HotkeyManager`: global hotkey registration (start/stop recording)
- `TrayManager`: system tray icon with status and quick actions
- `OnboardingFlow`: guided setup (audio input, model selection, test recording)

**New UI Components (for flexible input and multi-provider support):**
- `ImportPage.tsx` (NEW): Drag-and-drop zone, file browser, watch folder status
- `ProviderSettingsPage.tsx` (NEW): Configure STT/LLM providers, API keys, test connectivity
- `PromptEditorPage.tsx` (NEW): View/edit prompt templates, preview with sample data
- `MetadataEntryDialog.tsx` (NEW): Form for meeting type, location, participants, language, topic
- `ProviderSelector.tsx` (NEW): Dropdown for STT/LLM provider selection
- `APIKeyInput.tsx` (NEW): Secure input for API keys (masked, encrypted storage)
- `CostEstimator.tsx` (NEW): Display estimated cost for paid APIs before processing
- `PromptTemplateSelector.tsx` (NEW): Dropdown for prompt template selection
- `PromptEditor.tsx` (NEW): Monaco editor for customizing prompts

**Dependencies:** All other modules (orchestrator)

**Deployment:** Tauri application binary

**Key Constraints:**
- Cold start to ready: ≤5 seconds
- Idle CPU: ≤30% of one core (tray mode)
- Binary size target: ~10MB (Tauri advantage over Electron)

---

### 2.7 Team Server Module (`team-server`)

**Responsibility:** Provide shared state coordination for team deployments — authentication, RBAC, audit logging, retention policy enforcement, sync rule management, admin dashboard. Does NOT process audio or transcripts.

**Inputs:**
- API requests from desktop clients (auth, sync metadata, policy queries)
- Admin configuration commands

**Outputs:**
- JWT authentication tokens
- Policy decisions
- Audit log entries
- Aggregated usage metrics

**Internal Sub-components:**
- `AuthHandler`: login, token refresh, SSO callback (SAML 2.0 / OIDC)
- `RBACEnforcer`: role-based access control (Admin, Manager, Member)
- `AuditLogger`: records all access events (who, what, when)
- `PolicyManager`: retention policies, sync rules
- `VaultProxy`: transforms and forwards sync requests to Vault API
- `LoopProxy`: transforms and forwards sync requests to Loop API
- `AdminAPI`: usage metrics, user management, health status

**Dependencies:**
- PostgreSQL (shared metadata)
- Redis (session management, rate limiting)

**Deployment:** Single Docker container (docker-compose: server + postgres + redis)

**Key Constraints:**
- Support ≥200 concurrent users per instance
- No external connectivity required (fully self-hosted)
- Stateless application tier (horizontal scaling via load balancer)
- Does NOT store meeting content (only metadata references)

---

### 2.8 Model Manager Module (`model-manager`)

**Responsibility:** Manage lifecycle of ML model weights — discovery, download, installation, versioning, activation, deletion.

**Inputs:**
- User commands (download, switch, delete)
- Model manifest from registry endpoint (or local file for air-gapped)

**Outputs:**
- Downloaded model files in local model directory
- Active model selection persisted in configuration

**Internal Sub-components:**
- `ModelDownloader`: HTTP client with chunked download + resume capability
- `IntegrityVerifier`: SHA-256 verification on download completion
- `ModelRegistry`: tracks installed models, versions, active selection
- `AirGapLoader`: supports manual model file placement for air-gapped environments

**Dependencies:** Network (for download; optional in air-gapped mode)

**Deployment:** In-process within Tauri backend

**Key Constraints:**
- Model download is one-time per version
- Switching active model takes effect on next recording (no app restart)
- Multiple model versions can coexist on disk

---

## 3. Technical Stack Specification

### 3.1 Backend (Desktop Application)

| Layer | Technology | Version | Justification |
|---|---|---|---|
| Language | Rust | 1.75+ | Memory safety, performance, Tauri native |
| Framework | Tauri | 2.x | Small binary (~10MB), low memory, native OS integration |
| Audio (macOS) | CoreAudio + BlackHole | System | System audio loopback |
| Audio (Windows) | WASAPI loopback | Win10+ | Native system audio capture |
| Audio (Linux) | PulseAudio/PipeWire | 22.04+ | Monitor source for system audio |
| Transcription | whisper.cpp | Latest stable | C++ library with Rust bindings, GPU acceleration (Metal/CUDA) |
| Diarization | pyannote-audio or neMo | Latest | Speaker identification via native lib |
| Summarization (LLM) | llama.cpp | Latest stable | Local LLM inference via Rust FFI |
| Summarization (Alt) | Ollama | Latest | Alternative local LLM runtime |
| Summarization (Rule) | Custom Rust | — | Pattern matching, no external deps |
| Database | SQLite | 3.44+ | Zero-config, single-file, FTS5 for search |
| Encryption | AES-256-GCM | — | Via `aes-gcm` crate |
| Key Derivation | Argon2id | — | Via `argon2` crate |
| Serialization | serde + serde_json | 1.x | Standard Rust serialization |
| HTTP Client | reqwest | 0.11+ | For team server communication |

### 3.2 Frontend (WebView UI)

| Layer | Technology | Version | Justification |
|---|---|---|---|
| Language | TypeScript | 5.x | Type safety |
| Framework | React or Svelte | Latest | Lightweight, fast rendering in webview |
| Styling | Tailwind CSS | 3.x | Utility-first, small bundle |
| State | Zustand or signals | Latest | Minimal overhead |
| Build | Vite | 5.x | Fast dev/build |

### 3.3 Team Server

| Layer | Technology | Version | Justification |
|---|---|---|---|
| Language | Go | 1.22+ | Fast compilation, small binary, excellent HTTP stdlib |
| Framework | Chi or Echo | Latest | Lightweight HTTP router |
| Database | PostgreSQL | 16+ | Shared metadata, audit logs, JSONB for flexibility |
| Cache | Redis | 7+ | Session management, rate limiting |
| Auth | JWT (RS256) | — | Stateless authentication |
| SSO | SAML 2.0 / OIDC | — | Enterprise integration |
| Containerization | Docker + docker-compose | — | One-command deployment |

### 3.4 AI/ML Stack

| Component | Technology | Notes |
|---|---|---|
| Speech-to-Text | whisper.cpp | Primary ASR engine |
| ASR Models | Whisper large-v3, medium, small | User-selectable tiers |
| Speaker Diarization | pyannote-audio / neMo | Model-based, local |
| Summarization LLM | llama.cpp + any compatible model | Model-agnostic |
| Rule-based Extraction | Custom Rust | Fallback when no LLM available |

### 3.5 Data Layer

| Component | Technology | Notes |
|---|---|---|
| Primary DB (Desktop) | SQLite 3.44+ | WAL mode, FTS5, single-file |
| Primary DB (Server) | PostgreSQL 16+ | Multi-tenant metadata |
| Cache (Server) | Redis 7+ | Sessions, rate limits |
| File Storage | Encrypted filesystem | AES-256-GCM, Opus audio |
| Search | SQLite FTS5 | English stemming, diacritic removal |
| Future Search | SQLite-vec or HNSW | Vector embeddings for semantic search (v1.1+) |

### 3.6 DevOps Tooling

| Tool | Purpose |
|---|---|
| Docker + docker-compose | Team server deployment |
| GitHub Actions | CI/CD for desktop app + server |
| Tauri Builder | Cross-platform desktop app builds |
| cargo | Rust dependency management |
| go mod | Go dependency management |

---

## 4. Data Modeling & Schema Definitions

### 4.1 Desktop SQLite Schema

#### Table: `meetings`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | TEXT (UUIDv7) | PRIMARY KEY | Time-sortable UUID |
| title | TEXT | NULLABLE | User-editable, defaults to "Meeting on {date}" |
| started_at | INTEGER (unix ms) | NOT NULL | Recording start timestamp |
| ended_at | INTEGER (unix ms) | NULLABLE | Recording end timestamp (null if in progress) |
| duration_seconds | INTEGER | NULLABLE | Computed on end |
| status | TEXT | NOT NULL, DEFAULT 'recording' | Enum: recording, processing, completed, failed |
| audio_file_path | TEXT | NOT NULL | Relative path to encrypted Opus file |
| model_used | TEXT | NOT NULL | Model identifier (e.g., "whisper-large-v3") |
| tags | TEXT | NULLABLE | JSON array of strings |
| folder_id | TEXT | NULLABLE, FK → folders.id | Organization |
| created_at | INTEGER (unix ms) | NOT NULL | Record creation time |
| updated_at | INTEGER (unix ms) | NOT NULL | Last modification time |
| sync_status | TEXT | NOT NULL, DEFAULT 'not_synced' | Enum: not_synced, syncing, synced, failed, permanently_failed |
| meeting_type | TEXT | NULLABLE | Enum: team_meeting, one_on_one, client_call, interview, presentation, workshop, other |
| location | TEXT | NULLABLE | Free text (e.g., "Conference Room A", "Zoom", "Remote") |
| participants | TEXT | NULLABLE | JSON array of strings (e.g., '["Alice", "Bob"]') |
| language | TEXT | NOT NULL, DEFAULT 'en' | ISO 639-1 code (e.g., "en", "es", "fr", "de") |
| topic | TEXT | NULLABLE | Project or subject matter |
| audio_source | TEXT | NOT NULL, DEFAULT 'system_capture' | Enum: system_capture, file_import, mobile_upload, watch_folder |
| stt_provider | TEXT | NOT NULL, DEFAULT 'whisper_local' | Enum: whisper_local, openai_api, assemblyai, deepgram, aws_transcribe, azure_speech, google_speech |
| llm_provider | TEXT | NOT NULL, DEFAULT 'llama_local' | Enum: llama_local, ollama, openai, anthropic, google_gemini, aws_bedrock, azure_openai |
| prompt_template_used | TEXT | NULLABLE | Name of prompt template applied (e.g., "standard_meeting_summary") |

**Indexes:**
- `idx_meetings_started_at` B-tree on `started_at`
- `idx_meetings_status` B-tree on `status`
- `idx_meetings_sync_status` B-tree on `sync_status`
- `idx_meetings_folder_id` B-tree on `folder_id`

**Example Record:**
```json
{
  "id": "01904f3a-2b1c-7d8e-9f0a-1b2c3d4e5f6a",
  "title": "Sprint Planning - June 23",
  "started_at": 1719136800000,
  "ended_at": 1719140400000,
  "duration_seconds": 3600,
  "status": "completed",
  "audio_file_path": "audio/01904f3a.opus.enc",
  "model_used": "whisper-large-v3",
  "tags": "[\"sprint\", \"planning\", \"q3\"]",
  "folder_id": null,
  "created_at": 1719136800000,
  "updated_at": 1719140500000,
  "sync_status": "synced"
}
```

---

#### Table: `transcript_segments`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| meeting_id | TEXT | NOT NULL, FK → meetings.id | Parent meeting |
| start_ms | INTEGER | NOT NULL | Segment start time (ms from meeting start) |
| end_ms | INTEGER | NOT NULL | Segment end time |
| text | TEXT | NOT NULL | Transcribed text |
| speaker_id | TEXT | NULLABLE, FK → speakers.id | Identified speaker |
| confidence | REAL | NOT NULL, DEFAULT 0.0 | Model confidence score (0.0-1.0) |
| is_final | INTEGER (bool) | NOT NULL, DEFAULT 0 | 1 = final (batch), 0 = streaming draft |
| version | INTEGER | NOT NULL, DEFAULT 1 | Incremented on re-processing |

**Indexes:**
- `idx_segments_meeting_id` B-tree on `meeting_id`
- `idx_segments_meeting_start` B-tree on `(meeting_id, start_ms)`

**Example Record:**
```json
{
  "id": 1,
  "meeting_id": "01904f3a-2b1c-7d8e-9f0a-1b2c3d4e5f6a",
  "start_ms": 0,
  "end_ms": 4500,
  "text": "All right, let's get started. Today we're going to go over the sprint backlog.",
  "speaker_id": "spk_001",
  "confidence": 0.94,
  "is_final": 1,
  "version": 1
}
```

---

#### Table: `speakers`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | TEXT | PRIMARY KEY | UUID |
| meeting_id | TEXT | NOT NULL, FK → meetings.id | Parent meeting |
| label | TEXT | NOT NULL | "Speaker 1", "Speaker 2", etc. |
| display_name | TEXT | NULLABLE | User-assigned name |
| embedding | BLOB | NULLABLE | Speaker embedding vector (future) |

**Indexes:**
- `idx_speakers_meeting_id` B-tree on `meeting_id`

---

#### Table: `summaries`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| meeting_id | TEXT | NOT NULL, FK → meetings.id, UNIQUE | One summary per meeting |
| content | TEXT | NOT NULL | Markdown summary |
| generation_mode | TEXT | NOT NULL | Enum: rule_based, llm |
| model_used | TEXT | NULLABLE | LLM model identifier (if llm mode) |
| created_at | INTEGER (unix ms) | NOT NULL | Generation timestamp |

**Example Record:**
```json
{
  "id": 1,
  "meeting_id": "01904f3a-2b1c-7d8e-9f0a-1b2c3d4e5f6a",
  "content": "## Sprint Planning - June 23\n\nThe team reviewed the Q3 backlog...",
  "generation_mode": "llm",
  "model_used": "llama-3-8b-instruct",
  "created_at": 1719140500000
}
```

---

#### Table: `action_items`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | TEXT (UUIDv7) | PRIMARY KEY | Globally unique |
| meeting_id | TEXT | NOT NULL, FK → meetings.id | Source meeting |
| description | TEXT | NOT NULL | Action item text |
| assignee | TEXT | NULLABLE | Detected or user-assigned name |
| assignee_speaker_id | TEXT | NULLABLE, FK → speakers.id | Link to diarized speaker |
| deadline | TEXT | NULLABLE | Parsed deadline string (e.g., "2026-06-28") |
| source_segment_id | INTEGER | NULLABLE, FK → transcript_segments.id | Origin in transcript |
| status | TEXT | NOT NULL, DEFAULT 'pending' | Enum: pending, completed, cancelled |
| loop_task_id | TEXT | NULLABLE | ID in Loop after sync |
| sync_status | TEXT | NOT NULL, DEFAULT 'not_synced' | Sync state |
| created_at | INTEGER (unix ms) | NOT NULL | Creation time |

**Indexes:**
- `idx_action_items_meeting_id` B-tree on `meeting_id`
- `idx_action_items_status` B-tree on `status`
- `idx_action_items_sync_status` B-tree on `sync_status`

---

#### Table: `decisions`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | TEXT (UUIDv7) | PRIMARY KEY | Globally unique |
| meeting_id | TEXT | NOT NULL, FK → meetings.id | Source meeting |
| description | TEXT | NOT NULL | Decision text |
| context | TEXT | NULLABLE | Surrounding context |
| participants | TEXT | NULLABLE | JSON array of speaker IDs involved |
| source_segment_id | INTEGER | NULLABLE, FK → transcript_segments.id | Origin in transcript |
| vault_entry_id | TEXT | NULLABLE | ID in Vault after sync |
| sync_status | TEXT | NOT NULL, DEFAULT 'not_synced' | Sync state |
| created_at | INTEGER (unix ms) | NOT NULL | Creation time |

---

#### Table: `topic_tags`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| meeting_id | TEXT | NOT NULL, FK → meetings.id | Source meeting |
| tag | TEXT | NOT NULL | Tag string |

**Indexes:**
- `idx_topic_tags_meeting_id` B-tree on `meeting_id`
- `idx_topic_tags_tag` B-tree on `tag`

---

#### Table: `bookmarks`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| meeting_id | TEXT | NOT NULL, FK → meetings.id | Source meeting |
| timestamp_ms | INTEGER | NOT NULL | Position in meeting |
| note | TEXT | NULLABLE | User annotation |
| created_at | INTEGER (unix ms) | NOT NULL | Creation time |

---

#### Table: `folders`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | TEXT (UUIDv7) | PRIMARY KEY | Folder ID |
| name | TEXT | NOT NULL | Folder name |
| parent_id | TEXT | NULLABLE, FK → folders.id | Nested folders |
| created_at | INTEGER (unix ms) | NOT NULL | Creation time |

---

#### Table: `sync_queue`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| entity_type | TEXT | NOT NULL | Enum: meeting, action_item, decision |
| entity_id | TEXT | NOT NULL | ID of entity to sync |
| target | TEXT | NOT NULL | Enum: vault, loop |
| payload | TEXT | NOT NULL | JSON serialized sync payload |
| attempt_count | INTEGER | NOT NULL, DEFAULT 0 | Retry counter |
| next_retry_at | INTEGER (unix ms) | NOT NULL | Next retry timestamp |
| status | TEXT | NOT NULL, DEFAULT 'pending' | Enum: pending, in_progress, failed, permanently_failed |
| last_error | TEXT | NULLABLE | Last error message |
| created_at | INTEGER (unix ms) | NOT NULL | Queue entry creation |
| expires_at | INTEGER (unix ms) | NOT NULL | 30-day TTL |

**Indexes:**
- `idx_sync_queue_next_retry` B-tree on `next_retry_at`
- `idx_sync_queue_status` B-tree on `status`

---

#### Table: `models`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | TEXT | PRIMARY KEY | Model identifier (e.g., "whisper-large-v3") |
| display_name | TEXT | NOT NULL | Human-readable name |
| file_path | TEXT | NOT NULL | Local path to model weights |
| size_bytes | INTEGER | NOT NULL | Disk usage |
| sha256 | TEXT | NOT NULL | Integrity hash |
| model_type | TEXT | NOT NULL | Enum: asr, diarization, llm |
| is_active | INTEGER (bool) | NOT NULL, DEFAULT 0 | Currently selected |
| installed_at | INTEGER (unix ms) | NOT NULL | Installation time |

---

#### Table: `prompt_templates` (NEW)

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| name | TEXT | NOT NULL, UNIQUE | Template identifier (e.g., "standard_meeting_summary") |
| content | TEXT | NOT NULL | Prompt text with variables (e.g., "{{transcript}}") |
| meeting_type | TEXT | NULLABLE | Applies to specific meeting type (NULL = all types) |
| version | INTEGER | NOT NULL, DEFAULT 1 | Incremented on edit |
| is_custom | INTEGER (bool) | NOT NULL, DEFAULT 0 | 1 = user-edited, 0 = pre-built |
| created_at | INTEGER (unix ms) | NOT NULL | Creation time |
| updated_at | INTEGER (unix ms) | NOT NULL | Last modification time |

**Indexes:**
- `idx_prompt_templates_name` B-tree on `name`
- `idx_prompt_templates_meeting_type` B-tree on `meeting_type`

**Example Record:**
```json
{
  "id": 1,
  "name": "standard_meeting_summary",
  "content": "You are a meeting assistant. Summarize the following transcript...",
  "meeting_type": null,
  "version": 1,
  "is_custom": 0,
  "created_at": 1719136800000,
  "updated_at": 1719136800000
}
```

---

#### Table: `api_keys` (NEW)

| Field | Type | Constraints | Notes |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Row ID |
| provider | TEXT | NOT NULL, UNIQUE | Provider identifier (e.g., "openai", "anthropic", "assemblyai") |
| key_encrypted | TEXT | NOT NULL | AES-256-GCM encrypted API key |
| created_at | INTEGER (unix ms) | NOT NULL | Creation time |
| updated_at | INTEGER (unix ms) | NOT NULL | Last modification time |

**Indexes:**
- `idx_api_keys_provider` B-tree on `provider`

**Security:** Keys encrypted using AES-256-GCM with key derived from user passphrase (same as meeting data). Never logged or exposed in UI (masked input only).

---

#### Table: `configuration`

| Field | Type | Constraints | Notes |
|---|---|---|---|
| key | TEXT | PRIMARY KEY | Config key |
| value | TEXT | NOT NULL | JSON-encoded value |
| updated_at | INTEGER (unix ms) | NOT NULL | Last change |

**Keys include:** `audio_input_method`, `active_asr_model`, `active_llm_model`, `summarization_mode`, `data_directory`, `encryption_enabled`, `team_server_url`, `sync_rules`, `retention_policy`, `hotkey_record`, `hotkey_stop`, `language`, `theme`.

---

#### FTS5 Virtual Table: `transcript_search`

```sql
CREATE VIRTUAL TABLE transcript_search USING fts5(
  text,
  content='transcript_segments',
  content_rowid='id',
  tokenize='porter unicode61 remove_diacritics 2'
);
```

**Triggers:** Auto-populated on INSERT/UPDATE/DELETE of `transcript_segments`.

---

### 4.2 Team Server PostgreSQL Schema

#### Table: `users`

| Field | Type | Constraints |
|---|---|---|
| id | UUID | PRIMARY KEY, DEFAULT gen_random_uuid() |
| email | VARCHAR(255) | NOT NULL, UNIQUE |
| display_name | VARCHAR(255) | NOT NULL |
| password_hash | TEXT | NULLABLE (null if SSO-only) |
| role | VARCHAR(20) | NOT NULL, DEFAULT 'member' |
| organization_id | UUID | NOT NULL, FK → organizations.id |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() |
| last_login_at | TIMESTAMPTZ | NULLABLE |

#### Table: `organizations`

| Field | Type | Constraints |
|---|---|---|
| id | UUID | PRIMARY KEY, DEFAULT gen_random_uuid() |
| name | VARCHAR(255) | NOT NULL |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() |

#### Table: `meeting_metadata`

| Field | Type | Constraints |
|---|---|---|
| id | UUID | PRIMARY KEY |
| user_id | UUID | NOT NULL, FK → users.id |
| external_meeting_id | TEXT | NOT NULL (maps to desktop meeting UUID) |
| title | TEXT | NULLABLE |
| started_at | TIMESTAMPTZ | NOT NULL |
| duration_seconds | INTEGER | NULLABLE |
| sync_status | VARCHAR(20) | NOT NULL |
| vault_entry_ids | JSONB | DEFAULT '[]' |
| loop_task_ids | JSONB | DEFAULT '[]' |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() |

**Note:** Server does NOT store transcript content, audio, or summaries. Only metadata references.

#### Table: `audit_logs`

| Field | Type | Constraints |
|---|---|---|
| id | BIGSERIAL | PRIMARY KEY |
| user_id | UUID | NULLABLE, FK → users.id |
| action | VARCHAR(100) | NOT NULL |
| resource_type | VARCHAR(50) | NOT NULL |
| resource_id | TEXT | NULLABLE |
| metadata | JSONB | NULLABLE |
| ip_address | INET | NULLABLE |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() |

**Partitioning:** Monthly range partitioning on `created_at` for retention enforcement.

#### Table: `retention_policies`

| Field | Type | Constraints |
|---|---|---|
| id | UUID | PRIMARY KEY |
| organization_id | UUID | NOT NULL, FK → organizations.id |
| resource_type | VARCHAR(50) | NOT NULL (audio, transcript, summary) |
| retention_days | INTEGER | NOT NULL |
| action | VARCHAR(20) | NOT NULL (delete, archive) |
| enabled | BOOLEAN | NOT NULL, DEFAULT true |

#### Table: `sync_rules`

| Field | Type | Constraints |
|---|---|---|
| id | UUID | PRIMARY KEY |
| organization_id | UUID | NOT NULL, FK → organizations.id |
| target | VARCHAR(20) | NOT NULL (vault, loop) |
| conditions | JSONB | NOT NULL |
| enabled | BOOLEAN | NOT NULL, DEFAULT true |

---

### 4.3 Migration Strategy

- **Desktop:** SQL migration files in `migrations/` directory, applied sequentially on app start. Migration version tracked in `schema_version` table.
- **Server:** Go migrations via `golang-migrate` library. Applied on server startup. Version tracked in `schema_migrations` table.
- **Versioning:** Semantic versioning. Breaking schema changes require data migration scripts.

---

## 5. API Contracts

### 5.1 Team Server REST API

All endpoints return JSON. Standard error envelope:
```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable message",
    "retryable": false
  }
}
```

---

#### POST /auth/login

**Auth:** None
**Request:**
```json
{
  "email": "user@example.com",
  "password": "securepassword"
}
```
**Response (200):**
```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIs...",
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2g...",
  "expires_in": 28800,
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "display_name": "Jane Doe",
    "role": "member"
  }
}
```
**Errors:** 401 (invalid credentials), 429 (rate limited)

---

#### POST /auth/refresh

**Auth:** Bearer refresh_token
**Request:**
```json
{
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2g..."
}
```
**Response (200):** Same structure as /auth/login
**Errors:** 401 (invalid/expired refresh token)

---

#### POST /auth/sso/callback

**Auth:** None (SSO provider redirects here)
**Request:** SAML assertion or OIDC code in query params
**Response (302):** Redirect to desktop app with token in callback URL
**Errors:** 401 (SSO validation failed)

---

#### POST /api/v1/meetings/sync

**Auth:** Bearer JWT required
**Request:**
```json
{
  "meeting_id": "01904f3a-2b1c-7d8e-9f0a-1b2c3d4e5f6a",
  "title": "Sprint Planning - June 23",
  "started_at": "2026-06-23T10:00:00Z",
  "ended_at": "2026-06-23T11:00:00Z",
  "duration_seconds": 3600,
  "tags": ["sprint", "planning"],
  "action_item_count": 5,
  "decision_count": 2
}
```
**Response (201):**
```json
{
  "id": "server-uuid",
  "status": "recorded"
}
```
**Errors:** 400 (validation), 401 (auth), 409 (duplicate meeting_id)

---

#### POST /api/v1/vault/sync

**Auth:** Bearer JWT required
**Request:**
```json
{
  "meeting_id": "01904f3a-...",
  "summary": "## Sprint Planning\n\nThe team reviewed...",
  "decisions": [
    {
      "id": "decision-uuid",
      "description": "Adopt two-week sprint cadence",
      "context": "Team discussed sprint length options",
      "participants": ["Speaker 1", "Speaker 3"]
    }
  ],
  "tags": ["sprint", "planning", "q3"]
}
```
**Response (200):**
```json
{
  "vault_entry_ids": ["vault-entry-001", "vault-entry-002"],
  "status": "synced"
}
```
**Errors:** 400, 401, 502 (Vault unreachable), 503 (Vault unavailable)

---

#### POST /api/v1/loop/sync

**Auth:** Bearer JWT required
**Request:**
```json
{
  "meeting_id": "01904f3a-...",
  "action_items": [
    {
      "id": "action-uuid",
      "description": "Update sprint backlog with new priorities",
      "assignee": "Jane Doe",
      "deadline": "2026-06-28"
    }
  ]
}
```
**Response (200):**
```json
{
  "task_ids": ["loop-task-001"],
  "status": "synced"
}
```
**Errors:** 400, 401, 502 (Loop unreachable), 503 (Loop unavailable)

---

#### GET /api/v1/policies/retention

**Auth:** Bearer JWT required
**Response (200):**
```json
{
  "policies": [
    {
      "id": "policy-uuid",
      "resource_type": "audio",
      "retention_days": 90,
      "action": "delete",
      "enabled": true
    }
  ]
}
```

---

#### GET /api/v1/policies/sync-rules

**Auth:** Bearer JWT required
**Response (200):**
```json
{
  "rules": [
    {
      "id": "rule-uuid",
      "target": "vault",
      "conditions": {"tags_include": ["important"]},
      "enabled": true
    }
  ]
}
```

---

#### GET /api/v1/admin/audit

**Auth:** Bearer JWT, role=admin
**Query Params:** `page`, `per_page`, `user_id`, `action`, `from`, `to`
**Response (200):**
```json
{
  "entries": [
    {
      "id": 1,
      "user_id": "uuid",
      "action": "meeting.sync",
      "resource_type": "meeting",
      "resource_id": "meeting-uuid",
      "created_at": "2026-06-23T11:05:00Z"
    }
  ],
  "total": 150,
  "page": 1,
  "per_page": 50
}
```

---

#### GET /api/v1/admin/metrics

**Auth:** Bearer JWT, role=admin
**Response (200):**
```json
{
  "total_users": 45,
  "active_users_7d": 38,
  "meetings_processed_7d": 210,
  "storage_used_bytes": 5368709120,
  "sync_health": {
    "vault": "healthy",
    "loop": "healthy"
  }
}
```

---

#### GET /api/v1/admin/users

**Auth:** Bearer JWT, role=admin
**Response (200):**
```json
{
  "users": [
    {
      "id": "uuid",
      "email": "user@example.com",
      "display_name": "Jane Doe",
      "role": "member",
      "last_login_at": "2026-06-23T09:00:00Z",
      "meetings_count": 42
    }
  ]
}
```

---

### 5.2 Internal Module Interfaces (Rust)

#### AudioCapture → TranscriptionPipeline

```rust
trait AudioCaptureOutput {
    fn on_audio_chunk(&self, chunk: AudioChunk) -> Result<()>;
    fn on_recording_start(&self, session_id: Uuid) -> Result<()>;
    fn on_recording_stop(&self, session_id: Uuid) -> Result<()>;
}

struct AudioChunk {
    session_id: Uuid,
    data: Vec<i16>,        // PCM 16-bit samples
    sample_rate: u32,       // 16000
    channels: u8,           // 2 (system + mic)
    timestamp_ms: u64,
}
```

#### TranscriptionPipeline → SummarizationEngine

```rust
struct Transcript {
    meeting_id: Uuid,
    segments: Vec<TranscriptSegment>,
    speakers: Vec<Speaker>,
    total_duration_ms: u64,
}

struct TranscriptSegment {
    id: Option<i64>,
    start_ms: u64,
    end_ms: u64,
    text: String,
    speaker_id: Option<String>,
    confidence: f32,
    is_final: bool,
}
```

#### SummarizationEngine → Storage

```rust
struct MeetingOutput {
    meeting: MeetingRecord,
    transcript: Transcript,
    summary: SummaryRecord,
    action_items: Vec<ActionItemRecord>,
    decisions: Vec<DecisionRecord>,
    topic_tags: Vec<String>,
}
```

---

## 6. Business Logic Specification

### 6.1 Meeting Capture Flow

**Trigger:** User presses Record (hotkey or UI button)

**Steps:**
1. Desktop Shell receives record command
2. Audio Capture Engine initializes:
   - Detect active audio output device (system audio)
   - Detect microphone device (if enabled)
   - Create recording session (UUID)
   - Start platform-specific audio capture
3. Audio chunks flow to two destinations simultaneously:
   - **Real-time path:** PCM chunks → Transcription Pipeline (streaming mode) → UI display
   - **Storage path:** PCM chunks → BufferedWriter → flush to encrypted Opus file every 5s
4. Streaming transcription produces draft segments (is_final=false) displayed in UI
5. User optionally marks bookmarks during meeting
6. User presses Stop

**On Stop:**
1. Audio Capture Engine stops, closes audio file
2. Transcription Pipeline switches to batch mode:
   - Re-processes complete audio file with higher accuracy settings
   - Runs speaker diarization
   - Produces final Transcript (is_final=true for all segments)
3. Summarization Engine receives final Transcript:
   - If LLM mode: constructs prompt, invokes llama.cpp, parses structured output
   - If rule-based: applies pattern matching heuristics
   - Produces Summary, ActionItem[], Decision[], TopicTag[]
4. Storage Module persists all artifacts atomically (single SQLite transaction):
   - Meeting record (status=completed)
   - Transcript segments (final versions)
   - Summary
   - Action items
   - Decisions
   - Topic tags
   - Bookmarks
5. FTS5 index updated synchronously
6. "Meeting saved" event emitted → Sync Engine triggered

**Edge Cases:**
- Audio device disconnected mid-recording → recording stops with error, partial data preserved
- Disk full → recording stops with clear error, existing data preserved
- App crash → on restart, recover last flushed buffer (max 5s loss), process recovered audio

---

### 6.1.1 Import Audio File Flow (NEW)

**Trigger:** User drags audio file into Recap OR clicks "Import" button OR places file in watch folder

**Steps:**
1. AudioInputManager detects new file:
   - Drag-drop: `FileImporter::import_from_drag_drop(dropped_files)`
   - File browser: `FileImporter::import_from_path(path)`
   - Watch folder: `WatchFolderMonitor::on_new_file(callback)` triggered
2. Validate audio format (M4A, WAV, MP3, OGG, WebM, FLAC):
   - If unsupported: show error "Unsupported audio format. Supported: M4A, WAV, MP3, OGG, WebM, FLAC"
   - If supported: continue
3. Copy file to `~/RecapData/audio/{uuid}.{ext}` (or move if from watch folder)
4. Create meeting record in SQLite:
   ```sql
   INSERT INTO meetings (id, title, started_at, status, audio_file_path, audio_source, language)
   VALUES (?, ?, ?, 'imported', ?, 'file_import', 'en');
   ```
5. Show MetadataEntryDialog:
   - Pre-fill title from filename (without extension)
   - Auto-detect language if possible (from filename or audio analysis)
   - Fields: meeting_type, location, participants, language, topic
   - User can skip (metadata optional, can edit later)
6. Update meeting record with metadata (if provided)
7. Trigger transcription flow:
   - `STTRouter::transcribe(audio_file, language)` using selected STT provider
   - If provider fails and fallback configured: retry with fallback provider
   - Produce final Transcript
8. Continue with existing summarization flow (Section 6.1, steps 3-6)

**Watch Folder Debouncing:**
```rust
// Wait for file write completion before processing
let debounce_ms = 2000; // 2 seconds
let mut last_size = 0;
let mut stable_count = 0;
loop {
    let current_size = fs::metadata(&path)?.len();
    if current_size == last_size {
        stable_count += 1;
        if stable_count >= 3 { break; } // File size stable for 3 checks
    } else {
        stable_count = 0;
    }
    last_size = current_size;
    sleep(Duration::from_millis(500));
}
```

---

### 6.1.2 HTTP Upload from Smartphone Flow (NEW)

**Trigger:** Smartphone POSTs to `http://{desktop-ip}:8765/upload/audio`

**Steps:**
1. HttpUploadServer receives multipart form data:
   ```
   POST /upload/audio
   Content-Type: multipart/form-data
   
   - file: audio file (binary)
   - metadata: {"title": "...", "meeting_type": "client_call", ...} (JSON, optional)
   ```
2. Validate request:
   - Check file size (max 500MB)
   - Check audio format (M4A, WAV, MP3, OGG, WebM, FLAC)
3. Save audio file to `~/RecapData/inbox/{uuid}.{ext}`
4. If metadata provided: save to `~/RecapData/inbox/{uuid}.metadata.json`
5. Return response:
   ```json
   {
     "status": "uploaded",
     "meeting_id": "uuid",
     "message": "Audio uploaded successfully. Processing will begin shortly."
   }
   ```
6. WatchFolderMonitor detects new file in inbox (if watch folder enabled) OR HttpUploadServer directly triggers processing
7. Continue with "Import Audio File" flow from step 4

**Axum Handler Example:**
```rust
use axum::{extract::Multipart, response::Json};
use serde_json::json;

async fn upload_audio(mut multipart: Multipart) -> Result<Json<Value>, StatusCode> {
    let mut file_data = None;
    let mut metadata = None;
    
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        if name == "file" {
            file_data = Some(field.bytes().await.unwrap());
        } else if name == "metadata" {
            metadata = Some(field.text().await.unwrap());
        }
    }
    
    if let Some(data) = file_data {
        let uuid = Uuid::new_v4();
        let path = format!("~/RecapData/inbox/{}.m4a", uuid);
        fs::write(&path, data).await.unwrap();
        
        if let Some(meta) = metadata {
            let meta_path = format!("~/RecapData/inbox/{}.metadata.json", uuid);
            fs::write(&meta_path, meta).await.unwrap();
        }
        
        Ok(Json(json!({
            "status": "uploaded",
            "meeting_id": uuid,
            "message": "Audio uploaded successfully"
        })))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
```

---

### 6.2 Sync Flow

**Trigger:** "Meeting saved" event from Storage Module

**Steps:**
1. Sync Engine receives event with meeting_id
2. Load sync rules from configuration (or server in team mode)
3. Evaluate rules:
   - Vault sync: default = all meetings (configurable by tags)
   - Loop sync: default = all action items (configurable)
4. For each enabled sync target:
   - Transform data into target format
   - Create sync_queue entry (status=pending)
   - Attempt HTTP POST to team server proxy endpoint
5. On success:
   - Update sync_queue status=completed
   - Update meeting/action_item/decision sync_status=synced
   - Store returned IDs (vault_entry_id, loop_task_id)
6. On failure:
   - Update sync_queue: attempt_count++, next_retry_at (exponential backoff)
   - Update entity sync_status=failed
   - UI shows "sync pending" indicator

**Retry Logic:**
```
backoff = min(60 * 2^(attempt_count - 1), 3600)  // seconds
next_retry_at = now() + backoff
if attempt_count > max_attempts or now() > expires_at:
    status = permanently_failed
    notify_admin()
```

**Circuit Breaker:**
```
if failures_in_last_10min >= 5:
    circuit_state = OPEN
    suspend_syncs_for(15 minutes)
    after 15 min: circuit_state = HALF_OPEN, allow 1 probe request
    if probe succeeds: circuit_state = CLOSED
    if probe fails: circuit_state = OPEN, repeat
```

---

### 6.3 Authentication Flow (Team Mode)

**Trigger:** User opens app and team server URL is configured

**Steps:**
1. Desktop Shell checks for valid JWT in OS keychain
2. If no token or expired:
   - Show login screen (email/password or SSO button)
   - User submits credentials
   - POST /auth/login → receive access_token + refresh_token
   - Store tokens in OS keychain (encrypted)
3. If SSO:
   - Open system browser to /auth/sso/initiate
   - SSO provider authenticates user
   - Redirect to /auth/sso/callback with assertion/code
   - Server validates, returns tokens
   - Desktop app receives tokens via custom URL scheme
4. On each API request:
   - Attach Authorization: Bearer {access_token}
   - If 401 received: attempt refresh via POST /auth/refresh
   - If refresh fails: redirect to login

**Token Structure (JWT):**
```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "role": "member",
  "org_id": "org-uuid",
  "iat": 1719136800,
  "exp": 1719165600,  // 8 hours
  "iss": "recap-team-server"
}
```

---

### 6.4 Retention Policy Enforcement

**Trigger:** Scheduled job (daily) or server startup

**Steps (Server-side):**
1. Load all enabled retention policies for organization
2. For each policy:
   - Query meeting_metadata where created_at < (now - retention_days)
   - For each expired record:
     - If action=delete: send delete command to desktop client (via push or next poll)
     - If action=archive: mark as archived, exclude from active queries
3. Desktop client receives delete command:
   - Delete audio file from disk
   - Delete transcript/summary from SQLite (or mark archived)
   - Log deletion in local audit log

---

### 6.5 Model Selection & Switching

**Trigger:** User changes model in Settings

**Steps:**
1. User selects model from available models list
2. If model not installed:
   - Trigger download via Model Manager
   - Show progress bar (chunked download with resume)
   - Verify SHA-256 on completion
3. Update configuration: active_asr_model = selected_model_id
4. On next recording start:
   - Transcription Pipeline loads new model (lazy load)
   - Old model unloaded from memory
   - Recording proceeds with new model

**Decision Branch:** If model fails to load (corruption):
- Fall back to previously active model
- Notify user: "Model X failed to load, using Model Y"
- Offer re-download option

---

## 7. State Management & Data Flow

**State ownership:** Desktop client owns all meeting data (authoritative, local-first). Team server owns users, policies, audit logs, metadata references only. Vault/Loop own their derived data.

**Caching:** LRU memory cache (100MB default) for transcripts/summaries; SQLite page cache tuned for reads; model weights retained in memory until restart; Redis for server sessions/rate-limits.

**Flow:** `Audio Device → AudioCapture → [streaming→UI | buffer→disk 5s] → [stop] → Batch Transcription → Summarization → Storage (atomic SQLite+FTS5) → Sync Engine → Vault/Loop`

**Consistency:** Single-user = strong (SQLite ACID). Team mode = eventual (client authoritative, server async, last-write-wins for metadata).

---

## 8. Background Jobs & Asynchronous Processing

| Job | Trigger | Retry | Idempotency |
|---|---|---|---|
| Sync retry worker | Timer (60s) checks sync_queue | Exponential backoff, max 30d | meeting_id is unique key |
| Batch transcription | Recording stop event | Manual re-trigger | Creates new transcript version |
| Summarization | Batch transcription complete | Auto-fallback to rule-based | Re-run produces new summary |
| Retention enforcement | Daily cron (02:00) or startup | N/A | Idempotent by date check |
| Model download | User request | Resume from last chunk | Skip if SHA-256 matches |

**Queue system:** Desktop uses in-process Rust async channels (no external dependency). Server uses PostgreSQL-backed job table.

---

## 9. External Integrations

| Integration | Endpoint | Auth | Rate Limits | Fallback |
|---|---|---|---|---|
| Vault (via server proxy) | POST /api/v1/vault/sync | JWT + server API key | Token bucket (50 req/min) | Queue locally, retry 30d |
| Loop (via server proxy) | POST /api/v1/loop/sync | JWT + server API key | Token bucket (50 req/min) | Queue locally, retry 30d |
| Model Registry (CDN/HF) | HTTPS GET | None or HF token | CDN limits (chunked+resume) | Manual file for air-gap |
| SSO (SAML/OIDC) | Configurable per org | SAML assertion / OIDC code | Provider-specific | Local credentials fallback |

---

## 10. Configuration & Environment Variables

### 10.1 Desktop Application (configuration table + env overrides)

| Key | Purpose | Default | Example |
|---|---|---|---|
| `data_directory` | Meeting data storage location | `~/RecapData` | `/mnt/storage/recap` |
| `audio_input_method` | system_audio, microphone, both | `system_audio` | `both` |
| `active_asr_model` | Selected ASR model ID | `whisper-medium` | `whisper-large-v3` |
| `active_llm_model` | Selected LLM model ID | `none` (rule-based) | `llama-3-8b-instruct` |
| `summarization_mode` | rule_based, llm, auto | `auto` | `llm` |
| `encryption_enabled` | Encrypt data at rest | `true` | `true` |
| `team_server_url` | Team server base URL | `""` (disabled) | `https://recap.internal:8443` |
| `language` | UI language | `en` | `en`, `de`, `fr` |
| `hotkey_record` | Global hotkey for recording | `Ctrl+Shift+R` | `Cmd+Shift+R` |
| `hotkey_stop` | Global hotkey for stop | `Ctrl+Shift+S` | `Cmd+Shift+S` |
| `theme` | UI theme | `system` | `dark`, `light` |
| **Audio Input Flexibility (NEW)** | | | |
| `watch_folder_enabled` | Enable watch folder monitoring | `false` | `true` |
| `watch_folder_path` | Path to watch for new audio files | `~/RecapData/inbox` | `/home/user/recordings` |
| `http_upload_server_enabled` | Enable embedded HTTP upload server | `false` | `true` |
| `http_upload_server_port` | Port for HTTP upload server | `8765` | `9000` |
| **STT Provider Configuration (NEW)** | | | |
| `stt_provider` | Selected STT provider | `whisper_local` | `openai_api`, `assemblyai`, `deepgram`, `aws_transcribe`, `azure_speech`, `google_speech` |
| `stt_fallback_provider` | Secondary provider if primary fails | `""` (none) | `whisper_local` |
| `stt_openai_api_key` | OpenAI API key (encrypted) | `""` | (encrypted string) |
| `stt_assemblyai_api_key` | AssemblyAI API key (encrypted) | `""` | (encrypted string) |
| `stt_deepgram_api_key` | Deepgram API key (encrypted) | `""` | (encrypted string) |
| `stt_aws_access_key` | AWS access key (encrypted) | `""` | (encrypted string) |
| `stt_aws_secret_key` | AWS secret key (encrypted) | `""` | (encrypted string) |
| `stt_azure_key` | Azure Speech key (encrypted) | `""` | (encrypted string) |
| `stt_google_credentials` | Path to Google credentials JSON | `""` | `/path/to/credentials.json` |
| **LLM Provider Configuration (NEW)** | | | |
| `llm_provider` | Selected LLM provider | `llama_local` | `ollama`, `openai`, `anthropic`, `google_gemini`, `aws_bedrock`, `azure_openai` |
| `llm_model` | Model identifier for selected provider | `llama-3-8b-instruct` | `gpt-4`, `claude-3-sonnet`, `gemini-pro` |
| `llm_openai_api_key` | OpenAI API key (encrypted) | `""` | (encrypted string) |
| `llm_anthropic_api_key` | Anthropic API key (encrypted) | `""` | (encrypted string) |
| `llm_google_api_key` | Google Gemini API key (encrypted) | `""` | (encrypted string) |
| `llm_aws_access_key` | AWS access key (encrypted) | `""` | (encrypted string) |
| `llm_aws_secret_key` | AWS secret key (encrypted) | `""` | (encrypted string) |
| `llm_azure_key` | Azure OpenAI key (encrypted) | `""` | (encrypted string) |
| **Prompt Configuration (NEW)** | | | |
| `prompt_template` | Active prompt template name | `standard_meeting_summary` | `client_call_action_items` |
| `prompt_custom` | User-edited prompt content | `""` | (custom prompt text) |
| `prompt_version` | Version number for custom prompts | `1` | `2`, `3` |

### 10.2 Team Server (environment variables)

| Variable | Purpose | Dev | Staging | Prod |
|---|---|---|---|---|
| `DATABASE_URL` | PostgreSQL connection | `postgres://localhost:5432/recap` | `postgres://staging-db:5432/recap` | `postgres://prod-db:5432/recap?sslmode=require` |
| `REDIS_URL` | Redis connection | `redis://localhost:6379` | `redis://staging-redis:6379` | `redis://prod-redis:6379` |
| `JWT_SECRET_KEY` | JWT signing key (RS256 private key path) | `./keys/dev.pem` | `/run/secrets/jwt_key` | `/run/secrets/jwt_key` |
| `JWT_ISSUER` | JWT issuer claim | `recap-dev` | `recap-staging` | `recap` |
| `JWT_TTL_HOURS` | Access token TTL | `24` | `8` | `8` |
| `VAULT_API_URL` | Vault module base URL | `http://localhost:3001` | `http://vault:3001` | `https://vault.internal:3001` |
| `VAULT_API_KEY` | Vault service account key | `dev-key` | (secret) | (secret) |
| `LOOP_API_URL` | Loop module base URL | `http://localhost:3002` | `http://loop:3002` | `https://loop.internal:3002` |
| `LOOP_API_KEY` | Loop service account key | `dev-key` | (secret) | (secret) |
| `SERVER_PORT` | HTTP listen port | `8080` | `8443` | `8443` |
| `TLS_CERT_PATH` | TLS certificate | `""` (HTTP) | `/certs/server.crt` | `/certs/server.crt` |
| `TLS_KEY_PATH` | TLS private key | `""` | `/certs/server.key` | `/certs/server.key` |
| `RATE_LIMIT_RPM` | Requests per minute per user | `1000` | `100` | `100` |
| `LOG_LEVEL` | Logging verbosity | `debug` | `info` | `warn` |

---

## 11. Security Implementation Details

### Authentication Flow

1. User submits credentials (email/password or SSO)
2. Server validates against PostgreSQL (bcrypt hash comparison) or SSO provider
3. Server issues JWT (RS256, 8-hour TTL) + refresh token (30-day TTL, single-use)
4. Client stores tokens in OS keychain (encrypted)
5. Each request: `Authorization: Bearer {access_token}`
6. On 401: client attempts refresh; on refresh failure: redirect to login

### Authorization Model (RBAC)

| Role | Permissions |
|---|---|
| Admin | Full access: user management, policy config, audit logs, all meetings |
| Manager | Team workspace access, sync configuration, view team meetings |
| Member | Own meetings only, no admin functions |

Enforced server-side on all API endpoints. Client-side enforcement is UX guidance only.

### Data Encryption

- **At rest:** AES-256-GCM for all meeting data (audio, transcripts, summaries)
- **Key derivation:** Argon2id (memory=64MB, iterations=3, parallelism=4) from user passphrase + 32-byte random salt
- **Key hierarchy:** Master key → per-file DEKs (data encryption keys) → file content
- **No recovery:** If user forgets passphrase, data is unrecoverable (by design)

### Input Validation

- All API inputs validated against JSON schemas
- SQL injection prevented via parameterized queries
- Path traversal prevented via path canonicalization + allowlist
- Audio file types restricted to Opus/WAV
- Max upload size: 500MB (audio file)

---

## 12. Error Handling & Logging

### Standard Error Response

```json
{
  "error": {
    "code": "TRANSCRIPTION_MODEL_LOAD_FAILED",
    "message": "Failed to load whisper-large-v3 model. File may be corrupted.",
    "retryable": false,
    "details": {
      "model_id": "whisper-large-v3",
      "file_path": "/models/whisper-large-v3.bin",
      "expected_sha256": "abc123...",
      "actual_sha256": "def456..."
    }
  }
}
```

### Error Categories

- **Validation errors (4xx):** Invalid input, missing fields, schema violations
- **System errors (5xx):** Internal failures, database errors, disk full
- **External dependency failures (502/503):** Vault/Loop unreachable, model download failed

### Logging Format

Structured JSON logs:
```json
{
  "timestamp": "2026-06-23T10:05:00.123Z",
  "level": "info",
  "module": "transcription",
  "meeting_id": "01904f3a-...",
  "message": "Batch transcription completed",
  "duration_ms": 485000,
  "segments_count": 342,
  "trace_id": "abc123..."
}
```

### Correlation IDs

- **trace_id:** Generated per recording session, propagated through all pipeline stages
- **request_id:** Generated per API request (server), included in response headers

---

## 13. Performance Considerations

### Expected Load

- **Desktop:** Single user, single recording at a time. No concurrent load concerns.
- **Server:** ≤200 concurrent users. ~100 sync requests/minute peak (assuming 3 meetings/user/day).

### Bottleneck Analysis

| Bottleneck | Location | Mitigation |
|---|---|---|
| Transcription throughput | Desktop CPU/GPU | Model tier selection; GPU acceleration (Metal/CUDA); batch on stop (non-blocking) |
| Storage I/O during recording | Desktop disk | Buffered writes (5s); Opus compression (~10x); SSD recommended |
| Search at scale | SQLite FTS5 | Performant to ~10,000 hours; migrate to Tantivy/Meilisearch beyond that |
| Sync throughput | Server ↔ Vault/Loop | Async queue; batch sync; backpressure on throttle |
| Model download | Network | Chunked download + resume; CDN mirror; smaller default model |

### Optimization Strategies

- **Caching:** LRU for transcripts/summaries (100MB); model weights in memory; SQLite page cache
- **Batching:** Sync multiple meetings in single HTTP request when queue has ≥5 pending items
- **Parallelism:** Audio capture + streaming transcription run concurrently; batch transcription + summarization run sequentially (dependency)
- **Compression:** Opus audio encoding (~10x smaller than WAV); SQLite WAL mode for concurrent reads

---

## 14. Testing Strategy

### Unit Tests

- **Coverage target:** ≥80% for all modules
- **Focus areas:** Transcription backend trait implementations, sync rule evaluation, encryption/decryption, retry logic, RBAC enforcement
- **Framework:** Rust `#[test]` + `cargo test`; Go `testing` package

### Integration Tests

- **Scope:** Module interactions (AudioCapture → Transcription → Summarization → Storage), server API endpoints with PostgreSQL, sync flow end-to-end (mock Vault/Loop)
- **Framework:** Rust integration tests in `tests/` directory; Go integration tests with testcontainers for PostgreSQL/Redis

### End-to-End Tests

- **Scenarios:**
  1. Record 30-second audio → verify transcript appears in UI
  2. Complete meeting flow → verify summary + action items generated
  3. Team mode: login → record → sync → verify Vault/Loop entries created
  4. Crash recovery: kill app mid-recording → restart → verify partial recovery
  5. Air-gapped: manual model placement → verify transcription works offline
- **Framework:** Tauri driver (WebDriver-based) for UI; scripted audio files for deterministic input

### Mocking Strategy

- **Audio:** Pre-recorded WAV files with known transcripts (ground truth)
- **Vault/Loop:** Mock HTTP server returning canned responses
- **SSO:** Mock SAML/OIDC provider
- **Models:** Small test models (whisper-tiny) for fast CI runs

---

## 15. Deployment & Build Instructions

### Desktop Application Build

```bash
# Prerequisites: Rust 1.75+, Node.js 20+, system dependencies (see Tauri docs)
cd desktop-app
cargo tauri build --target x86_64-apple-darwin    # macOS Intel
cargo tauri build --target aarch64-apple-darwin   # macOS Apple Silicon
cargo tauri build --target x86_64-pc-windows-msvc # Windows
cargo tauri build --target x86_64-unknown-linux-gnu # Linux
```

**Output:** Platform-specific installer (.dmg, .msi, .AppImage)

### Team Server Build & Deploy

```bash
# Build
cd team-server
go build -o recap-server ./cmd/server

# Docker
docker build -t odwai/recap-server:1.0.0 .

# Deploy (one-command)
docker-compose up -d  # Starts server + PostgreSQL + Redis
```

**docker-compose.yml:**
```yaml
version: '3.8'
services:
  server:
    image: odwai/recap-server:1.0.0
    ports:
      - "8443:8443"
    environment:
      - DATABASE_URL=postgres://recap:secret@postgres:5432/recap
      - REDIS_URL=redis://redis:6379
    depends_on:
      - postgres
      - redis
    volumes:
      - ./certs:/certs:ro
      - ./keys:/keys:ro

  postgres:
    image: postgres:16
    environment:
      - POSTGRES_USER=recap
      - POSTGRES_PASSWORD=secret
      - POSTGRES_DB=recap
    volumes:
      - pgdata:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redisdata:/data

volumes:
  pgdata:
  redisdata:
```

### Service Startup Sequence

1. PostgreSQL starts, runs migrations
2. Redis starts
3. Team server starts, connects to DB + Redis, begins listening
4. Desktop clients connect to team server (if configured)

### CI/CD

- **GitHub Actions:** Lint → Test → Build → Sign → Release
- **Desktop:** Cross-compile on macOS (for all platforms) or platform-specific runners
- **Server:** Build Docker image, push to registry, deploy via docker-compose or Kubernetes

---

## 16. Observability Hooks

### Metrics (Server)

- `recap_http_requests_total{method, path, status}` — request counter
- `recap_http_request_duration_seconds{method, path}` — request latency histogram
- `recap_sync_operations_total{target, status}` — sync success/failure counter
- `recap_active_users` — gauge of authenticated users
- `recap_meetings_processed_total` — counter of meetings synced to server
- `recap_sync_queue_depth` — gauge of pending sync operations

### Metrics (Desktop)

- Transcription latency (streaming and batch)
- Model memory usage
- Storage usage (bytes)
- Sync queue depth

### Logs

- Structured JSON (see Section 12)
- Levels: error, warn, info, debug, trace
- Desktop: logs to `~/RecapData/logs/` (rotated, 10MB max per file, 5 files)
- Server: stdout (captured by Docker/container orchestrator)

### Traces

- OpenTelemetry-compatible trace IDs propagated through pipeline
- Spans: audio_capture, transcription_streaming, transcription_batch, summarization, sync_vault, sync_loop

### Health Check Endpoints (Server)

- `GET /health` → 200 if server is running
- `GET /health/ready` → 200 if DB + Redis connected
- `GET /health/vault` → 200 if Vault reachable
- `GET /health/loop` → 200 if Loop reachable

---

## 17. File & Folder Structure

```
recap/
├── desktop-app/                  # Tauri desktop application
│   ├── src-tauri/               # Rust backend
│   │   ├── src/
│   │   │   ├── main.rs          # Entry point
│   │   │   ├── audio/           # Audio capture module
│   │   │   │   ├── mod.rs
│   │   │   │   ├── coreaudio.rs
│   │   │   │   ├── wasapi.rs
│   │   │   │   └── pulseaudio.rs
│   │   │   ├── transcription/   # Transcription pipeline
│   │   │   │   ├── mod.rs
│   │   │   │   ├── whisper_cpp.rs
│   │   │   │   ├── diarization.rs
│   │   │   │   └── streaming.rs
│   │   │   ├── summarization/   # Summarization engine
│   │   │   │   ├── mod.rs
│   │   │   │   ├── rule_based.rs
│   │   │   │   └── llm.rs
│   │   │   ├── storage/         # Local storage manager
│   │   │   │   ├── mod.rs
│   │   │   │   ├── sqlite.rs
│   │   │   │   ├── encryption.rs
│   │   │   │   └── export.rs
│   │   │   ├── sync/            # Sync engine
│   │   │   │   ├── mod.rs
│   │   │   │   ├── vault.rs
│   │   │   │   ├── loop_sync.rs
│   │   │   │   └── retry.rs
│   │   │   ├── model_manager/   # Model lifecycle
│   │   │   │   ├── mod.rs
│   │   │   │   └── downloader.rs
│   │   │   └── commands/        # Tauri command handlers
│   │   │       ├── mod.rs
│   │   │       └── ...
│   │   ├── migrations/          # SQLite migrations
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   ├── src/                     # Frontend (TypeScript/React)
│   │   ├── components/
│   │   ├── pages/
│   │   ├── hooks/
│   │   ├── stores/
│   │   └── App.tsx
│   ├── package.json
│   └── vite.config.ts
│
├── team-server/                 # Go team server
│   ├── cmd/
│   │   └── server/
│   │       └── main.go
│   ├── internal/
│   │   ├── auth/                # Authentication handlers
│   │   ├── api/                 # REST API handlers
│   │   ├── middleware/          # Auth, logging, rate limiting
│   │   ├── models/              # Data models
│   │   ├── repository/          # PostgreSQL queries
│   │   ├── services/            # Business logic
│   │   └── proxy/               # Vault/Loop proxy
│   ├── migrations/              # PostgreSQL migrations
│   ├── Dockerfile
│   ├── docker-compose.yml
│   └── go.mod
│
├── docs/                        # Documentation
│   ├── api.md
│   ├── deployment.md
│   └── user-guide.md
│
├── tests/                       # Integration/E2E tests
│   ├── fixtures/                # Test audio files, mock responses
│   ├── integration/
│   └── e2e/
│
├── scripts/                     # Build/release scripts
│   ├── build-desktop.sh
│   └── build-server.sh
│
├── .github/
│   └── workflows/               # CI/CD
│       ├── ci.yml
│       └── release.yml
│
├── README.md
└── LICENSE
```

---

## 18. Coding Standards & Conventions

### Naming Conventions

- **Rust:** snake_case for functions/variables, PascalCase for types/traits, SCREAMING_SNAKE_CASE for constants
- **Go:** camelCase for unexported, PascalCase for exported, snake_case for DB columns
- **TypeScript:** camelCase for variables/functions, PascalCase for components/types
- **API paths:** kebab-case, plural nouns (`/api/v1/meetings`, `/api/v1/action-items`)
- **Database tables:** snake_case, plural (`meeting_segments`, `action_items`)
- **Environment variables:** SCREAMING_SNAKE_CASE (`DATABASE_URL`, `JWT_SECRET_KEY`)

### API Naming Rules

- RESTful resource-oriented URLs
- Version prefix: `/api/v1/`
- Consistent HTTP methods: GET (read), POST (create), PUT (full update), PATCH (partial update), DELETE
- Pagination: `?page=1&per_page=50` → response includes `total`, `page`, `per_page`
- Filtering: query params (`?status=completed&tag=sprint`)
- Sorting: `?sort=created_at&order=desc`

### Error Format Standard

```json
{
  "error": {
    "code": "UPPER_SNAKE_CASE_ERROR_CODE",
    "message": "Human-readable description",
    "retryable": boolean,
    "details": { ... }  // optional, context-specific
  }
}
```

### Code Organization Principles

- **Single responsibility:** Each module has one clear purpose
- **Dependency inversion:** Modules depend on traits/interfaces, not concrete implementations
- **Explicit error handling:** No panics in production code; all errors propagated via Result/Result types
- **Immutable data:** Transcript segments are immutable once finalized; re-processing creates new versions
- **Fail-fast:** Validate inputs at API boundary; propagate errors immediately
- **No silent failures:** All errors logged with context; user notified of actionable failures

---

## 19. Assumptions & Constraints

### Assumptions (from PRD/SAD)

- Users have sufficient local compute (CPU/GPU) for transcription workloads
- Whisper-family models provide acceptable accuracy (WER ≤8% on business English)
- Local LLMs (llama.cpp) can produce structured output reliably with proper prompting
- Users will tolerate one-time model download (~1.5GB for medium, ~3GB for large)
- Team server deployments have PostgreSQL and Redis available (via docker-compose)
- Vault and Loop modules expose REST APIs compatible with the proxy pattern
- OS audio subsystems allow system audio capture (BlackHole on macOS, WASAPI on Windows, PulseAudio monitor on Linux)
- Users can obtain and configure virtual audio devices if needed (macOS requires BlackHole)

### Technical Constraints

- **No cloud processing:** All audio, transcription, and summarization must occur on-device or customer infrastructure
- **No meeting bots:** Cannot join meetings as participants; must capture via system audio
- **Offline-first:** Desktop app must function fully without network (after initial model download)
- **Data sovereignty:** No meeting content leaves the device; server stores only metadata references
- **Encryption mandatory:** All meeting data encrypted at rest (user-controlled keys, no recovery)
- **Performance targets:** Streaming latency ≤2s; batch ≤10min for 60min meeting; search ≤1s
- **Platform support:** macOS (primary), Windows (primary), Linux (secondary)
- **Team server scale:** ≤200 concurrent users per instance (v1.0)
- **Storage capacity:** ≥10,000 hours of meetings per installation
- **Model agnosticism:** Architecture supports swapping ASR and LLM models without code changes

### Known Risks

- **Audio capture complexity:** Platform-specific audio APIs may have edge cases (device switching, permissions, conflicts)
- **Speaker diarization accuracy:** May require user correction; not perfect out-of-the-box
- **LLM hallucination:** Rule-based fallback mitigates, but LLM mode may produce inaccurate summaries
- **Encryption key loss:** No recovery by design; user education critical
- **Model size:** Large models may be prohibitive on low-resource machines
- **OS updates:** Audio capture APIs may change with OS updates (especially macOS)

---

**End of Technical Specification Document**
