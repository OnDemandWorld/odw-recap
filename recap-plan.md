# Plan: Update Recap Specification Documents

## Objective

Update the 4 specification documents (PRD, SAD, TSD, TBK) to support:
1. **Flexible audio input**: smartphone recording, file upload, folder monitoring
2. **Meeting metadata entry**: type, location, participants, language(s), topic
3. **Multiple STT backends**: local models, paid APIs, cloud-hosted services
4. **Multiple LLM options**: local models, cloud providers for summarization
5. **Prompt templates**: sample prompts, editable templates, prompt management

---

## Research Findings Summary

### Mobile Audio Upload
- **WhisperClip pattern**: Local HTTP server on desktop, phone POSTs audio via LAN
- **tauri-plugin-audio-recorder**: Tauri 2.x plugin for iOS/Android recording
- **LocalSend protocol** (83K ⭐): Zero-config LAN file transfer, Rust implementations available
- **notify crate** (62.7M downloads): File system watching with debouncing

### STT Backends
- **adk-audio** (Rust): Unified interface for OpenAI, Deepgram, AssemblyAI
- **whisrs** (Rust): Multi-provider (Groq, Deepgram, OpenAI, whisper.cpp)
- **aws-sdk-transcribe**: Official AWS SDK for Rust
- **whisper-rs**: Local whisper.cpp bindings

### LLM Abstraction
- **rig-core** (6K ⭐): 10+ providers, WASM support, RAG capabilities
- **Vercel AI SDK**: TypeScript, structured output with Zod schemas
- **instructor-rs**: Schema-validated structured output
- **Langfuse**: Prompt versioning and management (self-hosted)

---

## Document Updates

### 1. PRD (Product Requirements Document) Updates

#### Section 1.3 Solution (Update)
**Current**: "Captures audio from meetings via local system audio routing"
**Add**: Multiple audio input methods:
- System audio capture (existing)
- Smartphone recording via companion app or iOS Shortcuts
- File upload (drag-and-drop, file browser)
- Watch folder (automatic processing of new audio files)

#### Section 3.1 In Scope (Add new subsection)
**New**: "Audio Input Flexibility"
- Smartphone companion app (Tauri v2 mobile) for iOS/Android recording
- Local HTTP upload server (Axum-based) for LAN audio transfer
- Watch folder with automatic processing (notify crate)
- Support for common audio formats (M4A, WAV, MP3, OGG, WebM)

#### Section 3.2 Out of Scope (Update)
**Remove**: "Mobile apps: Desktop-only for v1.0"
**Replace with**: "Mobile apps: Companion recording app for iOS/Android (uploads to desktop, no local processing)"

#### Section 5.2 Journey: Capturing a Meeting (Add new journeys)
**New Journey**: "Importing Audio from Smartphone"
```
User records meeting on smartphone (companion app or voice recorder)
    → Opens Recap mobile companion OR uses iOS Shortcuts/Android share
    → Audio uploaded via local network to Recap desktop (HTTP POST or LocalSend)
    → Recap detects new file in inbox folder
    → User prompted to enter meeting metadata (type, participants, language)
    → Transcription and summarization proceed automatically
```

**New Journey**: "Uploading Audio File"
```
User has existing audio recording (from phone, recorder, etc.)
    → Drags file into Recap OR uses "Import" button
    → OR places file in watched folder (~/RecapData/inbox/)
    → Recap detects new file
    → User prompted for meeting metadata
    → Processing begins
```

**New Journey**: "Entering Meeting Metadata"
```
User starts recording or imports audio file
    → Metadata entry dialog appears (can skip, edit later)
    → Fields:
        - Meeting title (auto-generated from date/time if blank)
        - Meeting type (dropdown: team meeting, 1:1, client call, interview, presentation, workshop, other)
        - Location (text field: "Conference Room A", "Zoom", "Remote")
        - Participants (comma-separated names or select from team roster)
        - Language(s) (dropdown: English, Spanish, French, German, etc. - multi-select)
        - Topic/Project (text field or select from tags)
    → Metadata stored with meeting record
    → Used to enhance summarization (e.g., "client call" → focus on action items)
```

#### Section 6.1 Audio Capture (Expand)
**Add new requirements**:

| ID | Requirement | Priority |
|---|---|---|
| FR-AC-10 | Support audio file import (drag-and-drop, file browser, watch folder) | P0 |
| FR-AC-11 | Watch folder: auto-detect and process new audio files in ~/RecapData/inbox/ | P1 |
| FR-AC-12 | Local HTTP upload server for smartphone audio transfer (LAN only, no cloud) | P1 |
| FR-AC-13 | Companion mobile app for iOS/Android recording (uploads to desktop) | P2 |
| FR-AC-14 | Support audio formats: M4A, WAV, MP3, OGG, WebM, FLAC | P0 |
| FR-AC-15 | Meeting metadata entry form (type, location, participants, language, topic) | P0 |
| FR-AC-16 | iOS Shortcuts integration for audio upload | P2 |

#### Section 6.2 Transcription (Expand)
**Add new requirements**:

