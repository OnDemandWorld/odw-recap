# Recap — Task Breakdown Document

**Product:** Recap by ODW.ai  
**Version:** 1.0  
**Date:** June 23, 2026  
**Status:** Draft

---

## 1. Execution Overview

### 1.1 Implementation Strategy

Recap is a sovereign, on-device meeting intelligence system built with Tauri (Rust backend + webview frontend). The implementation follows a phased approach:

**Major Phases:**
1. **Foundation & Infrastructure** — Project setup, audio capture, transcription pipeline
2. **Core Processing** — Summarization, storage, encryption
3. **Desktop Shell** — UI, system tray, hotkeys, onboarding
4. **Team Server** — Go backend, PostgreSQL, RBAC, audit
5. **Integrations** — Vault/Loop sync, model management
6. **Testing & Hardening** — E2E tests, performance optimization
7. **Deployment** — Packaging, distribution, documentation

**Parallelization:**
- **Parallel Group 1:** Audio capture + Storage module (independent)
- **Parallel Group 2:** Transcription + Summarization (depends on audio capture)
- **Parallel Group 3:** Desktop shell UI + Team server (independent)
- **Sequential:** Sync engine (depends on storage + team server)

**Critical Path:**
Audio Capture → Transcription → Summarization → Storage → Desktop Shell → Testing → Release

### 1.2 Effort Estimate

- **Total tasks:** ~50 (was ~35, added 15 new tasks for multi-provider support and flexible input)
- **Single agent:** 60-80 days (was 45-60 days)
- **Parallel agents (3-4):** 25-35 days (was 18-25 days)

---

## 2. Task Breakdown Structure

### Epic 1: Foundation & Infrastructure (7 tasks)
- T1.1: Project scaffolding (Tauri + React/Svelte)
- T1.2: Audio capture module (platform adapters)
- T1.3: Storage module (SQLite + encryption)
- T1.4: Model manager (download, verify, activate)
- T1.5: Configuration system (settings, preferences)
- T1.6: Audio input manager (file import, watch folder, HTTP upload server) **NEW**
- T1.7: Meeting metadata entry system (type, location, participants, language, topic) **NEW**

### Epic 2: Core Processing Pipeline (21 tasks)
- T2.1: Transcription module (whisper.cpp integration)
- T2.2: Streaming transcription (real-time buffer)
- T2.3: Speaker diarization (pyannote/neMo)
- T2.4: Rule-based summarization (pattern matching)
- T2.5: LLM summarization (llama.cpp FFI)
- T2.6: Output formatting (templates, export)
- T2.7: STT provider router (trait definition, fallback logic) **NEW**
- T2.8: OpenAI STT integration (Whisper API client) **NEW**
- T2.9: AssemblyAI integration **NEW**
- T2.10: Deepgram integration **NEW**
- T2.11: AWS Transcribe integration **NEW**
- T2.12: Azure Speech integration **NEW**
- T2.13: Google Speech-to-Text integration **NEW**
- T2.14: LLM provider router (trait definition) **NEW**
- T2.15: OpenAI LLM integration (GPT-4 client) **NEW**
- T2.16: Anthropic Claude integration **NEW**
- T2.17: Google Gemini integration **NEW**
- T2.18: AWS Bedrock integration **NEW**
- T2.19: Azure OpenAI integration **NEW**
- T2.20: Prompt manager (template loading, versioning, rendering) **NEW**
- T2.21: Prompt template library (pre-built templates for meeting types) **NEW**

### Epic 3: Desktop Shell & UI (11 tasks)
- T3.1: Tauri app shell (window, tray, hotkeys)
- T3.2: Onboarding flow (audio setup, model selection)
- T3.3: Recording UI (real-time transcript display)
- T3.4: Meeting library (browse, search, filter)
- T3.5: Meeting detail view (transcript, summary, action items)
- T3.6: Settings panels (audio, models, sync, privacy)
- T3.7: System tray integration (status, quick actions)
- T3.8: Import dialog (drag-and-drop, file browser) **NEW**
- T3.9: Metadata entry dialog (type, location, participants, language, topic) **NEW**
- T3.10: Provider settings panel (STT/LLM configuration, API keys) **NEW**
- T3.11: Prompt editor page (view/edit templates, preview) **NEW**

### Epic 4: Team Server (6 tasks)
- T4.1: Go HTTP server scaffolding (Chi/Echo)
- T4.2: Authentication (JWT, SSO SAML/OIDC)
- T4.3: RBAC enforcement (Admin, Manager, Member)
- T4.4: Audit logging (append-only, tamper-evident)
- T4.5: Policy manager (retention, sync rules)
- T4.6: Admin dashboard API (metrics, user management)

### Epic 5: Integrations & Sync (8 tasks)
- T5.1: Sync engine (rule evaluation, transformation)
- T5.2: Vault sync client (summaries → knowledge base)
- T5.3: Loop sync client (action items → tasks)
- T5.4: Retry queue (exponential backoff, circuit breaker)
- T5.5: Team server proxy (Vault/Loop API forwarding)
- T5.6: Mobile companion app (Tauri v2 iOS/Android, recording + upload) **NEW**
- T5.7: iOS Shortcuts integration (audio upload from iPhone) **NEW**
- T5.8: LocalSend protocol support (zero-config phone→PC transfer) **NEW**

### Epic 6: Testing & Quality (4 tasks)
- T6.1: Unit tests (all modules, ≥80% coverage)
- T6.2: Integration tests (audio → transcript → summary)
- T6.3: E2E tests (desktop app workflows)
- T6.4: Performance tests (latency, throughput, memory)

### Epic 7: Deployment & Release (2 tasks)
- T7.1: Packaging (macOS .dmg, Windows .msi, Linux .AppImage)
- T7.2: Documentation (user guide, admin guide, API docs)

---

## 3. Task Definitions

### T1.1: Project Scaffolding

**ID:** REC-INFRA-001  
**Title:** Initialize Tauri + React/Svelte project structure  
**Description:** Create the foundational project structure with Tauri 2.x backend and React or Svelte frontend. Configure build tooling (Vite), TypeScript, Tailwind CSS, and development environment.  
**Inputs:** TSD Section 3.1 (Backend), Section 3.2 (Frontend)  
**Output:** 
- `recap/` directory with Tauri project
- `src-tauri/` (Rust backend)
- `src/` (React/Svelte frontend)
- `package.json`, `tauri.conf.json`, `vite.config.ts`
- Working dev server (`npm run tauri dev`)

**Acceptance Criteria:**
- `npm run tauri dev` launches app window
- Rust backend compiles without errors
- Frontend renders "Hello Recap" in webview
- Tailwind CSS styles apply correctly
- Hot reload works for both Rust and frontend

**Dependencies:** None  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** M (4-6 hours)

---

### T1.2: Audio Capture Module

**ID:** REC-INFRA-002  
**Title:** Implement platform-specific audio capture adapters  
**Description:** Build the `audio-capture` module with platform adapters for macOS (CoreAudio + BlackHole), Windows (WASAPI loopback), and Linux (PulseAudio/PipeWire). Implement audio mixing (system + mic) and buffered writing.  
**Inputs:** TSD Section 2.1 (Audio Capture Module)  
**Output:**
- `src-tauri/src/audio_capture/mod.rs`
- `src-tauri/src/audio_capture/platform_adapter.rs` (trait)
- `src-tauri/src/audio_capture/coreaudio.rs` (macOS)
- `src-tauri/src/audio_capture/wasapi.rs` (Windows)
- `src-tauri/src/audio_capture/pulseaudio.rs` (Linux)
- `src-tauri/src/audio_capture/mixer.rs`
- `src-tauri/src/audio_capture/buffered_writer.rs`

