# Product Requirements Document: Recap

**Product:** Recap by ODW.ai
**Version:** 1.0
**Date:** June 23, 2026
**Status:** Draft
**Author:** Product Team

---

## 1. Product Overview

### 1.1 Vision Statement

Recap turns meetings into structured action items, entirely on-device. Meeting recordings and transcripts never leave the user's machine — no cloud bot silently joining calls, no transcripts sitting on a vendor's servers. Recap captures conversations, transcribes them locally using open transcription models, and produces clean summaries and action items that auto-sync into the user's existing tools.

### 1.2 Problem Statement

Organizations face a growing tension between two imperatives:

- **Workflow automation:** Cloud-based meeting intelligence tools (Otter.ai, Fireflies, Grain, tl;dv) offer convenience but require surrendering sensitive conversation data to third-party servers.
- **Data sovereignty:** Regulated industries, privacy-conscious teams, and organizations in data-residency contexts need meeting intelligence without exposing conversations to external infrastructure.

Existing privacy-focused alternatives either still rely on cloud processing, lack integration with downstream workflows, or require significant technical expertise to self-host. Meanwhile, standalone meeting-notes tools are being rapidly commoditized — absorbed as free features into platforms like Zoom, Teams, and Google Meet — making a standalone transcription product strategically unviable.

### 1.3 Solution

Recap is a sovereign, suite-integrated meeting intelligence product that:

- **Captures audio from multiple sources:**
  - Live system audio capture (no cloud bots, no external participants)
  - Smartphone recording via companion app (iOS/Android)
  - File import (drag-and-drop, file browser)
  - Watch folder (automatic processing of audio files placed in designated folder)
  - Local HTTP upload server for LAN-based smartphone transfers
- **Transcribes conversations using flexible backends:**
  - Local open-source speech models (Whisper-family via whisper.cpp, faster-whisper)
  - Paid STT APIs (OpenAI Whisper API, AssemblyAI, Deepgram)
  - Cloud-hosted STT services (AWS Transcribe, Azure Speech, Google Speech-to-Text)
  - User-selectable provider with fallback chain support
- **Generates structured outputs using configurable LLMs:**
  - Local LLMs (llama.cpp, Ollama)
  - Cloud LLMs (OpenAI GPT-4, Anthropic Claude, Google Gemini, AWS Bedrock, Azure OpenAI)
  - Pre-built prompt templates for different meeting types
  - Editable, versioned prompts for customization
- **Enhances summarization with meeting metadata:**
  - Meeting type (team meeting, 1:1, client call, interview, presentation, workshop)
  - Location, participants, language(s), topic/project
  - Metadata-driven prompt selection for context-aware summaries
- Auto-syncs outputs into ODW.ai's Vault (knowledge base) and Loop (workflow automation) modules
- Operates model-agnostically and is fully self-hosted

### 1.4 Strategic Role Within ODW.ai

Recap is not positioned as a standalone transcription product. Its strategic value is as a **sovereign data source** feeding the broader ODW.ai suite:

- Meetings → Vault: Decisions, context, and knowledge from calls automatically enrich the organizational knowledge base.
- Meetings → Loop: Action items extracted from meetings automatically trigger follow-up workflows, assignments, and reminders.
- All processing occurs on the customer's infrastructure, maintaining the sovereignty promise end-to-end.

### 1.5 Target Market

**Primary:** Small and medium businesses (SMBs) operating in regulated or data-residency contexts, including:

- Legal firms and compliance-heavy consultancies
- Healthcare organizations (HIPAA-adjacent workflows)
- Financial services and fintech
- Government contractors and defense-adjacent SMBs
- EU-based organizations subject to GDPR data-minimization requirements
- Any organization where "no cloud bot joining our call" is a hard requirement

**Secondary:** Privacy-conscious teams and individuals at larger enterprises who cannot use sanctioned cloud tools for sensitive meetings.

### 1.6 Positioning

**Sovereign, on-device, suite-integrated meeting intelligence.**

Differentiated from:

| Competitor Category | Their Approach | Recap's Difference |
|---|---|---|
| Otter.ai, Fireflies, Grain, tl;dv | Cloud-hosted bots join calls, data processed on vendor servers | No bots, no cloud — everything on-device |
| Whisper-based DIY tools | Open models but no structured output or integrations | Structured action items + suite integration |
| WhisperLive, MeetingBaize | Real-time local transcription | Adds summarization, action extraction, and Vault/Loop sync |
| Platform-native features (Zoom AI, Teams Copilot) | Locked to one platform, cloud-processed | Platform-agnostic, on-device, sovereign |

### 1.7 Business Model

- **Free core:** On-device transcription, local summaries, basic action item extraction, single-user local notes.
- **Paid layer (bundle-first):** Cross-module sync (Vault, Loop), retention and governance controls, team features (shared meeting spaces, role-based access), admin dashboards, audit logs.
- **Not sold standalone:** Recap is bundled within ODW.ai suite pricing. The free core serves as an on-ramp; the paid layer unlocks suite integration value.

---

## 2. Goals & Success Metrics

### 2.1 Product Goals

| # | Goal | Timeframe |
|---|---|---|
| G1 | Deliver best-in-class on-device transcription accuracy that approaches cloud alternatives | v1.0 (6 months) |
| G2 | Establish Recap as the default meeting-intelligence layer for ODW.ai suite customers in regulated contexts | v1.0 + 12 months |
| G3 | Demonstrate measurable workflow automation value through Vault/Loop integration (meetings → knowledge → actions) | v1.1 (9 months) |
| G4 | Achieve zero-data-exit architecture — verifiable that no meeting data leaves the device/infrastructure | v1.0 |
| G5 | Convert free-core users to paid suite bundles at ≥15% rate within 6 months of activation | Ongoing post-launch |

### 2.2 Success Metrics

#### Acquisition & Activation
- **M1:** 5,000 free-core activations within 6 months of launch
- **M2:** 60% activation rate (users who complete first meeting transcription within 7 days of install)
- **M3:** <5 minute time-to-first-transcript from install

#### Engagement & Retention
- **M4:** 70% weekly active usage among activated users (≥1 meeting processed per week)
- **M5:** 40% monthly retention at 3 months
- **M6:** Average of 3+ meetings processed per active user per week by month 3

#### Quality
- **M7:** Word Error Rate (WER) ≤8% on standard business English meetings (measured against human transcription baseline)
- **M8:** Action item extraction precision ≥85%, recall ≥75% (human-evaluated sample)
- **M9:** Summary quality score ≥4.0/5.0 in user satisfaction surveys

#### Business
- **M10:** 15% free-to-paid conversion within 6 months
- **M11:** 25% of paid customers use Vault/Loop integration features within 3 months of upgrade
- **M12:** Net Revenue Retention (NRR) ≥110% among suite-bundled customers using Recap

#### Technical
- **M13:** Real-time transcription latency ≤2 seconds (streaming mode)
- **M14:** Batch transcription of a 60-minute meeting completes in ≤10 minutes on reference hardware
- **M15:** Zero confirmed data-exit incidents post-launch

---

## 3. Scope Definition

### 3.1 In Scope (v1.0)

#### Audio Input Flexibility
- System audio capture (macOS, Windows, Linux) without cloud bots
- Smartphone companion app (Tauri v2 mobile) for iOS/Android recording with LAN upload
- Local HTTP upload server (Axum-based) for smartphone audio transfer
- Watch folder with automatic processing (monitoring ~/RecapData/inbox/ for new audio files)
- File import via drag-and-drop and native file browser
- Support for common audio formats (M4A, WAV, MP3, OGG, WebM, FLAC)
- Meeting metadata entry form (type, location, participants, language, topic)
- iOS Shortcuts integration for audio upload (optional)

#### Core Transcription
- Multiple STT backend support:
  - Local models (whisper.cpp, faster-whisper)
  - Paid APIs (OpenAI Whisper API, AssemblyAI, Deepgram)
  - Cloud-hosted services (AWS Transcribe, Azure Speech, Google Speech-to-Text)
- User-selectable STT provider with cost/quality tradeoffs displayed
- Fallback chain: if primary STT provider fails, automatically try secondary provider
- API key management for paid STT services (encrypted storage)
- Speaker diarization (local, model-based)
- Multi-language support with per-meeting language selection
- Real-time streaming transcription and post-meeting batch processing
- Model selection UI (small/medium/large tradeoffs: speed vs. accuracy vs. resource usage)
- Cost estimation for paid STT APIs (displayed before transcription)

#### Structured Output
- Meeting summaries (auto-generated, editable)
- Action item extraction (assignee detection, deadline detection where stated)
- Decision logging
- Key topics/tags extraction
- Custom output templates (user-configurable)

#### Local Storage & Organization
- Local encrypted storage of recordings, transcripts, and summaries
- Search across past meetings (full-text, local index)
- Manual tagging and folder organization
- Export formats: Markdown, PDF, plain text, JSON

#### ODW Suite Integration (Paid Layer)
- Vault sync: meeting summaries and decisions auto-indexed into organizational knowledge base
- Loop sync: action items auto-created as tasks/workflow triggers
- Bi-directional linking: Vault entries can reference source meetings
- Configurable sync rules (which meetings sync, which tags trigger workflows)

#### Governance & Admin (Paid Layer)
- Retention policies (auto-delete after N days)
- Audit logs (who accessed what meeting data, when)
- Role-based access control (team workspaces)
- Admin dashboard (usage metrics, model versions, sync health)

#### Platform Support
- macOS (Apple Silicon + Intel) — primary
- Windows 10/11 — primary
- Linux (Ubuntu 22.04+, Fedora 38+) — secondary
- Self-hosted server mode for team deployments (Docker-based)

### 3.2 Out of Scope

#### v1.0 Explicitly Excludes
- **Mandatory cloud processing:** Cloud STT/LLM APIs are optional, not required — local processing remains the default and recommended path
- **Meeting platform bots:** No Zoom/Teams/Meet bots that join as participants
- **Video recording/capture:** Audio-only (video adds significant complexity and storage; not needed for transcription value)
- **Real-time translation:** Transcription only; translation is a future consideration
- **Mobile apps with local processing:** Companion mobile app uploads audio to desktop for processing; no local transcription on mobile (v1.0)
- **Telephony/SIP capture:** Only system audio and file import (no direct phone call recording)
- **Standalone API product:** No public API for third-party integrations outside ODW suite (v1.0)
- **Custom model training/fine-tuning UI:** Users can swap models manually, but no in-app fine-tuning workflow
- **CRM/sales-specific features:** No deal tracking, signal detection, or coaching features (this is general meeting intelligence, not a sales tool)
- **Calendar integration for auto-join:** Recap captures system audio regardless of calendar; it does not need to know about or join meetings via calendar

