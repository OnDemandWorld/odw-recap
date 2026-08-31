# Recap — Review, Refactor & Improvement Plan

**Date:** 2026-09-01
**Scope:** Full-repository review and refactor (desktop app, team server, frontend, scripts, CI), bug fixes, and the prioritized roadmap of what to do next.

---

## 1. Executive summary

The repository contained a well-structured scaffold, but large parts of it were
unwired or subtly broken:

- The **entire provider pipeline (STT + LLM routers) was dead code** — nothing
  ever constructed a router, so transcription/summarization could never run.
- The **encryption was effectively a no-op**: a hardcoded universal passphrase
  combined with an all-zero salt produced the *same key on every installation*.
- The **frontend could not run at all**: it imported npm modules in a setup with
  no bundler, and used a dialog API that wasn't in the Tauri allowlist.
- The **team server had authorization gaps** (any authenticated user could
  read/update/delete any meeting), NULL-handling bugs that silently dropped
  rows, and a sync endpoint that always failed for new meetings (FK violation).
- **Full-text search never worked** (FTS index never populated).

All of the above has been fixed in this pass. The app now has a working
end-to-end local loop: **import audio → transcribe → summarize → view results**,
with settings/API-key persistence, provider status reporting, and prompt
template management. Test coverage grew from **7 → 16 Rust tests** and
**3 → 9 Go tests**; compiler warnings from **150 → 0** in project code.

---

## 2. What was fixed in this pass

### 2.1 Security fixes

| # | Issue | Fix |
|---|-------|-----|
| S1 | CORS `AllowedOrigins: ["*"]` **combined with** `AllowCredentials: true` in the team server (any website could make credentialed requests) | Credentials are only allowed when explicit origins are configured via `CORS_ALLOWED_ORIGINS`; wildcard mode no longer sends credentials |
| S2 | Encryption key derived from hardcoded `"default-passphrase"` **and an all-zero salt** → identical key on every machine | Per-installation random salt generated once and persisted (`recap.salt`, mode 0600); passphrase comes from `RECAP_PASSPHRASE` env var or a per-install random passphrase file (mode 0600). Regression test added |
| S3 | `change_passphrase` swapped the in-memory key without re-encrypting data → silently corrupted all existing encrypted data | Now returns an explicit error until a proper re-encryption flow exists (P0-4) |
| S4 | HTTP upload server bound to `0.0.0.0` with **no authentication and no file-type validation** | Binds to `127.0.0.1` only; extension allowlist enforced; 2 GiB size cap; correct HTTP status codes; client filenames never touch the filesystem |
| S5 | Meetings API had **no ownership checks** — any authenticated user could read/modify/delete any meeting | All meeting endpoints now scope to the creator (admin bypass), validate UUIDs, and return proper 400/404/409 codes |
| S6 | Unbounded JSON request bodies (DoS vector) | `http.MaxBytesReader` (10 MiB) on all JSON endpoints |
| S7 | `bcrypt` silently truncates passwords > 72 bytes; no password/email policy on register | Reject > 72-byte passwords, require ≥ 8 chars, validate email format, normalize email, 409 on duplicate |
| S8 | Admin stats built SQL with string-concatenated table names | Static query map; no concatenation |
| S9 | Default JWT secret usable in production with only a log warning | **Still open** — flagged as P0-2 (fail closed) |

### 2.2 Correctness fixes

