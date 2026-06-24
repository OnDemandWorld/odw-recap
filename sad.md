# System Architecture Document: Recap

**Product:** Recap by ODW.ai
**Version:** 1.0
**Date:** June 23, 2026
**Status:** Draft
**Author:** Architecture Team

---

## 1. Architecture Overview

### 1.1 System Purpose

Recap is a sovereign, on-device meeting intelligence system that captures audio from any meeting platform, transcribes it locally using open-source speech models, generates structured outputs (summaries, action items, decisions), and syncs results into the ODW.ai suite (Vault for knowledge, Loop for workflows). No meeting data — audio, transcript, summary, or metadata — ever leaves the user's device or their self-hosted infrastructure.

### 1.2 Architectural Style

Recap employs a **modular monolith with an embedded pipeline and pluggable provider backends** running inside a desktop application shell, augmented by an optional **lightweight server process** for team-mode deployments.

The architecture decomposes into three deployment contexts:

- **Single-user desktop mode:** All processing, storage, and inference occur within a single desktop application process. No network communication required after initial model download (local providers) or except for API calls (cloud providers).
- **Team mode (self-hosted):** Desktop clients connect to a local server (Docker-based) on the customer's infrastructure for shared state, authentication, RBAC, audit logging, and Vault/Loop integration. The server never processes audio or transcripts — it coordinates metadata and sync operations only.
- **Air-gapped mode:** Identical to team mode but with no internet connectivity; all model weights and updates are delivered via local artifact transfer. Cloud providers disabled; only local providers available.

**Pluggable provider backends:**

The architecture uses trait-based abstraction layers for key services:
- **STT providers are pluggable:** Swap whisper.cpp ↔ OpenAI API ↔ AWS Transcribe without changing orchestration logic
- **LLM providers are pluggable:** Swap llama.cpp ↔ OpenAI ↔ Anthropic without changing summarization flow
- **Audio input sources are pluggable:** System capture ↔ file import ↔ HTTP upload ↔ watch folder all feed the same pipeline

This design enables users to choose between sovereignty (local-only) and convenience (cloud APIs) while maintaining a single codebase.

### 1.3 Architectural Justification & Trade-offs

**Why modular monolith over microservices:**

Recap's workload is inherently sequential and co-located: audio capture feeds transcription feeds summarization feeds storage feeds sync. Splitting these into networked microservices would introduce latency, operational complexity, and a dependency on network infrastructure — all antithetical to the sovereignty and offline-capability requirements. A modular monolith with clear internal boundaries (audio capture, transcription, summarization, storage, sync) provides separation of concerns without the overhead of distributed systems.

**Why embedded pipeline over external process orchestration:**

Running transcription and summarization as child processes (e.g., spawning whisper.cpp, llama.cpp as separate binaries) was considered. The trade-off: child processes offer better isolation and independent lifecycle management, but introduce IPC overhead, complicate error handling, and make the desktop app harder to package. The chosen approach uses native library integration (C++ bindings for whisper.cpp, Rust/C FFI for llama.cpp) with the desktop runtime, keeping everything in-process. This is acceptable because the ML workloads are CPU/GPU-bound and do not benefit from process-level isolation — they are the application's core function, not a peripheral service.

**Why optional server over always-connected:**

Team features (RBAC, audit, sync coordination) require shared state. Rather than making the desktop app always-connected (which would break offline capability and add a single point of failure), the server is an optional coordination layer. Desktop clients operate fully autonomously and sync to the server opportunistically. This preserves the sovereignty promise: if the server is unavailable, individual users still capture, transcribe, and store meetings locally.

**Trade-off summary:**

- Complexity vs. sovereignty: Chose sovereignty. No cloud, no always-connected requirement.
- Performance vs. simplicity: Chose in-process pipeline. Slightly higher memory footprint, but simpler deployment and no IPC latency.
- Scalability vs. control: Chose per-device processing. Each user's machine handles its own workload; the server coordinates but does not compute. This means Recap scales linearly with the number of devices, not with a central cluster.

---

## 2. High-Level System Components

### 2.1 Audio Capture Engine

- **Responsibility:** Capture system audio output and microphone input from the host operating system; merge into a unified audio stream with speaker channel separation.
- **Inputs:** OS audio subsystem output (system audio via virtual device or loopback API), microphone device input.
- **Outputs:** Raw PCM audio stream (16kHz, 16-bit mono per channel) fed to the Transcription Pipeline; buffered WAV/Opus file written to Local Storage.
- **Technology:** Platform-specific adapters — CoreAudio + BlackHole (macOS), WASAPI loopback (Windows), PulseAudio/PipeWire monitor source (Linux). Abstracted behind a unified `AudioCapture` interface.
- **Key constraint:** Must operate without introducing audible latency or degrading meeting audio quality for the user.

### 2.1.1 Audio Input Manager (New Sub-component)

- **Responsibility:** Abstract multiple audio input sources, provide unified interface to Transcription Pipeline. Handles file imports, watch folder monitoring, and HTTP upload server.
- **Inputs:** System audio capture, file imports (drag-drop, file browser), HTTP uploads, watch folder events, mobile companion uploads.
- **Outputs:** Audio file path or PCM stream to Transcription Pipeline.
- **Technology:**
  - `notify` crate (v8.2) for file system watching with debouncing
  - `axum` for embedded HTTP upload server
  - `localsend-rs` (optional) for zero-config phone→PC transfer
  - `tauri-plugin-audio-recorder` (if building mobile companion)
- **Sub-components:**
  - `FileImporter`: Handles drag-and-drop and file browser imports
  - `WatchFolderMonitor`: Watches inbox folder, debounces events, validates audio files
  - `HttpUploadServer`: Embedded Axum HTTP server (POST /upload/audio)
  - `MobileCompanionBridge`: Optional LocalSend protocol support
- **Key constraint:** Must validate audio format (M4A, WAV, MP3, OGG, WebM, FLAC) before processing. Debounce watch folder events to ensure file write completion.

### 2.2 Transcription Pipeline

- **Responsibility:** Convert audio streams into timestamped, speaker-labeled text. Operates in two modes: real-time streaming (during recording) and batch re-processing (after recording ends).
- **Inputs:** PCM audio chunks (streaming mode) or complete audio file (batch mode).
- **Outputs:** `Transcript` data structure containing ordered `TranscriptSegment` entries with timestamps, speaker labels, confidence scores, and text.
- **Technology:** whisper.cpp (primary) or faster-whisper (alternative), selected via the pluggable backend interface. Speaker diarization via pyannote-audio or neMo. Model weights stored locally; user-selectable (small/medium/large tiers).
- **Key constraint:** Streaming mode must produce text within 2 seconds of speech. Batch mode must process a 60-minute meeting in ≤10 minutes on reference hardware (M2 Mac, 16GB RAM).

### 2.2.1 STT Provider Router (New Sub-component)