#### Not Planned
- Hardware appliance / dedicated meeting-room device
- Browser extension
- Slack/Teams channel posting (beyond Vault/Loop sync)
- White-label / OEM licensing

---

## 4. User Personas

### 4.1 Persona: "Sovereign Sarah" — Compliance Officer at a Mid-Size Law Firm

**Demographics:**
- Age: 38, based in Frankfurt, Germany
- Role: Head of IT Compliance at a 50-person law firm
- Technical comfort: Moderate — can configure software, understands data residency requirements

**Context:**
- Firm handles client matters subject to attorney-client privilege and GDPR
- Currently uses Otter.ai but legal counsel has flagged data residency concerns (data processed on US servers)
- Needs meeting notes for case management but cannot risk privilege waiver via third-party processing
- Evaluating tools as part of broader "sovereign tech stack" initiative

**Goals:**
- Eliminate cloud-processed meeting data from the firm's workflow
- Maintain or improve note-taking quality vs. current tool
- Demonstrate compliance to clients and regulators

**Frustrations:**
- Current tool's privacy policy is opaque about data retention
- "Opt-out of AI training" toggles don't provide real assurance
- Self-hosted alternatives require DevOps resources the firm lacks

**What Recap delivers:**
- Verifiable on-device processing (no data leaves the machine)
- Quality comparable to cloud tools (Whisper large-v3 accuracy)
- Suite integration means the firm gets knowledge management (Vault) and workflow automation (Loop) as a bundle, justifying the investment beyond transcription alone

### 4.2 Persona: "Founder Felix" — Technical Co-Founder at a Privacy-First Startup

**Demographics:**
- Age: 29, based in Berlin
- Role: CTO of a 12-person B2B SaaS startup handling sensitive healthcare data
- Technical comfort: High — comfortable with Docker, self-hosting, model selection

**Context:**
- Team runs all meetings on Jitsi/Signal (privacy-first stack)
- Needs meeting notes but refuses to add a cloud bot to calls (brand promise to customers: "we never send your data to third parties")
- Currently taking notes manually; losing action items
- Wants automation that aligns with company values

**Goals:**
- Automated meeting notes without compromising privacy brand
- Action items that flow into project management (via Loop integration)
- Minimal setup and maintenance burden

**Frustrations:**
- Open-source Whisper tools require too much plumbing to be practical for a busy team
- Commercial tools all require cloud data processing
- Existing tools don't integrate with his workflow stack

**What Recap delivers:**
- Docker-based self-hosted deployment matching his infrastructure philosophy
- Model-agnostic (can swap in newer models as they release)
- Loop integration means action items become tracked tasks automatically
- Free core lets him validate before committing to bundle

### 4.3 Persona: "Manager Maya" — Operations Lead at a Regional Healthcare Provider

**Demographics:**
- Age: 44, based in Austin, TX
- Role: Operations Manager overseeing 3 clinic locations, ~80 staff
- Technical comfort: Low-moderate — uses software daily but doesn't configure infrastructure

**Context:**
- Regular meetings with clinic managers, compliance reviews, vendor calls
- Organization is HIPAA-adjacent (handles PHI in some meetings)
- IT director has blocked all cloud meeting-note tools due to PHI risk
- Team currently relies on inconsistent manual notes; action items fall through cracks

**Goals:**
- Consistent, reliable meeting notes across all locations
- Action items that don't get lost
- Something her team can use without IT involvement for day-to-day

**Frustrations:**
- Manual note-taking is inconsistent and incomplete
- IT blocks useful tools due to legitimate privacy concerns
- Needs something that "just works" on her existing laptop

**What Recap delivers:**
- Desktop app that works without IT configuration (free core)
- On-device processing satisfies IT/privacy requirements
- Paid team features (when IT approves) give her visibility across locations
- Vault integration means institutional knowledge doesn't leave when staff turnover

### 4.4 Persona: "Admin Alex" — IT Administrator at a Government Contractor

**Demographics:**
- Age: 52, based in Washington, DC metro
- Role: Senior IT Administrator, cleared facility, ~200 employees
- Technical comfort: High — manages infrastructure, security policies, deployment pipelines

**Context:**
- Facility handles CUI (Controlled Unclassified Information); some meetings touch CUI-adjacent topics
- NIST 800-171 compliance required; cloud SaaS tools require extensive ATO process
- Evaluating self-hosted alternatives for multiple tool categories
- Needs centralized deployment, audit trails, and retention controls

**Goals:**
- Deploy meeting intelligence without triggering a new ATO process (self-hosted = existing ATO boundary)
- Centralized policy management (retention, access, audit)
- Integration with existing knowledge management and task systems

**Frustrations:**
- Every new SaaS tool requires months of security review
- Open-source tools lack admin controls and audit capabilities
- Needs to demonstrate compliance to DCSA/assessors

**What Recap delivers:**
- Self-hosted deployment within existing security boundary
- Admin dashboard with audit logs and retention policies
- RBAC for team workspaces
- Bundle value: Vault and Loop integration reduce need for additional tools (fewer ATO processes)

---

## 5. User Journeys & Flows

### 5.1 Journey: First-Time Setup (Free Core)

```
User downloads Recap installer
    → Installs desktop application
    → Onboarding wizard:
        → Select audio input method (system audio / virtual audio device)
        → Choose transcription model (recommended: medium for balance)
        → Model download (one-time, ~1.5GB for medium)
        → Optional: configure output preferences (summary style, language)
    → "Test your setup" — record 30 seconds, verify transcription
    → Ready to use
```

**Key design principle:** Setup must complete in <10 minutes with no terminal/CLI interaction required for non-technical users.

### 5.2 Journey: Capturing a Meeting (Core Flow)

```
User starts a meeting (any platform: Zoom, Teams, Jitsi, Google Meet, in-person via mic)
    → Opens Recap (or it's already running in tray/menu bar)
    → Clicks "Record" (or uses hotkey)
    → [Optional] Metadata entry dialog appears (can skip, edit later)
        → Meeting type (dropdown: team meeting, 1:1, client call, interview, presentation, workshop, other)
        → Location (text field)
        → Participants (comma-separated or tag-style)
        → Language(s) (multi-select dropdown)
        → Topic/Project (text field)
    → Recap captures system audio (and optionally mic input)
    → Real-time transcription appears (streaming mode)
    → [Optional] User marks key moments during meeting (bookmark button)
    → Meeting ends → User clicks "Stop"
    → Processing:
        → Final transcription pass (batch mode for accuracy)
        → Speaker diarization
        → Summary generation (using selected LLM provider and meeting-type-specific prompt)
        → Action item extraction
    → Result presented:
        → Full transcript (editable)
        → Summary
        → Action items (with detected assignees/deadlines)
        → Decisions log
    → User reviews, edits if needed, saves
```

### 5.2.1 Journey: Importing Audio from Smartphone

```
User records meeting on smartphone (companion app or voice recorder app)
    → Opens Recap mobile companion OR uses iOS Shortcuts/Android share
    → Audio uploaded via local network to Recap desktop (HTTP POST or LocalSend protocol)
    → Recap detects new file in inbox folder
    → User prompted to enter meeting metadata (type, participants, language, topic)
    → Transcription begins (using selected STT provider)
    → Summarization proceeds (using selected LLM provider and meeting-type-specific prompt)
    → Result presented (same as core flow)
```

### 5.2.2 Journey: Uploading Audio File

```
User has existing audio recording (from phone, recorder, previous meeting, etc.)
    → Drags file into Recap window OR clicks "Import" button
    → OR places file in watched folder (~/RecapData/inbox/)
    → Recap detects new file
    → Audio format validated (M4A, WAV, MP3, OGG, WebM, FLAC)
    → User prompted for meeting metadata (can skip, edit later)
    → Processing begins (transcription → summarization → structured output)
    → Result presented (same as core flow)
```

### 5.2.3 Journey: Entering Meeting Metadata

```
User starts recording or imports audio file
    → Metadata entry dialog appears (can skip, edit later)
    → Fields:
        - Meeting title (auto-generated from date/time if blank)
        - Meeting type (dropdown: team meeting, 1:1, client call, interview, presentation, workshop, other)
        - Location (text field: "Conference Room A", "Zoom", "Remote")
        - Participants (comma-separated names or select from team roster)
        - Language(s) (dropdown: English, Spanish, French, German, etc. - multi-select)
        - Topic/Project (text field or select from tags)
    → Metadata stored with meeting record
    → Used to enhance summarization:
        - "client call" → prompt focuses on commitments, follow-ups, and next steps
        - "team meeting" → prompt focuses on blockers, action items, and decisions
        - "interview" → prompt focuses on candidate strengths, concerns, and evaluation
    → User can edit metadata later from meeting detail view
```

### 5.2.4 Journey: Configuring STT/LLM Providers

```
User wants to use a paid STT or LLM service instead of local models
    → Opens Settings → Providers
    → Selects STT provider:
        - Local (whisper.cpp) - default, free, requires model download
        - OpenAI Whisper API - paid, high accuracy, requires API key
        - AssemblyAI - paid, advanced features, requires API key
        - Deepgram - paid, real-time streaming, requires API key
        - AWS Transcribe - paid, enterprise, requires AWS credentials
        - Azure Speech - paid, enterprise, requires Azure key
        - Google Speech-to-Text - paid, enterprise, requires Google credentials
    → Enters API key (encrypted storage)
    → [Optional] Configures fallback provider (if primary fails, use fallback)
    → Selects LLM provider:
        - Local (llama.cpp/Ollama) - default, free, requires model download
        - OpenAI GPT-4 - paid, high quality, requires API key
        - Anthropic Claude - paid, strong reasoning, requires API key
        - Google Gemini - paid, multimodal, requires API key
        - AWS Bedrock - paid, enterprise, requires AWS credentials
        - Azure OpenAI - paid, enterprise, requires Azure key
    → Enters API key (encrypted storage)
    → [Optional] Tests connectivity to each provider
    → Views cost estimates (e.g., "$0.02/min for OpenAI Whisper API")
    → Saves settings
    → Next meeting processed using selected providers
```

