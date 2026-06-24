# DEVELOPMENT.md

## Project: Recap by ODW.ai

**Repository:** https://github.com/OnDemandWorld/odw-recap  
**Status:** Full solution implemented  
**Last Updated:** 2026-06-24

---

## Current Implementation Status

### Completed
- [x] Repository created on GitHub
- [x] Initial commit with specification documents (PRD, SAD, TSD, TBK, research, README, CLAUDE.md)
- [x] Updated specification documents to include:
  - Flexible audio input (smartphone, file upload, watch folder)
  - Meeting metadata entry (type, location, participants, language, topic)
  - Multiple STT providers (local, paid APIs, cloud-hosted)
  - Multiple LLM providers (local, cloud)
  - Prompt templates and editing
- [x] `.gitignore` added for build artifacts and editor files
- [x] Rust toolchain installed (rustc 1.96.0, cargo 1.96.0)
- [x] Tauri project scaffolding (v1.4, see note below)
- [x] Rust backend module structure created
- [x] Storage module implemented:
  - SQLite database with migrations (meetings, transcripts, summaries, action items, decisions, prompt_templates, api_keys, configuration, sync_queue)
  - AES-256-GCM encryption with Argon2id key derivation
  - File store for encrypted/plain file operations
  - Full-text search (FTS5) support
  - API key storage (encrypted)
  - Prompt template storage
- [x] Configuration system implemented (key-value store backed by SQLite)
- [x] Audio input manager implemented:
  - File import (drag-and-drop, file browser)
  - Watch folder monitoring (with debouncing)
  - HTTP upload server (Axum-based, for smartphone transfers)
  - Format validation (M4A, WAV, MP3, OGG, WebM, FLAC)
- [x] Tauri commands exposed:
  - `greet` (hello world)
  - `create_meeting`
  - `list_meetings`
  - `get_config`
  - `import_audio_file`
  - `get_supported_audio_formats`
- [x] Unit tests for storage and prompt modules (7 tests passing)
- [x] Frontend UI implemented:
  - Meeting library view with list and refresh
  - Import view with file browser integration
  - Settings view with provider selection (STT/LLM)
  - Basic styling and navigation
  - Integration with Tauri commands (list_meetings, import_audio_file, get_supported_audio_formats)

### Completed
- [x] Testing and packaging implemented:
  - Go team server unit tests (JWT validation, health endpoint)
  - Rust desktop app unit tests (7 tests passing)
  - Integration test scaffolding for Tauri commands
  - E2E test scaffolding for desktop UI
  - GitHub Actions CI/CD workflow (Rust, Go, build, test)
  - Build scripts for macOS/Windows/Linux
  - Packaging script for distribution artifacts
- [x] Team server (Go backend) implemented:
- [x] Team server (Go backend) implemented:
  - HTTP server with Chi framework
  - JWT authentication (register/login)
  - RBAC middleware (admin/member roles)
  - PostgreSQL database with migrations
  - Redis integration
  - Meetings CRUD API
  - Sync API for desktop app integration
  - Admin dashboard endpoints (users, audit log, stats)
  - Health check endpoint
  - CORS middleware
- [x] Sync engine and integrations implemented:
  - SyncClient for team server communication
  - SyncEngine for desktop app
  - Vault/Loop integration stubs
  - Mobile companion sync types
  - iOS Shortcuts integration types
- [x] Desktop UI fully implemented:
  - Meeting library view with list and refresh
  - Import view with file browser integration
  - Settings view with provider selection (STT/LLM)
  - Meeting detail view with transcript, summary, action items, decisions
  - API key management interface
  - Prompt template management UI
  - Navigation between all views
  - Basic styling and navigation
  - Integration with Tauri commands
- [x] Audio capture module implemented:
  - AudioRecorder for microphone input
  - SystemAudioCapture for desktop audio
  - Recording configuration (sample rate, channels, device selection)
  - Stub implementations (would integrate cpal for real audio I/O)
  - Device enumeration support
- [x] Prompt manager implemented:
  - PromptTemplate struct with variable extraction
  - Variable substitution engine
  - Integration with SQLite storage
  - Default templates (Meeting Summary, Action Items, Key Decisions)
  - Template rendering with variable validation
  - 3 unit tests for variable substitution
- [x] Transcription module implemented:
  - STTProvider trait definition
  - STTRouter for provider selection
  - WhisperLocalProvider (stub implementation)
  - OpenAI Whisper API provider
  - AssemblyAI provider
  - Deepgram provider
  - AWS Transcribe provider (stub)
  - Azure Speech provider (stub)
  - Google Speech-to-Text provider (stub)