- **Responsibility:** Route transcription requests to selected STT backend (local or cloud). Implement fallback chain if primary provider fails.
- **Inputs:** Audio file or PCM stream, language selection, provider configuration.
- **Outputs:** Unified `Transcript` structure (normalized across providers).
- **Technology:** Trait-based abstraction (`STTProvider` trait) with multiple implementations:
  - `WhisperCppBackend`: Local whisper.cpp (default, free)
  - `OpenAIWhisperAPIBackend`: OpenAI Whisper API (paid, high accuracy)
  - `AssemblyAIBackend`: AssemblyAI (paid, advanced features)
  - `DeepgramBackend`: Deepgram (paid, real-time streaming)
  - `AWSTranscribeBackend`: AWS Transcribe (enterprise, via `aws-sdk-transcribe` crate)
  - `AzureSpeechBackend`: Azure Speech Services (enterprise)
  - `GoogleSpeechBackend`: Google Speech-to-Text (enterprise)
- **Trait definition:**
  ```rust
  trait STTProvider {
      fn transcribe(&self, audio: AudioInput, language: Language) -> Result<Transcript>;
      fn supports_streaming(&self) -> bool;
      fn estimate_cost(&self, duration_seconds: u64) -> Money;
  }
  ```
- **Fallback logic:** If primary provider fails (network error, rate limit, timeout), automatically retry with fallback provider (if configured).
- **Key constraint:** Must normalize provider-specific output formats to unified internal format (timestamps, speaker labels, confidence scores). Cost estimation displayed before transcription begins.

### 2.3 Summarization Engine

- **Responsibility:** Generate structured outputs from transcripts — meeting summary, action items, decisions, and topic tags. Operates in two modes: rule-based (no LLM required) and local LLM-powered.
- **Inputs:** Complete `Transcript` (post batch processing), optional user-configured prompt template, optional custom output template.
- **Outputs:** `Summary` (markdown), `ActionItem[]` (description, assignee, deadline, source reference), `Decision[]` (description, context, participants), topic tags.
- **Technology:** Rule-based mode uses pattern matching and heuristic extraction (keyword detection for action phrases, temporal expression parsing for deadlines). LLM mode uses llama.cpp or Ollama as the inference runtime against any user-provided compatible local model. Model-agnostic by design — the engine communicates via a standardized prompt/response interface.
- **Key constraint:** Rule-based mode must produce output without any network calls or external model downloads. LLM mode must ground all summary claims in transcript segments (no hallucination).

### 2.3.1 LLM Provider Router (New Sub-component)

- **Responsibility:** Route summarization requests to selected LLM backend (local or cloud). Support structured output with JSON Schema validation.
- **Inputs:** Transcript, prompt template, provider configuration.
- **Outputs:** Structured meeting summary (validated against JSON Schema).
- **Technology:** Trait-based abstraction (`LLMProvider` trait) with multiple implementations:
  - `LlamaCppBackend`: Local llama.cpp (default, free)
  - `OllamaBackend`: Local Ollama (alternative local runtime)
  - `OpenAIBackend`: OpenAI GPT-4 (paid, high quality)
  - `AnthropicBackend`: Anthropic Claude (paid, strong reasoning)
  - `GoogleGeminiBackend`: Google Gemini (paid, multimodal)
  - `AWSBedrockBackend`: AWS Bedrock (enterprise, hosts Claude/Llama)
  - `AzureOpenAIBackend`: Azure OpenAI (enterprise)
- **Trait definition:**
  ```rust
  trait LLMProvider {
      fn complete(&self, prompt: String, schema: Option<JsonSchema>) -> Result<String>;
      fn complete_structured<T: DeserializeOwned>(&self, prompt: String) -> Result<T>;
      fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> Money;
  }
  ```
- **Key constraint:** Structured output validation ensures LLM output matches expected format (action items, decisions). Cost estimation displayed before summarization.

### 2.3.2 Prompt Manager (New Sub-component)

- **Responsibility:** Load, version, and apply prompt templates. Support user-edited custom prompts. Apply meeting-type-specific prompts based on metadata.
- **Inputs:** Meeting metadata (type, participants, language), transcript, user-selected template.
- **Outputs:** Rendered prompt with variables substituted.
- **Technology:**
  - Loads templates from `~/RecapData/prompts/` directory
  - Supports template variables: `{{transcript}}`, `{{meeting_type}}`, `{{participants}}`, `{{language}}`
  - Saves user-edited prompts as custom versions with incremented version number
  - Stores templates in SQLite `prompt_templates` table
- **Pre-built templates:**
  - `standard_meeting_summary.txt`: General-purpose summary
  - `client_call_action_items.txt`: Focus on commitments and follow-ups
  - `team_meeting_blockers.txt`: Focus on blockers and decisions
  - `interview_evaluation.txt`: Focus on candidate strengths/concerns
  - `presentation_qa.txt`: Focus on Q&A extraction
- **Key constraint:** Must apply meeting-type-specific prompts automatically based on metadata (e.g., meeting_type=client_call → use client_call template).

### 2.4 Local Storage Manager

- **Responsibility:** Persist all meeting data (audio, transcripts, summaries, metadata) to the local filesystem with encryption at rest. Manage storage lifecycle, retention policies, and data organization.
- **Inputs:** Meeting artifacts from upstream pipeline components; user commands (save, delete, export, search).
- **Outputs:** Encrypted files on disk; search query results; export files (Markdown, PDF, JSON, plain text).
- **Technology:** SQLite (metadata, search index via FTS5, application state) + encrypted file storage (audio as Opus, transcripts/summaries as encrypted blobs or files). AES-256-GCM encryption with key derived via Argon2id from user passphrase. All data resides in a single configurable directory (`~/RecapData` default).
- **Key constraint:** Must support ≥10,000 hours of meetings per installation. Search across full corpus must return results within 1 second.

### 2.5 Sync Engine

- **Responsibility:** Coordinate data flow between Recap and the ODW.ai suite modules (Vault and Loop). Evaluate sync rules, transform data into target formats, manage sync state, and handle retries on failure.
- **Inputs:** Meeting metadata and structured outputs (summaries, action items, decisions) from Local Storage; sync rule configuration; Vault/Loop API responses.
- **Outputs:** Synced entries in Vault (knowledge base entries linked to source meetings); tasks/workflows created in Loop; sync status updates reflected in Recap UI.
- **Technology:** HTTP client communicating with Vault/Loop REST APIs over the local network (team mode) or localhost (single-machine team deployments). Sync queue persisted in SQLite for crash recovery. Exponential backoff retry with configurable maximum attempts.
- **Key constraint:** Sync is opportunistic and non-blocking. If Vault/Loop are unavailable, syncs queue locally and retry indefinitely (up to 30-day queue TTL). Sync must never block the core capture-transcribe-summarize pipeline.

### 2.6 Desktop Application Shell