**Acceptance Criteria:**
- Captures system audio on macOS (requires BlackHole installation)
- Captures system audio on Windows (WASAPI loopback)
- Captures system audio on Linux (PulseAudio monitor)
- Mixes system audio (left channel) + mic (right channel)
- Flushes audio to disk every 5 seconds (Opus format)
- Handles audio device hot-plug/unplug gracefully
- No audible latency or degradation of meeting audio

**Dependencies:** T1.1  
**Execution Type:** Developer (platform-specific expertise required)  
**Priority:** Critical  
**Effort:** L (12-16 hours)

---

### T1.3: Storage Module

**ID:** REC-INFRA-003  
**Title:** Implement local storage with encryption and search  
**Description:** Build the `storage` module with SQLite database (WAL mode, FTS5), AES-256-GCM encryption, Argon2id key derivation, and file management.  
**Inputs:** TSD Section 2.4 (Storage Module)  
**Output:**
- `src-tauri/src/storage/mod.rs`
- `src-tauri/src/storage/sqlite_manager.rs` (connection, migrations, FTS5)
- `src-tauri/src/storage/encryption_manager.rs` (AES-256-GCM, Argon2id)
- `src-tauri/src/storage/file_store.rs` (encrypted audio files)
- `src-tauri/src/storage/export_engine.rs` (Markdown, PDF, JSON)
- `src-tauri/src/storage/retention_enforcer.rs`
- `migrations/` (SQLite migration files)

**Acceptance Criteria:**
- SQLite database created with WAL mode
- FTS5 virtual table for full-text search
- AES-256-GCM encryption/decryption works
- Key derivation via Argon2id from user passphrase
- Per-file DEKs encrypted by master key
- Supports ≥10,000 hours of meetings
- Full-text search across corpus: ≤1 second
- Atomic writes (transcript + summary + action items in single transaction)
- Export to Markdown, PDF, JSON formats

**Dependencies:** T1.1  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** L (12-16 hours)

---

### T1.4: Model Manager

**ID:** REC-INFRA-004  
**Title:** Implement model lifecycle management  
**Description:** Build the `model-manager` module for downloading, verifying, installing, and activating ML models (Whisper ASR, diarization, LLM). Support chunked downloads with resume, SHA-256 verification, and air-gapped loading.  
**Inputs:** TSD Section 2.8 (Model Manager Module)  
**Output:**
- `src-tauri/src/model_manager/mod.rs`
- `src-tauri/src/model_manager/downloader.rs` (chunked HTTP, resume)
- `src-tauri/src/model_manager/verifier.rs` (SHA-256)
- `src-tauri/src/model_manager/registry.rs` (installed models, versions)
- `src-tauri/src/model_manager/airgap_loader.rs`

**Acceptance Criteria:**
- Downloads Whisper models (small, medium, large) from registry
- Chunked download with resume capability
- SHA-256 verification on completion
- Tracks installed models and active selection
- Switching active model takes effect on next recording
- Multiple model versions coexist on disk
- Air-gap mode: loads models from local directory

**Dependencies:** T1.1  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T1.5: Configuration System

**ID:** REC-INFRA-005  
**Title:** Implement configuration and settings management  
**Description:** Build a configuration system for user preferences, audio settings, model selection, sync rules, and privacy controls. Persist to encrypted local file.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src-tauri/src/config/mod.rs`
- `src-tauri/src/config/settings.rs` (struct definitions)
- `src-tauri/src/config/persistence.rs` (encrypted file I/O)

**Acceptance Criteria:**
- Loads/saves settings to encrypted JSON file
- Settings include: audio input, model selection, sync rules, retention, privacy
- Default values provided for all settings
- Settings changes trigger UI updates
- Configuration validated on load

**Dependencies:** T1.3 (storage module for encryption)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** S (3-4 hours)

---

### T2.1: Transcription Module

**ID:** REC-CORE-001  
**Title:** Integrate whisper.cpp for batch transcription  
**Description:** Build the `transcription` module with whisper.cpp Rust bindings for batch transcription of audio files. Implement model loading, audio preprocessing, and transcript generation with timestamps.  
**Inputs:** TSD Section 2.2 (Transcription Module)  
**Output:**
- `src-tauri/src/transcription/mod.rs`
- `src-tauri/src/transcription/backend.rs` (trait)
- `src-tauri/src/transcription/whisper_cpp.rs` (primary implementation)
- `src-tauri/src/transcription/types.rs` (Transcript, TranscriptSegment)

**Acceptance Criteria:**
- Loads Whisper models (small, medium, large)
- Transcribes audio files (Opus, WAV) to timestamped text
- Output: `Transcript` struct with `TranscriptSegment[]` (timestamps, speaker labels, confidence, text)
- Batch processing: ≤10 minutes for 60-minute meeting on M2 Mac
- Supports multiple languages (English, Cantonese, Mandarin, etc.)
- Graceful error handling (model load failure, audio corruption)

**Dependencies:** T1.2 (audio capture), T1.4 (model manager)  
**Execution Type:** Developer (Rust + C++ FFI expertise)  
**Priority:** Critical  
**Effort:** L (16-20 hours)

---

### T2.2: Streaming Transcription

**ID:** REC-CORE-002  
**Title:** Implement real-time streaming transcription  
**Description:** Extend transcription module for real-time streaming mode. Process audio chunks as they arrive, maintain context window, and emit partial transcripts with ≤2 second latency.  
**Inputs:** TSD Section 2.2 (Transcription Module)  
**Output:**
- `src-tauri/src/transcription/streaming_buffer.rs` (ring buffer, backpressure)
- `src-tauri/src/transcription/streaming_transcriber.rs`

**Acceptance Criteria:**
- Processes audio chunks in real-time (16kHz, 16-bit mono)
- Emits partial transcripts with ≤2 second end-to-end latency
- Maintains context window for coherence
- Handles backpressure (slow processing → buffer overflow protection)
- Seamless transition to batch mode for final accuracy pass

**Dependencies:** T2.1 (batch transcription)  
**Execution Type:** Developer  
**Priority:** High  
**Effort:** L (12-16 hours)

---

### T2.3: Speaker Diarization

**ID:** REC-CORE-003  
**Title:** Implement speaker diarization  
**Description:** Integrate pyannote-audio or neMo for speaker identification. Label transcript segments with speaker IDs (Speaker 1, Speaker 2, etc.).  
**Inputs:** TSD Section 2.2 (Transcription Module)  
**Output:**
- `src-tauri/src/transcription/diarization.rs`
- `src-tauri/src/transcription/diarization_engine.rs` (pyannote/neMo FFI)

**Acceptance Criteria:**
- Identifies distinct speakers in audio
- Labels transcript segments with speaker IDs
- Supports ≥2 speakers, up to 10
- Diarization accuracy: ≥85% (speaker boundary detection)
- Works with both streaming and batch modes

**Dependencies:** T2.1 (transcription module)  
**Execution Type:** Developer (ML/audio expertise)  
**Priority:** High  
**Effort:** M (8-10 hours)

---

### T2.4: Rule-Based Summarization

**ID:** REC-CORE-004  
**Title:** Implement rule-based extraction (no LLM)  
**Description:** Build pattern-matching extraction for action items, decisions, and topics. Zero network calls, zero external dependencies.  
**Inputs:** TSD Section 2.3 (Summarization Module)  
**Output:**
- `src-tauri/src/summarization/mod.rs`
- `src-tauri/src/summarization/rule_based.rs`
- `src-tauri/src/summarization/action_extractor.rs`
- `src-tauri/src/summarization/decision_extractor.rs`
- `src-tauri/src/summarization/topic_extractor.rs`

**Acceptance Criteria:**
- Detects action phrases ("I'll...", "Action item:", "TODO:", "follow up")
- Parses temporal expressions for deadlines ("by Friday", "next week")
- Detects decision phrases ("We decided", "Let's go with", "Agreed that")
- Extracts topics via TF-IDF or keyword frequency
- Output: `Summary`, `ActionItem[]`, `Decision[]`, `TopicTag[]`
- Zero network calls, zero external model downloads
- Fallback mode: automatically activates if LLM fails to load

**Dependencies:** T2.1 (transcription)  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** M (8-10 hours)

---

### T2.5: LLM Summarization

**ID:** REC-CORE-005  
**Title:** Implement LLM-powered summarization  
**Description:** Integrate llama.cpp or Ollama for local LLM inference. Generate summaries, action items, decisions with grounding (all claims reference transcript segments).  
**Inputs:** TSD Section 2.3 (Summarization Module)  
**Output:**
- `src-tauri/src/summarization/llm_summarizer.rs` (llama.cpp FFI or Ollama client)
- `src-tauri/src/summarization/prompt_templates.rs`

**Acceptance Criteria:**
- Loads local LLM via llama.cpp or connects to Ollama
- Generates structured outputs: `Summary`, `ActionItem[]`, `Decision[]`, `TopicTag[]`
- All summary claims reference transcript segments (no hallucination)
- Model-agnostic (any compatible local model)
- Fallback: degrades to rule-based mode if LLM fails

**Dependencies:** T2.4 (rule-based fallback), T1.4 (model manager)  
**Execution Type:** Developer (Rust + LLM FFI expertise)  
**Priority:** High  
**Effort:** L (12-16 hours)

---

### T2.6: Output Formatting

**ID:** REC-CORE-006  
**Title:** Implement output formatting and templates  
**Description:** Apply user-configured templates to raw extraction results. Support custom templates for summary, action items, decisions.  
**Inputs:** TSD Section 2.3 (Summarization Module)  
**Output:**
- `src-tauri/src/summarization/output_formatter.rs`
- `src-tauri/src/summarization/templates/` (default templates)

**Acceptance Criteria:**
- Applies templates to `Summary`, `ActionItem[]`, `Decision[]`
- Supports custom user templates (Markdown-based)
- Default templates provided (standard meeting notes format)
- Output formats: Markdown, plain text

**Dependencies:** T2.4, T2.5 (summarization modules)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** S (4-6 hours)

---

### T3.1: Tauri App Shell

**ID:** REC-UI-001  
**Title:** Build Tauri application shell  
**Description:** Create the main Tauri app window, system tray/menu bar integration, hotkey handling, and module orchestration.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src-tauri/src/main.rs` (app entry, module orchestration)
- `src-tauri/src/hotkey_manager.rs` (global hotkeys)
- `src-tauri/src/tray_manager.rs` (system tray)
- `src/main.tsx` (React entry) or `src/App.svelte`