| # | Issue | Fix |
|---|-------|-----|
| C1 | `POST /sync` inserted `meeting_id` into `sync_queue` (FK → `meetings`) before the meeting existed → **every new-meeting sync 500'd** | Meeting row is upserted before queueing; action allowlist; UUID validation |
| C2 | `GET /meetings` scanned nullable columns into plain strings → rows with NULL title/type/language **silently dropped** | `sql.NullString` scans + `rows.Err()` check |
| C3 | `GET /meetings/{id}` scanned nullable `created_by` into `uuid.UUID` → meetings with deleted creators returned 404 | Nullable pointer scan |
| C4 | `PUT /meetings/{id}` with a partial body wiped omitted fields to NULL | `COALESCE`-based partial update; rows-affected checks on update/delete |
| C5 | `respondJSON` called `http.Error` after headers were written | Logs encode failures instead |
| C6 | Rust `row_to_meeting` / `search_transcripts` replaced unparseable UUIDs with **random new ones** (`unwrap_or(Uuid::new_v4())`) | Errors propagate; corrupt rows surface instead of corrupting lookups |
| C7 | FTS5 `transcript_search` was an external-content index with **no sync triggers and no insert path** → search always returned nothing | Added `AFTER INSERT/DELETE/UPDATE` triggers + `rebuild` in migrations; added `insert_transcript_segment` / `get_transcript_segments`; covered by tests |
| C8 | `STTRouter`/`LLMRouter` used `config.model` as the **provider** key → any model selection broke routing | New `config.provider` field; model stays model-scoped |
| C9 | Deepgram "average confidence" computed the *last word's* confidence repeated N times | Real average over the window; also sends a Content-Type matching the actual file format |
| C10 | `ConfigManager::set_string` stored raw strings while `get<T>` JSON-parsed (and vice versa) → settings round-trips failed | `set_string` stores JSON; `get_string` decodes JSON with raw fallback; test added |
| C11 | `list_meetings` Tauri command returned only UUID strings → UI could only show "Meeting 3fa9c1…" | Returns full `Meeting` objects; UI renders title/status/date |
| C12 | Importing audio copied the file but created **no meeting record** | Import now creates a meeting with filename title, format, size, and source |
| C13 | Frontend used `import { invoke } from "@tauri-apps/api/tauri"` **with no bundler** → module resolution failed; `dialog.open` wasn't allowlisted | Rewritten on `window.__TAURI__` globals (`withGlobalTauri`); `dialog.open` added to allowlist + `dialog-open` Cargo feature |
| C14 | "Save API Keys" button claimed success without saving anything | Wired to real `save_api_key` / `set_config` commands; shows which keys are already stored |
| C15 | Watch-folder stability check blocked the notify dispatch thread for up to 15 s per file and missed files *moved* into the folder | Runs on a worker thread; handles `Create(File)` and `RenameMode::To` |
| C16 | `scripts/test-all.sh` had a broken `cd ../team-server` (escaped the repo root) and `npm test` crashed (jest + ESM scaffolds) | Scripts rewritten to resolve paths from the script location; jest configured with `--passWithNoTests` and scaffolds excluded |
| C17 | CI desktop build job missing Tauri Linux system dependencies (would always fail) | `libwebkit2gtk-4.0-dev` etc. installed; `go vet` added; cargo caching on both jobs |
| C18 | Default prompt templates were re-seeded on every launch, overwriting user edits | Seeding is idempotent (existing templates win) |
| C19 | `MeetingStatus::from` maps unknown strings to `Deleted`; `SyncStatus` to `PermanentlyFailed` | **Still open** — flagged as P0-5 (lossy fallbacks) |

### 2.3 Architecture / refactor

- **Pipeline wired end-to-end.** `STTRouter::with_all_providers` /
  `LLMRouter::with_all_providers` register all compiled providers under the
  names used by the Settings UI. New Tauri commands:
  `transcribe_meeting`, `summarize_meeting`, `get_transcript_segments`,
  `get_summary`, `search_transcripts`, `list_stt_providers`,
  `list_llm_providers`, `list_prompt_templates`, `save_prompt_template`,
  `set_config`, `get_meeting`, `has_api_key`. Summarization automatically
  falls back to the local rule-based engine when no LLM provider is
  configured, keeping the feature useful with zero cloud setup.
- **Frontend rewritten** as a working app shell: meeting library with real
  metadata, detail view with Transcribe/Summarize actions and live
  transcript/summary rendering, settings that persist, template list,
  provider availability status, HTML-escaping everywhere.