- **Responsibility:** Provide the user interface, system tray/menu bar integration, hotkey handling, onboarding flow, settings management, and orchestrate all internal components.
- **Inputs:** User interactions (UI clicks, hotkeys, settings changes); system events (audio device changes, OS notifications).
- **Outputs:** Visual transcription display, meeting library, search interface, settings panels, sync status indicators.
- **Technology:** Tauri (primary) or Electron (fallback). Tauri preferred for smaller binary size (~10MB vs ~150MB), lower memory footprint, and native OS integration. Frontend rendered via web technologies (HTML/CSS/JS) in a webview; backend logic in Rust (Tauri) or Node.js (Electron).
- **Key constraint:** Cold start to ready state in ≤5 seconds. Must not consume >30% of one CPU core during idle (tray mode).
- **New UI components (for flexible input and multi-provider support):**
  - `MetadataEntryDialog`: Form for meeting type, location, participants, language(s), topic
  - `ProviderSettingsPanel`: Configure STT/LLM providers, API keys, fallback settings, test connectivity
  - `PromptEditor`: Monaco editor for viewing/editing prompt templates, preview with sample transcript
  - `ImportDialog`: Drag-and-drop zone, file browser, watch folder status indicator
  - `CostEstimator`: Display estimated cost for paid APIs before processing

### 2.7 Team Server (Optional)

- **Responsibility:** Provide shared state coordination for team deployments — user authentication, RBAC enforcement, audit logging, retention policy enforcement, sync rule management, and admin dashboard. Does NOT process audio or transcripts.
- **Inputs:** API requests from desktop clients (auth, sync metadata, policy queries); admin configuration commands.
- **Outputs:** Authentication tokens; policy decisions; audit log entries; aggregated usage metrics for admin dashboard.
- **Technology:** Single Docker container running a lightweight HTTP server (Go or Rust). PostgreSQL for shared metadata (users, policies, audit logs, sync state). Redis for session management and rate limiting. Docker Compose for one-command deployment.
- **Key constraint:** Must support ≥200 concurrent users per instance. Must operate entirely within the customer's network — no external connectivity required.

### 2.8 Provider & Model Registry (Expanded from Model Registry & Manager)

- **Responsibility:** Manage the lifecycle of ML model weights AND API provider configurations. Handles model discovery, download, installation, versioning, activation, deletion, and API key management for cloud providers.
- **Inputs:** User commands (download model, switch model, configure provider, enter API key); model manifest from registry endpoint (or local file for air-gapped).
- **Outputs:** Downloaded model files in local model directory; active model selection persisted in configuration; encrypted API keys in SQLite `api_keys` table.
- **Technology:**
  - HTTP client for model download (from ODW.ai mirror or Hugging Face)
  - SHA-256 integrity verification for model files
  - Model metadata stored in SQLite `models` table
  - API keys stored encrypted (AES-256-GCM) in SQLite `api_keys` table
  - Connectivity validation for API providers (test endpoint before saving)
  - Cost estimation for paid providers (pricing tables embedded or fetched)
- **Supports manual model file placement for air-gapped environments.
- **Key constraints:**
  - Model download is a one-time operation per model version
  - Switching active model takes effect on next recording without app restart
  - API keys encrypted at rest; never logged or exposed in UI (masked input)
  - Connectivity test prevents saving invalid API keys

---

## 3. Component Interaction & Data Flow

### 3.1 Normal Operation: Meeting Capture & Processing

```
User presses Record (hotkey or UI)
    │
    ▼
Desktop Shell ──▶ Audio Capture Engine
    │                  │
    │                  ├──▶ [Real-time path]
    │                  │       │
    │                  │       ▼
    │                  │    Transcription Pipeline (streaming mode)
    │                  │       │
    │                  │       ▼
    │                  │    Desktop Shell ──▶ UI (live transcript display)
    │                  │
    │                  ├──▶ [Storage path]
    │                  │       │
    │                  │       ▼
    │                  │    Local Storage Manager (buffered write, every 5s)
    │                  │
    ▼
User presses Stop
    │
    ▼
Audio Capture Engine ──▶ Transcription Pipeline (batch mode)
    │                          │
    │                          ▼
    │                     Transcript (final, high-accuracy)
    │                          │
    │                          ▼
    │                     Summarization Engine
    │                          │
    │                          ├──▶ Summary
    │                          ├──▶ ActionItem[]
    │                          ├──▶ Decision[]
    │                          └──▶ Topic tags
    │
    ▼
Local Storage Manager (persist all artifacts, encrypted)
    │
    ▼
Sync Engine (evaluate rules, queue syncs)
    │
    ├──▶ Vault API (summary + decisions → knowledge base)
    └──▶ Loop API (action items → tasks/workflows)
```

### 3.2 Failure Scenario: Application Crash Mid-Recording

```
Audio Capture Engine (recording active)
    │
    ├──▶ Buffer flush to disk every 5 seconds (Local Storage Manager)
    │
    ▼
[CRASH]
    │
    ▼
On restart:
    Desktop Shell detects incomplete recording session
        │
        ▼
    Local Storage Manager recovers last flushed audio buffer
        │
        ▼
    Transcription Pipeline processes recovered audio (batch mode)
        │
        ▼
    User notified: "Previous session recovered. Review transcript."
```

**Data loss window:** Maximum 5 seconds of audio (time since last buffer flush). Zero data loss for transcripts/summaries of completed meetings.

### 3.3 Failure Scenario: Vault/Loop Unavailable During Sync

```
Sync Engine attempts sync
    │
    ▼
HTTP request to Vault/Loop API ──▶ Connection timeout / error
    │
    ▼
Sync Engine:
    1. Mark sync status as "failed"
    2. Persist sync request to retry queue (SQLite)
    3. Schedule retry with exponential backoff (1min → 2min → 4min → ... → max 1hr)
    4. UI shows "sync pending" indicator
    │
    ▼
On retry success: sync completes, status updated to "synced"
On max retries exhausted (30 days): status set to "permanently_failed", admin notified
```

### 3.4 Async vs. Sync Boundaries

- **Synchronous:** Audio capture → streaming transcription → UI display (latency-sensitive, must be ≤2s end-to-end).
- **Asynchronous:** Batch re-transcription after meeting ends; summarization; Vault/Loop sync; model downloads; retention policy enforcement.
- **Boundary mechanism:** Internal task queue (in-process for desktop mode, message queue for server mode). Long-running operations report progress via callback to the UI thread.

---

## 4. API & Service Boundaries

### 4.1 Internal Component Boundaries

Components communicate via well-defined interfaces within the desktop application process:

- **AudioCapture → TranscriptionPipeline:** Streaming audio callback interface. The capture engine pushes PCM chunks to a ring buffer; the transcription pipeline consumes from the buffer. Decoupled via producer-consumer pattern with backpressure signaling.
- **TranscriptionPipeline → SummarizationEngine:** Synchronous handoff of completed `Transcript` object. The summarization engine subscribes to a "transcript ready" event.
- **SummarizationEngine → LocalStorageManager:** Synchronous persistence call. All outputs (transcript, summary, action items, decisions) are persisted atomically within a single SQLite transaction.
- **LocalStorageManager → SyncEngine:** Event-driven. Storage manager emits a "meeting saved" event; sync engine subscribes and evaluates rules asynchronously.

### 4.2 Desktop ↔ Server Communication (Team Mode)

Communication between desktop clients and the team server uses **REST over HTTPS** (TLS 1.3, self-signed or user-provided certificates):