### 5.2.5 Journey: Customizing Prompts

```
User wants to customize summarization prompts for better results
    → Opens Settings → Prompts
    → Views list of pre-built prompt templates:
        - "Standard Meeting Summary" (default)
        - "Client Call - Focus on Commitments"
        - "Team Meeting - Focus on Blockers"
        - "Interview Evaluation"
        - "Presentation Q&A Extraction"
    → Selects template to view/edit
    → Monaco editor displays prompt with variables:
        - {{transcript}} - full transcript text
        - {{meeting_type}} - meeting type from metadata
        - {{participants}} - list of participants
        - {{language}} - meeting language
    → User edits prompt (e.g., adds "Focus on technical decisions" for engineering meetings)
    → Clicks "Save as Custom" → creates user-edited version with incremented version number
    → [Optional] Previews prompt with sample transcript to see expected output
    → Clicks "Use This Prompt" → sets as active template
    → [Optional] Clicks "Reset to Default" → reverts to original template
    → Next meeting summary generated using custom prompt
```

### 5.3 Journey: Suite Integration (Paid Layer)

```
Meeting is saved in Recap
    → Sync engine evaluates sync rules:
        → Is this meeting tagged for Vault sync? (default: all meetings)
        → Are there action items for Loop sync? (default: all action items)
    → Vault sync:
        → Summary + decisions indexed into knowledge base
        → Linked to relevant Vault topics/projects (auto-tagged or user-configured)
        → Searchable alongside other Vault content
    → Loop sync:
        → Each action item becomes a task in Loop
        → Assignee mapped to team member (if recognized)
        → Due date set (if detected, or default: "next meeting")
        → Task linked back to source meeting transcript
    → User sees:
        → In Vault: meeting knowledge alongside project docs
        → In Loop: auto-created tasks with meeting context
        → In Recap: sync status indicators (synced/pending/error)
```

### 5.4 Journey: Team Administration (Paid Layer)

```
Admin deploys Recap for team:
    → Docker-based server deployment (or managed self-hosted)
    → Team members install desktop client, authenticate against local server
    → Admin configures:
        → Retention policies (e.g., delete recordings after 90 days, keep transcripts indefinitely)
        → Model defaults (organization-wide model selection)
        → Sync rules (which Vault namespaces, which Loop projects)
        → RBAC (who can see which meetings)
        → Audit log retention
    → Team members use Recap normally
    → Admin dashboard shows:
        → Usage metrics (meetings processed, storage used)
        → Sync health (Vault/Loop connection status)
        → Audit trail (who accessed what, policy changes)
        → Compliance status (retention adherence, data residency confirmation)
```

### 5.5 Journey: Searching & Retrieving Past Meetings

```
User needs to recall a decision from last month's meeting
    → Opens Recap search
    → Searches by keyword, date range, participant, or tag
    → Results show matching transcripts with highlighted context
    → User clicks into meeting → sees full transcript + summary + action items
    → If synced to Vault: can also find via Vault search (meeting appears as knowledge source)
    → If action item synced to Loop: can trace from task back to originating meeting
```

### 5.6 Journey: Model Management

```
User or admin wants to update/change transcription model:
    → Opens Settings → Models
    → Sees installed models + available downloads
    → Options:
        → Download new model (e.g., newer Whisper version, or alternative like Distil-Whisper)
        → Switch active model (immediate effect)
        → Delete unused models (free disk space)
    → For advanced users: custom model path (point to any compatible model on disk)
    → Model swap takes effect on next recording (no restart required)
```

---

## 6. Functional Requirements

### 6.1 Audio Capture & Input (FR-AC)

| ID | Requirement | Priority |
|---|---|---|
| FR-AC-01 | Capture system audio output (what other participants say) on macOS, Windows, Linux | P0 |
| FR-AC-02 | Capture microphone input (what the local user says) simultaneously | P0 |
| FR-AC-03 | Merge system audio + mic into unified transcript with speaker distinction | P0 |
| FR-AC-04 | Support virtual audio device setup (guide user through BlackHole/VB-Cable/PulseAudio setup if needed) | P0 |
| FR-AC-05 | Provide visual audio level indicators during recording | P1 |
| FR-AC-06 | Support manual start/stop via UI button and configurable hotkey | P0 |
| FR-AC-07 | Support "always listening" mode with voice-activity detection (record only when speech detected) | P2 |
| FR-AC-08 | Pause/resume recording within a single session | P1 |
| FR-AC-09 | Display recording duration and status in menu bar / system tray | P1 |
| FR-AC-10 | Support audio file import via drag-and-drop into application window | P0 |
| FR-AC-11 | Support audio file import via native file browser dialog | P0 |
| FR-AC-12 | Watch folder: monitor configured directory (default: ~/RecapData/inbox/) for new audio files | P1 |
| FR-AC-13 | Watch folder: auto-detect and process new audio files with debouncing (wait for file write completion) | P1 |
| FR-AC-14 | Local HTTP upload server: embedded Axum server for LAN-based smartphone audio transfer | P1 |
| FR-AC-15 | HTTP upload server: accept POST /upload/audio with multipart form data (audio file + optional metadata JSON) | P1 |
| FR-AC-16 | HTTP upload server: save uploaded files to inbox folder for processing | P1 |
| FR-AC-17 | Companion mobile app (Tauri v2) for iOS/Android recording with upload to desktop | P2 |
| FR-AC-18 | Support audio formats: M4A, WAV, MP3, OGG, WebM, FLAC | P0 |
| FR-AC-19 | Audio format validation: reject unsupported formats with clear error message | P0 |
| FR-AC-20 | Meeting metadata entry form: meeting type, location, participants, language(s), topic | P0 |
| FR-AC-21 | Metadata entry: appear after recording stops or file is imported (skippable, editable later) | P0 |
| FR-AC-22 | iOS Shortcuts integration for audio upload from iPhone | P2 |
| FR-AC-23 | LocalSend protocol support for zero-config phone→PC transfer (optional) | P2 |

### 6.2 Transcription (FR-TR)

| ID | Requirement | Priority |
|---|---|---|
| FR-TR-01 | Transcribe audio using open-source models (Whisper-family via whisper.cpp, faster-whisper) | P0 |
| FR-TR-02 | Support real-time streaming transcription (latency ≤2s) for local models | P0 |
| FR-TR-03 | Support batch re-transcription after meeting ends (higher accuracy pass) | P0 |
| FR-TR-04 | Speaker diarization (identify distinct speakers, label them) | P0 |
| FR-TR-05 | Allow user to name/label speakers post-recording | P1 |
| FR-TR-06 | Support model selection (small/medium/large or equivalent tiers) for local models | P0 |
| FR-TR-07 | Display transcription progress for batch processing | P1 |
| FR-TR-08 | Support punctuation and formatting in transcript output | P0 |
| FR-TR-09 | Handle overlapping speech gracefully (best-effort, not perfect separation) | P1 |
| FR-TR-10 | Support English with ≥95% of target market meetings (architecture for multilingual in future) | P0 |
| FR-TR-11 | Multiple STT backend support: local (whisper.cpp), paid APIs (OpenAI Whisper API, AssemblyAI, Deepgram), cloud (AWS Transcribe, Azure Speech, Google Speech-to-Text) | P0 |
| FR-TR-12 | User-selectable STT provider in settings with cost/quality tradeoffs displayed | P0 |
| FR-TR-13 | Fallback chain: if primary STT provider fails (network error, rate limit, timeout), automatically try secondary provider | P1 |
| FR-TR-14 | API key management for paid STT services (encrypted storage using AES-256-GCM) | P0 |
| FR-TR-15 | Per-meeting language selection (override default language for specific meetings) | P0 |
| FR-TR-16 | Cost estimation for paid STT APIs: display estimated cost before transcription begins (e.g., "$0.12 for 60-min meeting") | P2 |
| FR-TR-17 | Test connectivity: verify API key and network connectivity before saving provider settings | P0 |
| FR-TR-18 | Provider-specific features: support provider-specific capabilities (e.g., Deepgram real-time streaming, AssemblyAI speaker labels) | P1 |
| FR-TR-19 | Transcript format normalization: convert provider-specific output formats to unified internal format (timestamps, speaker labels, confidence scores) | P0 |

### 6.3 Structured Output (FR-SO)

| ID | Requirement | Priority |
|---|---|---|
| FR-SO-01 | Generate meeting summary (configurable length: brief/detailed) | P0 |
| FR-SO-02 | Extract action items with: description, detected assignee, detected deadline | P0 |
| FR-SO-03 | Extract and log decisions made during meeting | P0 |
| FR-SO-04 | Extract key topics/tags (auto-generated) | P1 |
| FR-SO-05 | Allow user to edit all generated outputs (summary, action items, decisions) | P0 |
| FR-SO-06 | Support custom output templates (user defines structure/format) | P2 |
| FR-SO-07 | Support summary generation via local LLM (user provides model) OR rule-based extraction (no LLM required for free tier) | P0 |
| FR-SO-08 | Generate "TL;DR" one-paragraph summary for quick scanning | P1 |
| FR-SO-09 | Highlight moments of agreement/disagreement in transcript | P2 |
| FR-SO-10 | Multiple LLM provider support: local (llama.cpp, Ollama), cloud (OpenAI GPT-4, Anthropic Claude, Google Gemini, AWS Bedrock, Azure OpenAI) | P0 |
| FR-SO-11 | User-selectable LLM provider in settings with cost/quality tradeoffs displayed | P0 |
| FR-SO-12 | Prompt template library: pre-built templates for different meeting types (team meeting, client call, interview, presentation, workshop) | P0 |
| FR-SO-13 | Editable prompts: users can customize summarization prompts via Monaco editor | P1 |
| FR-SO-14 | Prompt versioning: track changes to prompts, allow rollback to previous versions | P2 |
| FR-SO-15 | Structured output schemas: JSON Schema validation for action items, decisions (ensure LLM output matches expected format) | P0 |
| FR-SO-16 | Meeting-type-specific prompts: automatically select prompt based on meeting metadata (e.g., "client call" → focus on commitments and follow-ups) | P1 |
| FR-SO-17 | API key management for cloud LLM services (encrypted storage) | P0 |
| FR-SO-18 | Cost estimation for cloud LLM APIs: display estimated cost before summarization (e.g., "$0.05 for 10K token transcript") | P2 |
| FR-SO-19 | Prompt preview: test prompt with sample transcript to see expected output before applying | P2 |
| FR-SO-20 | Prompt variables: support dynamic variables in prompts ({{transcript}}, {{meeting_type}}, {{participants}}, {{language}}) | P1 |