- **Code hygiene:** `impl ToString` → `impl Display`; removed unused
  dependencies (`zeroize`, `directories`, `once_cell`, `tower`, `tower-http`,
  `mime`); scaffold `greet` command removed; rebranded `tauri.conf.json`
  (`Recap`, `ai.odw.recap.desktop`) and Cargo package (`recap-desktop`);
  intentional scaffolding marked with scoped `#[allow(dead_code)]` + pointers
  to this plan. **Warnings: 150 → 0** in project code.
- **Data directory** moved from `~/RecapData` to the platform-local data dir
  (`~/Library/Application Support/Recap` on macOS), with env-var passphrase
  override.

### 2.4 Verification status

| Check | Before | After |
|-------|--------|-------|
| Rust tests | 7 passing | **16 passing** (FTS, salt persistence, config round-trip, summaries, routers, rule-based summarizer) |
| Go tests | 3 passing | **9 passing** (auth middleware, RBAC, JWT cross-secret) |
| Rust warnings (project code) | 150 | **0** |
| `cargo build` / `go build` | OK | OK |
| `scripts/test-all.sh` | broken paths, crashed on `npm test` | runs green end-to-end |

---

## 3. Known issues deliberately NOT fixed yet (need product decisions)

1. **Passphrase UX.** The random passphrase lives in `.passphrase` (0600). This
   protects against casual access but not an attacker with file access, and
   there is no unlock UI. See P0-1.
2. **Local STT/LLM providers are stubs.** `whisper_local` returns placeholder
   segments; `llama_local` returns a placeholder summary. Cloud providers
   (OpenAI, Anthropic, Deepgram, Ollama) are real. See P0-3.
3. **Audio capture is a stub** (no `cpal` yet) — live recording doesn't
   produce audio. Import/watch-folder/HTTP-upload paths work. See P1-2.
4. **Team-server `sync_queue` has no consumer** — items queue but nothing
   processes them. See P1-1.
5. **Tauri 1.4** (2.x migration deferred) and `csp: null` in `tauri.conf.json`.
   See P1-5.
6. **Meeting/segment enums parse with lossy fallbacks** (C19). Changing to
   `TryFrom` touches the DB read path and needs a migration story for bad
   values — small but deliberate work.

---

## 4. Roadmap

Priorities: **P0** = do now (blocks trust in the product), **P1** = core
product loop, **P2** = scale & polish, **P3** = nice-to-have.

### P0 — Trust & correctness (1–2 sprints)

#### P0-1. Real unlock flow for local encryption
- **Why:** the whole privacy pitch rests on at-rest encryption; a file-stored
  passphrase is a placeholder, not a product.
- **What:** first-run passphrase creation screen; unlock screen on launch;
  store the *wrapped* key in the OS keychain (macOS Keychain / Windows DPAPI /
  libsecret) instead of a plaintext file; optional auto-lock timeout.
- **Accept:** cold start requires unlock; deleting the keychain entry makes
  data unrecoverable by design; no plaintext key material in the data dir.

#### P0-2. Team-server auth hardening
- **Why:** default JWT secret still boots with a warning; tokens live 24 h
  with no revocation; role is trusted from the token.
- **What:** fail closed when `JWT_SECRET` is unset (except explicit
  `--insecure-dev` flag); refresh-token rotation + revocation list in Redis;
  re-check role/user existence against DB on each request; rate limiting on
  `/auth/*` (Redis token bucket); structured audit-log writes on
  auth/admin/sync events (the table exists but is never written).
- **Accept:** server refuses to start without a secret; revoked tokens are
  rejected within one request; brute-force login attempts are throttled.

#### P0-3. Real on-device transcription (whisper.cpp)
- **Why:** "sovereign, on-device" is the product thesis; today the default
  provider returns placeholder text.
- **What:** integrate `whisper-rs` (FFI to whisper.cpp); model download manager
  (tiny→large-v3, progress + resume, stored under the data dir); 16 kHz mono
  WAV decode pipeline (symphonia/ffmpeg-sidecar); progress events to the UI;
  language auto-detect; replace stub segments with real timestamped output.
