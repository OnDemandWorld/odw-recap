# ODW Recap

A sovereign, on-device meeting intelligence system that captures, transcribes, and summarizes meetings without sending audio to third-party clouds.

## What is Recap?

ODW Recap turns meetings into structured action items, summaries, and decisions — all on your local machine. Its core promise: meeting recordings and transcripts never leave your device unless you explicitly choose a cloud provider.

No cloud bot silently joins your calls. No transcripts sit on a vendor's servers by default.

## Status

⚠️ **Early release.** ODW [Name] is an early, functional release — core features work, but it is not yet hardened for production. We are refining every module toward a first full public release in **Q3 2026**. Until then, it is best used as a foundation to build on with AI coding agents (see below).

## Key Features

- **On-device transcription**: Uses Whisper-family models locally via whisper.cpp
- **Local AI summarization**: Generates summaries, action items, and decisions with local LLMs
- **Pluggable providers**: Choose from local models or cloud STT/LLM providers (OpenAI, Anthropic, Deepgram, etc.)
- **Flexible audio input**: Record system audio, use your microphone, import files, watch folders, or upload from your smartphone
- **Encrypted storage**: AES-256-GCM encryption with Argon2id key derivation
- **Team server (optional)**: Go backend for multi-user sync, RBAC, audit logging, and admin dashboard
- **Vault/Loop integration**: Designed to feed meeting knowledge into ODW.ai suite modules

## Target Audience

Privacy-conscious teams, regulated industries, and organizations that need sovereign control over their meeting data.

## Architecture

```
┌─────────────────────────────────────┐
│           Desktop App               │
│  Tauri (Rust backend + webview UI)  │
│  - Audio input & capture            │
│  - Storage (SQLite + encryption)    │
│  - Transcription (STT providers)    │
│  - Summarization (LLM providers)    │
│  - Prompt manager                   │
│  - Sync engine                      │
└──────────────┬──────────────────────┘
               │ (optional)
               ▼
┌─────────────────────────────────────┐
│           Team Server               │
│  Go + Chi + PostgreSQL + Redis      │
│  - JWT auth & RBAC                  │
│  - Meeting sync                     │
│  - Admin dashboard API              │
└─────────────────────────────────────┘
```

## Quick Start

### Desktop App

```bash
cd desktop-app
npm install
cd src-tauri
cargo build
```

### Team Server

```bash
cd team-server
go build -o recap-team-server ./cmd/server
./recap-team-server
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for the full build, test, and packaging instructions.

## Documentation

| Document | Purpose |
|----------|---------|
| [prd.md](prd.md) | Product Requirements Document |
| [sad.md](sad.md) | System Architecture Document |
| [tsd.md](tsd.md) | Technical Specification Document |
| [tbk.md](tbk.md) | Task Breakdown Document |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Current implementation status and build instructions |
| [research.md](research.md) | Market research and competitive analysis |
| [CLAUDE.md](CLAUDE.md) | Guidance for AI assistants working on this codebase |

## Tech Stack

- **Desktop app**: Tauri 1.4, Rust, vanilla JS/HTML/CSS
- **Desktop database**: SQLite 3.44+ with FTS5
- **Backend language**: Rust (Tauri commands)
- **Team server**: Go, Chi, PostgreSQL, Redis
- **Transcription**: whisper.cpp, OpenAI, Deepgram, AssemblyAI, AWS, Azure, Google
- **Summarization**: llama.cpp, Ollama, OpenAI, Anthropic, Google, AWS, Azure

## Working with AI agents
This repository is built to be extended with AI coding agents. Rather than a turnkey product, ODW [Name] is a working, well-structured codebase you can clone and adapt to your own needs with an agent like Claude Code. The repo includes agent context files (e.g. `CLAUDE.md`) and clear architecture docs so an agent can quickly understand the structure and help you customise, integrate, and extend it. To get started: clone the repo, open it with your coding agent, point it at this README and the docs, and describe what you want to build.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

