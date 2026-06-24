# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Purpose

This is a **product documentation repository** for Recap by ODW.ai — a sovereign, on-device meeting intelligence system. It contains no application code; all files are product planning and specification documents.

## Document Structure

| File | Purpose |
|------|---------|
| `README.md` | Product overview and positioning |
| `prd.md` | Product Requirements Document — features, goals, success metrics |
| `sad.md` | System Architecture Document — architectural decisions, component design |
| `tsd.md` | Technical Specification Document — implementation-level technical details |
| `tbk.md` | Task Breakdown Document — implementation tasks, phases, effort estimates |
| `research.md` | Market research and competitive analysis |

## Product Context

Recap is a privacy-first meeting intelligence tool that:
- Captures and transcribes meetings locally (no cloud bots, no external processing)
- Uses open-source speech models (Whisper-family) for on-device transcription
- Generates structured outputs (summaries, action items, decisions)
- Syncs to ODW.ai suite modules (Vault for knowledge, Loop for workflows)
- Targets regulated industries and privacy-conscious teams

**Tech stack:** Tauri (Rust backend + webview frontend), whisper.cpp, llama.cpp, SQLite, optional Go team server

## Working with Documents

- Documents cross-reference each other (e.g., TSD references SAD components)
- Maintain consistency across documents when updating specifications
- All documents use standard Markdown with tables and structured sections
- Research citations use `[[N]](URL)` format with a Sources section at the end