- **Authentication endpoints:** `/auth/login`, `/auth/refresh`, `/auth/sso/callback` — JWT-based session tokens.
- **Sync metadata endpoints:** `/api/v1/meetings/sync` (POST meeting metadata to server), `/api/v1/vault/sync` (proxy to Vault), `/api/v1/loop/sync` (proxy to Loop).
- **Policy endpoints:** `/api/v1/policies/retention`, `/api/v1/policies/sync-rules` — server pushes policy updates to clients.
- **Admin endpoints:** `/api/v1/admin/audit`, `/api/v1/admin/metrics`, `/api/v1/admin/users` — admin dashboard queries.

**Contract expectations:** All API responses are JSON. Error responses follow a standard envelope (`{error: {code, message, retryable}}`). Clients must handle 429 (rate limit) and 503 (service unavailable) gracefully.

### 4.3 Server ↔ Vault/Loop Integration

The team server acts as a **sync proxy** between desktop clients and Vault/Loop modules:

- **Vault integration:** Server exposes `/api/v1/vault/sync` which accepts meeting summaries and decisions, transforms them into Vault-compatible entries, and forwards to Vault's ingestion API. Returns the Vault entry ID for bi-directional linking.
- **Loop integration:** Server exposes `/api/v1/loop/sync` which accepts action items, maps assignees to Loop team members, creates tasks via Loop's task creation API, and returns task IDs.

**Decoupling opportunity:** The sync proxy pattern means Recap desktop clients never communicate directly with Vault or Loop. If Vault/Loop APIs change, only the server's proxy layer needs updating — desktop clients are unaffected.

### 4.4 Transcription Backend Plugin Interface

The transcription pipeline exposes a plugin interface for swapping backends:

```
TranscriptionBackend {
    load_model(path: string) → Result
    transcribe_streaming(audio_chunk: bytes) → TranscriptSegment
    transcribe_batch(audio_file: string) → Transcript
    diarize(audio_file: string) → SpeakerSegment[]
    get_model_info() → ModelInfo
}
```

This allows future integration of alternative ASR engines without modifying the pipeline orchestration logic.

---

## 5. Data Architecture

### 5.1 Storage Strategy

Recap uses a **hybrid storage approach** optimized for local-first, single-tenant operation:

- **SQLite (primary):** All structured metadata — meeting records, transcript segments, speakers, summaries, action items, decisions, sync state, configuration, audit logs, and full-text search index (FTS5). SQLite chosen for zero-configuration deployment, single-file portability, and excellent read performance for the expected data volumes (≤10,000 hours of meetings).
- **Encrypted file storage:** Audio recordings stored as Opus-encoded files (compressed, ~10x smaller than WAV). Encryption at rest via AES-256-GCM with per-file keys derived from the user's master passphrase. Files stored alongside the SQLite database in the configurable data directory.
- **In-memory cache:** Recently accessed transcripts and summaries cached in application memory for fast UI rendering. Eviction based on LRU policy with configurable size limit.

### 5.2 Data Ownership

- **Desktop client owns:** All meeting data (audio, transcripts, summaries, action items, decisions, bookmarks, tags, local search index, configuration).
- **Team server owns:** User accounts, RBAC policies, retention policies, sync rules, audit logs, aggregated usage metrics. The server does NOT own or store meeting content — it only stores metadata references (meeting IDs, sync status, timestamps).
- **Vault owns:** Knowledge base entries derived from meeting summaries and decisions. Bi-directional links back to source meetings.
- **Loop owns:** Task records derived from action items. Bi-directional links back to source meetings.

### 5.3 Data Flow Between Systems

- **Capture → Storage:** Audio chunks written to disk every 5 seconds (buffered write). On meeting end, final audio file closed and metadata committed to SQLite.
- **Transcription → Storage:** Transcript segments written to SQLite as they are produced (streaming) or in bulk (batch). Each transcript version is immutable; re-processing creates a new version.
- **Summarization → Storage:** Summary, action items, and decisions written to SQLite in a single transaction linked to the meeting record.
- **Storage → Sync:** "Meeting saved" event triggers sync engine. Sync engine reads meeting data from SQLite, evaluates rules, and pushes to Vault/Loop via server proxy.
- **Storage → Search:** SQLite FTS5 index updated synchronously on transcript write. Search queries execute against the FTS5 virtual table.

### 5.4 Consistency Model

- **Single-user mode:** Strong consistency. All writes go through SQLite's ACID transaction model. No replication or distribution.
- **Team mode:** Eventual consistency for sync operations. Desktop client writes are authoritative for local data; server metadata is updated asynchronously. Conflict resolution: last-write-wins for metadata (e.g., sync status); client data is never overwritten by server.

### 5.5 Indexing Strategy

- **Full-text search:** SQLite FTS5 index on transcript text, summary text, and action item descriptions. Tokenized with English stemming and diacritic removal.
- **Metadata queries:** Standard SQLite B-tree indexes on meeting date, duration, status, tags, and speaker names.
- **Future (v1.1+):** Vector embeddings for semantic search. Architecture supports adding a vector index (e.g., SQLite-vec or standalone HNSW) without schema migration.

### 5.6 Data Partitioning

Not applicable for single-user mode (single SQLite database). For team server mode: PostgreSQL with schema-per-tenant isolation. Meeting metadata partitioned by organization ID; audit logs partitioned by month for efficient retention enforcement.

---

## 6. Scalability & Performance Design

### 6.1 Scaling Model

Recap scales **horizontally by device**, not by central infrastructure. Each desktop client is an independent processing unit. Adding users means adding devices, each handling its own workload. The team server scales vertically (larger instance) for metadata coordination, but does not process audio or transcripts.

- **Desktop client:** Scales with user hardware. Performance tiered by model selection (small = fast/low-resource, large = slow/high-resource). No horizontal scaling needed — each client is autonomous.
- **Team server:** Single instance supports ≤200 concurrent users. Beyond that, horizontal scaling via load-balanced server instances with shared PostgreSQL backend. Stateless application tier; stateful database tier.

### 6.2 Bottleneck Identification & Mitigation

| Bottleneck | Location | Mitigation |
|---|---|---|
| Transcription throughput | Desktop client CPU/GPU | Model tier selection; batch processing on meeting end (non-blocking); GPU acceleration via Metal (macOS) / CUDA (Windows/Linux) where available |
| Storage I/O during recording | Desktop client disk | Buffered writes (5s intervals); Opus compression reduces write volume; SSD recommended for large libraries |
| Search latency at scale | SQLite FTS5 | FTS5 is performant to ~10,000 hours; beyond that, migrate to dedicated search engine (Meilisearch or Tantivy) |
| Sync throughput | Team server ↔ Vault/Loop | Async queue with configurable concurrency; batch sync for multiple meetings; backpressure if downstream APIs throttle |
| Model download time | Network (one-time) | Offer smaller default model; progressive download with progress indicator; mirror on ODW.ai infrastructure for faster CDN delivery |

### 6.3 Caching Layers