### 6.4 Storage & Organization (FR-ST)

| ID | Requirement | Priority |
|---|---|---|
| FR-ST-01 | Store all data locally (recordings, transcripts, summaries, metadata) | P0 |
| FR-ST-02 | Encrypt data at rest (AES-256 or equivalent, key derived from user credential) | P0 |
| FR-ST-03 | Full-text search across all transcripts and summaries | P0 |
| FR-ST-04 | Filter/search by date, participant, tag, duration | P1 |
| FR-ST-05 | Manual tagging and folder/namespace organization | P1 |
| FR-ST-06 | Export individual meetings as Markdown, PDF, plain text, or JSON | P0 |
| FR-ST-07 | Bulk export (all meetings or filtered set) | P1 |
| FR-ST-08 | Display storage usage and per-meeting storage breakdown | P1 |
| FR-ST-09 | Configurable storage location (user chooses where data lives on disk) | P1 |

### 6.5 Vault Integration (FR-VI) — Paid Layer

| ID | Requirement | Priority |
|---|---|---|
| FR-VI-01 | Auto-sync meeting summaries to Vault knowledge base | P0 |
| FR-VI-02 | Auto-sync decisions to Vault as knowledge entries | P0 |
| FR-VI-03 | Link Vault entries back to source meeting (bi-directional) | P0 |
| FR-VI-04 | Configurable sync rules (which meetings sync, which namespaces) | P0 |
| FR-VI-05 | Auto-tag synced content based on meeting tags/topics | P1 |
| FR-VI-06 | Sync status indicator in Recap UI (pending/synced/error) | P0 |
| FR-VI-07 | Handle Vault unavailability gracefully (queue syncs, retry) | P0 |
| FR-VI-08 | Support incremental sync (only sync changes, not full re-sync) | P1 |

### 6.6 Loop Integration (FR-LI) — Paid Layer

| ID | Requirement | Priority |
|---|---|---|
| FR-LI-01 | Auto-create Loop tasks from extracted action items | P0 |
| FR-LI-02 | Map detected assignees to Loop team members | P0 |
| FR-LI-03 | Set due dates from detected deadlines (or configurable defaults) | P0 |
| FR-LI-04 | Link Loop tasks back to source meeting transcript | P0 |
| FR-LI-05 | Configurable sync rules (which action items sync, which Loop projects) | P0 |
| FR-LI-06 | Support triggering Loop workflows (not just task creation) based on meeting tags/content | P1 |
| FR-LI-07 | Sync status indicator in Recap UI | P0 |
| FR-LI-08 | Handle Loop unavailability gracefully (queue, retry) | P0 |

### 6.7 Team & Admin Features (FR-TA) — Paid Layer

| ID | Requirement | Priority |
|---|---|---|
| FR-TA-01 | Team workspace: shared meeting library with RBAC | P0 |
| FR-TA-02 | Role-based access: Admin, Manager, Member roles with configurable permissions | P0 |
| FR-TA-03 | Retention policies: auto-delete recordings/transcripts after configurable period | P0 |
| FR-TA-04 | Audit logs: record access events, policy changes, sync events | P0 |
| FR-TA-05 | Admin dashboard: usage metrics, sync health, storage, compliance status | P0 |
| FR-TA-06 | Centralized model management (admin sets default model for team) | P1 |
| FR-TA-07 | Centralized sync rule management | P1 |
| FR-TA-08 | SSO integration (SAML/OIDC) for team authentication | P1 |
| FR-TA-09 | Bulk user provisioning (SCIM or CSV import) | P2 |

### 6.8 Deployment & Installation (FR-DE)

| ID | Requirement | Priority |
|---|---|---|
| FR-DE-01 | Desktop installer for macOS (universal binary: Apple Silicon + Intel) | P0 |
| FR-DE-02 | Desktop installer for Windows (x64) | P0 |
| FR-DE-03 | Desktop package for Linux (.deb, .rpm, AppImage) | P0 |
| FR-DE-04 | Docker-based server deployment for team mode | P0 |
| FR-DE-05 | Auto-update mechanism for desktop client (with user approval) | P1 |
| FR-DE-06 | Offline-capable: all core features work without internet after initial setup/model download | P0 |
| FR-DE-07 | System requirements clearly documented; app checks compatibility on launch | P0 |

---

## 7. User Stories & Acceptance Criteria

### 7.1 Audio Capture Stories

**US-AC-01:** As a user in a Zoom meeting, I want Recap to capture what all participants are saying so that I get a complete transcript without adding a bot to the call.

*Acceptance Criteria:*
- Given Recap is recording, when system audio is playing from any application, then the audio is captured and transcribed in real-time.
- Given the user is on macOS, when they start recording, then Recap captures audio routed through the configured virtual audio device (BlackHole or equivalent).
- Given the user is on Windows, when they start recording, then Recap captures audio via WASAPI loopback.
- Given the user is on Linux, when they start recording, then Recap captures audio via PulseAudio/PipeWire monitor source.

**US-AC-02:** As a user, I want my own voice (microphone) captured alongside system audio so that my contributions appear in the transcript.

*Acceptance Criteria:*
- Given recording is active, when the user speaks into their microphone, then their speech appears in the transcript attributed to a distinct speaker label.
- Given both system audio and mic are active, then the transcript distinguishes between "remote participants" and "me" (or user-named labels).

**US-AC-03:** As a user, I want to start and stop recording with a keyboard shortcut so that I don't fumble through menus when a meeting starts.

*Acceptance Criteria:*
- Given Recap is running, when the user presses the configured hotkey (default: Ctrl+Shift+R / Cmd+Shift+R), then recording starts immediately.
- Given recording is active, when the user presses the hotkey again, then recording stops and processing begins.
- Given the user has customized the hotkey in settings, then the custom hotkey works globally (even when Recap is not focused).

### 7.2 Transcription Stories

**US-TR-01:** As a user, I want to see words appearing in real-time as people speak so that I can follow along during the meeting.

*Acceptance Criteria:*
- Given recording is active, then transcribed text appears in the UI within 2 seconds of speech.
- Given the user is in streaming mode, then text updates incrementally (not in large chunks after long pauses).
- Given the transcription model is processing, then a visual indicator shows the system is working.

**US-TR-02:** As a user, I want the transcript to identify different speakers so that I know who said what.

*Acceptance Criteria:*
- Given a meeting with 2+ speakers, when transcription completes, then the transcript labels segments with distinct speaker identifiers (Speaker 1, Speaker 2, etc.).
- Given the user renames speakers post-recording, then all instances of that speaker are updated throughout the transcript.
- Given a meeting with 1 speaker (monologue), then the transcript does not artificially create multiple speaker labels.

**US-TR-03:** As a user with a powerful machine, I want to use the largest available model for maximum accuracy.

*Acceptance Criteria:*
- Given the user selects the "large" model in settings, then subsequent recordings use that model.
- Given the user's machine lacks sufficient resources for the large model, then the app warns them and recommends a smaller model.
- Given the user switches models, then the change takes effect on the next recording without requiring app restart.

**US-TR-04:** As a user, I want a higher-accuracy batch pass after the meeting so that the final transcript is as correct as possible.

*Acceptance Criteria:*
- Given a meeting recording ends, then Recap automatically runs a batch transcription pass (using the selected model, non-streaming mode for higher accuracy).
- Given the batch pass completes, then the final transcript replaces the streaming transcript (streaming transcript is preserved in version history).
- Given the batch pass takes time, then the user is notified when it's complete and can view the updated transcript.

### 7.3 Structured Output Stories

**US-SO-01:** As a user, I want an automatic summary of my meeting so that I can quickly recall what was discussed without re-reading the full transcript.

*Acceptance Criteria:*
- Given a meeting transcript exists, then a summary is generated automatically (within 60 seconds of batch processing completing).
- Given the summary is generated, then it covers: key topics discussed, decisions made, and action items identified.
- Given the user edits the summary, then their edits are saved and the original generated summary is preserved in version history.

**US-SO-02:** As a user, I want action items extracted automatically so that I don't have to manually create tasks after every meeting.

*Acceptance Criteria:*
- Given a transcript contains phrases like "I'll handle X", "Can you follow up on Y", "Let's have this done by Friday", then these are extracted as action items.
- Given an action item is extracted, then it includes: description of the task, detected assignee (if stated), detected deadline (if stated).
- Given the user reviews extracted action items, then they can edit, add, remove, or reassign items before saving.

**US-SO-03:** As a user who doesn't want to set up a local LLM, I want basic summarization and action extraction to work without one.

*Acceptance Criteria:*
- Given no local LLM is configured, then Recap still produces: a structured outline (topics + timestamps), action items (via keyword/pattern matching), and a template-based summary.
- Given a local LLM IS configured, then summary quality improves (natural language summary vs. outline) and action item detection is more accurate (semantic understanding vs. pattern matching).
- Given the free tier, then rule-based extraction is available; LLM-powered extraction is available in paid tier or with user-provided local LLM.

### 7.4 Storage & Organization Stories

**US-ST-01:** As a user, I want all my meeting data stored locally and encrypted so that no one can access it without my permission.

*Acceptance Criteria:*
- Given the user has completed setup, then all recordings, transcripts, and summaries are stored on the local filesystem (not in any cloud service).
- Given data is stored, then it is encrypted at rest using AES-256 with a key derived from the user's passphrase.
- Given the user forgets their passphrase, then they are warned during setup that data recovery is not possible (no backdoor).

**US-ST-02:** As a user, I want to search across all my past meetings so that I can find specific discussions quickly.

*Acceptance Criteria:*
- Given the user enters a search term, then results appear from transcripts, summaries, and action items within 1 second (for up to 500 hours of meetings).
- Given search results are displayed, then each result shows: meeting date, matching text snippet (highlighted), and link to full transcript.
- Given the user filters by date range, then only meetings within that range are searched.

**US-ST-03:** As a user, I want to export a meeting as Markdown so that I can paste it into my notes tool.