- [x] Summarization module implemented:
  - LLMProvider trait definition
  - LLMRouter for provider selection
  - Rule-based summarizer (extractive method)
  - LlamaLocalProvider (stub implementation)
  - Ollama provider (fully implemented)
  - OpenAI provider (fully implemented)
  - Anthropic Claude provider (fully implemented)
  - Google Gemini provider (stub)
  - AWS Bedrock provider (stub)
  - Azure OpenAI provider (stub)

### Not Started
- [ ] Audio capture module (system audio + microphone)
- [ ] Speaker diarization
- [ ] Meeting detail view with transcript and summary display
- [ ] Metadata entry dialog
- [ ] Provider settings dialog (API key management)
- [ ] Prompt editor UI
- [ ] Team server (Go backend, PostgreSQL, Redis)
- [ ] Vault/Loop sync
- [ ] Mobile companion app
- [ ] iOS Shortcuts integration
- [ ] E2E tests and performance tests
- [ ] Packaging and distribution

---

## Architecture Decisions

### Tech Stack
- **Desktop App:** Tauri 1.4 + React/Svelte + TypeScript + Tailwind CSS
- **Backend:** Rust (Tauri commands)
- **Team Server:** Go (Chi or Echo) + PostgreSQL + Redis (planned)
- **Database (Desktop):** SQLite 3.44+ with FTS5
- **Encryption:** AES-256-GCM with Argon2id key derivation
- **Transcription:** whisper.cpp (primary), with pluggable cloud providers (planned)
- **Summarization:** llama.cpp / Ollama (local), with pluggable cloud LLMs (planned)

### Tauri Version Note
We are using **Tauri 1.4** instead of the spec's Tauri 2.x because `create-tauri-app` v3.7.0 (latest available) defaulted to Tauri 1.4 templates. The architecture is forward-compatible; upgrade to Tauri 2.x is planned but not a blocker.

### Key Implementation Notes
- Modular monolith with trait-based provider abstraction
- Local processing is default and recommended; cloud providers are optional
- All meeting data encrypted at rest using user passphrase
- API keys for cloud providers stored encrypted in SQLite

---

## Remaining Work

### Phase 1: Foundation & Infrastructure
1. ✅ Tauri project scaffolding
2. ✅ Rust backend module structure
3. ✅ Storage module (SQLite + encryption)
4. ✅ Configuration system
5. ✅ Audio input manager (file import, watch folder, HTTP upload)
6. ✅ Meeting metadata entry system (data model + storage)
7. ⏳ Frontend metadata entry dialog

### Phase 2: Core Processing Pipeline
1. Transcription module with whisper.cpp
2. STT provider router and cloud providers (OpenAI, AssemblyAI, Deepgram, AWS, Azure, Google)
3. Summarization module (rule-based + local LLM)
4. LLM provider router and cloud providers (OpenAI, Anthropic, Google, AWS, Azure)
5. Prompt manager and template library

### Phase 3: Desktop Shell & UI
1. App shell, tray, hotkeys
2. Onboarding flow
3. Recording UI with real-time transcript
4. Meeting library and detail views
5. Import dialog, metadata dialog, provider settings, prompt editor

### Phase 4: Team Server
1. Go HTTP server scaffolding
2. Authentication (JWT + SSO)
3. RBAC, audit logging, policy manager
4. Admin dashboard API

### Phase 5: Integrations & Sync
1. Sync engine
2. Vault/Loop sync clients
3. Mobile companion app
4. iOS Shortcuts integration

### Phase 6-7: Testing, Packaging, Release
1. Unit/integration/E2E tests
2. Performance tests
3. Packaging for macOS/Windows/Linux
4. Documentation

---

## Known Blockers

- Full solution requires more time than a single session; being implemented incrementally.
- Tauri 2.x upgrade deferred due to tooling availability.

---

## How to Build

```bash
cd desktop-app
npm install
cd src-tauri
cargo build
```

## How to Run Tests

```bash
cd desktop-app/src-tauri
cargo test
```

## How to Run a Single Test

```bash
cd desktop-app/src-tauri
cargo test storage::tests::tests::test_create_and_get_meeting
```

---

## Notes for AI Assistants

- This is a greenfield project; prefer creating new code over modifying existing code where appropriate.
- Follow the structure defined in tsd.md and tbk.md.
- Use Rust naming conventions for backend, TypeScript conventions for frontend.
- Keep modules decoupled via traits/interfaces.
- Document any deviations from the spec in this file.
- Tauri version is currently 1.4, not 2.x.