**Acceptance Criteria:**
- App window opens on launch
- System tray icon with status indicator
- Global hotkey: start/stop recording (configurable)
- Cold start to ready: ≤5 seconds
- Idle CPU: ≤30% of one core (tray mode)
- Binary size: ~10MB (Tauri advantage)

**Dependencies:** T1.1 (project scaffolding)  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** M (6-8 hours)

---

### T3.2: Onboarding Flow

**ID:** REC-UI-002  
**Title:** Implement onboarding wizard  
**Description:** Build guided setup flow: audio input selection, model download, test recording.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/Onboarding.tsx` (or `.svelte`)
- `src/components/AudioSetup.tsx`
- `src/components/ModelSelection.tsx`
- `src/components/TestRecording.tsx`

**Acceptance Criteria:**
- Step 1: Select audio input (system audio device, mic)
- Step 2: Download Whisper model (small/medium/large)
- Step 3: Test recording (30 seconds, playback)
- Progress indicators, error handling
- Skippable for returning users

**Dependencies:** T3.1 (app shell), T1.2 (audio capture), T1.4 (model manager)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T3.3: Recording UI

**ID:** REC-UI-003  
**Title:** Build recording interface with real-time transcript  
**Description:** Create the recording screen with real-time transcript display, speaker labels, and stop/pause controls.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/Recording.tsx`
- `src/components/TranscriptDisplay.tsx`
- `src/components/RecordingControls.tsx`

**Acceptance Criteria:**
- Real-time transcript display (streaming mode)
- Speaker labels (Speaker 1, Speaker 2, etc.)
- Stop/pause controls
- Recording duration timer
- Visual indicator: recording in progress

**Dependencies:** T3.1 (app shell), T2.2 (streaming transcription)  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** M (6-8 hours)

---

### T3.4: Meeting Library

**ID:** REC-UI-004  
**Title:** Build meeting library browser  
**Description:** Create the meeting library page with browse, search, filter, and sort capabilities.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/MeetingLibrary.tsx`
- `src/components/MeetingCard.tsx`
- `src/components/SearchBar.tsx`
- `src/components/FilterPanel.tsx`

**Acceptance Criteria:**
- Displays list of meetings (title, date, duration, participants)
- Full-text search across transcripts and summaries
- Filter by date range, duration, participants
- Sort by date, duration, title
- Pagination or infinite scroll
- Click to open meeting detail

**Dependencies:** T3.1 (app shell), T1.3 (storage module)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T3.5: Meeting Detail View

**ID:** REC-UI-005  
**Title:** Build meeting detail page  
**Description:** Create the meeting detail page with transcript, summary, action items, decisions, and export options.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/MeetingDetail.tsx`
- `src/components/TranscriptView.tsx`
- `src/components/SummaryView.tsx`
- `src/components/ActionItemsList.tsx`
- `src/components/ExportMenu.tsx`

**Acceptance Criteria:**
- Displays full transcript with timestamps and speaker labels
- Displays summary, action items, decisions
- Export to Markdown, PDF, JSON
- Edit transcript/summary (optional, v2)
- Delete meeting (with confirmation)

**Dependencies:** T3.4 (meeting library), T1.3 (storage module)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T3.6: Settings Panels