*Acceptance Criteria:*
- Given the user selects "Export → Markdown" on a meeting, then a .md file is generated containing: meeting metadata (date, duration, participants), summary, action items, and full transcript.
- Given the export is complete, then the file is saved to the user's configured export directory (or Downloads by default).

### 7.5 Vault Integration Stories

**US-VI-01:** As a paying user, I want meeting summaries to automatically appear in my Vault knowledge base so that organizational knowledge accumulates without manual effort.

*Acceptance Criteria:*
- Given Vault sync is enabled, when a meeting is saved, then its summary and decisions are indexed into Vault within 60 seconds.
- Given the content is in Vault, then it is searchable alongside other Vault content.
- Given the user clicks the Vault entry, then they can navigate back to the full meeting transcript in Recap.

**US-VI-02:** As an admin, I want to configure which meetings sync to Vault so that only relevant content enters the knowledge base.

*Acceptance Criteria:*
- Given the admin configures a sync rule (e.g., "sync meetings tagged 'project-alpha' to Vault namespace 'Project Alpha'"), then only matching meetings are synced.
- Given a meeting does not match any sync rule, then it is not synced to Vault (but remains available in Recap).
- Given the admin changes a sync rule, then the change applies to future meetings (existing synced content is not retroactively removed unless explicitly configured).

### 7.6 Loop Integration Stories

**US-LI-01:** As a paying user, I want action items from meetings to automatically become tasks in Loop so that nothing falls through the cracks.

*Acceptance Criteria:*
- Given Loop sync is enabled, when a meeting with action items is saved, then each action item becomes a task in the configured Loop project within 60 seconds.
- Given an action item has a detected assignee, then the task is assigned to the matching Loop team member (or flagged for manual assignment if no match).
- Given an action item has a detected deadline, then the task due date is set accordingly.
- Given the user clicks the Loop task, then they can navigate back to the source meeting in Recap.

**US-LI-02:** As a user, I want to review and approve action items before they sync to Loop so that I maintain control over what becomes a tracked task.

*Acceptance Criteria:*
- Given the user has enabled "review before sync" in settings, then action items are held in a "pending review" state until the user approves them.
- Given the user approves selected action items, then only approved items sync to Loop.
- Given the user has NOT enabled "review before sync" (default: auto-sync), then all action items sync automatically.

### 7.7 Admin Stories

**US-TA-01:** As an admin, I want to set a retention policy so that recordings are automatically deleted after a defined period to meet compliance requirements.

*Acceptance Criteria:*
- Given the admin sets a retention policy (e.g., "delete recordings after 90 days, keep transcripts"), then recordings older than 90 days are automatically deleted.
- Given a recording is deleted, then the transcript and summary remain (unless separately configured for deletion).
- Given the admin views the audit log, then deletion events are recorded with timestamp and policy reference.

**US-TA-02:** As an admin, I want an audit log of who accessed which meeting data so that I can demonstrate compliance during audits.

*Acceptance Criteria:*
- Given a team member views a meeting transcript, then an audit entry is created: who, what, when.
- Given the admin views the audit log, then they can filter by user, date range, and event type.
- Given audit logs are configured for 1-year retention, then logs older than 1 year are archived (not deleted) per policy.

---

## 8. Non-Functional Requirements (NFRs)

### 8.1 Performance (NFR-PF)

| ID | Requirement | Target |
|---|---|---|
| NFR-PF-01 | Real-time transcription latency (speech-to-text appearance) | ≤2 seconds |
| NFR-PF-02 | Batch transcription speed (60-min meeting) | ≤10 minutes on reference hardware (M2 Mac, 16GB RAM) |
| NFR-PF-03 | Summary generation time (post-transcription) | ≤60 seconds |
| NFR-PF-04 | Search response time (full-text across 500 hours) | ≤1 second |
| NFR-PF-05 | Application startup time (cold start to ready) | ≤5 seconds |
| NFR-PF-06 | CPU usage during real-time transcription | ≤30% of one core (medium model) |
| NFR-PF-07 | Memory usage during real-time transcription | ≤2GB (medium model), ≤4GB (large model) |
| NFR-PF-08 | Disk I/O during recording | ≤5MB/s sustained write |

### 8.2 Accuracy (NFR-AC)

| ID | Requirement | Target |
|---|---|---|
| NFR-AC-01 | Word Error Rate (WER) on standard business English | ≤8% (large model), ≤12% (medium model) |
| NFR-AC-02 | Speaker diarization accuracy (speaker assignment) | ≥85% (for meetings with 2-6 speakers) |
| NFR-AC-03 | Action item extraction precision | ≥85% |
| NFR-AC-04 | Action item extraction recall | ≥75% |
| NFR-AC-05 | Summary factual accuracy (no hallucinated content) | ≥95% (claims in summary must be grounded in transcript) |

### 8.3 Security & Privacy (NFR-SP)

| ID | Requirement | Target |
|---|---|---|
| NFR-SP-01 | Zero data exit: no meeting data (audio, transcript, summary, metadata) leaves the device/infrastructure | 100% — verifiable via network monitoring |
| NFR-SP-02 | Encryption at rest | AES-256-GCM, key derived via Argon2id from user passphrase |
| NFR-SP-03 | Encryption in transit (for team/server mode, between client and local server) | TLS 1.3, self-signed or user-provided certs |
| NFR-SP-04 | No telemetry containing meeting content | Zero content telemetry; only anonymous usage metrics (opt-in) |
| NFR-SP-05 | No external network calls during core operation | Verified via network isolation testing |
| NFR-SP-06 | Secure deletion (for retention policy enforcement) | Cryptographic erasure or secure overwrite (configurable) |
| NFR-SP-07 | Authentication for team mode | Local auth + optional SAML/OIDC SSO |
| NFR-SP-08 | Dependency security | All dependencies audited; no known critical CVEs at release |

### 8.4 Reliability & Availability (NFR-RA)

| ID | Requirement | Target |
|---|---|---|
| NFR-RA-01 | Application crash rate | <0.5% of sessions |
| NFR-RA-02 | Data loss on crash | Zero — recording buffer flushed to disk every 5 seconds |
| NFR-RA-03 | Recovery from interrupted recording | If app crashes mid-recording, audio captured up to last flush is recoverable |
| NFR-RA-04 | Sync reliability (Vault/Loop) | ≥99.5% successful sync; failed syncs queued and retried |
| NFR-RA-05 | Offline operation | Core features (capture, transcribe, summarize, store) work fully offline |

### 8.5 Scalability (NFR-SC)

| ID | Requirement | Target |
|---|---|---|
| NFR-SC-01 | Local storage capacity | Support ≥10,000 hours of meetings per installation |
| NFR-SC-02 | Team size (server mode) | Support ≥200 concurrent users per server instance |
| NFR-SC-03 | Search index size | Performant search across ≥10,000 hours of transcripts |
| NFR-SC-04 | Sync throughput (Vault/Loop) | Process ≥100 meeting syncs per hour without backlog |

### 8.6 Usability (NFR-US)

| ID | Requirement | Target |
|---|---|---|
| NFR-US-01 | Time to first transcript (install → first recording complete) | ≤15 minutes (including model download) |
| NFR-US-02 | Setup requiring terminal/CLI | Zero for free core (GUI-only setup) |
| NFR-US-03 | Accessibility | WCAG 2.1 AA compliance for UI; keyboard-navigable |
| NFR-US-04 | Localization | UI in English (v1.0); architecture supports i18n |
| NFR-US-05 | Error messages | Human-readable, actionable; no raw stack traces to end users |

### 8.7 Compatibility (NFR-CO)

| ID | Requirement | Target |
|---|---|---|
| NFR-CO-01 | macOS | 12 (Monterey) and later; Apple Silicon + Intel |
| NFR-CO-02 | Windows | 10 (1903+) and 11; x64 |
| NFR-CO-03 | Linux | Ubuntu 22.04+, Fedora 38+, Debian 12+; x64 |
| NFR-CO-04 | Meeting platforms (via system audio) | Any platform that outputs audio to system (Zoom, Teams, Meet, Jitsi, Webex, etc.) |
| NFR-CO-05 | Hardware minimum | 8GB RAM, 4-core CPU, 10GB free disk (for app + medium model) |
| NFR-CO-06 | Hardware recommended | 16GB RAM, 6+ cores (or Apple Silicon), SSD, 20GB free disk |

### 8.8 Maintainability (NFR-MN)

| ID | Requirement | Target |
|---|---|---|
| NFR-MN-01 | Model updates | New transcription models can be added without app update (downloadable model registry) |
| NFR-MN-02 | Configuration | All settings stored in human-readable config files (YAML/JSON) |
| NFR-MN-03 | Logging | Structured logs (JSON) for debugging; configurable log levels |
| NFR-MN-04 | Plugin architecture | Transcription backends are pluggable (interface-based) for future model support |

---

## 9. Data & State Requirements

### 9.1 Core Data Entities

#### Meeting
```
Meeting {
  id: UUID
  title: string (auto-generated or user-set)
  created_at: timestamp
  duration_seconds: integer
  status: enum [recording, processing, ready, archived, deleted]
  audio_file_path: string (encrypted path)
  audio_format: string (wav/ogg/opus/m4a/mp3/webm/flac)
  audio_size_bytes: integer
  transcript_id: UUID → Transcript
  summary_id: UUID → Summary
  tags: [string]
  participants: [Participant]
  source_app: string (detected: "Zoom", "Teams", etc. or "Unknown")
  bookmarks: [Bookmark]
  sync_status: SyncStatus
  retention_policy_applied: boolean
  deleted_at: timestamp? (null if not deleted)
  
  // New metadata fields for enhanced summarization
  meeting_type: enum [team_meeting, one_on_one, client_call, interview, presentation, workshop, other]
  location: string? (e.g., "Conference Room A", "Zoom", "Remote")
  language: string (ISO 639-1 code, e.g., "en", "es", "fr", "de")
  topic: string? (project or subject matter)
  
  // Audio source tracking
  audio_source: enum [system_capture, file_import, mobile_upload, watch_folder]
  
  // Provider tracking
  stt_provider: enum [whisper_local, openai_api, assemblyai, deepgram, aws_transcribe, azure_speech, google_speech]
  llm_provider: enum [llama_local, ollama, openai, anthropic, google_gemini, aws_bedrock, azure_openai]
  
  // Prompt tracking
  prompt_template_used: string? (name of prompt template applied)
}
```