- **Application memory:** LRU cache for recently viewed transcripts and summaries (configurable, default 100MB).
- **SQLite page cache:** Tuned for read-heavy workloads (meeting library browsing, search).
- **Model weights:** Loaded into memory on first use, retained until app restart or model switch. No repeated disk I/O for model loading during a session.

### 6.4 Load Balancing (Team Server)

For deployments exceeding 200 concurrent users: stateless application servers behind a TCP load balancer (HAProxy or cloud LB). Session affinity via JWT (stateless authentication — no server-side sessions to pin). PostgreSQL as shared state backend with read replicas for audit log queries.

### 6.5 Rate Limiting

- **Desktop client:** No rate limiting (single user, local resources).
- **Team server:** Per-user rate limiting on API endpoints (default: 100 requests/minute for sync, 10 requests/minute for admin). Prevents runaway clients from overwhelming the coordination layer.
- **Vault/Loop proxy:** Server-side rate limiting to protect downstream modules from sync storms (e.g., bulk meeting import triggering hundreds of syncs simultaneously). Token bucket algorithm with configurable limits.

---

## 7. Reliability & Fault Tolerance

### 7.1 Failover

- **Desktop client:** No failover needed — single process, single user. Crash recovery via buffered audio writes and SQLite WAL mode (write-ahead log ensures no data corruption on crash).
- **Team server:** Active-passive failover via health-check monitored replica. If primary server fails, replica promotes automatically (via Patroni or similar). Desktop clients retry with backoff; no data loss because clients maintain local state independently.

### 7.2 Retry Mechanisms

- **Sync retries:** Exponential backoff (1min, 2min, 4min, 8min, ... capped at 1hr). Maximum retry duration: 30 days. After expiry, marked as permanently failed; admin notified via dashboard.
- **Batch transcription retries:** If batch re-processing fails (e.g., model error), the streaming transcript remains available. User can manually trigger re-processing.
- **Model download retries:** Chunked download with resume capability. SHA-256 verification on completion; re-download on integrity failure.

### 7.3 Circuit Breakers

- **Vault/Loop sync:** If 5 consecutive sync attempts fail within 10 minutes, circuit breaker opens. Sync attempts suspended for 15 minutes before half-open probe. Prevents resource waste on downstream outages.
- **Server health (team mode):** If server fails 3 consecutive health checks, desktop client enters "offline team mode" — all features work locally, syncs queue, UI shows "server disconnected" indicator.

### 7.4 Graceful Degradation

| Component Failure | Degraded Behavior |
|---|---|
| Transcription model fails to load | App warns user; offers model re-download or model switch |
| Summarization LLM unavailable | Falls back to rule-based extraction automatically |
| Vault unreachable | Syncs queue locally; UI shows "sync pending"; core features unaffected |
| Loop unreachable | Same as Vault — queue and retry |
| Team server unreachable | Desktop operates in standalone mode; all local features work; syncs queue |
| Disk full | Recording stops with clear error; existing data preserved; user prompted to free space or change storage location |
| GPU unavailable | Falls back to CPU inference (slower but functional); user notified of performance impact |

### 7.5 Single Points of Failure & Mitigations

| SPOF | Mitigation |
|---|---|
| SQLite database corruption | WAL mode + periodic integrity checks; automatic backup of database file on app start; user can restore from backup |
| Encryption key loss (user forgets passphrase) | By design — no recovery. Warned prominently during setup. Data is unrecoverable without passphrase (this is a feature, not a bug, for sovereignty). |
| Team server single instance | Documented scaling path to HA deployment; desktop clients operate independently so server failure does not block individual users |
| Model weight corruption | SHA-256 verification on load; re-download on failure; multiple model versions can coexist |

---

## 8. Security Architecture

### 8.1 Authentication & Authorization

- **Single-user mode:** Passphrase-derived encryption key. No authentication server. The application unlocks the encrypted data store on launch via user passphrase entry (or OS keychain integration for convenience).
- **Team mode:** JWT-based authentication. Users authenticate against the team server via local credentials or SSO (SAML 2.0 / OIDC). JWT tokens have configurable TTL (default: 8 hours) with refresh token rotation.
- **RBAC:** Three roles — Admin (full access, policy management, audit), Manager (team workspace access, sync configuration), Member (own meetings, no admin functions). Enforced server-side on all API endpoints; client-side for UX guidance only.

### 8.2 Data Protection

- **Encryption at rest:** AES-256-GCM for all meeting data (audio files, transcript content, summaries). Key derivation: Argon2id (memory-hard, resistant to GPU brute-force) from user passphrase with per-installation random salt. Each file encrypted with a unique data encryption key (DEK); DEKs encrypted by the master key (key hierarchy).
- **Encryption in transit:** TLS 1.3 for all client-server communication (team mode). Certificates: self-signed by default (generated on server first launch) or user-provided (for enterprise PKI integration). Certificate pinning optional for high-security deployments.
- **Memory protection:** Sensitive data (passphrase, encryption keys) zeroized from memory after use. No core dumps containing key material (platform-specific: `madvise(MADV_DONTDUMP)` on Linux, `PT_DENY_ATTACH` on macOS).

### 8.3 Secrets Management

- **Single-user:** Passphrase stored only in user's memory. Optional OS keychain integration (macOS Keychain, Windows Credential Manager, Linux Secret Service) for passphrase caching — user opts in during setup.
- **Team mode:** Server secrets (database credentials, TLS private key, JWT signing key) stored in environment variables or Docker secrets. No secrets in configuration files committed to version control.
- **API tokens (Vault/Loop):** Stored encrypted in the server's database; never transmitted to desktop clients.

### 8.4 API Security

- **Authentication required:** All team server API endpoints require valid JWT. Unauthenticated requests receive 401.
- **Input validation:** All API inputs validated against strict schemas. SQL injection prevented via parameterized queries. No user input interpolated into queries or commands.
- **Rate limiting:** Per-user, per-endpoint rate limits prevent abuse.
- **CORS:** Restricted to same-origin or explicitly configured origins (team mode).

### 8.5 Threat Considerations

| Threat | Mitigation |
|---|---|
| Unauthorized access to local data | Encryption at rest; passphrase required to unlock; OS-level file permissions restrict data directory access |
| Man-in-the-middle on team server traffic | TLS 1.3 mandatory; certificate verification; optional certificate pinning |
| Malicious model weights | SHA-256 integrity verification; models sourced from trusted registries only; documentation warns against untrusted model files |
| Privilege escalation in team mode | RBAC enforced server-side; admin actions logged to immutable audit trail |
| Data exfiltration via telemetry | Zero content telemetry by design; only anonymous usage metrics (opt-in); network monitoring tools provided for verification |
| Supply chain attack (dependencies) | Dependency audit before each release; lockfile pinning; no runtime dependency fetching; reproducible builds |

### 8.6 Sovereignty Verification

The architecture supports external verification of the zero-data-exit promise:

- No outbound network connections during core operation (capture, transcribe, summarize, store) — verifiable via firewall rules or network monitoring.
- Open-source audit guide published for security teams to verify network behavior.
- Optional built-in network monitor (admin dashboard) showing all outbound connection attempts.