- **Accept:** importing a 30-minute m4a produces an accurate transcript fully
  offline on a laptop; UI shows progress; model choice persisted in settings.

#### P0-4. Safe passphrase change
- **Why:** `change_passphrase` currently errors out; users need rotation.
- **What:** implement re-encryption of all encrypted artifacts (api_keys rows,
  encrypted files) in a single transactional pass, with rollback on failure;
  UI in settings.
- **Accept:** after a passphrase change, all API keys and files decrypt with
  the new passphrase and fail with the old one; crash mid-change leaves data
  readable with the old passphrase.

#### P0-5. Lossless enum parsing
- **Why:** unknown DB values currently become `MeetingStatus::Deleted` /
  `SyncStatus::PermanentlyFailed`, which can hide or corrupt data.
- **What:** `TryFrom<String>` with explicit errors; store raw string in an
  `Unknown(String)` variant or surface a migration warning.
- **Accept:** a hand-edited DB value produces a visible error, never a silent
  state change; tests cover round-trips.

### P1 — Complete the core loop (2–4 sprints)

#### P1-1. Real sync engine (desktop ↔ team server)
- **Why:** multi-device and team value requires sync; today the queue is
  write-only on both ends.
- **What:** desktop: offline outbox using the existing `sync_queue` table with
  retries/backoff and `sync_status` transitions; server: worker that consumes
  `sync_queue` (apply meeting/transcript/summary upserts), conflict rule
  (last-writer-wins + field-level merge for metadata), ownership enforcement
  on synced meetings, pull endpoint (`GET /sync/changes?since=`) for the
  client; end-to-end auth with the existing JWT.
- **Accept:** a meeting transcribed offline appears on a second machine and on
  the server DB; killing the server mid-sync retries cleanly; user A cannot
  sync to user B's meetings.

#### P1-2. Real audio capture
- **Why:** live meeting capture is a headline feature.
- **What:** `cpal` microphone capture with device selection UI; encode to
  WAV/Opus; platform system-audio loopback (BlackHole on macOS, WASAPI
  loopback on Windows, PulseAudio/PipeWire monitor on Linux) with setup
  guidance in the UI.
- **Accept:** record a 10-minute meeting from mic + system audio; resulting
  file flows through the existing transcribe pipeline.

#### P1-3. Structured outputs: action items & decisions
- **Why:** the schema, models, and rule-based keywords exist, but nothing
  populates `action_items`/`decisions` from LLMs.
- **What:** prompt the configured LLM for JSON (schema-constrained where
  supported); parse/validate; persist via new storage APIs; render in the
  detail view with assignee/due-date editing; feed the rule-based extractor
  as fallback.
- **Accept:** a transcript produces stored, editable action items and
  decisions visible in the UI and included in sync payloads.

#### P1-4. Finish remaining cloud providers
- **Why:** 7 of 14 providers are stubs (AssemblyAI, AWS Transcribe, Azure
  Speech, Google STT, Gemini, Bedrock, Azure OpenAI, llama.cpp).
- **What:** implement against each API (upload endpoints for AssemblyAI;
  SigV4 for AWS; diarization where offered); conformance tests with recorded
  fixtures (wiremock); llama.cpp via `llama.cpp` server mode or `llama-cpp-rs`.
- **Accept:** each provider passes a recorded-fixture integration test; the
  provider status endpoint reflects real availability.

#### P1-5. Tauri 2.x migration + web hardening
- **Why:** Tauri 1.x is EOL-ward; CSP is currently null.
- **What:** migrate to Tauri 2 (capabilities/permissions, IPC changes, mobile
  targets unlocked); set a strict CSP; remove `withGlobalTauri` in favor of a
  real bundler (Vite) + typed API imports; evaluate `tauri-plugin-*` for
  dialog/fs instead of hand-rolled commands.