**ID:** REC-UI-006  
**Title:** Build settings interface  
**Description:** Create settings panels for audio, models, sync, privacy, and retention.  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src/pages/Settings.tsx`
- `src/components/AudioSettings.tsx`
- `src/components/ModelSettings.tsx`
- `src/components/SyncSettings.tsx`
- `src/components/PrivacySettings.tsx`

**Acceptance Criteria:**
- Audio settings: input device selection, test
- Model settings: download, switch, delete
- Sync settings: enable/disable, rules, Vault/Loop endpoints
- Privacy settings: encryption passphrase, data directory
- Retention settings: auto-delete after N days
- Settings persist across app restarts

**Dependencies:** T3.1 (app shell), T1.5 (config system)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** M (6-8 hours)

---

### T3.7: System Tray Integration

**ID:** REC-UI-007  
**Title:** Implement system tray/menu bar  
**Description:** Build system tray icon with status indicator, quick actions (start/stop recording), and menu (open app, settings, quit).  
**Inputs:** TSD Section 2.6 (Desktop Shell Module)  
**Output:**
- `src-tauri/src/tray_manager.rs` (update)
- `src-tauri/tray-icons/` (icon assets)

**Acceptance Criteria:**
- Tray icon shows recording status (idle, recording, paused)
- Right-click menu: Open App, Start Recording, Settings, Quit
- Double-click: open app window
- Works on macOS, Windows, Linux

**Dependencies:** T3.1 (app shell)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** S (4-6 hours)

---

### T4.1: Team Server Scaffolding

**ID:** REC-SRV-001  
**Title:** Initialize Go HTTP server  
**Description:** Create the team server with Go (Chi or Echo framework), PostgreSQL connection, Redis connection, and basic routing.  
**Inputs:** TSD Section 2.7 (Team Server Module), Section 3.3 (Team Server Tech Stack)  
**Output:**
- `team-server/` directory
- `team-server/main.go`
- `team-server/internal/handler/` (HTTP handlers)
- `team-server/internal/db/` (PostgreSQL connection)
- `team-server/internal/cache/` (Redis connection)
- `team-server/docker-compose.yml` (server + postgres + redis)

**Acceptance Criteria:**
- Go server compiles and runs
- PostgreSQL connection established
- Redis connection established
- Health endpoint: `GET /health`
- Docker Compose starts server + postgres + redis
- Support ≥200 concurrent users

**Dependencies:** None  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** M (4-6 hours)

---

### T4.2: Authentication

**ID:** REC-SRV-002  
**Title:** Implement JWT authentication and SSO  
**Description:** Build authentication with JWT (RS256), local login, and SSO (SAML 2.0, OIDC).  
**Inputs:** TSD Section 2.7 (Team Server Module)  
**Output:**
- `team-server/internal/auth/jwt.go`
- `team-server/internal/auth/handler.go` (login, refresh, SSO callback)
- `team-server/internal/auth/saml.go`
- `team-server/internal/auth/oidc.go`

**Acceptance Criteria:**
- Local login: email/password → JWT access token (24h) + refresh token (7d)
- Token refresh: refresh token → new access token
- SSO SAML: redirect to IdP, handle callback, create/update user
- SSO OIDC: redirect to IdP, handle callback, create/update user
- JWT validation on all protected endpoints
- Password hashing: bcrypt

**Dependencies:** T4.1 (server scaffolding)  
**Execution Type:** Developer (auth expertise)  
**Priority:** Critical  
**Effort:** L (12-16 hours)

---

### T4.3: RBAC Enforcement

**ID:** REC-SRV-003  
**Title:** Implement role-based access control  
**Description:** Build RBAC with three roles: Admin, Manager, Member. Enforce permissions on all endpoints.  
**Inputs:** TSD Section 2.7 (Team Server Module)  
**Output:**
- `team-server/internal/auth/rbac.go`
- `team-server/internal/middleware/rbac_middleware.go`

**Acceptance Criteria:**
- Admin: full access (user management, policies, audit logs)
- Manager: read/write meetings, manage sync rules
- Member: read/write own meetings only
- Middleware enforces permissions on all endpoints
- Role assignment via admin API

**Dependencies:** T4.2 (authentication)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T4.4: Audit Logging

**ID:** REC-SRV-004  
**Title:** Implement tamper-evident audit logging  
**Description:** Build append-only audit log with SHA-256 hash chain. Record all access events (who, what, when).  
**Inputs:** TSD Section 2.7 (Team Server Module)  
**Output:**
- `team-server/internal/audit/logger.go`
- `team-server/internal/audit/handler.go` (query, export)
- `team-server/migrations/audit_table.sql`

**Acceptance Criteria:**
- Append-only audit table (PostgreSQL)
- SHA-256 hash chain (each entry hashes previous entry's hash)
- Records: user_id, action, resource, timestamp, IP address
- Query API: filter by user, action, date range
- Export: CSV, JSON
- Tamper-evident: hash chain validation detects modifications

**Dependencies:** T4.1 (server scaffolding)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T4.5: Policy Manager

**ID:** REC-SRV-005  
**Title:** Implement retention and sync policy management  
**Description:** Build policy manager for retention policies (auto-delete after N days) and sync rules (which meetings to sync to Vault/Loop).  
**Inputs:** TSD Section 2.7 (Team Server Module)  
**Output:**
- `team-server/internal/policy/manager.go`
- `team-server/internal/policy/handler.go`
- `team-server/migrations/policy_tables.sql`

**Acceptance Criteria:**
- Retention policies: auto-delete meetings after N days
- Sync rules: filter by date, duration, participants, tags
- Policy CRUD API (admin only)
- Policies enforced by background job (retention) and sync engine (sync rules)

**Dependencies:** T4.1 (server scaffolding)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** M (6-8 hours)

---

### T4.6: Admin Dashboard API

**ID:** REC-SRV-006  
**Title:** Implement admin dashboard API  
**Description:** Build admin API for usage metrics, user management, and health status.  
**Inputs:** TSD Section 2.7 (Team Server Module)  
**Output:**
- `team-server/internal/admin/handler.go`
- `team-server/internal/admin/metrics.go`

**Acceptance Criteria:**
- Usage metrics: total meetings, total duration, active users
- User management: list, create, update, delete users
- Health status: server health, database health, Redis health
- Admin-only endpoints (RBAC enforced)

**Dependencies:** T4.3 (RBAC)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** M (6-8 hours)

---

### T5.1: Sync Engine

**ID:** REC-SYNC-001  
**Title:** Implement sync rule evaluation and orchestration  
**Description:** Build the sync engine that evaluates sync rules, determines which meetings to sync, and orchestrates Vault/Loop sync clients.  
**Inputs:** TSD Section 2.5 (Sync Module)  
**Output:**
- `src-tauri/src/sync/mod.rs`
- `src-tauri/src/sync/rule_evaluator.rs`
- `src-tauri/src/sync/orchestrator.rs`

**Acceptance Criteria:**
- Evaluates sync rules from team server
- Determines which meetings to sync (based on rules)
- Orchestrates Vault sync client and Loop sync client
- Sync is opportunistic and non-blocking (never blocks capture-transcribe-summarize)
- Sync status reflected in UI

**Dependencies:** T1.3 (storage), T4.5 (policy manager)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (8-10 hours)

---

### T5.2: Vault Sync Client

**ID:** REC-SYNC-002  
**Title:** Implement Vault integration  
**Description:** Build Vault sync client that transforms summaries/decisions into Vault-compatible entries and POSTs to Vault API.  
**Inputs:** TSD Section 2.5 (Sync Module)  
**Output:**
- `src-tauri/src/sync/vault_client.rs`

**Acceptance Criteria:**
- Transforms `Summary`, `Decision[]` → Vault knowledge base entry
- POSTs to Vault API (via team server proxy)
- Links Vault entry to source meeting (metadata)
- Handles Vault API errors gracefully

**Dependencies:** T5.1 (sync engine)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T5.3: Loop Sync Client

**ID:** REC-SYNC-003  
**Title:** Implement Loop integration  
**Description:** Build Loop sync client that transforms action items into Loop tasks and POSTs to Loop API.  
**Inputs:** TSD Section 2.5 (Sync Module)  
**Output:**
- `src-tauri/src/sync/loop_client.rs`

**Acceptance Criteria:**
- Transforms `ActionItem[]` → Loop tasks
- POSTs to Loop API (via team server proxy)
- Includes assignee, deadline, description, source meeting link
- Handles Loop API errors gracefully

**Dependencies:** T5.1 (sync engine)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T5.4: Retry Queue

**ID:** REC-SYNC-004  
**Title:** Implement retry queue with exponential backoff  
**Description:** Build SQLite-backed retry queue for failed syncs. Implement exponential backoff (1min → 2min → 4min → ... → max 1hr) and circuit breaker (5 failures in 10 min → open for 15 min).  
**Inputs:** TSD Section 2.5 (Sync Module)  
**Output:**
- `src-tauri/src/sync/retry_queue.rs`
- `src-tauri/src/sync/circuit_breaker.rs`

**Acceptance Criteria:**
- Failed syncs added to retry queue (SQLite)
- Exponential backoff: 1min → 2min → 4min → ... → max 1hr
- Max retry duration: 30 days (then marked permanently failed)
- Circuit breaker: 5 failures in 10 min → open for 15 min → half-open probe
- Queue processing: background job, non-blocking

**Dependencies:** T5.1 (sync engine)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** M (6-8 hours)

---

### T5.5: Team Server Proxy

**ID:** REC-SYNC-005  
**Title:** Implement team server proxy for Vault/Loop  
**Description:** Build team server endpoints that transform and forward sync requests to Vault/Loop APIs.  
**Inputs:** TSD Section 2.7 (Team Server Module)  
**Output:**
- `team-server/internal/proxy/vault_proxy.go`
- `team-server/internal/proxy/loop_proxy.go`

**Acceptance Criteria:**
- Receives sync requests from desktop clients
- Transforms data (if needed) and forwards to Vault/Loop APIs
- Handles authentication (service account)
- Returns sync status to desktop client

**Dependencies:** T4.1 (server scaffolding)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (6-8 hours)

---

### T6.1: Unit Tests

**ID:** REC-TEST-001  
**Title:** Write unit tests for all modules  
**Description:** Write unit tests for all Rust modules (audio capture, transcription, summarization, storage, sync) and Go modules (team server). Target ≥80% coverage.  
**Inputs:** All module implementations  
**Output:**
- `src-tauri/src/**/*_test.rs` (Rust unit tests)
- `team-server/internal/**/*_test.go` (Go unit tests)

**Acceptance Criteria:**
- ≥80% code coverage for all modules
- Tests run via `cargo test` (Rust) and `go test` (Go)
- Mock external dependencies (audio devices, databases, APIs)
- Fast execution: <5 minutes for full test suite

**Dependencies:** All module implementations  
**Execution Type:** AI-Agent  
**Priority:** Critical  
**Effort:** L (16-20 hours)

---

### T6.2: Integration Tests

**ID:** REC-TEST-002  
**Title:** Write integration tests for end-to-end workflows  
**Description:** Write integration tests for key workflows: audio → transcript → summary → storage, sync to Vault/Loop, team server auth + RBAC.  
**Inputs:** All module implementations  
**Output:**
- `src-tauri/tests/integration/` (Rust integration tests)
- `team-server/tests/integration/` (Go integration tests)

**Acceptance Criteria:**
- Test: audio file → transcript → summary → stored in SQLite
- Test: meeting synced to Vault (mock Vault API)
- Test: action items synced to Loop (mock Loop API)
- Test: team server login → JWT → RBAC enforcement
- Tests run in CI (GitHub Actions)

**Dependencies:** T6.1 (unit tests)  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** L (12-16 hours)

---

### T6.3: E2E Tests

**ID:** REC-TEST-003  
**Title:** Write end-to-end tests for desktop app  
**Description:** Write E2E tests for desktop app workflows: onboarding, recording, meeting library, settings.  
**Inputs:** Desktop app implementation  
**Output:**
- `tests/e2e/` (Playwright or Cypress tests)

**Acceptance Criteria:**
- Test: onboarding flow (audio setup, model download, test recording)
- Test: start recording → stop → view meeting detail
- Test: search meeting library
- Test: change settings
- Tests run in CI (headless mode)

**Dependencies:** T3.1-T3.7 (desktop shell)  
**Execution Type:** AI-Agent  
**Priority:** Medium  
**Effort:** M (8-10 hours)

---

### T6.4: Performance Tests

**ID:** REC-TEST-004  
**Title:** Write performance and load tests  
**Description:** Write performance tests for transcription latency, search speed, sync throughput, and team server concurrency.  
**Inputs:** All module implementations  
**Output:**
- `tests/performance/` (benchmark scripts)

**Acceptance Criteria:**
- Transcription latency: ≤2 seconds (streaming), ≤10 min for 60-min meeting (batch)
- Search speed: ≤1 second across 10,000 hours of meetings
- Team server: ≥200 concurrent users
- Sync throughput: ≥10 meetings/minute

**Dependencies:** T6.1, T6.2 (unit + integration tests)  
**Execution Type:** Developer  
**Priority:** Medium  
**Effort:** M (6-8 hours)

---

### T7.1: Packaging

**ID:** REC-REL-001  
**Title:** Package desktop app for distribution  
**Description:** Package Recap for macOS (.dmg), Windows (.msi), and Linux (.AppImage). Configure code signing (macOS, Windows) and auto-update.  
**Inputs:** Desktop app implementation  
**Output:**
- `recap-macos.dmg` (signed)
- `recap-windows.msi` (signed)
- `recap-linux.AppImage`
- `tauri.conf.json` (packaging config)

**Acceptance Criteria:**
- macOS: .dmg with code signing, notarization
- Windows: .msi with code signing
- Linux: .AppImage
- Auto-update configured (Tauri updater)
- Binary size: ~10MB (Tauri advantage)

**Dependencies:** T3.1-T3.7 (desktop shell), T6.3 (E2E tests pass)  
**Execution Type:** Developer (platform expertise)  
**Priority:** Critical  
**Effort:** L (12-16 hours)

---

### T7.2: Documentation

**ID:** REC-REL-002  
**Title:** Write user and admin documentation  
**Description:** Write user guide (installation, onboarding, recording, search, settings), admin guide (team server deployment, RBAC, audit), and API docs.  
**Inputs:** All module implementations  
**Output:**
- `docs/user-guide.md`
- `docs/admin-guide.md`
- `docs/api.md` (team server API)

**Acceptance Criteria:**
- User guide: installation, onboarding, recording, search, settings, sync
- Admin guide: team server deployment, RBAC, audit logs, policies
- API docs: all team server endpoints (OpenAPI spec)
- Documentation hosted on GitHub Pages or similar

**Dependencies:** All implementations complete  
**Execution Type:** AI-Agent  
**Priority:** High  
**Effort:** M (8-10 hours)

---

### T1.6: Audio Input Manager (NEW)

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

### T1.7: Meeting Metadata Entry System (NEW)

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

### T2.7: STT Provider Router (NEW)

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

### T2.8-T2.13: STT Provider Integrations (NEW)

**Tasks:**
- T2.8: OpenAI STT integration (Whisper API client)
- T2.9: AssemblyAI integration
- T2.10: Deepgram integration
- T2.11: AWS Transcribe integration
- T2.12: Azure Speech integration
- T2.13: Google Speech-to-Text integration

**Description:** Implement client libraries for each STT provider. Each provider implements the `STTProvider` trait defined in T2.7.  
**Inputs:** TSD Section 2.2 (Transcription Module), provider API documentation  
**Output:**
- `src-tauri/src/transcription/providers/openai_stt.rs`
- `src-tauri/src/transcription/providers/assemblyai.rs`
- `src-tauri/src/transcription/providers/deepgram.rs`
- `src-tauri/src/transcription/providers/aws_transcribe.rs`
- `src-tauri/src/transcription/providers/azure_speech.rs`
- `src-tauri/src/transcription/providers/google_speech.rs`

**Acceptance Criteria (per provider):**
- Authenticates with provider API using user-provided credentials
- Uploads audio file, receives transcript with timestamps
- Supports language selection
- Handles API errors gracefully (rate limits, network failures)
- Displays cost estimate before transcription
- Normalizes output to unified `Transcript` format

**Dependencies:** T2.7 (STT router)  
**Execution Type:** AI-Agent (can parallelize across providers)  
**Priority:** High  
**Effort:** S (4-6 hours per provider, total ~24-36 hours)

---

### T2.14: LLM Provider Router (NEW)

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

### T2.15-T2.19: LLM Provider Integrations (NEW)

**Tasks:**
- T2.15: OpenAI LLM integration (GPT-4 client)
- T2.16: Anthropic Claude integration
- T2.17: Google Gemini integration
- T2.18: AWS Bedrock integration
- T2.19: Azure OpenAI integration

**Description:** Implement client libraries for each LLM provider. Each provider implements the `LLMProvider` trait defined in T2.14.  
**Inputs:** TSD Section 2.3 (Summarization Module), provider API documentation  
**Output:**
- `src-tauri/src/summarization/providers/openai_llm.rs`
- `src-tauri/src/summarization/providers/anthropic.rs`
- `src-tauri/src/summarization/providers/google_gemini.rs`
- `src-tauri/src/summarization/providers/aws_bedrock.rs`
- `src-tauri/src/summarization/providers/azure_openai.rs`

**Acceptance Criteria (per provider):**
- Authenticates with provider API using user-provided credentials
- Sends prompt, receives completion
- Supports structured output (JSON Schema validation)
- Handles API errors gracefully (rate limits, network failures)
- Displays cost estimate before summarization

**Dependencies:** T2.14 (LLM router)  
**Execution Type:** AI-Agent (can parallelize across providers)  
**Priority:** High  
**Effort:** M (6-8 hours per provider, total ~30-40 hours)

---

### T2.20: Prompt Manager (NEW)

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

### T2.21: Prompt Template Library (NEW)

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

### T3.8: Import Dialog (NEW)

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

### T3.9: Metadata Entry Dialog (NEW)

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

### T3.10: Provider Settings Panel (NEW)

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

### T3.11: Prompt Editor Page (NEW)

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

### T5.6: Mobile Companion App (NEW)

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

### T5.7: iOS Shortcuts Integration (NEW)

**ID:** REC-MOB-002  
**Title:** Create iOS Shortcut for audio upload  
**Description:** Build an iOS Shortcut that records audio and uploads to Recap desktop HTTP server.  
**Inputs:** TSD Section 2.1 (Audio Input Manager)  
**Output:**
- `shortcuts/recap-upload.shortcut` (iOS Shortcut file)
- Documentation for installation

**Acceptance Criteria:**
- Shortcut records audio using iPhone microphone
- Prompts for desktop server IP (or uses saved value)
- Uploads audio via HTTP POST to desktop server
- Shows success/error notification
- Shortcut shareable via iCloud link

**Dependencies:** T1.6 (HTTP upload server)  
**Execution Type:** Developer (iOS Shortcuts expertise)  
**Priority:** P2 (Medium)  
**Effort:** S (4-6 hours)

---

### T5.8: LocalSend Protocol Support (NEW)

**ID:** REC-MOB-003  
**Title:** Implement LocalSend protocol for zero-config phone→PC transfer  
**Description:** Integrate `localsend-rs` library to support LocalSend protocol for automatic device discovery and file transfer.  
**Inputs:** TSD Section 2.1 (Audio Input Manager)  
**Output:**
- `src-tauri/src/audio_input/localsend_bridge.rs`
- Configuration for LocalSend service name

**Acceptance Criteria:**
- Recap advertises itself on LAN via LocalSend protocol
- User can share audio file from phone's LocalSend app to Recap
- File received and saved to inbox folder
- Zero-config (no IP address entry required)

**Dependencies:** T1.6 (audio input manager)  
**Execution Type:** Developer (networking expertise)  
**Priority:** P2 (Optional)  
**Effort:** M (8-10 hours)

---

## 4. Dependency Graph

### 4.1 Blocking Dependencies

**Critical Path:**
```
T1.1 → T1.2 → T2.1 → T2.2 → T3.3 → T3.1 → T7.1
                  ↓
              T2.3 (diarization)
                  ↓
              T2.4 → T2.5 → T2.6
                  ↓
              T1.3 → T3.4 → T3.5