---

## 9. Infrastructure & Deployment Architecture

### 9.1 Deployment Model

Recap deploys entirely **on-premises / on-device**. No ODW.ai cloud infrastructure is involved in data processing. ODW.ai infrastructure is used only for:

- Model weight distribution (CDN for downloads — optional, can be air-gapped)
- Application update distribution (auto-update server — optional, can be disabled)
- License validation for paid tier (phone-home optional; offline license keys supported)

### 9.2 Desktop Client Deployment

- **macOS:** Universal binary installer (.dmg) — Apple Silicon + Intel. Notarized and signed with Apple Developer certificate.
- **Windows:** MSI/MSIX installer (.x64). Signed with EV code-signing certificate.
- **Linux:** .deb (Ubuntu/Debian), .rpm (Fedora), AppImage (universal). Signed with GPG.
- **Auto-update:** Background check for updates (opt-out available). Updates downloaded, verified (signature check), and applied on user approval. Rollback capability if new version fails health check.

### 9.3 Team Server Deployment

- **Containerization:** Single Docker image containing the server application, database migration tooling, and health check endpoint.
- **Orchestration:** Docker Compose for simple deployments (single server + PostgreSQL + Redis in containers). Kubernetes Helm chart available for enterprise deployments (HA, horizontal scaling).
- **Deployment command:** `docker compose up -d` — one command to stand up the full team server stack.
- **Data persistence:** PostgreSQL data volume and Redis data volume mounted to host filesystem for durability.

### 9.4 Environments

| Environment | Purpose | Infrastructure |
|---|---|---|
| Development | Local development and testing | Developer machines; SQLite in-memory; mock Vault/Loop |
| Staging | Integration testing, pre-release validation | Docker Compose on shared server; real PostgreSQL; mock Vault/Loop or sandbox instances |
| Production (customer) | Customer deployment | Customer's infrastructure; Docker Compose or Kubernetes; customer-managed PostgreSQL |
| Production (ODW.ai) | Model distribution, update server, license validation | ODW.ai CDN and infrastructure (no meeting data touches this) |

### 9.5 CI/CD Approach

- **Source control:** Git (monorepo for desktop + server, or polyrepo with shared libraries).
- **Build pipeline:** GitHub Actions or GitLab CI. Builds triggered on every push; full test suite on every PR.
- **Desktop builds:** Platform-specific build runners (macOS runner for universal binary, Windows runner for MSI, Linux runner for .deb/.rpm/AppImage). Build artifacts signed and published to distribution channels.
- **Server builds:** Docker image built, scanned (Trivy for CVEs), pushed to container registry.
- **Release process:** Semantic versioning. Release candidates deployed to staging for validation; production release after sign-off. Desktop auto-update rolls out progressively (10% → 50% → 100% over 48 hours).

---

## 10. Observability & Operations

### 10.1 Logging Strategy

- **Desktop client:** Structured JSON logs written to local file (`~/RecapData/logs/`). Log levels: ERROR, WARN, INFO, DEBUG, TRACE (configurable). Rotation: 10MB per file, 30 files max (configurable). Logs contain no meeting content — only operational metadata (component names, timings, error codes, model versions).
- **Team server:** Structured JSON logs to stdout (container standard). Collected by customer's logging infrastructure (ELK, Loki, etc.). Includes request IDs for distributed tracing across client-server interactions.
- **Audit logs (team mode):** Separate from operational logs. Immutable append-only log of security-relevant events (authentication, access, policy changes, sync events). Stored in PostgreSQL with configurable retention.

### 10.2 Metrics Collection

- **Desktop client:** Internal metrics counter (meetings processed, transcription duration, model load time, storage usage). Exposed via local HTTP endpoint (`localhost:9090/metrics`) in Prometheus format — opt-in, for power users and admins who want to monitor.
- **Team server:** Prometheus metrics endpoint (`/metrics`) exposing: request rates, error rates, latency histograms, active connections, database pool utilization, sync queue depth.
- **Opt-in telemetry:** Anonymous usage metrics (feature adoption, model selection distribution, error rates) sent to ODW.ai — strictly opt-in, no content, no identifiers. Disabled by default.

### 10.3 Distributed Tracing

- **Team mode:** OpenTelemetry traces for client-server interactions. Trace ID propagated from client request through server to Vault/Loop proxy calls. Enables end-to-end latency debugging for sync operations.
- **Single-user mode:** Not applicable (single process, no distributed calls). Internal span timing for pipeline stages (capture → transcription → summarization) logged for performance debugging.

### 10.4 Alerting & Monitoring

- **Desktop client:** In-app notifications for actionable issues (model download failed, disk space low, sync permanently failed). No external alerting (user is the operator).
- **Team server:** Alerting delegated to customer's monitoring stack. Provided: Prometheus alerting rules for common failure modes (high error rate, sync queue backlog, database connection exhaustion, disk space low). Grafana dashboard template for operational visibility.
- **Admin dashboard:** Built-in web UI showing: usage metrics (meetings processed, storage used), sync health (Vault/Loop connection status, queue depth), compliance status (retention adherence, data residency confirmation), recent audit events.

---

## 11. Cost & Resource Considerations

### 11.1 Major Cost Drivers

| Cost Driver | Location | Nature | Optimization |
|---|---|---|---|
| Transcription compute | Desktop client (CPU/GPU) | Per-meeting processing cost borne by user hardware | Model tier selection (small = less compute); GPU acceleration where available; batch processing during idle time |
| LLM inference (summarization) | Desktop client (CPU/GPU) | Optional; only if user configures local LLM | Smaller quantized models (Q4/Q5); rule-based fallback for zero-cost summarization |
| Storage | Desktop client (disk) | Audio + transcripts + models | Opus compression (~10x vs WAV); retention policies for auto-cleanup; configurable storage location |
| Team server infrastructure | Customer infrastructure | PostgreSQL, Redis, application server | Single Docker Compose stack; minimal resource requirements (2 vCPU, 4GB RAM for ≤200 users) |
| Model distribution CDN | ODW.ai infrastructure | Bandwidth for model weight downloads | CDN caching; differential updates (only changed weights); compression |
| Development & maintenance | ODW.ai engineering | Platform-specific audio adapters, ML pipeline, multi-OS testing | Modular architecture reduces cross-cutting changes; community contributions for platform-specific fixes |

### 11.2 Resource Optimization Strategies

- **Model selection guidance:** On first launch, hardware detection recommends appropriate model tier. Prevents users from selecting models their hardware cannot run efficiently.
- **Audio compression:** Opus encoding at 32kbps (speech-optimized) reduces storage by ~10x vs. uncompressed WAV. Quality sufficient for transcription; original audio quality is not the goal.
- **Incremental sync:** Only changed data synced to Vault/Loop (not full re-sync). Reduces network and processing overhead for repeated sync operations.
- **Lazy model loading:** Transcription and summarization models loaded into memory on first use, not at application startup. Reduces baseline memory footprint.
- **Batch processing scheduling:** Batch re-transcription and summarization can be deferred to periods of low system activity (configurable: immediate, when idle, or manual).

