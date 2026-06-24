# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working on the Recap by ODW.ai repository.

## Repository Purpose

This is the **product + implementation repository** for Recap by ODW.ai — a sovereign, on-device meeting intelligence system. It contains both product planning/specification documents and the application code.

## Document Structure

| File / Directory | Purpose |
|------------------|---------|
| `README.md` | Product overview and positioning |
| `prd.md` | Product Requirements Document |
| `sad.md` | System Architecture Document |
| `tsd.md` | Technical Specification Document |
| `tbk.md` | Task Breakdown Document |
| `research.md` | Market research and competitive analysis |
| `DEVELOPMENT.md` | Current implementation status, build/test instructions, known blockers, and future roadmap |
| `CLAUDE.md` | This file — guidance for AI assistants |
| `desktop-app/` | Tauri desktop application (Rust backend + webview frontend) |
| `team-server/` | Go backend for multi-user sync, admin, and team features |
| `scripts/` | Build, test, and packaging scripts |
| `.github/workflows/` | CI/CD workflows |

## Product Context

Recap is a privacy-first meeting intelligence tool that:
- Captures and transcribes meetings locally (no cloud bots by default)
- Uses open-source speech models (Whisper-family) for on-device transcription
- Generates structured outputs (summaries, action items, decisions)
- Syncs to ODW.ai suite modules (Vault for knowledge, Loop for workflows)
- Targets regulated industries and privacy-conscious teams

## Tech Stack

- **Desktop App:** Tauri 1.4 (Rust backend + webview frontend)
- **Backend:** Rust (Tauri commands)
- **Desktop Database:** SQLite 3.44+ with FTS5
- **Encryption:** AES-256-GCM with Argon2id key derivation
- **Transcription:** whisper.cpp (primary), with pluggable cloud providers
- **Summarization:** llama.cpp / Ollama (local), with pluggable cloud LLMs
- **Team Server:** Go (Chi) + PostgreSQL + Redis

## Working with the Code

- Documents cross-reference each other (e.g., TSD references SAD components)
- Maintain consistency across documents when updating specifications
- All documents use standard Markdown with tables and structured sections
- Research citations use `[[N]](URL)` format with a Sources section at the end

## Code Conventions

- **Rust:** Follow idiomatic Rust naming and module organization
- **Frontend:** Vanilla JS, TypeScript-ready, standard CSS
- **Go:** Standard Go conventions; handlers live in `team-server/internal/api`
- Keep modules decoupled via traits/interfaces
- When adding a new STT/LLM provider, implement the trait and register it in the router
- Prefer updating existing code and docs over duplicating files

## Important Notes

- Tauri version is currently **1.4**, not 2.x (upgrade deferred)
- Many cloud providers are implemented as stubs and need real API integration
- Audio capture is stubbed and needs `cpal` + platform-specific loopback integration
- Local LLM/STT providers (whisper.cpp, llama.cpp) are stubs and need FFI bindings
- Before starting a major feature, run the test suite and update `DEVELOPMENT.md`
- Use the build and test commands in `DEVELOPMENT.md`