| ID | Requirement | Priority |
|---|---|---|
| FR-TR-11 | Multiple STT backend support: local (whisper.cpp), paid APIs (OpenAI, AssemblyAI, Deepgram), cloud (AWS Transcribe, Azure Speech, Google Speech-to-Text) | P0 |
| FR-TR-12 | User-selectable STT provider in settings (with cost/quality tradeoffs displayed) | P0 |
| FR-TR-13 | Fallback chain: if primary STT fails, try secondary provider | P1 |
| FR-TR-14 | API key management for paid STT services (encrypted storage) | P0 |
| FR-TR-15 | Language selection per meeting (multi-language support) | P0 |
| FR-TR-16 | Cost estimation for paid STT APIs (before transcription) | P2 |

#### Section 6.3 Structured Output (Expand)
**Add new requirements**:

| ID | Requirement | Priority |
|---|---|---|
| FR-SO-10 | Multiple LLM provider support: local (llama.cpp, Ollama), cloud (OpenAI, Anthropic, Google Gemini, AWS Bedrock, Azure OpenAI) | P0 |
| FR-SO-11 | User-selectable LLM provider in settings | P0 |
| FR-SO-12 | Prompt template library (pre-built templates for different meeting types) | P0 |
| FR-SO-13 | Editable prompts: users can customize summarization prompts | P1 |
| FR-SO-14 | Prompt versioning (track changes, rollback to previous versions) | P2 |
| FR-SO-15 | Structured output schemas (JSON Schema validation for action items, decisions) | P0 |
| FR-SO-16 | Meeting-type-specific prompts (client call → focus on commitments; team meeting → focus on blockers) | P1 |
| FR-SO-17 | API key management for cloud LLM services | P0 |
| FR-SO-18 | Cost estimation for cloud LLM APIs | P2 |

#### Section 9.1 Core Data Entities (Update Meeting entity)
**Add fields to Meeting**:
```typescript
Meeting {
  // ... existing fields ...
  meeting_type: enum [team_meeting, one_on_one, client_call, interview, presentation, workshop, other]
  location: string?
  participants: [string]
  language: string (ISO 639-1 code, e.g., "en", "es", "fr")
  topic: string?
  audio_source: enum [system_capture, file_import, mobile_upload, watch_folder]
  stt_provider: enum [whisper_local, openai_api, assemblyai, deepgram, aws_transcribe, azure_speech, google_speech]
  llm_provider: enum [llama_local, ollama, openai, anthropic, google_gemini, aws_bedrock, azure_openai]
}
```

#### Section 9.2 Configuration State (Update)
**Add new config fields**:
```typescript
AppConfig {
  // ... existing fields ...
  
  // Audio Input
  watch_folder_enabled: boolean (default: false)
  watch_folder_path: string (default: "~/RecapData/inbox")
  http_upload_server_enabled: boolean (default: false)
  http_upload_server_port: number (default: 8765)
  
  // STT Providers
  stt_provider: enum [whisper_local, openai_api, assemblyai, deepgram, aws_transcribe, azure_speech, google_speech]
  stt_openai_api_key: string? (encrypted)
  stt_assemblyai_api_key: string? (encrypted)
  stt_deepgram_api_key: string? (encrypted)
  stt_aws_access_key: string? (encrypted)
  stt_aws_secret_key: string? (encrypted)
  stt_azure_key: string? (encrypted)
  stt_google_credentials: string? (encrypted, path to JSON)
  stt_fallback_provider: enum? (secondary provider if primary fails)
  
  // LLM Providers
  llm_provider: enum [llama_local, ollama, openai, anthropic, google_gemini, aws_bedrock, azure_openai]
  llm_openai_api_key: string? (encrypted)
  llm_anthropic_api_key: string? (encrypted)
  llm_google_api_key: string? (encrypted)
  llm_aws_access_key: string? (encrypted)
  llm_aws_secret_key: string? (encrypted)
  llm_azure_key: string? (encrypted)
  llm_model: string (e.g., "gpt-4", "claude-3-sonnet", "gemini-pro")
  
  // Prompts
  prompt_template: string (default: "standard_meeting_summary")
  prompt_custom: string? (user-edited prompt)
  prompt_version: number (for versioning)
}
```

---

### 2. SAD (System Architecture Document) Updates

#### Section 1.2 Architectural Style (Update)
**Add**: "Modular monolith with pluggable provider backends"
- STT providers are pluggable (trait-based, can swap whisper.cpp ↔ OpenAI API ↔ AWS Transcribe)
- LLM providers are pluggable (trait-based, can swap llama.cpp ↔ OpenAI ↔ Anthropic)
- Audio input sources are pluggable (system capture ↔ file import ↔ HTTP upload ↔ watch folder)

#### Section 2.1 Audio Capture Engine (Expand)
**Add new sub-component**: `AudioInputManager`
- Responsibility: Abstract audio input sources (system capture, file import, upload server, watch folder)
- Inputs: User commands (record, import, upload), file system events (watch folder)
- Outputs: Unified audio stream (PCM or file path) to Transcription Pipeline