---

## 12. Trade-offs & Design Decisions

### 12.1 Tauri vs. Electron for Desktop Shell

| | Tauri | Electron |
|---|---|---|
| **Binary size** | ~10MB | ~150MB |
| **Memory footprint** | Lower (system webview) | Higher (bundled Chromium) |
| **Maturity** | Younger ecosystem | Mature, battle-tested |
| **Native integration** | Rust backend; excellent FFI for ML libraries | Node.js backend; good FFI but higher overhead |
| **Cross-platform consistency** | Webview rendering varies by OS | Consistent rendering (bundled Chromium) |

**Decision:** Tauri as primary, Electron as fallback. Tauri's smaller footprint and lower resource usage align with the product's on-device performance constraints. Electron retained as fallback if Tauri's webview inconsistencies prove unacceptable on target platforms.

### 12.2 whisper.cpp vs. faster-whisper vs. Original Whisper

| | whisper.cpp | faster-whisper | Original Whisper (PyTorch) |
|---|---|---|---|
| **Runtime** | C++ (GGML) | Python (CTranslate2) | Python (PyTorch) |
| **Performance** | Fastest on CPU; Metal/CUDA support | Fast (4x original) | Baseline |
| **Dependencies** | Minimal (C++ only) | Python + CTranslate2 | Python + PyTorch (heavy) |
| **Embedding** | Easy to embed in desktop app | Requires Python runtime | Requires Python + PyTorch |
| **Streaming support** | Yes (with patches) | Limited | No |

**Decision:** whisper.cpp as primary backend. Minimal dependencies, fastest CPU performance, embeddable without Python runtime, streaming support. faster-whisper available as alternative backend via plugin interface for users who prefer Python ecosystem.

### 12.3 SQLite vs. PostgreSQL for Local Storage

| | SQLite | PostgreSQL |
|---|---|---|
| **Deployment** | Zero-config, embedded | Requires server process |
| **Offline capability** | Full | Requires connection |
| **Concurrency** | Single-writer (sufficient for single-user) | Multi-writer |
| **Full-text search** | FTS5 (built-in) | Requires pg_trgm or external |
| **Scalability** | Adequate for ≤10,000 hours | Unlimited |

**Decision:** SQLite for desktop client (zero-config, offline, sufficient scale). PostgreSQL for team server (multi-user, concurrent writes, advanced querying). This split aligns with the deployment contexts.

### 12.4 Rule-Based vs. LLM Summarization (Free Tier)

| | Rule-Based | Local LLM |
|---|---|---|
| **Quality** | Structured outlines; no prose | Natural language summaries |
| **Hallucination risk** | Zero (extractive only) | Present (mitigated by grounding checks) |
| **Resource cost** | Negligible | Significant (model in memory) |
| **Setup complexity** | Zero (built-in) | User must provide model |
| **User perception** | May feel "limited" | Feels "intelligent" |

**Decision:** Rule-based for free tier (zero setup, zero hallucination, works on all hardware). Local LLM as opt-in enhancement (paid tier or user-provided model). Framing: "structured outline" vs. "AI summary" — sets expectations correctly. Rule-based output designed to feel complete (not broken), just less prose-heavy.

### 12.5 Opus vs. WAV vs. FLAC for Audio Storage

| | Opus | WAV | FLAC |
|---|---|---|---|
| **Compression ratio** | ~10x (32kbps speech) | None | ~2x |
| **Quality for transcription** | More than sufficient | Perfect | Perfect |
| **File size (60min)** | ~15MB | ~600MB | ~300MB |
| **Encoding cost** | Real-time capable | None | Real-time capable |

**Decision:** Opus as default format. 10x storage savings with no meaningful impact on transcription accuracy. WAV available as option for users who want lossless archival.

### 12.6 Sync Proxy (Server-Mediated) vs. Direct Client-to-Vault/Loop

| | Server Proxy | Direct Client |
|---|---|---|
| **Sovereignty** | All traffic stays on customer network | Client needs network access to Vault/Loop |
| **API coupling** | Only server couples to Vault/Loop APIs | Every client couples to Vault/Loop APIs |
| **Offline capability** | Client queues; server syncs when available | Client must be online to sync |
| **Complexity** | Server adds a coordination layer | Simpler (no proxy) |

**Decision:** Server proxy. Reduces coupling (Vault/Loop API changes only affect server), maintains sovereignty (client never needs direct network path to Vault/Loop), and enables centralized sync rule enforcement.

---

## 13. Risks & Mitigations

### 13.1 Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| **Transcription accuracy below expectations** (WER 8-12% vs. cloud <5%) | High | High | Batch re-processing pass for accuracy; post-processing (punctuation, formatting); clear expectation-setting in UX; model swap support for power users; track open-source model improvements |
| **OS audio subsystem changes break capture** | High | Medium | Platform-specific adapter abstraction; multiple capture methods per platform (virtual device + native loopback); compatibility testing on OS beta builds; rapid-response patch process |
| **Speaker diarization quality insufficient** | Medium | Medium | Best-available open models (pyannote/neMo); manual speaker correction UI; post-processing for speaker merging/splitting; confidence labeling |
| **Performance degradation on low-end hardware** | Medium | Medium | Hardware detection at setup; automatic model tier recommendation; graceful degradation (small model on weak hardware); clear hardware requirements documentation |
| **SQLite scalability ceiling** (beyond 10,000 hours) | Low | Low | FTS5 benchmarked to target scale; documented migration path to dedicated search engine (Meilisearch/Tantivy) if needed; architecture supports storage backend swap |
| **Model supply chain compromise** | High | Low | SHA-256 integrity verification; mirrored models on ODW.ai infrastructure; documented manual model installation for air-gapped environments; dependency audit process |
| **Tauri webview inconsistencies across platforms** | Medium | Medium | Electron fallback available; platform-specific testing matrix; webview abstraction layer for critical UI components |

### 13.2 External Dependency Risks

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| **Whisper model development stalls** | High | Low | Model-agnostic architecture; plugin interface supports alternative ASR backends; active monitoring of ASR research landscape |
| **pyannote model access restrictions** (gated Hugging Face) | Medium | Medium | neMo as alternative diarization backend; simpler energy-based segmentation as last resort; model weights mirrored on ODW.ai infrastructure |
| **Vault/Loop API instability** (internal dependency) | High | Medium | Stable API contract agreed before integration; versioned APIs with backward compatibility; sync proxy isolates clients from API changes; graceful degradation if sync unavailable |
| **Meeting platforms block system audio capture** | High | Low | System audio capture is OS-level (not platform-level); platforms would need to implement exclusive audio output (unlikely for compatibility reasons); multiple capture methods provide redundancy |

### 13.3 Operational Risks

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| **Self-hosted deployment support burden** | Medium | High | Docker Compose one-liner deployment; guided setup wizard; comprehensive documentation; known-environment testing matrix; community forum |
| **Key person dependency on audio capture expertise** | Medium | Medium | Thorough architecture documentation; modular adapter design (platform-specific code isolated); cross-training; community engagement for platform-specific contributions |