```

**Team Server Path:**
```
T4.1 → T4.2 → T4.3 → T4.6
         ↓
      T4.4 (audit)
         ↓
      T4.5 (policies)
         ↓
      T5.5 (proxy)
```

**Sync Path:**
```
T1.3 + T4.5 → T5.1 → T5.2, T5.3 → T5.4
```

### 4.2 Parallelizable Groups

**Group 1 (Foundation):**
- T1.1 (scaffolding)
- T4.1 (team server)

**Group 2 (Core Processing):**
- T1.2 (audio capture)
- T1.3 (storage)
- T1.4 (model manager)

**Group 3 (UI):**
- T3.1 (app shell)
- T3.2 (onboarding)
- T3.6 (settings)
- T3.7 (tray)

**Group 4 (Testing):**
- T6.1 (unit tests)
- T6.2 (integration tests)

### 4.3 No Circular Dependencies

Verified: All dependencies flow forward. No cycles detected.

---

## 5. Execution Phases

### Phase 1: Foundation & Infrastructure (Days 1-5)

**Tasks:**
- T1.1: Project scaffolding (Day 1)
- T1.2: Audio capture (Days 2-3)
- T1.3: Storage (Days 2-3)
- T1.4: Model manager (Day 4)
- T1.5: Configuration (Day 5)
- T4.1: Team server scaffolding (Days 1-2)

**Parallelization:** T1.1 + T4.1 in parallel, then T1.2 + T1.3 + T1.4 in parallel.

**Verification Gate:**
- Tauri app launches
- Audio capture works on all platforms
- Storage encryption/decryption works
- Model download works
- Team server health endpoint responds

---

### Phase 2: Core Processing Pipeline (Days 6-12)

**Tasks:**
- T2.1: Batch transcription (Days 6-7)
- T2.2: Streaming transcription (Days 8-9)
- T2.3: Diarization (Days 8-9)
- T2.4: Rule-based summarization (Days 10-11)
- T2.5: LLM summarization (Days 10-11)
- T2.6: Output formatting (Day 12)

**Parallelization:** T2.1 first, then T2.2 + T2.3 in parallel, then T2.4 + T2.5 in parallel.

**Verification Gate:**
- Audio file → transcript with timestamps
- Real-time transcription latency ≤2 seconds
- Speaker diarization accuracy ≥85%
- Summary, action items, decisions extracted
- Output formatted correctly

---

### Phase 3: Desktop Shell & UI (Days 13-19)

**Tasks:**
- T3.1: App shell (Day 13)
- T3.2: Onboarding (Days 14-15)
- T3.3: Recording UI (Days 14-15)
- T3.4: Meeting library (Days 16-17)
- T3.5: Meeting detail (Days 16-17)
- T3.6: Settings (Days 18-19)
- T3.7: System tray (Days 18-19)

**Parallelization:** T3.1 first, then T3.2 + T3.3 in parallel, then T3.4 + T3.5 in parallel, then T3.6 + T3.7 in parallel.

**Verification Gate:**
- Onboarding flow completes
- Recording UI shows real-time transcript
- Meeting library search works
- Meeting detail displays all data
- Settings persist across restarts

---

### Phase 4: Team Server (Days 13-19, parallel with Phase 3)

**Tasks:**
- T4.2: Authentication (Days 13-14)
- T4.3: RBAC (Day 15)
- T4.4: Audit logging (Days 16-17)
- T4.5: Policy manager (Days 16-17)
- T4.6: Admin dashboard (Days 18-19)

**Parallelization:** T4.2 first, then T4.3, then T4.4 + T4.5 in parallel, then T4.6.

**Verification Gate:**
- JWT auth works (login, refresh)
- RBAC enforced on all endpoints
- Audit log entries created
- Policies CRUD works
- Admin metrics displayed

---

### Phase 5: Integrations & Sync (Days 20-24)

**Tasks:**
- T5.1: Sync engine (Days 20-21)
- T5.2: Vault sync (Day 22)
- T5.3: Loop sync (Day 22)
- T5.4: Retry queue (Days 23-24)
- T5.5: Team server proxy (Days 23-24)

**Parallelization:** T5.1 first, then T5.2 + T5.3 in parallel, then T5.4 + T5.5 in parallel.

**Verification Gate:**
- Sync rules evaluated correctly
- Meetings synced to Vault (mock)
- Action items synced to Loop (mock)
- Retry queue handles failures
- Team server proxy forwards correctly

---

### Phase 6: Testing & Hardening (Days 25-30)

**Tasks:**
- T6.1: Unit tests (Days 25-27)
- T6.2: Integration tests (Days 28-29)
- T6.3: E2E tests (Day 30)
- T6.4: Performance tests (Day 30)

**Parallelization:** T6.1 first, then T6.2 + T6.3 in parallel, then T6.4.

**Verification Gate:**
- ≥80% code coverage
- All integration tests pass
- E2E tests pass
- Performance targets met

---

### Phase 7: Deployment & Release (Days 31-33)

**Tasks:**
- T7.1: Packaging (Days 31-32)
- T7.2: Documentation (Days 31-33)

**Parallelization:** T7.1 + T7.2 in parallel.

**Verification Gate:**
- macOS, Windows, Linux packages built
- Code signing works
- Documentation complete
- Release notes written

---

## 6. AI-Agent Optimization Layer

| Task ID | Single-Shot / Iterative | Context Window | Risk Level |
|---------|-------------------------|----------------|------------|
| T1.1 | Single-shot | Minimal | Low |
| T1.2 | Iterative | Medium | High (platform-specific) |
| T1.3 | Iterative | Medium | Medium |
| T1.4 | Single-shot | Minimal | Low |
| T1.5 | Single-shot | Minimal | Low |
| T2.1 | Iterative | Large | High (Rust + C++ FFI) |
| T2.2 | Iterative | Large | High (real-time) |
| T2.3 | Iterative | Medium | High (ML/audio) |
| T2.4 | Single-shot | Medium | Low |
| T2.5 | Iterative | Large | High (LLM FFI) |
| T2.6 | Single-shot | Minimal | Low |
| T3.1 | Iterative | Medium | Medium |
| T3.2 | Single-shot | Medium | Low |
| T3.3 | Iterative | Medium | Medium |
| T3.4 | Single-shot | Medium | Low |
| T3.5 | Single-shot | Medium | Low |
| T3.6 | Single-shot | Medium | Low |
| T3.7 | Single-shot | Minimal | Low |
| T4.1 | Single-shot | Minimal | Low |
| T4.2 | Iterative | Medium | High (auth) |
| T4.3 | Single-shot | Minimal | Medium |
| T4.4 | Single-shot | Medium | Medium |
| T4.5 | Single-shot | Minimal | Low |
| T4.6 | Single-shot | Minimal | Low |
| T5.1 | Iterative | Medium | Medium |
| T5.2 | Single-shot | Medium | Medium |
| T5.3 | Single-shot | Medium | Medium |
| T5.4 | Iterative | Medium | Medium |
| T5.5 | Single-shot | Minimal | Low |
| T6.1 | Iterative | Large | Low |
| T6.2 | Iterative | Large | Medium |
| T6.3 | Iterative | Large | Medium |
| T6.4 | Iterative | Medium | Medium |
| T7.1 | Iterative | Medium | High (platform) |
| T7.2 | Single-shot | Large | Low |

---

## 7. File-Level Mapping

### Rust Backend (`src-tauri/src/`)

```
src-tauri/src/
├── main.rs                          (T3.1)
├── audio_capture/
│   ├── mod.rs                       (T1.2)
│   ├── platform_adapter.rs          (T1.2)
│   ├── coreaudio.rs                 (T1.2)
│   ├── wasapi.rs                    (T1.2)
│   ├── pulseaudio.rs                (T1.2)
│   ├── mixer.rs                     (T1.2)
│   └── buffered_writer.rs           (T1.2)
├── transcription/
│   ├── mod.rs                       (T2.1)
│   ├── backend.rs                   (T2.1)
│   ├── whisper_cpp.rs               (T2.1)
│   ├── types.rs                     (T2.1)
│   ├── streaming_buffer.rs          (T2.2)
│   ├── streaming_transcriber.rs     (T2.2)
│   ├── diarization.rs               (T2.3)
│   └── diarization_engine.rs        (T2.3)
├── summarization/
│   ├── mod.rs                       (T2.4)
│   ├── rule_based.rs                (T2.4)
│   ├── action_extractor.rs          (T2.4)
│   ├── decision_extractor.rs        (T2.4)
│   ├── topic_extractor.rs           (T2.4)
│   ├── llm_summarizer.rs            (T2.5)
│   ├── prompt_templates.rs          (T2.5)
│   └── output_formatter.rs          (T2.6)
├── storage/
│   ├── mod.rs                       (T1.3)
│   ├── sqlite_manager.rs            (T1.3)
│   ├── encryption_manager.rs        (T1.3)
│   ├── file_store.rs                (T1.3)
│   ├── export_engine.rs             (T1.3)
│   └── retention_enforcer.rs        (T1.3)
├── sync/
│   ├── mod.rs                       (T5.1)
│   ├── rule_evaluator.rs            (T5.1)
│   ├── orchestrator.rs              (T5.1)
│   ├── vault_client.rs              (T5.2)
│   ├── loop_client.rs               (T5.3)
│   ├── retry_queue.rs               (T5.4)
│   └── circuit_breaker.rs           (T5.4)
├── model_manager/
│   ├── mod.rs                       (T1.4)
│   ├── downloader.rs                (T1.4)
│   ├── verifier.rs                  (T1.4)
│   ├── registry.rs                  (T1.4)
│   └── airgap_loader.rs             (T1.4)
├── config/
│   ├── mod.rs                       (T1.5)
│   ├── settings.rs                  (T1.5)
│   └── persistence.rs               (T1.5)
├── hotkey_manager.rs                (T3.1)
└── tray_manager.rs                  (T3.1, T3.7)
```

### Frontend (`src/`)

```
src/
├── main.tsx (or App.svelte)         (T3.1)
├── pages/
│   ├── Onboarding.tsx               (T3.2)
│   ├── Recording.tsx                (T3.3)
│   ├── MeetingLibrary.tsx           (T3.4)
│   ├── MeetingDetail.tsx            (T3.5)
│   └── Settings.tsx                 (T3.6)
├── components/
│   ├── AudioSetup.tsx               (T3.2)
│   ├── ModelSelection.tsx           (T3.2)
│   ├── TestRecording.tsx            (T3.2)
│   ├── TranscriptDisplay.tsx        (T3.3)
│   ├── RecordingControls.tsx        (T3.3)
│   ├── MeetingCard.tsx              (T3.4)
│   ├── SearchBar.tsx                (T3.4)
│   ├── FilterPanel.tsx              (T3.4)
│   ├── TranscriptView.tsx           (T3.5)
│   ├── SummaryView.tsx              (T3.5)
│   ├── ActionItemsList.tsx          (T3.5)
│   ├── ExportMenu.tsx               (T3.5)
│   ├── AudioSettings.tsx            (T3.6)
│   ├── ModelSettings.tsx            (T3.6)
│   ├── SyncSettings.tsx             (T3.6)
│   └── PrivacySettings.tsx          (T3.6)
```

### Team Server (`team-server/`)

```
team-server/
├── main.go                          (T4.1)
├── docker-compose.yml               (T4.1)
├── internal/
│   ├── handler/                     (T4.1)
│   ├── db/                          (T4.1)
│   ├── cache/                       (T4.1)
│   ├── auth/
│   │   ├── jwt.go                   (T4.2)
│   │   ├── handler.go               (T4.2)
│   │   ├── saml.go                  (T4.2)
│   │   ├── oidc.go                  (T4.2)
│   │   └── rbac.go                  (T4.3)
│   ├── middleware/
│   │   └── rbac_middleware.go       (T4.3)
│   ├── audit/
│   │   ├── logger.go                (T4.4)
│   │   └── handler.go               (T4.4)
│   ├── policy/
│   │   ├── manager.go               (T4.5)
│   │   └── handler.go               (T4.5)
│   ├── admin/
│   │   ├── handler.go               (T4.6)
│   │   └── metrics.go               (T4.6)
│   └── proxy/
│       ├── vault_proxy.go           (T5.5)
│       └── loop_proxy.go            (T5.5)
├── migrations/
│   ├── audit_table.sql              (T4.4)
│   └── policy_tables.sql            (T4.5)
```

---

## 8. Test Task Mapping

### Unit Tests

| Module | Test File | Coverage Target |
|--------|-----------|-----------------|
| Audio Capture | `audio_capture/*_test.rs` | 80% |
| Transcription | `transcription/*_test.rs` | 80% |
| Summarization | `summarization/*_test.rs` | 80% |
| Storage | `storage/*_test.rs` | 80% |
| Sync | `sync/*_test.rs` | 80% |
| Model Manager | `model_manager/*_test.rs` | 80% |
| Config | `config/*_test.rs` | 80% |
| Team Server Auth | `auth/*_test.go` | 80% |
| Team Server Audit | `audit/*_test.go` | 80% |
| Team Server Policy | `policy/*_test.go` | 80% |

### Integration Tests

| Workflow | Test File | Description |
|----------|-----------|-------------|
| Audio → Transcript → Summary | `tests/integration/pipeline_test.rs` | End-to-end processing |
| Vault Sync | `tests/integration/vault_sync_test.rs` | Meeting → Vault (mock) |
| Loop Sync | `tests/integration/loop_sync_test.rs` | Action items → Loop (mock) |
| Team Server Auth | `team-server/tests/integration/auth_test.go` | Login → JWT → RBAC |
| Team Server Audit | `team-server/tests/integration/audit_test.go` | Audit log creation |

### E2E Tests

| Workflow | Test File | Description |
|----------|-----------|-------------|
| Onboarding | `tests/e2e/onboarding.spec.ts` | Audio setup, model download, test recording |
| Recording | `tests/e2e/recording.spec.ts` | Start → stop → view transcript |
| Meeting Library | `tests/e2e/library.spec.ts` | Search, filter, sort |
| Settings | `tests/e2e/settings.spec.ts` | Change settings, verify persistence |

---

## 9. Definition of Done (Global)

### Code Quality
- [ ] All modules implemented per TSD specifications
- [ ] ≥80% unit test coverage
- [ ] All integration tests pass
- [ ] All E2E tests pass
- [ ] No critical security vulnerabilities
- [ ] Code reviewed (if team)

### Functionality
- [ ] Audio capture works on macOS, Windows, Linux
- [ ] Transcription latency ≤2 seconds (streaming)
- [ ] Batch processing ≤10 minutes for 60-minute meeting
- [ ] Speaker diarization accuracy ≥85%
- [ ] Summary, action items, decisions extracted correctly
- [ ] Storage encryption/decryption works
- [ ] Full-text search ≤1 second across 10,000 hours
- [ ] Sync to Vault/Loop works (with retry)
- [ ] Team server auth, RBAC, audit work
- [ ] Desktop app UI complete (onboarding, recording, library, settings)

### Infrastructure
- [ ] Docker Compose for team server deployment
- [ ] macOS, Windows, Linux packages built
- [ ] Code signing configured (macOS, Windows)
- [ ] Auto-update configured

### Testing
- [ ] Unit tests pass (`cargo test`, `go test`)
- [ ] Integration tests pass
- [ ] E2E tests pass
- [ ] Performance targets met

### Documentation
- [ ] User guide complete
- [ ] Admin guide complete
- [ ] API documentation complete
- [ ] Release notes written

### Deployment
- [ ] Packages uploaded to release platform (GitHub Releases, etc.)
- [ ] Documentation hosted (GitHub Pages, etc.)
- [ ] Support channel established

---

## 10. Risk & Bottleneck Identification

### High-Risk Tasks

| Task | Risk | Mitigation |
|------|------|------------|
| T1.2 (Audio Capture) | Platform-specific audio APIs are complex and fragile | Use established libraries (cpal, rodio), test on all platforms early |
| T2.1 (Transcription) | whisper.cpp FFI in Rust is non-trivial | Start with simple bindings, iterate; consider pre-built crates |
| T2.2 (Streaming) | Real-time processing with ≤2s latency is challenging | Optimize buffer sizes, use async processing, profile early |
| T2.3 (Diarization) | Speaker diarization accuracy may be <85% | Test multiple models (pyannote, neMo), tune thresholds |
| T2.5 (LLM Summarization) | llama.cpp FFI is complex, model loading slow | Consider Ollama as alternative, lazy-load models |
| T7.1 (Packaging) | Code signing and notarization are platform-specific | Use Tauri's built-in signing, test on all platforms |

### External Dependencies

| Dependency | Risk | Mitigation |
|------------|------|------------|
| whisper.cpp | Upstream changes may break bindings | Pin to specific version, maintain fork if needed |
| pyannote-audio / neMo | Model availability, licensing | Test both, have fallback |
| Vault API | API changes may break sync | Version API, maintain compatibility layer |
| Loop API | API changes may break sync | Version API, maintain compatibility layer |

### Performance Bottlenecks

| Bottleneck | Impact | Mitigation |
|------------|--------|------------|
| Transcription latency | User experience | Optimize buffer sizes, use GPU acceleration (Metal/CUDA) |
| Search speed | Large corpora | Use FTS5 efficiently, partition database |
| Sync throughput | Large backlogs | Batch sync, parallel processing |
| Team server concurrency | Scale | Use connection pooling, optimize queries |

---

## 11. Output Requirements

### Deliverables

- [ ] Desktop app (macOS, Windows, Linux)
- [ ] Team server (Docker image)
- [ ] User guide
- [ ] Admin guide
- [ ] API documentation
- [ ] Source code (GitHub)
- [ ] Test suite (unit, integration, E2E)
- [ ] Performance benchmarks

### Quality Gates

- [ ] All unit tests pass (≥80% coverage)
- [ ] All integration tests pass
- [ ] All E2E tests pass
- [ ] Performance targets met
- [ ] Security review complete
- [ ] Documentation review complete

### Performance Targets

- Transcription latency: ≤2 seconds (streaming)
- Batch processing: ≤10 minutes for 60-minute meeting
- Search speed: ≤1 second across 10,000 hours
- Team server: ≥200 concurrent users
- Sync throughput: ≥10 meetings/minute

### Security Requirements

- AES-256-GCM encryption at rest
- Argon2id key derivation
- JWT authentication (RS256)
- RBAC enforcement
- Tamper-evident audit logs
- No data leaves user's device (by design)

---

**End of Task Breakdown Document**