**New sub-components**:
- `FileImporter`: Handles drag-and-drop and file browser imports
- `WatchFolderMonitor`: Uses `notify` crate to watch inbox folder, debounces events, validates audio files
- `HttpUploadServer`: Embedded Axum HTTP server for LAN uploads (POST /upload/audio)
- `MobileCompanionBridge`: Optional LocalSend protocol support for zero-config phone→PC transfer

#### Section 2.2 Transcription Pipeline (Expand)
**Add**: `STTProviderRouter`
- Responsibility: Route transcription requests to selected STT backend
- Implements `STTProvider` trait with multiple backends:
  - `WhisperCppBackend` (existing)
  - `OpenAIAPIBackend` (new)
  - `AssemblyAIBackend` (new)
  - `DeepgramBackend` (new)
  - `AWSTranscribeBackend` (new)
  - `AzureSpeechBackend` (new)
  - `GoogleSpeechBackend` (new)

**New trait definition**:
```rust
trait STTProvider {
    fn transcribe(&self, audio: AudioInput, language: Language) -> Result<Transcript>;
    fn supports_streaming(&self) -> bool;
    fn estimate_cost(&self, duration_seconds: u64) -> Money;
}
```

#### Section 2.3 Summarization Engine (Expand)
**Add**: `LLMProviderRouter`
- Responsibility: Route summarization requests to selected LLM backend
- Implements `LLMProvider` trait with multiple backends:
  - `LlamaCppBackend` (existing)
  - `OllamaBackend` (new)
  - `OpenAIBackend` (new)
  - `AnthropicBackend` (new)
  - `GoogleGeminiBackend` (new)
  - `AWSBedrockBackend` (new)
  - `AzureOpenAIBackend` (new)

**New sub-component**: `PromptManager`
- Responsibility: Load, version, and apply prompt templates
- Loads templates from `~/RecapData/prompts/` directory
- Supports user-edited prompts (saves custom versions)
- Applies meeting-type-specific prompts based on metadata

**New trait definition**:
```rust
trait LLMProvider {
    fn complete(&self, prompt: String, schema: Option<JsonSchema>) -> Result<String>;
    fn complete_structured<T: DeserializeOwned>(&self, prompt: String) -> Result<T>;
    fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> Money;
}
```

#### Section 2.6 Desktop Application Shell (Update)
**Add new UI components**:
- `MetadataEntryDialog`: Form for meeting type, location, participants, language, topic
- `ProviderSettingsPanel`: Configure STT/LLM providers, API keys, fallback settings
- `PromptEditor`: View/edit prompt templates, preview with sample transcript
- `ImportDialog`: Drag-and-drop zone, file browser, watch folder status

#### Section 2.8 Model Registry & Manager (Update)
**Expand to**: "Provider & Model Registry"
- Manages both ML models (whisper, llama) AND API provider configurations
- Stores API keys (encrypted) for paid services
- Validates API connectivity on configuration change
- Displays cost estimates for paid providers

#### New Section: Audio Input Flexibility
**Add new section** (Section 2.9 or insert after 2.1):

**2.9 Audio Input Manager**
- Responsibility: Abstract multiple audio input sources, provide unified interface to Transcription Pipeline
- Inputs: System audio capture, file imports, HTTP uploads, watch folder events, mobile companion uploads
- Outputs: Audio file path or PCM stream to Transcription Pipeline
- Technology:
  - `notify` crate (v8.2) for file system watching
  - `axum` for embedded HTTP upload server
  - `localsend-rs` (optional) for zero-config phone→PC transfer
  - `tauri-plugin-audio-recorder` (if building mobile companion)

**Data flow**:
```
Audio Source (system capture / file import / upload / watch folder)
    ↓
AudioInputManager (validates format, normalizes metadata)
    ↓
[If file] Copy to ~/RecapData/audio/ with UUID filename
    ↓
Trigger MetadataEntryDialog (if not already provided)
    ↓
Transcription Pipeline (with selected STT provider)
```

---

### 3. TSD (Technical Specification Document) Updates

#### Section 1.3 System Boundaries (Update)
**Add to "Included"**:
- Audio file import (drag-and-drop, file browser)
- Watch folder with automatic processing
- Local HTTP upload server for smartphone audio transfer
- Multiple STT providers (local, paid APIs, cloud)
- Multiple LLM providers (local, cloud)
- Prompt template management and editing
- Meeting metadata entry (type, location, participants, language, topic)

**Update "Excluded"**:
- Remove: "Mobile apps" (now included as companion recording app)
- Add: "Mobile apps with local processing" (companion app uploads only, no local transcription)

#### Section 2.1 Audio Capture Module (Expand)
**Add new sub-components**:

**`file_importer.rs`**:
```rust
pub struct FileImporter {
    supported_formats: Vec<AudioFormat>, // M4A, WAV, MP3, OGG, WebM, FLAC
}

impl FileImporter {
    pub fn import_from_path(path: PathBuf) -> Result<AudioFile>;
    pub fn import_from_drag_drop(dropped_files: Vec<PathBuf>) -> Result<Vec<AudioFile>>;
}
```