---

## 14. Assumptions & Constraints

### 14.1 Assumptions

| ID | Assumption | Validation Approach |
|---|---|---|
| SA-01 | Whisper-family models continue to improve, approaching cloud ASR quality within 12-18 months | Track WER benchmarks on standard datasets (LibriSpeech, earnings22); maintain model-swap capability |
| SA-02 | Target users have machines meeting minimum specs (8GB RAM, 4-core CPU, 10GB disk) | Hardware telemetry (opt-in) from early adopters; adjust recommendations based on real-world data |
| SA-03 | System audio capture remains legally permissible and technically feasible across target platforms | Legal review of target jurisdictions; OS beta testing program; multiple capture method fallbacks |
| SA-04 | ODW.ai Vault and Loop modules will be available with stable APIs at Recap v1.0 launch | Joint roadmap alignment with Vault/Loop teams; API contract freeze date enforced |
| SA-05 | SQLite FTS5 performs adequately for full-text search across ≤10,000 hours of transcripts | Benchmark with synthetic data at target scale before release; performance regression tests in CI |
| SA-06 | Tauri provides acceptable cross-platform UX for target platforms | Platform testing matrix executed during development; Electron fallback ready if Tauri fails validation |
| SA-07 | Local LLM quality (with user-provided model) is sufficient for summarization grounding | Evaluation framework for summary factual accuracy; grounding checks enforced in prompt engineering |
| SA-08 | Opus audio quality at 32kbps is sufficient for Whisper transcription accuracy | A/B testing: transcribe same meetings from Opus and WAV sources; verify WER difference is negligible |

### 14.2 Constraints

| ID | Constraint | Source | Impact on Architecture |
|---|---|---|---|
| SC-01 | **No cloud processing** — absolute constraint | Product promise / sovereignty | All compute must run on user hardware or customer infrastructure; no cloud API fallback |
| SC-02 | **English-only for v1.0** | Scope decision | Architecture must support multilingual model loading without schema changes; language detection field present but fixed to "en" |
| SC-03 | **Desktop-only for v1.0** | Scope decision | No mobile SDK or API; audio capture limited to desktop OS audio subsystems |
| SC-04 | **Bundle-first pricing** | Business model | Free core must be functional standalone; paid features are additive (sync, team, governance); cannot gate core transcription behind paywall |
| SC-05 | **Open-source model dependency** | Technical choice | Roadmap partially coupled to external project velocity; must maintain model-agnostic architecture to mitigate |
| SC-06 | **≤2 second streaming transcription latency** | NFR | Constrains model selection for real-time mode; may require smaller model for streaming with batch pass for accuracy |
| SC-07 | **Zero content telemetry** | Privacy promise | Cannot use cloud-based analytics for debugging; must rely on opt-in anonymous metrics and local logs |
| SC-08 | **Self-hosted only for team mode** | Sovereignty promise | No ODW.ai-managed cloud option; customers must have infrastructure or IT resources |
| SC-09 | **On-device resource limits** | Physical reality | Transcription and summarization compete with user's applications; cannot guarantee uniform experience across hardware tiers |
| SC-10 | **Regulatory compliance (GDPR, HIPAA-adjacent)** | Target market requirement | Architecture must support data minimization, retention policies, audit trails, and cryptographic erasure |

---

## 15. Future Evolution & Extensibility

### 15.1 Planned Extensions (Post-v1.0)

- **Multilingual support:** Architecture already supports loading language-specific models. Extension requires: language detection pipeline, multilingual model registry, UI localization framework. No schema changes needed — `language` field exists on Transcript entity.
- **Semantic search:** Add vector embeddings for transcript segments. Architecture supports pluggable search backends; SQLite-vec or standalone vector index can be added without migrating existing FTS5 data. Hybrid search (keyword + semantic) as v1.1 feature.
- **Real-time translation:** Extend summarization engine with translation capability. Model-agnostic design means any translation model (local) can be integrated via the plugin interface.
- **Mobile companion:** Read-only mobile app for viewing transcripts and summaries (synced from desktop). Does not require mobile audio capture. Sync via local network (desktop as server) or via team server.
- **Calendar integration:** Optional calendar awareness for meeting auto-tagging and participant pre-identification. Does not require auto-join (still captures system audio); calendar data used for metadata enrichment only.
- **Sales vertical features:** Deal signal detection, coaching insights, CRM sync. Built as extensions on top of the core pipeline — structured outputs (action items, decisions) already provide the data foundation.

### 15.2 Extension Points

| Extension Point | Current State | Future Capability |
|---|---|---|
| Transcription backend plugin | whisper.cpp + faster-whisper | Any ASR engine implementing `TranscriptionBackend` interface |
| Summarization provider | Rule-based + local LLM (llama.cpp/Ollama) | Any LLM implementing prompt/response interface; cloud LLM option (if sovereignty constraint relaxed for premium tier) |
| Audio capture adapter | macOS/Windows/Linux system audio | Telephony/SIP capture; dedicated microphone array support; hardware audio device integration |
| Search backend | SQLite FTS5 | Pluggable search interface; swap to Meilisearch, Tantivy, or vector-capable engine |
| Storage backend | SQLite + encrypted files | Pluggable storage interface; support for alternative databases or encrypted object stores |
| Sync target | Vault + Loop | Pluggable sync interface; third-party integrations (Notion, Jira, Asana) via sync adapters |
| Export format | Markdown, PDF, JSON, plain text | Pluggable export interface; custom templates; API-based export for integrations |

### 15.3 Scaling Evolution

- **From single-user to team:** Already supported via optional team server. No architecture changes needed.
- **From team to enterprise:** Add horizontal server scaling (load-balanced instances), multi-tenancy in PostgreSQL, SCIM provisioning, advanced audit (immutable append-only log with cryptographic chaining).
- **From on-prem to hybrid:** If cloud transcription option added (post-v1.1, as premium tier), add encrypted upload pipeline with zero-knowledge architecture (server cannot decrypt audio). Core on-device path unchanged.
- **From desktop to platform-agnostic:** Extract core pipeline (capture → transcribe → summarize → sync) into a standalone library/SDK. Desktop app becomes one consumer; CLI tool, server API, and mobile app become additional consumers.

### 15.4 Architecture Preservation Principles

As the system evolves, these invariants must be maintained:

1. **Sovereignty by default:** On-device processing remains the default and recommended path. Any cloud option must be explicitly opt-in with clear user consent.
2. **Offline capability:** Core features must always work without network connectivity. Network-dependent features (sync, updates) are additive and gracefully degradable.
3. **Model agnosticism:** No hard coupling to specific model architectures. Plugin interfaces allow swapping transcription and summarization engines without system-wide changes.
4. **Data portability:** Users can always export their data in open formats. No vendor lock-in.
5. **Bundle value:** Recap's strategic value is as a data source for the ODW.ai suite. New features should strengthen Vault/Loop integration, not diverge into standalone functionality.

---

*End of System Architecture Document*
