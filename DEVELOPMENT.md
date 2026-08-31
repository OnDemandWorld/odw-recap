# DEVELOPMENT.md

## Project: Recap by ODW.ai

**Repository:** https://github.com/OnDemandWorld/odw-recap  
**Status:** Full review & refactor completed — end-to-end local pipeline wired (import → transcribe → summarize)  
**Last Updated:** 2026-09-01

> **See `IMPROVEMENT_PLAN.md` for the full list of fixes made in the 2026-09
> review and the prioritized roadmap (P0–P3) of what to build next.**

---

## Current Implementation Status

### ✅ Completed

#### Foundation & Infrastructure
- [x] Repository created on GitHub and pushed to remote
- [x] Tauri 1.4 desktop app scaffolding
- [x] Rust backend module structure (`audio_input`, `audio_capture`, `storage`, `config`, `transcription`, `summarization`, `prompt_manager`, `sync`)
- [x] Storage module: SQLite with migrations, AES-256-GCM encryption, Argon2id key derivation, file store, FTS5
- [x] Configuration system backed by SQLite
- [x] `.gitignore` for build artifacts and editor files

#### Audio Input & Capture
- [x] File import (drag-and-drop, file browser)
- [x] Watch folder monitoring with debouncing
- [x] HTTP upload server (Axum) for smartphone transfers
- [x] Audio format validation (M4A, WAV, MP3, OGG, WebM, FLAC)
- [x] Microphone recording module
- [x] System audio capture module (with platform-specific stubs)

#### Transcription (7 Providers)
- [x] `STTProvider` trait and `STTRouter`
- [x] Whisper.cpp local provider (stub)
- [x] OpenAI Whisper API provider (full implementation)
- [x] Deepgram provider (full implementation)
- [x] AssemblyAI provider (stub)
- [x] AWS Transcribe provider (stub)
- [x] Azure Speech provider (stub)
- [x] Google Speech-to-Text provider (stub)

#### Summarization (7 Providers + Rule-Based)
- [x] `LLMProvider` trait and `LLMRouter`
- [x] Rule-based extractive summarizer
- [x] Llama.cpp local provider (stub)
- [x] Ollama provider (full implementation)
- [x] OpenAI GPT provider (full implementation)
- [x] Anthropic Claude provider (full implementation)
- [x] Google Gemini provider (stub)
- [x] AWS Bedrock provider (stub)
- [x] Azure OpenAI provider (stub)

#### Prompt Manager
- [x] `PromptTemplate` struct with automatic variable extraction
- [x] `{{variable}}` substitution engine
- [x] Default templates (Meeting Summary, Action Items, Key Decisions)
- [x] Storage integration

#### Desktop UI
- [x] Meeting library view
- [x] Import view with file browser
- [x] Settings view with STT/LLM provider selection and API key management
- [x] Meeting detail view (transcript, summary, action items, decisions)
- [x] Prompt template management UI
- [x] Navigation and responsive styling

#### Team Server (Go Backend)
- [x] Chi HTTP server with middleware (logger, CORS, timeout, recoverer)
- [x] JWT authentication (register/login)
- [x] RBAC middleware (`admin`, `member` roles)
- [x] PostgreSQL database with automatic migrations
- [x] Redis client integration
- [x] Meetings CRUD API
- [x] Sync API for desktop integration
- [x] Admin endpoints (users, audit log, stats)
- [x] Health check endpoint
- [x] `README.md` with setup instructions

#### Sync & Integrations
- [x] Rust `SyncClient` for team server API
- [x] `SyncEngine` for desktop app orchestration
- [x] Vault/Loop sync type stubs
- [x] Mobile companion sync types
- [x] iOS Shortcuts integration types

#### Testing, CI/CD, Packaging
- [x] Rust unit tests (16 passing)
- [x] Go unit tests (9 passing)
- [x] Integration test scaffolding for Tauri commands
- [x] E2E test scaffolding for desktop UI
- [x] GitHub Actions CI workflow (Rust + Go tests, desktop build, team server build; includes Tauri Linux system dependencies)
- [x] Build scripts for macOS/Windows/Linux
- [x] Packaging script for distribution artifacts

#### 2026-09 Review & Refactor (see IMPROVEMENT_PLAN.md for details)
- [x] Provider routers wired end-to-end (`STTRouter`/`LLMRouter::with_all_providers`)
- [x] Tauri commands for the full local loop: `transcribe_meeting`, `summarize_meeting`,
      `get_transcript_segments`, `get_summary`, `search_transcripts`,
      `list_stt_providers`, `list_llm_providers`, prompt template CRUD,
      `set_config`/`save_api_key`/`has_api_key`
- [x] Frontend rewritten on `window.__TAURI__` (no bundler needed); real settings,
      API key storage, meeting details with Transcribe/Summarize actions
- [x] Encryption: per-install persisted random salt + per-install passphrase
      (env override `RECAP_PASSPHRASE`); dangerous `change_passphrase` disabled
      until re-encryption is implemented
- [x] FTS5 transcript search working (sync triggers + rebuild)
- [x] Team server: meeting ownership scoping, sync FK fix, CORS fix, request
      size limits, registration validation, 409 on duplicate email
- [x] Import now creates a meeting record; data dir moved to platform-local dir
- [x] Compiler warnings in project code: 150 → 0; unused dependencies removed

---

## Test Status

| Component | Tests | Status |
|-----------|-------|--------|
| Rust desktop backend | 16 | ✅ Passing |
| Go team server | 9 | ✅ Passing |
| Frontend (jest) | scaffolds excluded (`--passWithNoTests`) | ✅ Exits clean |
| Desktop build | — | ✅ Compiles |
| Team server build | — | ✅ Compiles |