**`watch_folder_monitor.rs`**:
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

**`http_upload_server.rs`**:
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

#### Section 2.2 Transcription Module (Expand)
**Add new sub-components**:

**`stt_provider.rs`** (trait):
```rust
#[async_trait]
pub trait STTProvider: Send + Sync {
    async fn transcribe(&self, audio: AudioInput, language: Language) -> Result<Transcript>;
    fn supports_streaming(&self) -> bool;
    fn estimate_cost(&self, duration_seconds: u64) -> Money;
}
```

**`stt_router.rs`**:
```rust
pub struct STTRouter {
    providers: HashMap<STTProviderType, Box<dyn STTProvider>>,
    primary: STTProviderType,
    fallback: Option<STTProviderType>,
}

impl STTRouter {
    pub async fn transcribe(&self, audio: AudioInput, language: Language) -> Result<Transcript> {
        match self.providers[&self.primary].transcribe(audio, language).await {
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

**New provider implementations**:
- `openai_stt.rs`: OpenAI Whisper API client
- `assemblyai.rs`: AssemblyAI API client
- `deepgram.rs`: Deepgram API client
- `aws_transcribe.rs`: AWS Transcribe client (using `aws-sdk-transcribe` crate)
- `azure_speech.rs`: Azure Speech Services client
- `google_speech.rs`: Google Speech-to-Text client

#### Section 2.3 Summarization Module (Expand)
**Add new sub-components**:

**`llm_provider.rs`** (trait):
```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, prompt: String) -> Result<String>;
    async fn complete_structured<T: DeserializeOwned>(&self, prompt: String, schema: JsonSchema) -> Result<T>;
    fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> Money;
}
```

**`llm_router.rs`**:
```rust
pub struct LLMRouter {
    providers: HashMap<LLMProviderType, Box<dyn LLMProvider>>,
    primary: LLMProviderType,
}

impl LLMRouter {
    pub async fn summarize(&self, transcript: Transcript, prompt: String) -> Result<MeetingSummary>;
}
```

**`prompt_manager.rs`**:
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

**New provider implementations**:
- `openai_llm.rs`: OpenAI GPT-4/3.5 client
- `anthropic.rs`: Anthropic Claude client
- `google_gemini.rs`: Google Gemini client
- `aws_bedrock.rs`: AWS Bedrock client (Claude, Llama via Bedrock)
- `azure_openai.rs`: Azure OpenAI client

**`prompt_templates/`** directory:
- `standard_meeting_summary.txt`
- `client_call_action_items.txt`
- `team_meeting_blockers.txt`
- `interview_evaluation.txt`
- `custom_user_template.txt` (user-edited)

#### Section 2.6 Desktop Shell Module (Update)
**Add new pages/components**:

**Pages**:
- `ImportPage.tsx`: Drag-and-drop zone, file browser, watch folder status
- `ProviderSettingsPage.tsx`: Configure STT/LLM providers, API keys, test connectivity
- `PromptEditorPage.tsx`: View/edit prompt templates, preview with sample data

**Components**:
- `MetadataEntryDialog.tsx`: Form for meeting type, location, participants, language, topic
- `ProviderSelector.tsx`: Dropdown for STT/LLM provider selection
- `APIKeyInput.tsx`: Secure input for API keys (masked, encrypted storage)
- `CostEstimator.tsx`: Display estimated cost for paid APIs before processing
- `PromptTemplateSelector.tsx`: Dropdown for prompt template selection
- `PromptEditor.tsx`: Monaco editor for customizing prompts

#### Section 4.1 Desktop SQLite Schema (Update)
**Add fields to `meetings` table**:
```sql
ALTER TABLE meetings ADD COLUMN meeting_type TEXT; -- 'team_meeting', 'client_call', etc.
ALTER TABLE meetings ADD COLUMN location TEXT;
ALTER TABLE meetings ADD COLUMN participants TEXT; -- JSON array of strings
ALTER TABLE meetings ADD COLUMN language TEXT DEFAULT 'en'; -- ISO 639-1 code
ALTER TABLE meetings ADD COLUMN topic TEXT;
ALTER TABLE meetings ADD COLUMN audio_source TEXT DEFAULT 'system_capture'; -- 'file_import', 'mobile_upload', 'watch_folder'
ALTER TABLE meetings ADD COLUMN stt_provider TEXT DEFAULT 'whisper_local';
ALTER TABLE meetings ADD COLUMN llm_provider TEXT DEFAULT 'llama_local';
```

**Add new table `prompt_templates`**:
```sql
CREATE TABLE prompt_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    meeting_type TEXT, -- NULL = applies to all types
    version INTEGER NOT NULL DEFAULT 1,
    is_custom INTEGER NOT NULL DEFAULT 0, -- 1 = user-edited
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_prompt_templates_name ON prompt_templates(name);
CREATE INDEX idx_prompt_templates_meeting_type ON prompt_templates(meeting_type);
```

**Add new table `api_keys`**:
```sql
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL UNIQUE, -- 'openai', 'anthropic', 'assemblyai', etc.
    key_encrypted TEXT NOT NULL, -- AES-256-GCM encrypted
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_api_keys_provider ON api_keys(provider);
```

#### Section 4.1 Configuration Table (Update)
**Add new configuration keys**:
```rust
// Audio Input
"watch_folder_enabled" => "false",
"watch_folder_path" => "~/RecapData/inbox",
"http_upload_server_enabled" => "false",
"http_upload_server_port" => "8765",