#### Transcript
```
Transcript {
  id: UUID
  meeting_id: UUID → Meeting
  version: integer (incremented on batch re-process)
  segments: [TranscriptSegment]
  speakers: [Speaker]
  language: string (detected, default "en")
  model_used: string (e.g., "whisper-large-v3")
  created_at: timestamp
  updated_at: timestamp
  word_count: integer
  confidence_score: float (average segment confidence)
}
```

#### TranscriptSegment
```
TranscriptSegment {
  id: UUID
  transcript_id: UUID → Transcript
  speaker_id: UUID → Speaker
  start_time_ms: integer
  end_time_ms: integer
  text: string
  confidence: float
  is_streaming_draft: boolean (true if from real-time pass, false if from batch pass)
}
```

#### Speaker
```
Speaker {
  id: UUID
  transcript_id: UUID → Transcript
  diarization_label: string (e.g., "SPEAKER_00")
  display_name: string? (user-assigned, null if unnamed)
  is_local_user: boolean
  embedding: vector? (for speaker identification across meetings — future)
}
```

#### Summary
```
Summary {
  id: UUID
  meeting_id: UUID → Meeting
  version: integer
  content: string (markdown)
  tldr: string (one paragraph)
  generated_by: enum [rule_based, local_llm]
  model_used: string? (if local_llm)
  created_at: timestamp
  updated_at: timestamp
  user_edited: boolean
}
```

#### ActionItem
```
ActionItem {
  id: UUID
  meeting_id: UUID → Meeting
  description: string
  assignee: string? (detected name or user-assigned)
  assignee_matched_to_team_member: UUID? (if matched in Loop)
  due_date: date? (detected or user-set)
  source_text: string (exact quote from transcript)
  source_segment_id: UUID → TranscriptSegment
  status: enum [pending, synced, completed, cancelled]
  loop_task_id: UUID? (if synced to Loop)
  sync_status: enum [pending_review, approved, synced, sync_failed, not_applicable]
  created_at: timestamp
  updated_at: timestamp
}
```

#### Decision
```
Decision {
  id: UUID
  meeting_id: UUID → Meeting
  description: string
  context: string (surrounding discussion)
  source_segment_ids: [UUID] → TranscriptSegment
  decided_by: [string] (participants who agreed)
  vault_entry_id: UUID? (if synced to Vault)
  sync_status: enum [pending, synced, sync_failed, not_applicable]
  created_at: timestamp
}
```

#### Bookmark
```
Bookmark {
  id: UUID
  meeting_id: UUID → Meeting
  timestamp_ms: integer
  label: string? (user-added note)
  created_at: timestamp
}
```

#### SyncStatus
```
SyncStatus {
  vault_synced: boolean
  vault_synced_at: timestamp?
  vault_entry_id: UUID?
  loop_synced: boolean
  loop_synced_at: timestamp?
  loop_task_ids: [UUID]
  last_error: string?
  retry_count: integer
}
```

### 9.2 Configuration State

```
AppConfig {
  // Audio Capture
  audio_input_method: enum [system_audio, virtual_device, manual]
  virtual_device_name: string?
  mic_device_id: string?
  hotkey_start_stop: string (default: "Ctrl+Shift+R" / "Cmd+Shift+R")
  
  // Audio Input Flexibility
  watch_folder_enabled: boolean (default: false)
  watch_folder_path: string (default: "~/RecapData/inbox")
  http_upload_server_enabled: boolean (default: false)
  http_upload_server_port: number (default: 8765)
  
  // Transcription - Local Models
  active_model: string (e.g., "whisper-medium")
  models_installed: [ModelInfo]
  model_download_dir: string
  streaming_enabled: boolean (default: true)
  batch_rerun_enabled: boolean (default: true)
  diarization_enabled: boolean (default: true)
  
  // Transcription - Provider Selection
  stt_provider: enum [whisper_local, openai_api, assemblyai, deepgram, aws_transcribe, azure_speech, google_speech]
  stt_fallback_provider: enum? (secondary provider if primary fails)
  
  // Transcription - API Keys (encrypted storage)
  stt_openai_api_key: string?
  stt_assemblyai_api_key: string?
  stt_deepgram_api_key: string?
  stt_aws_access_key: string?
  stt_aws_secret_key: string?
  stt_azure_key: string?
  stt_google_credentials: string? (path to JSON file)
  
  // Summarization - Provider Selection
  summary_provider: enum [rule_based, local_llm, openai, anthropic, google_gemini, aws_bedrock, azure_openai]
  llm_provider: enum [llama_local, ollama, openai, anthropic, google_gemini, aws_bedrock, azure_openai]
  llm_model: string (e.g., "gpt-4", "claude-3-sonnet", "gemini-pro", "llama-3-8b-instruct")
  
  // Summarization - API Keys (encrypted storage)
  llm_openai_api_key: string?
  llm_anthropic_api_key: string?
  llm_google_api_key: string?
  llm_aws_access_key: string?
  llm_aws_secret_key: string?
  llm_azure_key: string?
  
  // Summarization - Local Models
  llm_model_path: string? (if local_llm or ollama)
  
  // Prompts
  prompt_template: string (default: "standard_meeting_summary")
  prompt_custom: string? (user-edited prompt content)
  prompt_version: number (for versioning)
  summary_length: enum [brief, detailed]
  
  // Storage
  data_directory: string (default: ~/RecapData or platform equivalent)
  encryption_enabled: boolean (default: true)
  export_directory: string
  
  // Sync (paid)
  vault_sync_enabled: boolean
  vault_sync_rules: [SyncRule]
  loop_sync_enabled: boolean
  loop_sync_rules: [SyncRule]
  loop_default_project: UUID?
  auto_sync: boolean (default: true)
  review_before_sync: boolean (default: false)
  
  // Team/Admin (paid)
  server_url: string? (for team mode)
  auth_token: string? (encrypted)
  retention_policy: RetentionPolicy?
  
  // Privacy
  telemetry_enabled: boolean (default: false, opt-in)
  update_check_enabled: boolean (default: true)
}
```

### 9.3 State Transitions

#### Meeting Lifecycle
```
[created] → [recording] → [processing] → [ready] → [archived] → [deleted]
                                    ↑
                                    └── [ready] (re-processing triggered by model change or manual re-run)
```

#### Action Item Lifecycle
```
[extracted] → [pending_review] → [approved] → [synced] → [completed]
                    ↓                               ↓
              [rejected]                     [sync_failed] → [approved] (retry)
```

#### Sync State Machine
```
[not_configured] → [pending] → [syncing] → [synced]
                                    ↓
                              [failed] → [retrying] → [synced]
                                              ↓
                                        [permanently_failed] (after max retries)
```

### 9.4 Data Retention & Lifecycle

| Data Type | Default Retention | Configurable | Deletion Method |
|---|---|---|---|
| Audio recordings | Until manually deleted or policy-triggered | Yes (admin policy) | Secure delete (overwrite) |
| Transcripts | Until manually deleted or policy-triggered | Yes (admin policy) | Secure delete |
| Summaries | Until manually deleted or policy-triggered | Yes (admin policy) | Secure delete |
| Action items | Until marked completed + retention period | Yes | Soft delete → hard delete after grace period |
| Sync queue (failed) | 30 days | No (fixed) | Auto-purge after expiry |
| Audit logs | 1 year (configurable) | Yes | Archive → delete |
| Application logs | 30 days | Yes | Rotation + delete |
| Search index | Mirrors transcript lifecycle | N/A | Rebuilt on transcript deletion |