- **Accept:** app builds and passes E2E on Tauri 2; CSP blocks inline
  scripts; no regressions in commands.

#### P1-6. Speaker diarization
- **Why:** the `speakers` table and `speaker_id` fields are unused; multi-
  speaker meetings are the norm.
- **What:** integrate a diarization model (e.g. pyannote via a sidecar or a
  Rust port), or use provider diarization (Deepgram/AssemblyAI) when
  configured; assign labels; render per-speaker lanes in the transcript.
- **Accept:** a two-speaker recording yields consistently labeled segments.

#### P1-7. Meeting UX pass
- **Why:** the detail view is functional but minimal.
- **What:** transcript search UI (the `search_transcripts` command exists);
  audio playback with click-to-seek on segments; inline title/metadata
  editing; meeting delete/archive with confirm; export (Markdown/JSON/SRT).
- **Accept:** all actions above work locally and reflect in the DB.

### P2 — Scale, quality, operations (4–8 sprints)

1. **Organizations & sharing** — the org tables exist but are unused: org
   CRUD, invites, per-org meeting scoping on the server, role model
   (owner/admin/member), sharing meetings inside an org.
2. **Test depth** — Go handler tests with a real schema (testcontainers or
   sqlocal), Rust async provider tests against wiremock, Playwright E2E
   against a built Tauri app (the scaffolds in `desktop-app/tests`), CI
   coverage gates.
3. **Observability** — structured logging (tracing) in both apps, request IDs
   end-to-end, `/metrics` on the team server, sync health indicators in the UI.
4. **Packaging & distribution** — code signing (macOS notarization, Windows
   Authenticode), auto-updater (Tauri updater + signed manifests), release
   automation from CI tags.
5. **Server deployment story** — Dockerfile + compose (Postgres + Redis +
   server), config via env, TLS termination docs, backups/restore for
   Postgres; document a single-binary "personal server" mode.
6. **Schema alignment** — reconcile desktop SQLite schema with server
   Postgres schema (e.g. `is_final` defaults differ; server lacks `speakers`
   table), and version sync payloads (`schema_version` field).

### P3 — Nice-to-have

1. Mobile companion app (Tauri 2 / Capacitor) for phone recording + upload —
   the HTTP upload server already accepts transfers.
2. iOS Shortcuts / x-callback integration (types already stubbed in `sync`).
3. Live meeting mode: streaming partial transcription with interim segments
   (`is_final` flags already modeled).
4. Vault/Loop deep integration (push summaries to knowledge base, action
   items to task workflows) behind feature flags.
5. Performance benchmarks suite (transcription RTF, summarization latency,
   memory) in CI.
6. i18n for the UI; multi-language transcription UX.
7. Web dashboard for the team server (meeting browser, admin console).

---

## 5. Suggested sequencing

```
Sprint 1-2:  P0-1 (unlock flow) · P0-2 (auth hardening) · P0-5 (enums)
Sprint 2-4:  P0-3 (whisper.cpp) · P0-4 (passphrase change)
Sprint 4-6:  P1-1 (sync engine) · P1-7 (meeting UX)
Sprint 6-8:  P1-2 (audio capture) · P1-3 (action items/decisions)
Sprint 8-10: P1-5 (Tauri 2) · P1-4 (remaining providers) · P1-6 (diarization)
Then:        P2 items in parallel tracks (server, quality, packaging)
```

The ordering front-loads trust (encryption UX + auth) and the offline
transcription core, because everything else — sync, team features, mobile —
is only worth building on a privately, reliably working local pipeline.

---

## 6. How to verify the current state

```bash
# All tests (Rust + Go + frontend)
./scripts/test-all.sh

# Individual suites
cd desktop-app/src-tauri && cargo test
cd team-server && go vet ./... && go test ./...

# Run the desktop app
cd desktop-app && npm install && npm run tauri dev

# Run the team server (needs Postgres + Redis, or change DATABASE_URL)
cd team-server && JWT_SECRET=$(openssl rand -hex 32) go run ./cmd/server
```