---

## Architecture Decisions

### Tech Stack
- **Desktop App:** Tauri 1.4 + vanilla JS + TypeScript-ready + CSS
- **Backend:** Rust (Tauri commands)
- **Team Server:** Go (Chi) + PostgreSQL + Redis
- **Desktop Database:** SQLite 3.44+ with FTS5
- **Encryption:** AES-256-GCM with Argon2id key derivation
- **Transcription:** whisper.cpp (local), with pluggable cloud providers
- **Summarization:** llama.cpp / Ollama (local), with pluggable cloud LLMs

### Tauri Version Note
We are using **Tauri 1.4** instead of the spec's Tauri 2.x because `create-tauri-app` v3.7.0 defaulted to Tauri 1.4 templates. The architecture is forward-compatible; an upgrade to Tauri 2.x is planned.

### Key Implementation Notes
- Modular monolith with trait-based provider abstraction
- Local processing is default and recommended; cloud providers are optional
- All meeting data encrypted at rest using user passphrase
- API keys for cloud providers stored encrypted in SQLite
- Team server uses JWT auth and RBAC for multi-user scenarios

---

## How to Build

### Desktop App

```bash
cd desktop-app
npm install
cd src-tauri
cargo build
```

### Desktop Tests

```bash
cd desktop-app/src-tauri
cargo test
```

### Team Server

```bash
cd team-server
go build -o recap-team-server ./cmd/server
```

### Team Server Tests

```bash
cd team-server
go test ./...
```

### Package Everything

```bash
./scripts/build-all.sh
./scripts/package.sh
```

---

## Known Blockers & Limitations

1. **Tauri 2.x upgrade deferred** due to scaffolding tooling availability; CSP is `null` until the migration.
2. **Stub providers** (AssemblyAI, AWS Transcribe, Azure Speech, Google STT, Google Gemini, AWS Bedrock, Azure OpenAI, Llama.cpp, Whisper.cpp) require real API/FFI integration before production use. The OpenAI, Anthropic, Deepgram, and Ollama providers are real.
3. **Audio capture** is implemented as a stub structure; real I/O requires `cpal` and platform-specific loopback devices. File import, watch folder, and HTTP upload (localhost-only) work.
4. **Local encryption passphrase** is stored in a 0600 file (or `RECAP_PASSPHRASE` env var) until the real unlock/keychain flow lands (IMPROVEMENT_PLAN.md P0-1). Passphrase *change* is intentionally disabled until re-encryption exists (P0-4).
5. **Team server**: `sync_queue` has no consumer worker yet; no rate limiting; default JWT secret still boots with a warning (all tracked in IMPROVEMENT_PLAN.md P0-2 / P1-1).
6. **Mobile companion app** and **iOS Shortcuts** are represented only as data types and sync contracts; no native mobile code exists yet.
7. **E2E tests** are scaffolding only and require Playwright + built Tauri app to run.

---

## Future Improvement Suggestions

### High Priority
1. **Integrate real STT engines**
   - Replace whisper.cpp stub with `whisper-rs` or `whisper.cpp` FFI binding.
   - Complete AssemblyAI, AWS Transcribe, Azure Speech, and Google STT providers with real REST clients.
2. **Integrate real LLM engines**
   - Replace Llama.cpp stub with `llama-cpp-rs` or Ollama server.
   - Complete Google Gemini, AWS Bedrock, and Azure OpenAI providers.
3. **Audio capture real I/O**
   - Add `cpal` dependency and implement microphone recording.
   - Implement platform-specific system audio loopback (BlackHole on macOS, WASAPI on Windows, PulseAudio on Linux).

### Medium Priority
4. **Tauri 2.x migration**
   - Upgrade templates, IPC, and permissions when tooling is available.
5. **Rich meeting detail UI**
   - Fetch and display real transcript, summary, action items, and decisions.
   - Add inline editing of meeting metadata.
6. **Provider settings persistence**
   - Save selected STT/LLM provider and API keys via secure storage.
7. **Team server hardening**
   - Add refresh tokens, rate limiting, input validation, and audit logging middleware.
   - Add organization management endpoints.
8. **Sync engine implementation**
   - Implement bidirectional sync between desktop app and team server.
   - Add conflict resolution and offline queue.

### Low Priority / Nice-to-Have
9. **Mobile companion app**
   - Build a Tauri 2.0 / Capacitor app for audio upload and meeting review.
10. **iOS Shortcuts integration**
    - Implement a Shortcuts intent or x-callback URL handler.
11. **Performance benchmarks**
    - Add transcription and summarization latency benchmarks.
    - Profile memory usage for long meetings.
12. **Packaging polish**
    - Codesigning for macOS/Windows, auto-updater, release notes automation.
13. **Observability**
    - Structured logging, metrics, health checks, and tracing in team server.

---

## Notes for AI Assistants

- This project is now a **mixed documentation + code repository**; it contains both specification documents and implementation.
- Follow the structure defined in `tsd.md` and `tbk.md`.
- Use Rust naming conventions for backend, TypeScript conventions for frontend, and Go conventions for team server.
- Keep modules decoupled via traits/interfaces.
- When adding a new provider, implement both the trait and the registration in the router.
- Document any deviations from the spec in this file.
- Tauri version is currently 1.4, not 2.x.
- Before starting a major feature, run the test suite and update `DEVELOPMENT.md`.