// STT Providers
"stt_provider" => "whisper_local",
"stt_fallback_provider" => "",
"stt_openai_api_key" => "", // encrypted
"stt_assemblyai_api_key" => "", // encrypted
"stt_deepgram_api_key" => "", // encrypted
"stt_aws_access_key" => "", // encrypted
"stt_aws_secret_key" => "", // encrypted
"stt_azure_key" => "", // encrypted
"stt_google_credentials" => "", // encrypted, path to JSON

// LLM Providers
"llm_provider" => "llama_local",
"llm_model" => "llama-3-8b-instruct",
"llm_openai_api_key" => "", // encrypted
"llm_anthropic_api_key" => "", // encrypted
"llm_google_api_key" => "", // encrypted
"llm_aws_access_key" => "", // encrypted
"llm_aws_secret_key" => "", // encrypted
"llm_azure_key" => "", // encrypted

// Prompts
"prompt_template" => "standard_meeting_summary",
"prompt_custom" => "", // user-edited prompt
"prompt_version" => "1",
```

#### Section 6.1 Meeting Capture Flow (Update)
**Add new flow**: "Import Audio File"

**Trigger**: User drags audio file into Recap OR clicks "Import" OR places file in watch folder

**Steps**:
1. AudioInputManager detects new file (drag-drop, file browser, or watch folder event)
2. Validate audio format (M4A, WAV, MP3, OGG, WebM, FLAC)
3. Copy file to `~/RecapData/audio/{uuid}.{ext}`
4. Create meeting record (status=imported, audio_source=file_import/mobile_upload/watch_folder)
5. Show MetadataEntryDialog (pre-fill title from filename, auto-detect language if possible)
6. User enters metadata (or skips, can edit later)
7. Update meeting record with metadata
8. Trigger transcription flow (using selected STT provider)
9. Continue with existing flow (summarization, storage, sync)

**Add new flow**: "HTTP Upload from Smartphone"

**Trigger**: Smartphone POSTs to `http://{desktop-ip}:8765/upload/audio`

**Steps**:
1. HttpUploadServer receives multipart form data (audio file + optional metadata JSON)
2. Save audio file to `~/RecapData/inbox/{uuid}.{ext}`
3. WatchFolderMonitor detects new file (if watch folder enabled) OR HttpUploadServer directly triggers processing
4. Continue with "Import Audio File" flow from step 4

#### Section 10 Configuration & Environment Variables (Update)
**Add new desktop configuration keys** (see Section 4.1 updates above)

**Add new team server environment variables**:
```bash
# STT Provider Configuration (for team-wide defaults)
STT_DEFAULT_PROVIDER=whisper_local
STT_OPENAI_API_KEY_SECRET_NAME=recap/stt/openai
STT_ASSEMBLYAI_API_KEY_SECRET_NAME=recap/stt/assemblyai

# LLM Provider Configuration (for team-wide defaults)
LLM_DEFAULT_PROVIDER=llama_local
LLM_OPENAI_API_KEY_SECRET_NAME=recap/llm/openai
LLM_ANTHROPIC_API_KEY_SECRET_NAME=recap/llm/anthropic
```

---

### 4. TBK (Task Breakdown Document) Updates

#### Section 2 Task Breakdown Structure (Add new tasks)

**Epic 1: Foundation & Infrastructure (Expand)**
- T1.6: Audio input manager (file import, watch folder, HTTP upload server)
- T1.7: Meeting metadata entry system

**Epic 2: Core Processing Pipeline (Expand)**
- T2.7: STT provider router (trait definition, fallback logic)
- T2.8: OpenAI STT integration
- T2.9: AssemblyAI integration
- T2.10: Deepgram integration
- T2.11: AWS Transcribe integration
- T2.12: Azure Speech integration
- T2.13: Google Speech-to-Text integration
- T2.14: LLM provider router (trait definition)
- T2.15: OpenAI LLM integration
- T2.16: Anthropic Claude integration
- T2.17: Google Gemini integration
- T2.18: AWS Bedrock integration
- T2.19: Azure OpenAI integration
- T2.20: Prompt manager (template loading, versioning, rendering)
- T2.21: Prompt template library (pre-built templates for meeting types)

**Epic 3: Desktop Shell & UI (Expand)**
- T3.8: Import dialog (drag-and-drop, file browser)
- T3.9: Metadata entry dialog (type, location, participants, language, topic)
- T3.10: Provider settings panel (STT/LLM configuration, API keys)
- T3.11: Prompt editor page (view/edit templates, preview)

