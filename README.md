# ODW Recap

A sovereign, on-device meeting intelligence system that captures, transcribes, and summarizes meetings without sending audio to third-party clouds.

## What is Recap?

ODW Recap turns meetings into structured action items, summaries, and decisions — all on your local machine. Its core promise: meeting recordings and transcripts never leave your device unless you explicitly choose a cloud provider.

No cloud bot silently joins your calls. No transcripts sit on a vendor's servers by default.

## Status

⚠️ **Early release.** ODW Recap is an early, functional release — core features work, but it is not yet hardened for production. We are refining every module toward a first full public release in **Q3 2026**. Until then, it is best used as a foundation to build on with AI coding agents (see below).

## Key Features

- **On-device transcription**: Whisper-family models run locally via whisper.cpp — ⚠️ the local engine is a stub in this release; end-to-end transcription currently requires a cloud provider (OpenAI/Deepgram) or a remote Ollama. See [team-server/KNOWN_LIMITATIONS.md](team-server/KNOWN_LIMITATIONS.md)
- **Local AI summarization**: Generates summaries, action items, and decisions — local LLM summarization shares the same stub caveat as above
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

前置：Rust 工具链（rustup）、Node.js 20+，以及 Tauri 1.x 的系统依赖
（macOS：Xcode CLT；Linux：libwebkit2gtk/webkit2gtk-4.1 等开发包）。

```bash
cd desktop-app
npm install
npm run dev     # 开发运行（推荐）；npm run tauri build 产出安装包
```

（顶层 `cargo build` 只产出裸二进制；打包/签名走 `npm run tauri build`，见 desktop-app/README.md）

### Team Server

前置：Go ≥1.22、PostgreSQL 16、Redis 7；启动前建库建用户（建表自动迁移）：

```bash
psql -c "CREATE ROLE recap WITH LOGIN PASSWORD 'recap';" -c "CREATE DATABASE recap OWNER recap;"
cd team-server
DATABASE_URL='postgres://recap:recap@localhost:5432/recap?sslmode=disable' \
  REDIS_URL=localhost:6379 JWT_SECRET='<openssl rand -hex 32，≥32 字符>' \
  go run ./cmd/server
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
| [NEXT-STEPS.md](NEXT-STEPS.md) | 下一步开发计划：按优先级排列的待办、验收标准 (DoD) 与工作量 |
| [CHANGELOG.md](CHANGELOG.md) | Notable changes, most-recent first |
| [USER-GUIDE.zh-CN.md](USER-GUIDE.zh-CN.md) | 面向非技术人员的使用指南（中文，含截图级操作步骤） |
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
This repository is built to be extended with AI coding agents. Rather than a turnkey product, ODW Recap is a working, well-structured codebase you can clone and adapt to your own needs with an agent like Claude Code. The repo includes agent context files (e.g. `CLAUDE.md`) and clear architecture docs so an agent can quickly understand the structure and help you customise, integrate, and extend it. To get started: clone the repo, open it with your coding agent, point it at this README and the docs, and describe what you want to build.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

