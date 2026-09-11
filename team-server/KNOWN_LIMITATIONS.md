# Known Limitations

This document honestly tracks what is **not** yet production-ready in ODW Recap.

The Go **team-server** (this directory) builds, vets, and tests cleanly and is
suitable for local development and integration testing. The blockers below live
in the **Tauri/Rust desktop app** (`../desktop-app`) and require a Rust
toolchain (`cargo`) to implement and verify. They are listed here so the gap
between the marketing promise ("on-device meeting intelligence") and the current
code is explicit.

## Rust-side P0 blockers (require a Rust toolchain to fix)

These are currently **stubbed** in the desktop app and must be implemented
before Recap can be sold as an on-device Otter/Fireflies/Fathom alternative.

1. **Real audio capture (cpal / loopback).**
   `desktop-app/src-tauri/src/audio_capture/` (`recorder.rs`, `system_audio.rs`)
   returns stub device lists and does not capture real microphone or system
   (loopback) audio. Needs a real capture backend such as `cpal`, including
   per-OS loopback (e.g. WASAPI loopback on Windows, BlackHole/ScreenCaptureKit
   on macOS).

2. **Local whisper.cpp / llama.cpp inference.**
   `transcription/providers/whisper_local.rs` and
   `summarization/providers/llama_local.rs` are explicit stubs ("would call
   whisper.cpp / llama.cpp FFI") that emit fake segments/summaries. Real
   on-device inference requires binding whisper.cpp and llama.cpp (FFI) and
   shipping/downloading model weights.

3. **Speaker diarization.**
   No diarization is implemented; transcript segments carry no real speaker
   attribution. Needs an on-device diarization model (e.g. pyannote-style
   embeddings + clustering) wired into the transcription pipeline.

4. **Real-time streaming transcription.**
   There is no streaming ASR path; transcription is batch-only and stubbed.
   Real-time partial/final segments require a streaming decoder and incremental
   UI updates.

5. **Pipeline wiring is PARTIALLY done (2026-09-11).**
   The desktop app now runs import → transcribe → summarize → persist → UI
   end-to-end for **cloud providers only**: STT via OpenAI Whisper API and
   Deepgram, summaries via OpenAI/Anthropic with an offline rule-based
   fallback. The remaining cloud adapters (AssemblyAI, Azure, Google, AWS)
   and the local engines (whisper.cpp, llama.cpp) are still stubs; selecting
   one returns an actionable error instead of fake output.

6. **Code signing, notarization, and auto-update.**
   The Tauri bundle is not code-signed or notarized (macOS Gatekeeper /
   Windows SmartScreen will block it), and there is no `tauri.conf.json`
   updater configuration. Required for trustworthy distribution.

## Security note (desktop app)

The storage encryption passphrase is read from the
`RECAP_ENCRYPTION_PASSPHRASE` environment variable with **no hardcoded
default**. When unset, the app logs a warning and falls back to an empty
passphrase — a documented development-only behavior, **not** a production
default. Production builds must set the variable (and ideally derive the key
from an OS keychain rather than a plain env var). Since 2026-09-11 the
Argon2 salt is random per installation and persisted to
`~/RecapData/salt.bin` (pre-existing zero-salt installs keep working via a
legacy fallback). The desktop app has been compile-verified and its test
suite (15 Rust tests) passes.

## What IS working (team-server, Go)

- JWT auth (register/login), RBAC middleware.
- Meetings CRUD now enforces **tenant isolation**: list/get/update/delete are
  scoped to the authenticated user's id; non-owners receive `403` (missing
  meetings return `404`); admins may still access all meetings.
- **Audit logging** on meeting create/update/delete and on `/sync`
  (best-effort; an audit write failure never fails the request).
- Cross-product sync forwarder: meeting summary → Vault
  (`POST {VAULT_API_URL}/files/upload`), action items → Loop webhook
  (`POST {LOOP_API_URL}/webhooks/{trigger_id}`, HMAC-signed when configured).