**Epic 5: Integrations & Sync (Add new sub-epic)**
- T5.6: Mobile companion app (Tauri v2 iOS/Android, recording + upload)
- T5.7: iOS Shortcuts integration
- T5.8: LocalSend protocol support (optional)

#### New Task Definitions

**T1.6: Audio Input Manager**

**ID:** REC-INFRA-006  
**Title:** Implement audio input manager (file import, watch folder, HTTP upload)  
**Description:** Build the `audio-input-manager` module to abstract multiple audio input sources: file import (drag-and-drop, file browser), watch folder (automatic processing), and HTTP upload server (for smartphone transfers).  
**Inputs:** TSD Section 2.1 (Audio Input Manager)  
**Output:**
- `src-tauri/src/audio_input/mod.rs`
- `src-tauri/src/audio_input/file_importer.rs`
- `src-tauri/src/audio_input/watch_folder_monitor.rs` (using `notify` crate)
- `src-tauri/src/audio_input/http_upload_server.rs` (using `axum`)
- `src-tauri/src/audio_input/audio_validator.rs` (format validation)

**Acceptance Criteria:**
- Drag-and-drop audio files into Recap → auto-import
- File browser import (supports M4A, WAV, MP3, OGG, WebM, FLAC)
- Watch folder: place file in ~/RecapData/inbox/ → auto-detect and process
- HTTP upload server: POST /upload/audio → save to inbox folder
- Audio format validation (reject unsupported formats)
- Debouncing for watch folder (wait for file write to complete)

**Dependencies:** T1.1 (scaffolding)  
**Execution Type:** Developer (Rust + async expertise)  
**Priority:** High  
**Effort:** L (12-16 hours)

---

**T1.7: Meeting Metadata Entry System**

**ID:** REC-INFRA-007  
**Title:** Implement meeting metadata entry system  
**Description:** Build metadata entry dialog and storage for meeting type, location, participants, language(s), and topic. Metadata enhances summarization quality.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/components/MetadataEntryDialog.tsx`
- `src/components/MeetingTypeSelector.tsx`
- `src/components/ParticipantInput.tsx`
- `src/components/LanguageSelector.tsx`
- Database migration: add metadata fields to `meetings` table

**Acceptance Criteria:**
- Metadata dialog appears after recording stops or file is imported
- Fields: meeting type (dropdown), location (text), participants (comma-separated), language (dropdown, multi-select), topic (text)
- Metadata stored in `meetings` table
- User can skip dialog (metadata optional, can edit later)
- Metadata used to select meeting-type-specific prompts

**Dependencies:** T1.3 (storage module)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

**T2.7: STT Provider Router**

**ID:** REC-CORE-007  
**Title:** Implement STT provider router with fallback logic  
**Description:** Build a trait-based abstraction layer for multiple STT providers (local, paid APIs, cloud). Implement fallback chain (if primary fails, try secondary).  
**Inputs:** TSD Section 2.2 (Transcription Module)  
**Output:**
- `src-tauri/src/transcription/stt_provider.rs` (trait)
- `src-tauri/src/transcription/stt_router.rs` (routing logic)
- Provider implementations (T2.8-T2.13)

**Acceptance Criteria:**
- `STTProvider` trait with `transcribe()`, `supports_streaming()`, `estimate_cost()` methods
- `STTRouter` selects primary provider, falls back to secondary if primary fails
- Cost estimation for paid APIs (display before transcription)
- Provider selection persisted in configuration

**Dependencies:** T2.1 (transcription module)  
**Execution Type:** Developer (Rust + API integration)  
**Priority:** High  
**Effort:** M (8-10 hours)

---

**T2.8: OpenAI STT Integration**

**ID:** REC-CORE-008  
**Title:** Integrate OpenAI Whisper API  
**Description:** Implement OpenAI's Whisper API client for cloud-based transcription.  
**Inputs:** TSD Section 2.2 (Transcription Module)  
**Output:**
- `src-tauri/src/transcription/providers/openai_stt.rs`

**Acceptance Criteria:**
- Authenticates with OpenAI API using user-provided API key
- Uploads audio file, receives transcript with timestamps
- Supports language selection
- Handles API errors gracefully (rate limits, network failures)
- Displays cost estimate before transcription

**Dependencies:** T2.7 (STT router)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** S (4-6 hours)

---

(Similar task definitions for T2.9-T2.13: AssemblyAI, Deepgram, AWS Transcribe, Azure Speech, Google Speech)

---

**T2.14: LLM Provider Router**

**ID:** REC-CORE-014  
**Title:** Implement LLM provider router  
**Description:** Build a trait-based abstraction layer for multiple LLM providers (local, cloud). Support structured output with JSON Schema validation.  
**Inputs:** TSD Section 2.3 (Summarization Module)  
**Output:**
- `src-tauri/src/summarization/llm_provider.rs` (trait)
- `src-tauri/src/summarization/llm_router.rs` (routing logic)
- Provider implementations (T2.15-T2.19)

**Acceptance Criteria:**
- `LLMProvider` trait with `complete()`, `complete_structured()`, `estimate_cost()` methods
- `LLMRouter` selects provider based on configuration
- Structured output validation (JSON Schema for action items, decisions)
- Cost estimation for cloud APIs

**Dependencies:** T2.4, T2.5 (summarization modules)  
**Execution Type:** Developer (Rust + API integration)  
**Priority:** High  
**Effort:** M (8-10 hours)

---

**T2.20: Prompt Manager**

**ID:** REC-CORE-020  
**Title:** Implement prompt template manager  
**Description:** Build a system for loading, versioning, and applying prompt templates. Support user-edited custom prompts.  
**Inputs:** TSD Section 2.3 (Summarization Module)  
**Output:**
- `src-tauri/src/summarization/prompt_manager.rs`
- `prompt_templates/` directory with pre-built templates
- Database migration: `prompt_templates` table

**Acceptance Criteria:**
- Load prompt templates from `~/RecapData/prompts/` directory
- Support template variables (meeting_type, participants, transcript_length)
- Save user-edited prompts as custom versions
- List available templates in UI
- Render prompt with context (inject transcript, metadata)

**Dependencies:** T1.3 (storage module)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (8-10 hours)

---

**T2.21: Prompt Template Library**

**ID:** REC-CORE-021  
**Title:** Create pre-built prompt template library  
**Description:** Write prompt templates for different meeting types: standard meeting, client call, team meeting, interview, presentation.  
**Inputs:** TSD Section 2.3 (Summarization Module)  
**Output:**
- `prompt_templates/standard_meeting_summary.txt`
- `prompt_templates/client_call_action_items.txt`
- `prompt_templates/team_meeting_blockers.txt`
- `prompt_templates/interview_evaluation.txt`
- `prompt_templates/presentation_qa.txt`

**Acceptance Criteria:**
- 5+ pre-built templates for common meeting types
- Each template includes: system prompt, user prompt, output schema
- Templates tested against sample transcripts
- Templates optimized for action item extraction, decision logging

**Dependencies:** T2.20 (prompt manager)  
**Execution Type:** AI-Agent (prompt engineering)  
**Priority:** Medium  
**Effort:** M (6-8 hours)

---

**T3.8: Import Dialog**

**ID:** REC-UI-008  
**Title:** Build import dialog (drag-and-drop, file browser)  
**Description:** Create UI for importing audio files via drag-and-drop or file browser.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/ImportPage.tsx`
- `src/components/DropZone.tsx`
- `src/components/FileBrowser.tsx`