### 9.5 Data Flow Diagram (Conceptual)

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER DEVICE                               │
│                                                                   │
│  ┌──────────┐    ┌──────────────┐    ┌─────────────────────┐   │
│  │  Audio    │───▶│ Transcription│───▶│  Structured Output  │   │
│  │  Capture  │    │   Engine     │    │  (Summary, Actions, │   │
│  │           │    │  (Whisper)   │    │   Decisions)        │   │
│  └──────────┘    └──────────────┘    └─────────────────────┘   │
│        │                                       │                  │
│        ▼                                       ▼                  │
│  ┌──────────┐                          ┌─────────────────────┐   │
│  │  Local   │                          │   Sync Engine       │   │
│  │  Storage │◀─────────────────────────│   (Vault / Loop)    │   │
│  │ (Encrypted│                         │                     │   │
│  └──────────┘                          └─────────────────────┘   │
│                                                    │              │
└────────────────────────────────────────────────────│──────────────┘
                                                     │
                                          ┌──────────▼──────────┐
                                          │  LOCAL SERVER        │
                                          │  (Team mode only)    │
                                          │  ┌───────┐ ┌─────┐  │
                                          │  │ Vault │ │Loop │  │
                                          │  └───────┘ └─────┘  │
                                          └──────────────────────┘
```

**Key invariant:** No arrow crosses the device boundary. All processing and storage is local. In team mode, the "local server" is on the customer's infrastructure (self-hosted), not ODW.ai's cloud.

### 9.6 Data Portability

| Capability | Supported | Format |
|---|---|---|
| Export single meeting | Yes | Markdown, PDF, JSON, plain text |
| Bulk export | Yes | JSON (full data) or per-meeting Markdown files |
| Import from other tools | No (v1.0) | N/A |
| Data migration on uninstall | User copies data directory | All data in single configurable directory |
| API access to data | No public API (v1.0) | N/A — export is the portability mechanism |

---

## 10. Assumptions & Constraints

### 10.1 Assumptions

| ID | Assumption | Risk if Wrong |
|---|---|---|
| A-01 | Whisper-family models (or successors) will continue to improve in accuracy, approaching cloud ASR quality | Product quality ceiling may be lower than expected; users compare unfavorably to Otter/Fireflies |
| A-02 | Target users have machines capable of running medium/large Whisper models (8GB+ RAM, modern CPU or Apple Silicon) | Addressable market shrinks if hardware requirements are too high |
| A-03 | System audio capture is legally permissible in the user's jurisdiction for their own meetings (one-party consent or business exception) | Legal risk in two-party consent jurisdictions; mitigated by user responsibility disclaimer |
| A-04 | ODW.ai Vault and Loop modules will be available and stable for integration at Recap v1.0 launch | If Vault/Loop are delayed, paid layer value proposition is weakened |
| A-05 | Target market (regulated SMBs) will pay for suite bundle even if free-core meets basic transcription needs | Revenue model fails if conversion rate is too low |
| A-06 | Local LLMs (for summarization) are optional enhancement, not required for acceptable free-tier output | If rule-based summarization is too poor, free tier feels incomplete |
| A-07 | Meeting platforms (Zoom, Teams, etc.) will not block system audio capture in ways that break our approach | Platform updates could break capture; mitigated by multiple capture methods |
| A-08 | Speaker diarization quality from open models is sufficient for business meetings (2-8 speakers, relatively clean audio) | If diarization is poor, user experience degrades significantly |
| A-09 | Customers in regulated contexts have IT resources to deploy self-hosted server mode (Docker) | If deployment is too complex, adoption stalls; mitigated by guided setup |
| A-10 | The "no cloud bot" positioning resonates strongly enough to drive adoption over more convenient cloud alternatives | If users don't value sovereignty enough, differentiation fails |

### 10.2 Constraints

| ID | Constraint | Impact |
|---|---|---|
| C-01 | **No cloud processing** — this is the core product promise; cannot be relaxed even for quality improvements | Limits model options to what runs locally; cannot use cloud APIs as fallback |
| C-02 | **Bundle-first pricing** — Recap is not sold as a standalone product; revenue depends on suite attachment | Limits standalone market; requires suite sales motion |
| C-03 | **On-device resource limits** — transcription and summarization compete with user's other applications for CPU/RAM/GPU | Performance ceiling tied to user hardware; cannot guarantee uniform experience |
| C-04 | **Open-source model dependency** — core transcription relies on community-maintained models (Whisper, etc.) | Roadmap partially dependent on external project velocity |
| C-05 | **English-only for v1.0** — multilingual support deferred despite market demand | Limits international market in v1.0; architecture must support future expansion |
| C-06 | **Desktop-only for v1.0** — no mobile; meetings are assumed to happen on desktop | Misses phone-call recording use cases; acceptable for target market |
| C-07 | **Self-hosted only for team mode** — no ODW.ai-managed cloud option | Limits convenience for teams without IT resources; by design for sovereignty |
| C-08 | **No meeting platform bots** — audio capture only via system audio | Cannot capture meetings the user isn't present for; by design (no external participant) |
| C-09 | **Regulatory landscape uncertainty** — regulations around AI processing of meeting data are evolving | May need to adapt; on-device positioning is defensively strong here |
| C-10 | **Model download size** — large models are 1.5-3GB; users need bandwidth and disk space | Onboarding friction for users with limited bandwidth; mitigated by offering smaller models |

---

## 11. Risks & Mitigations

### 11.1 Technical Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R-01 | **Transcription accuracy below user expectations** — Whisper WER of 8-12% may feel poor vs. cloud tools claiming <5% | High | High | Invest in post-processing (punctuation, formatting, context-aware correction); clearly set expectations; enable batch re-processing for accuracy; support model swapping so power users can try alternatives |
| R-02 | **System audio capture breaks on OS updates** — macOS/Windows/Linux audio subsystems change, breaking capture | Medium | High | Abstract capture behind platform-specific adapters; maintain compatibility matrix; rapid-response patch process; support multiple capture methods per platform as fallback |
| R-03 | **Performance on low-end hardware** — users with 8GB RAM machines have poor experience with medium/large models | Medium | Medium | Default to "small" model on low-spec machines; clearly communicate hardware requirements; offer tiered experience (small model = faster, less accurate); graceful degradation |
| R-04 | **Speaker diarization quality insufficient** — open-source diarization may struggle with overlapping speech or similar voices | Medium | Medium | Use best-available open diarization (pyannote, neMo); allow manual speaker correction; invest in post-processing to merge/split speakers; clearly label confidence |
| R-05 | **Local LLM summarization quality gap** — rule-based free tier may feel too limited; local LLMs may hallucinate | Medium | Medium | Rule-based tier produces structured outlines (not prose) to avoid hallucination risk; LLM tier includes grounding checks (summary claims must trace to transcript segments); clear labeling of generation method |
| R-06 | **Storage consumption** — audio recordings are large (60-min meeting ≈ 50-100MB WAV); users fill disk | Medium | Low | Default to compressed format (Opus/OGG, ~10x smaller); configurable quality settings; storage usage dashboard; retention policies to auto-clean |
| R-07 | **Model supply chain risk** — dependency on Whisper model weights hosted externally (e.g., Hugging Face) | Low | High | Mirror models on ODW.ai infrastructure; support multiple download sources; document manual model installation for air-gapped environments |

### 11.2 Market & Business Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R-08 | **Commoditization by platform vendors** — Zoom/Teams/Meet add built-in transcription, reducing perceived need for Recap | High | High | Position Recap as suite-integrated (Vault/Loop value), not just transcription; emphasize sovereignty (platform features are cloud-processed); target regulated contexts where platform features are blocked |
| R-09 | **Low free-to-paid conversion** — users satisfied with free core don't upgrade to suite bundle | Medium | High | Make Vault/Loop integration visibly valuable (demo workflows); time-limited trial of paid features; case studies showing ROI from meeting→knowledge→action automation |
| R-10 | **Competitor privacy-washing** — cloud tools add "privacy" marketing (e.g., "we don't train on your data") blurring differentiation | Medium | Medium | Maintain verifiable architecture (open-source, auditable); publish transparency reports; offer network-monitoring tools so customers can verify zero data exit |
| R-11 | **Narrow TAM** — "regulated SMBs who want on-device meeting intelligence" may be too small | Medium | High | Expand to privacy-conscious (not just regulated) market; emphasize suite value (Recap as gateway to Vault/Loop adoption); international expansion with multilingual support |
| R-12 | **Enterprise sales complexity** — self-hosted deployment requires sales/engineering support | Medium | Medium | Productize deployment (Docker Compose one-liner, guided setup wizard); build self-service admin tools; create partner channel for deployment services |

### 11.3 Operational Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R-13 | **Support burden from self-hosted deployments** — diverse infrastructure environments create unique issues | High | Medium | Comprehensive documentation; community forum; structured bug reports; known-environment testing matrix; FAQ for common deployment issues |
| R-14 | **Model update coordination** — new Whisper versions may change output format or require migration | Medium | Low | Version-pin models; test new models before recommending; support multiple model versions simultaneously; migration scripts for data format changes |
| R-15 | **Legal liability from meeting recording** — users may record without consent in two-party-consent jurisdictions | Low | High | Prominent legal disclaimer in onboarding; documentation on consent requirements; no legal advice (user responsibility); consult legal counsel on liability shielding |
| R-16 | **Key person dependency** — small team maintaining complex multi-platform audio capture + ML pipeline | Medium | Medium | Document architecture thoroughly; modular design (platform adapters are independent); cross-train team members; engage community contributors for platform-specific fixes |

### 11.4 Risk Matrix Summary

```
                    ┌─────────────────────────────────┐
          High      │  R-01, R-08, R-11              │
                    │  (Accuracy, Commoditization,    │
   IMPACT           │   TAM)                          │
                    │                                  │
          Medium    │  R-02, R-03, R-04, R-05,       │
                    │  R-09, R-10, R-12, R-13, R-16  │
                    │                                  │
          Low       │  R-06, R-07, R-14              │
                    │                                  │
                    └──────┬──────────┬───────────────┘
                        Low        Medium       High
                              LIKELIHOOD
```

**Top 3 risks requiring active mitigation:** R-01 (accuracy expectations), R-08 (commoditization), R-11 (TAM size).

---

## 12. Dependencies

### 12.1 Internal Dependencies (ODW.ai Suite)

| ID | Dependency | Owner | Status | Criticality | Notes |
|---|---|---|---|---|---|
| D-01 | **Vault module** — Recap syncs summaries/decisions to Vault knowledge base | Vault team | In development | Critical (paid layer) | Paid layer value depends on Vault availability; API contract needed by Recap v1.0 beta |
| D-02 | **Loop module** — Recap syncs action items to Loop as tasks/workflows | Loop team | In development | Critical (paid layer) | Same as D-01; Loop task creation API needed |
| D-03 | **ODW.ai authentication/identity** — shared auth for suite (SSO, user management) | Platform team | Available | High (team mode) | Team mode depends on shared identity; can use local auth for single-user free tier |
| D-04 | **ODW.ai billing/subscription** — suite bundle pricing and license management | Business team | Available | Medium | Needed for paid tier activation; can defer to post-launch if free core launches first |
| D-05 | **ODW.ai design system** — shared UI components for consistent suite experience | Design team | Available | Medium | Ensures Recap feels part of the suite; can build custom UI if needed |

### 12.2 External Dependencies (Open Source / Third-Party)

| ID | Dependency | Purpose | License | Risk | Fallback |
|---|---|---|---|---|---|
| D-06 | **Whisper / faster-whisper / whisper.cpp** | Core transcription engine | MIT | Low (stable, well-maintained) | Alternative Whisper implementations; model-agnostic architecture |
| D-07 | **pyannote-audio or neMo** | Speaker diarization | Apache 2.0 / Apache 2.0 | Medium (pyannote has gated model access) | Fallback to simpler energy-based segmentation; or neMo |
| D-08 | **llama.cpp / Ollama / local LLM runtime** | Local LLM inference for summarization (paid/optional) | MIT / MIT | Low | Rule-based fallback (always available); user can provide any compatible local LLM |
| D-09 | **BlackHole (macOS) / VB-Cable (Windows) / PulseAudio (Linux)** | Virtual audio device for system audio capture | GPL / Various | Low | Platform-native loopback APIs (WASAPI on Windows, PipeWire on Linux); BlackHole alternative: Soundflower (legacy) |
| D-10 | **SQLite** | Local database for meeting metadata, search index | Public Domain | Very Low | No realistic alternative needed |
| D-11 | **Electron / Tauri** | Desktop application framework | MIT / Apache 2.0 | Low | Tauri preferred (smaller binary, lower resource usage); Electron as fallback |
| D-12 | **Docker** | Server-mode deployment for teams | Apache 2.0 | Very Low | No alternative needed for containerized deployment |
| D-13 | **Hugging Face Hub** | Model distribution (Whisper weights, diarization models) | Various | Medium (availability, gating) | Mirror models on ODW.ai infrastructure; support manual model file placement |

### 12.3 Platform Dependencies

| ID | Dependency | Risk | Mitigation |
|---|---|---|---|
| D-14 | **macOS audio subsystem (CoreAudio)** | OS updates may change behavior | Test on macOS beta; abstract behind adapter; support both virtual device and (future) ScreenCaptureKit audio |
| D-15 | **Windows audio subsystem (WASAPI)** | OS updates may change loopback behavior | Test on Windows Insider builds; WASAPI loopback is stable API |
| D-16 | **Linux audio (PulseAudio / PipeWire)** | Fragmented ecosystem; multiple audio servers | Support both PulseAudio and PipeWire; detect at runtime; test on Ubuntu + Fedora |
| D-17 | **Meeting platform audio output** | Platforms may change how they output audio (e.g., exclusive mode) | System audio capture is platform-agnostic (captures whatever is output); unlikely to break unless platforms move to exclusive audio paths |

### 12.4 Dependency Timeline

```
Month 1-2:  Lock D-06 (transcription engine), D-09 (audio capture), D-11 (app framework)
Month 2-3:  Lock D-07 (diarization), D-08 (LLM runtime), D-10 (database)
Month 3-4:  Finalize D-01/D-02 API contracts with Vault/Loop teams
Month 4-5:  Integration testing with Vault/Loop (D-01, D-02)
Month 5-6:  Final dependency audit; all D-* items locked and tested
```

---

## 13. Open Questions

### 13.1 Product & Strategy

| ID | Question | Owner | Due Date | Impact |
|---|---|---|---|---|
| Q-01 | Should we offer a "cloud transcription" option as a premium tier (contradicting sovereignty positioning) for users who want higher accuracy and don't have sensitive content? | Product Lead | Before v1.1 | Could expand TAM but dilutes positioning; decision needed on whether sovereignty is absolute or tiered |
| Q-02 | What is the minimum viable Vault/Loop integration for v1.0 paid layer? Can we launch paid tier with basic sync and expand, or does it need full bi-directional linking at launch? | Product + Vault/Loop teams | Before v1.0 feature freeze | Affects paid tier value proposition and launch timeline |
| Q-03 | Should Recap support recording meetings the user isn't present for (e.g., "send Recap to this meeting" via calendar integration)? | Product Lead | Post-v1.0 | Expands use cases but requires bot/participant approach (contradicts no-bot design); likely out of scope |
| Q-04 | How do we handle the "meeting intelligence for sales" use case (deal signals, coaching, etc.)? Do we build vertical features or stay horizontal? | Product + Business | Post-v1.0 | Sales vertical is high-value but narrow; horizontal positioning is broader; may revisit based on demand |
| Q-05 | Should we open-source the core transcription pipeline to build community trust and contributions, or keep it proprietary? | Leadership | Before v1.0 launch | Open-source builds trust (aligns with sovereignty message) but reduces IP moat; partial open-source (adapters, not models) is a middle ground |

### 13.2 Technical

| ID | Question | Owner | Due Date | Impact |
|---|---|---|---|---|
| Q-06 | What is the best real-time transcription architecture: streaming Whisper (whisper.cpp with streaming patches) vs. chunked processing vs. alternative streaming ASR models (e.g., Distil-Whisper, WhisperLive)? | Engineering | Month 2 | Affects latency, accuracy, and resource usage; prototype needed early |
| Q-07 | Can we achieve acceptable speaker diarization with purely local models, or do we need a hybrid approach (local embedding extraction + lightweight clustering)? | ML Engineering | Month 3 | Diarization quality directly affects user experience; may need to invest in custom pipeline |
| Q-08 | What is the optimal storage format for encrypted recordings? Full-disk encryption (OS-level) vs. application-level encryption (per-file AES-256)? | Security Engineering | Month 2 | Affects performance, portability, and security guarantees; app-level is more portable but more complex |
| Q-09 | Should the search index be full-text (SQLite FTS5) or vector-based (for semantic search)? Or both? | Engineering | Month 3 | Full-text is simpler and sufficient for v1.0; semantic search is a differentiator for v1.1+ |
| Q-10 | How do we handle model updates that change output format (e.g., new Whisper version with different timestamp granularity)? Do we need a transcript migration system? | Engineering | Month 4 | Affects data model stability; design for forward-compatibility from the start |
| Q-11 | What is the minimum viable team/server architecture? Single Docker container vs. microservices? What about high availability? | Infrastructure | Month 3 | Affects deployment complexity and scalability; start simple (single container) with documented scaling path |

### 13.3 Go-to-Market

| ID | Question | Owner | Due Date | Impact |
|---|---|---|---|---|
| Q-12 | Do we launch free core first (build user base) and add paid layer later, or launch both simultaneously? | Product + Business | Before launch | Free-first builds community and validates quality; simultaneous launch captures revenue earlier; recommendation: free core first (4-6 week head start) |
| Q-13 | What is the pricing model for the paid layer? Per-seat? Per-meeting-hour? Flat bundle with Vault/Loop? | Business | Before paid launch | Affects conversion and revenue; bundle pricing aligns with suite strategy; per-seat is standard for team features |
| Q-14 | How do we reach regulated SMBs? Direct sales? Partner channel (MSPs, compliance consultants)? Community/word-of-mouth? | Marketing + Sales | Before launch | Regulated SMBs are reached through trust channels (consultants, peer recommendations); community credibility matters |
| Q-15 | Should we pursue SOC 2 / HIPAA / FedRAMP certifications for the self-hosted deployment, or is the on-device architecture sufficient for compliance claims? | Compliance + Legal | Post-v1.0 | Certifications are expensive but unlock enterprise deals; on-device architecture may qualify for lighter compliance paths; legal review needed |

### 13.4 Legal & Compliance

| ID | Question | Owner | Due Date | Impact |
|---|---|---|---|---|
| Q-16 | What is our liability exposure if a user records a meeting without proper consent in a two-party-consent jurisdiction? | Legal | Before launch | Need clear Terms of Service, in-app disclaimers, and possibly jurisdiction-aware warnings; legal review of model language |
| Q-17 | Do we need to provide a "data processing agreement" (DPA) for EU customers, even though we don't process their data (it's all on-device)? | Legal + Compliance | Before EU launch | EU customers may request DPAs as standard procurement; need a template that clarifies on-device architecture means we're not a data processor |
| Q-18 | What open-source license obligations do we inherit from Whisper, pyannote, llama.cpp, etc.? Do any require source disclosure of our modifications? | Legal + Engineering | Before launch | Most are MIT/Apache (permissive); pyannote has gated model access (not a code license issue); verify all dependencies and document obligations |

### 13.5 User Experience

| ID | Question | Owner | Due Date | Impact |
|---|---|---|---|---|
| Q-19 | How do we handle the "first run" experience when no virtual audio device is installed? Do we auto-install one, guide the user, or require manual setup? | Product + Engineering | Month 2 | Auto-install is smoothest but has security/trust implications; guided setup is safer but adds friction; user testing needed |
| Q-20 | Should we provide a "meeting quality" indicator (audio quality, speaker count, noise level) to help users optimize their setup? | Product + Design | Month 4 | Helps users get better transcripts; reduces support burden; nice-to-have for v1.0, should-have for v1.1 |
| Q-21 | How do we present the choice between rule-based (free) and LLM-powered (paid/local LLM) summarization without making free tier feel broken? | Product + Design | Month 3 | Framing matters: "structured outline" vs. "AI summary" sets expectations; free tier should feel complete, not limited |
| Q-22 | What does the "sovereignty verification" experience look like? How do users confirm/verify that no data leaves their machine? | Product + Security | Month 4 | Could include: network monitor in admin panel, open-source audit guide, transparency report; important for trust but scope needs definition |

---

## Appendix A: Competitive Landscape Summary

| Competitor | Deployment | Privacy Model | Suite Integration | Target Market | Pricing |
|---|---|---|---|---|---|
| **Otter.ai** | Cloud | Data on Otter servers; opt-out of training | Slack, Zoom, Teams integrations | General business | Freemium, $10-30/user/mo |
| **Fireflies.ai** | Cloud | Data on Fireflies servers | CRM, PM tool integrations | Sales teams | $10-29/user/mo |
| **Grain** | Cloud | Data on Grain servers; SOC 2 | Notion, Slack, etc. | Product/research teams | $20-50/user/mo |
| **tl;dv** | Cloud | Data on tl;dv servers | Various integrations | General business | Freemium, paid tiers |
| **Whisper-based DIY** | Self-hosted | Full control | None (raw transcript only) | Technical users | Free (open source) |
| **WhisperLive** | Self-hosted | Full control | None (real-time only) | Technical users | Free (open source) |
| **MeetingBaize** | Local (desktop) | Local processing | Limited | Privacy-conscious individuals | Free/paid |
| **Recap (ODW.ai)** | Self-hosted / on-device | Zero data exit; verifiable | Vault + Loop (suite-native) | Regulated SMBs | Bundle (suite pricing) |

**Recap's defensible position:** Only product combining (1) verifiable on-device processing, (2) structured output with action extraction, (3) deep suite integration (knowledge + workflow), and (4) self-hosted team features with governance controls.

---

## Appendix B: Glossary

| Term | Definition |
|---|---|
| **ASR** | Automatic Speech Recognition — the technology that converts audio to text |
| **Diarization** | The process of partitioning an audio stream into segments according to speaker identity |
| **WER** | Word Error Rate — standard metric for transcription accuracy (lower is better) |
| **Sovereignty** | In this context: data never leaves the user's infrastructure; full control over processing and storage |
| **Vault** | ODW.ai's knowledge base module — stores organizational knowledge, documents, decisions |
| **Loop** | ODW.ai's workflow automation module — manages tasks, triggers, and process automation |
| **System audio** | The audio output from applications (what you hear through speakers) — distinct from microphone input |
| **Virtual audio device** | Software that creates a "fake" audio device to route system audio into another application (e.g., BlackHole on macOS) |
| **WASAPI loopback** | Windows Audio Session API feature that allows capturing audio being played by other applications |
| **Air-gapped** | A security measure where a system is physically isolated from unsecured networks (no internet) |
| **ATO** | Authority to Operate — formal authorization for a system to process sensitive data (government/defense context) |
| **CUI** | Controlled Unclassified Information — sensitive but unclassified information in US government contexts |

---

## Appendix C: Revision History

| Version | Date | Author | Changes |
|---|---|---|---|
| 1.0 | 2026-06-23 | Product Team | Initial draft |

---

*End of Product Requirements Document*