**Acceptance Criteria:**
- Drag-and-drop zone accepts audio files (M4A, WAV, MP3, OGG, WebM, FLAC)
- File browser button opens native file picker
- Display file name, size, format after selection
- "Import" button triggers file import flow
- Progress indicator during import

**Dependencies:** T3.1 (app shell), T1.6 (audio input manager)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

**T3.9: Metadata Entry Dialog**

**ID:** REC-UI-009  
**Title:** Build metadata entry dialog  
**Description:** Create form for meeting metadata: type, location, participants, language, topic.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/components/MetadataEntryDialog.tsx`
- `src/components/MeetingTypeSelector.tsx`
- `src/components/ParticipantInput.tsx`
- `src/components/LanguageSelector.tsx`

**Acceptance Criteria:**
- Dialog appears after recording stops or file is imported
- Meeting type dropdown: team meeting, 1:1, client call, interview, presentation, workshop, other
- Location text field
- Participants input (comma-separated or tag-style)
- Language dropdown (multi-select, ISO 639-1 codes)
- Topic/project text field
- "Save" and "Skip" buttons
- Metadata saved to `meetings` table

**Dependencies:** T3.1 (app shell), T1.7 (metadata system)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

**T3.10: Provider Settings Panel**

**ID:** REC-UI-010  
**Title:** Build provider settings panel (STT/LLM configuration)  
**Description:** Create settings page for configuring STT and LLM providers, API keys, and fallback settings.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/ProviderSettingsPage.tsx`
- `src/components/ProviderSelector.tsx`
- `src/components/APIKeyInput.tsx`
- `src/components/CostEstimator.tsx`

**Acceptance Criteria:**
- STT provider selector (local, OpenAI, AssemblyAI, Deepgram, AWS, Azure, Google)
- LLM provider selector (local, Ollama, OpenAI, Anthropic, Gemini, Bedrock, Azure)
- API key inputs (masked, encrypted storage)
- "Test Connection" button for each provider
- Cost estimator (display estimated cost per minute of audio)
- Fallback provider selector for STT

**Dependencies:** T3.6 (settings panels), T2.7, T2.14 (provider routers)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** L (12-16 hours)

---

**T3.11: Prompt Editor Page**

**ID:** REC-UI-011  
**Title:** Build prompt editor page  
**Description:** Create page for viewing, editing, and testing prompt templates.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/PromptEditorPage.tsx`
- `src/components/PromptTemplateSelector.tsx`
- `src/components/PromptEditor.tsx` (Monaco editor)
- `src/components/PromptPreview.tsx`

**Acceptance Criteria:**
- List available prompt templates
- Select template to view/edit
- Monaco editor for customizing prompts
- "Save as Custom" button (creates user-edited version)
- Preview prompt with sample transcript
- Reset to default button

**Dependencies:** T3.1 (app shell), T2.20 (prompt manager)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** M (8-10 hours)

---

**T5.6: Mobile Companion App**

**ID:** REC-MOB-001  
**Title:** Build mobile companion app (iOS/Android)  
**Description:** Create a Tauri v2 mobile app for recording meetings and uploading to desktop via LAN. No local processing.  
**Inputs:** TSD Section 2.1 (Audio Input Manager)  
**Output:**
- `mobile-app/` directory (Tauri v2 project)
- `mobile-app/src/` (React/Svelte frontend)
- `mobile-app/src-tauri/` (Rust backend)
- Recording UI, upload to desktop HTTP server

**Acceptance Criteria:**
- Record audio on iOS/Android (M4A/AAC format)
- Display desktop server IP address (auto-discover via mDNS or manual entry)
- Upload button: POST audio to desktop HTTP server
- Progress indicator during upload
- Success/error feedback

**Dependencies:** T1.6 (HTTP upload server)  
**Execution Type:** Developer (mobile + Tauri v2 expertise)  
**Priority:** P2 (Medium)  
**Effort:** L (16-20 hours)

---

#### Section 5 Execution Phases (Update)

**Phase 1: Foundation & Infrastructure (Days 1-6)**
- Add: T1.6 (Audio input manager) on Day 5-6
- Add: T1.7 (Metadata entry system) on Day 6

**Phase 2: Core Processing Pipeline (Days 7-18)**
- Add: T2.7 (STT router) on Day 10-11
- Add: T2.8-T2.13 (STT providers) on Days 12-14 (parallelize)
- Add: T2.14 (LLM router) on Day 15-16
- Add: T2.15-T2.19 (LLM providers) on Days 17-18 (parallelize)
- Add: T2.20 (Prompt manager) on Day 16-17
- Add: T2.21 (Prompt templates) on Day 18

**Phase 3: Desktop Shell & UI (Days 19-26)**
- Add: T3.8 (Import dialog) on Day 20-21
- Add: T3.9 (Metadata dialog) on Day 21-22
- Add: T3.10 (Provider settings) on Day 23-24
- Add: T3.11 (Prompt editor) on Day 25-26

**Phase 5: Integrations & Sync (Days 27-32)**
- Add: T5.6 (Mobile companion) on Days 30-32 (optional, P2)
- Add: T5.7 (iOS Shortcuts) on Day 32 (optional, P2)

**Updated effort estimate**:
- **Total tasks:** ~50 (was ~35)
- **Single agent:** 60-80 days (was 45-60)
- **Parallel agents (3-4):** 25-35 days (was 18-25)

---

## Open Source Dependencies to Add

### Rust Crates
```toml
# Audio Input
notify = "8.2"  # File system watching
axum = "0.7"    # HTTP upload server
tokio = { version = "1", features = ["full"] }  # Async runtime

# STT Providers
reqwest = { version = "0.11", features = ["json", "multipart"] }  # HTTP client
aws-sdk-transcribe = "1.0"  # AWS Transcribe
aws-config = "1.0"  # AWS config

# LLM Providers
rig-core = "0.38"  # Multi-provider LLM framework
# OR individual clients:
async-openai = "0.18"  # OpenAI
anthropic = "0.2"  # Anthropic
google-generativeai = "0.3"  # Google Gemini

# Structured Output
instructor-rs = "0.1"  # Schema-validated LLM output
# OR use serde_json + jsonschema for validation

# Utilities
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "v7"] }
```

### TypeScript Dependencies
```json
{
  "dependencies": {
    "@monaco-editor/react": "^4.6.0",  // Prompt editor
    "react-dropzone": "^14.2.3",       // Drag-and-drop
    "react-select": "^5.8.0"           // Multi-select dropdowns (languages, participants)
  }
}
```

---

## Summary of Changes

| Document | Sections Updated | New Sections | New Tables | New Tasks |
|----------|-----------------|--------------|------------|-----------|
| PRD | 1.3, 3.1, 3.2, 5.2, 6.1, 6.2, 6.3, 9.1, 9.2 | - | - | - |
| SAD | 1.2, 2.1, 2.2, 2.3, 2.6, 2.8 | 2.9 (Audio Input Manager) | - | - |
| TSD | 1.3, 2.1, 2.2, 2.3, 2.6, 4.1, 6.1, 10 | - | prompt_templates, api_keys | - |
| TBK | 2, 5 | - | - | T1.6, T1.7, T2.7-T2.21, T3.8-T3.11, T5.6-T5.8 |

**Total new tasks**: 15-20 tasks  
**Estimated additional effort**: 15-20 days (single agent), 7-10 days (parallel)

---

## Next Steps

1. Review and approve this plan
2. Update PRD with new requirements and journeys
3. Update SAD with new architectural components
4. Update TSD with new technical specifications
5. Update TBK with new task definitions and timeline
6. Begin implementation (Phase 1: Foundation)
