// Recap desktop frontend.
//
// Note: this project serves the frontend directly (no bundler), so the
// `@tauri-apps/api` npm package cannot be imported here. We use the global
// API injected by Tauri (`withGlobalTauri: true` in tauri.conf.json).
const { invoke } = window.__TAURI__.tauri;
const { open: openDialog } = window.__TAURI__.dialog;

// State
let meetings = [];
let currentView = "library"; // library | import | settings | detail
let currentMeetingId = null;

// DOM Elements
let meetingListEl;
let statusEl;
let meetingDetailEl;
let templateListEl;

const SUPPORTED_EXTENSIONS = ["m4a", "wav", "mp3", "ogg", "webm", "flac"];

// Initialize app
async function init() {
  meetingListEl = document.querySelector("#meeting-list");
  statusEl = document.querySelector("#status-bar");
  meetingDetailEl = document.querySelector("#meeting-detail");
  templateListEl = document.querySelector("#template-list");

  // Navigation
  document.querySelector("#nav-library").addEventListener("click", () => showView("library"));
  document.querySelector("#nav-import").addEventListener("click", () => showView("import"));
  document.querySelector("#nav-settings").addEventListener("click", () => showView("settings"));
  document.querySelector("#import-file-btn").addEventListener("click", importAudioFile);
  document.querySelector("#refresh-btn").addEventListener("click", loadMeetings);
  document.querySelector("#save-api-keys-btn").addEventListener("click", saveSettings);
  document.querySelector("#create-template-btn").addEventListener("click", createPromptTemplate);
  document.querySelector("#lock-vault-btn").addEventListener("click", lockVault);
  document.querySelector("#change-passphrase-btn").addEventListener("click", changePassphrase);

  bindVaultGateHandlers();
  await refreshVaultGate();
}

// --- Vault gate ---------------------------------------------------------------

function bindVaultGateHandlers() {
  const createBtn = document.querySelector("#vault-create-btn");
  const unlockBtn = document.querySelector("#vault-unlock-btn");
  const unlockInput = document.querySelector("#vault-passphrase");
  const confirmInput = document.querySelector("#vault-confirm-passphrase");

  createBtn.addEventListener("click", createVault);
  confirmInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") createVault();
  });
  unlockBtn.addEventListener("click", unlockVault);
  unlockInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") unlockVault();
  });
}

async function refreshVaultGate() {
  try {
    const status = await invoke("vault_status");
    if (!status.initialized) {
      showVaultOverlay("setup");
      return;
    }
    if (status.locked) {
      showVaultOverlay("unlock");
      return;
    }
  } catch (error) {
    console.error("Failed to query vault status:", error);
  }

  hideVaultOverlay();
  await Promise.all([
    loadMeetings(),
    loadSupportedFormats(),
    loadSettings(),
    loadPromptTemplates(),
    loadProviderStatus(),
    loadWhisperModels(),
  ]);
  showView("library");
  bindWhisperDownloadHandler();
}

function showVaultOverlay(mode) {
  const overlay = document.querySelector("#vault-overlay");
  const setupForm = document.querySelector("#vault-setup-form");
  const unlockForm = document.querySelector("#vault-unlock-form");
  const subtitle = document.querySelector("#vault-subtitle");

  setVaultError("");
  if (mode === "setup") {
    subtitle.textContent = "Set up encrypted local storage to get started.";
    setupForm.classList.remove("hidden");
    unlockForm.classList.add("hidden");
    document.querySelector("#vault-new-passphrase").focus();
  } else {
    subtitle.textContent = "Enter your passphrase to unlock your meetings.";
    setupForm.classList.add("hidden");
    unlockForm.classList.remove("hidden");
    document.querySelector("#vault-passphrase").focus();
  }
  overlay.classList.remove("hidden");
}

function hideVaultOverlay() {
  document.querySelector("#vault-overlay").classList.add("hidden");
}

function setVaultError(message) {
  document.querySelector("#vault-error").textContent = message || "";
}

async function createVault() {
  const passphrase = document.querySelector("#vault-new-passphrase").value;
  const confirm = document.querySelector("#vault-confirm-passphrase").value;
  try {
    await invoke("initialize_vault", { passphrase, confirm });
    document.querySelector("#vault-new-passphrase").value = "";
    document.querySelector("#vault-confirm-passphrase").value = "";
    await refreshVaultGate();
  } catch (error) {
    setVaultError(String(error));
  }
}

async function unlockVault() {
  const passphrase = document.querySelector("#vault-passphrase").value;
  try {
    await invoke("unlock_vault", { passphrase });
    document.querySelector("#vault-passphrase").value = "";
    await refreshVaultGate();
  } catch (error) {
    setVaultError(String(error));
  }
}

async function lockVault() {
  try {
    await invoke("lock_vault");
    meetings = [];
    currentMeetingId = null;
    setStatus("Vault locked");
    await refreshVaultGate();
  } catch (error) {
    setStatus(`Failed to lock vault: ${error}`);
  }
}

async function changePassphrase() {
  const current = document.querySelector("#current-passphrase").value;
  const newPass = document.querySelector("#new-passphrase").value;
  try {
    setStatus("Changing passphrase...");
    await invoke("change_vault_passphrase", { current, new: newPass });
    document.querySelector("#current-passphrase").value = "";
    document.querySelector("#new-passphrase").value = "";
    setStatus("Passphrase changed. All stored data was re-encrypted.");
  } catch (error) {
    setStatus(`Passphrase change failed: ${error}`);
  }
}

// Show/hide views
function showView(view) {
  currentView = view;
  document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
  document.querySelectorAll(".nav-btn").forEach((el) => el.classList.remove("active"));

  if (view === "detail") {
    document.querySelector("#detail-view").classList.add("active");
  } else {
    document.querySelector(`#${view}-view`).classList.add("active");
    document.querySelector(`#nav-${view}`).classList.add("active");
  }
}

// --- Meetings ---------------------------------------------------------------

async function loadMeetings() {
  try {
    setStatus("Loading meetings...");
    meetings = await invoke("list_meetings", { limit: 100, offset: 0 });
    renderMeetings();
    setStatus(`Loaded ${meetings.length} meeting(s)`);
  } catch (error) {
    setStatus(`Error loading meetings: ${error}`);
    console.error("Failed to load meetings:", error);
  }
}

function formatDate(epochMillis) {
  if (!epochMillis) return "Unknown";
  return new Date(epochMillis).toLocaleString();
}

function renderMeetings() {
  if (meetings.length === 0) {
    meetingListEl.innerHTML =
      '<p class="empty-state">No meetings yet. Import an audio file to get started.</p>';
    return;
  }

  meetingListEl.innerHTML = meetings
    .map((m) => {
      const title = escapeHtml(m.title || `Meeting ${m.id.substring(0, 8)}`);
      return `
    <div class="meeting-item" data-id="${m.id}">
      <div class="meeting-title">${title}</div>
      <div class="meeting-meta">
        <span class="badge">${escapeHtml(m.status)}</span>
        ${formatDate(m.started_at)} &middot; ${escapeHtml(m.audio_format || "")}
      </div>
      <button class="btn-small view-details-btn" data-id="${m.id}">View Details</button>
    </div>
  `;
    })
    .join("");

  document.querySelectorAll(".meeting-item").forEach((el) => {
    el.addEventListener("click", (e) => {
      if (!e.target.classList.contains("view-details-btn")) {
        selectMeeting(el.dataset.id);
      }
    });
  });

  document.querySelectorAll(".view-details-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      viewMeetingDetails(btn.dataset.id);
    });
  });
}

function selectMeeting(id) {
  document.querySelectorAll(".meeting-item").forEach((el) => {
    el.classList.toggle("selected", el.dataset.id === id);
  });
  setStatus(`Selected meeting: ${id}`);
}

async function viewMeetingDetails(id) {
  currentMeetingId = id;
  showView("detail");
  setStatus("Loading meeting details...");

  let meeting;
  try {
    meeting = await invoke("get_meeting", { id });
  } catch (error) {
    setStatus(`Error loading meeting: ${error}`);
    return;
  }
  if (!meeting) {
    meetingDetailEl.innerHTML = '<p class="placeholder-text">Meeting not found.</p>';
    return;
  }

  meetingDetailEl.innerHTML = `
    <div class="detail-header">
      <h2>${escapeHtml(meeting.title || "Untitled meeting")}</h2>
      <button id="back-to-library-btn" class="btn-secondary">Back to Library</button>
    </div>
    <div class="detail-content">
      <div class="detail-section">
        <h3>Meeting Information</h3>
        <p><strong>Status:</strong> <span class="badge">${escapeHtml(meeting.status)}</span></p>
        <p><strong>Started:</strong> ${formatDate(meeting.started_at)}</p>
        <p><strong>Source:</strong> ${escapeHtml(meeting.audio_source)}</p>
        <p><strong>Audio:</strong> ${escapeHtml(meeting.audio_format || "n/a")} (${meeting.audio_size_bytes} bytes)</p>
        <div class="detail-actions">
          <button id="transcribe-btn" class="btn-primary">Transcribe</button>
          <button id="summarize-btn" class="btn-secondary">Summarize</button>
        </div>
      </div>
      <div class="detail-section">
        <h3>Transcript</h3>
        <div id="transcript-content"><p class="placeholder-text">Loading...</p></div>
      </div>
      <div class="detail-section">
        <h3>Summary</h3>
        <div id="summary-content"><p class="placeholder-text">Loading...</p></div>
      </div>
    </div>
  `;

  document.querySelector("#back-to-library-btn").addEventListener("click", () => showView("library"));
  document.querySelector("#transcribe-btn").addEventListener("click", transcribeCurrentMeeting);
  document.querySelector("#summarize-btn").addEventListener("click", summarizeCurrentMeeting);

  await Promise.all([loadTranscript(id), loadSummary(id)]);
  setStatus(`Viewing meeting: ${meeting.title || id}`);
}

async function loadTranscript(meetingId) {
  const el = document.querySelector("#transcript-content");
  try {
    const segments = await invoke("get_transcript_segments", { meetingId });
    if (!segments || segments.length === 0) {
      el.innerHTML = '<p class="placeholder-text">No transcript yet. Click Transcribe to generate one.</p>';
      return;
    }
    el.innerHTML = segments
      .map((s) => `<p class="segment"><span class="segment-time">${formatMs(s.start_ms)}</span> ${escapeHtml(s.text)}</p>`)
      .join("");
  } catch (error) {
    el.innerHTML = `<p class="placeholder-text">Failed to load transcript: ${escapeHtml(String(error))}</p>`;
  }
}

async function loadSummary(meetingId) {
  const el = document.querySelector("#summary-content");
  try {
    const summary = await invoke("get_summary", { meetingId });
    if (!summary) {
      el.innerHTML = '<p class="placeholder-text">No summary yet. Click Summarize to generate one.</p>';
      return;
    }
    el.innerHTML = `<p>${escapeHtml(summary.content)}</p>
      <p class="meeting-meta">Generated by: ${escapeHtml(summary.generation_mode)}</p>`;
  } catch (error) {
    el.innerHTML = `<p class="placeholder-text">Failed to load summary: ${escapeHtml(String(error))}</p>`;
  }
}

async function transcribeCurrentMeeting() {
  if (!currentMeetingId) return;
  const btn = document.querySelector("#transcribe-btn");
  btn.disabled = true;
  setStatus("Transcribing meeting (this can take a while)...");
  try {
    const count = await invoke("transcribe_meeting", { meetingId: currentMeetingId });
    setStatus(`Transcription complete: ${count} segment(s) stored`);
    await Promise.all([loadTranscript(currentMeetingId), loadMeetingsQuiet()]);
  } catch (error) {
    setStatus(`Transcription failed: ${error}`);
    console.error("Transcription failed:", error);
  } finally {
    btn.disabled = false;
  }
}

async function summarizeCurrentMeeting() {
  if (!currentMeetingId) return;
  const btn = document.querySelector("#summarize-btn");
  btn.disabled = true;
  setStatus("Summarizing meeting...");
  try {
    await invoke("summarize_meeting", { meetingId: currentMeetingId });
    setStatus("Summary generated");
    await loadSummary(currentMeetingId);
  } catch (error) {
    setStatus(`Summarization failed: ${error}`);
    console.error("Summarization failed:", error);
  } finally {
    btn.disabled = false;
  }
}

// Refresh the in-memory list without disrupting the current view.
async function loadMeetingsQuiet() {
  try {
    meetings = await invoke("list_meetings", { limit: 100, offset: 0 });
  } catch (error) {
    console.error("Failed to refresh meetings:", error);
  }
}

// --- Import -----------------------------------------------------------------

async function importAudioFile() {
  try {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "Audio Files", extensions: SUPPORTED_EXTENSIONS }],
    });

    if (!selected) {
      setStatus("Import cancelled");
      return;
    }

    setStatus("Importing audio file...");
    const meetingId = await invoke("import_audio_file", { path: selected });
    setStatus("File imported. Opening meeting details...");
    await loadMeetingsQuiet();
    viewMeetingDetails(meetingId);
  } catch (error) {
    setStatus(`Import failed: ${error}`);
    console.error("Failed to import file:", error);
  }
}

async function loadSupportedFormats() {
  try {
    const formats = await invoke("get_supported_audio_formats");
    document.querySelector("#supported-formats").textContent = formats.join(", ").toUpperCase();
  } catch (error) {
    console.error("Failed to load formats:", error);
  }
}

// --- Settings ---------------------------------------------------------------

const API_KEY_FIELDS = [
  { inputId: "openai-api-key", provider: "openai" },
  { inputId: "anthropic-api-key", provider: "anthropic" },
  { inputId: "deepgram-api-key", provider: "deepgram" },
];

async function loadSettings() {
  try {
    const [stt, llm] = await Promise.all([
      invoke("get_config", { key: "stt_provider" }),
      invoke("get_config", { key: "llm_provider" }),
    ]);
    if (stt) document.querySelector("#stt-provider").value = stt;
    if (llm) document.querySelector("#llm-provider").value = llm;

    // Show which providers already have a stored key.
    for (const { inputId, provider } of API_KEY_FIELDS) {
      const hasKey = await invoke("has_api_key", { provider });
      if (hasKey) {
        document.querySelector(`#${inputId}`).placeholder = "Stored (enter a new value to replace)";
      }
    }
  } catch (error) {
    console.error("Failed to load settings:", error);
  }
}

// --- Whisper model management -----------------------------------------------

async function loadWhisperModels() {
  try {
    const models = await invoke("list_whisper_models");
    const select = document.querySelector("#whisper-model");
    const savedModel = await invoke("get_config", { key: "whisper_model" });
    select.innerHTML = models
      .map((m) => {
        const label = `${m.name}${m.installed ? " (installed)" : ""} — ${m.description}`;
        return `<option value="${escapeHtml(m.name)}">${escapeHtml(label)}</option>`;
      })
      .join("");
    if (savedModel && models.some((m) => m.name === savedModel)) {
      select.value = savedModel;
    }
    select.addEventListener("change", async () => {
      await invoke("set_config", { key: "whisper_model", value: select.value });
      updateWhisperStatus(models);
    });
    updateWhisperStatus(models);
  } catch (error) {
    console.error("Failed to load whisper models:", error);
  }
}

function updateWhisperStatus(models) {
  const status = document.querySelector("#whisper-model-status");
  const selected = document.querySelector("#whisper-model").value;
  const current = models.find((m) => m.name === selected);
  if (!current || !status) return;
  status.textContent = current.installed
    ? `✓ ${current.name} ready`
    : `${current.name} not downloaded (~${Math.round(current.size_bytes / 1_000_000)} MB)`;
}

function bindWhisperDownloadHandler() {
  const btn = document.querySelector("#download-whisper-model-btn");
  const progressBar = document.querySelector("#whisper-download-progress");
  const status = document.querySelector("#whisper-model-status");
  if (!btn || !progressBar || !status) return;

  const { event } = window.__TAURI__;

  event.listen("whisper-download-progress", (e) => {
    const payload = e.payload;
    if (!payload) return;
    const { downloaded, total, done } = payload;
    if (done) {
      progressBar.style.display = "none";
      status.textContent = "✓ Download complete";
      btn.disabled = false;
      btn.textContent = "Download Model";
      loadWhisperModels();
      return;
    }
    if (total > 0) {
      const percent = Math.min(100, Math.round((downloaded / total) * 100));
      progressBar.value = percent;
      progressBar.style.display = "block";
      status.textContent = `Downloading... ${percent}% (${Math.round(downloaded / 1_000_000)} MB / ${Math.round(total / 1_000_000)} MB)`;
    }
  });

  btn.addEventListener("click", async () => {
    const name = document.querySelector("#whisper-model").value;
    if (!name) return;
    btn.disabled = true;
    btn.textContent = "Downloading...";
    progressBar.value = 0;
    progressBar.style.display = "block";
    status.textContent = "Starting download...";
    try {
      await invoke("download_whisper_model", { name });
    } catch (error) {
      status.textContent = `Download failed: ${error}`;
      btn.disabled = false;
      btn.textContent = "Download Model";
      progressBar.style.display = "none";
    }
  });
}

async function loadProviderStatus() {
  try {
    const [sttProviders, llmProviders] = await Promise.all([
      invoke("list_stt_providers"),
      invoke("list_llm_providers"),
    ]);
    const describe = (list) =>
      list.map((p) => `${p.name}${p.available ? " ✓" : ""}`).join(", ");
    const sttEl = document.querySelector("#stt-provider-status");
    const llmEl = document.querySelector("#llm-provider-status");
    if (sttEl) sttEl.textContent = `Ready: ${describe(sttProviders.filter((p) => p.available)) || "none (local stub active)"}`;
    if (llmEl) llmEl.textContent = `Ready: ${describe(llmProviders.filter((p) => p.available)) || "none (rule-based fallback active)"}`;
  } catch (error) {
    console.error("Failed to load provider status:", error);
  }
}

async function saveSettings() {
  try {
    setStatus("Saving settings...");

    const sttProvider = document.querySelector("#stt-provider").value;
    const llmProvider = document.querySelector("#llm-provider").value;
    await invoke("set_config", { key: "stt_provider", value: sttProvider });
    await invoke("set_config", { key: "llm_provider", value: llmProvider });

    let savedKeys = 0;
    for (const { inputId, provider } of API_KEY_FIELDS) {
      const input = document.querySelector(`#${inputId}`);
      const value = input.value.trim();
      if (value) {
        await invoke("save_api_key", { provider, apiKey: value });
        input.value = "";
        input.placeholder = "Stored (enter a new value to replace)";
        savedKeys += 1;
      }
    }

    setStatus(`Settings saved (providers updated, ${savedKeys} API key(s) stored)`);
    await loadProviderStatus();
  } catch (error) {
    setStatus(`Error saving settings: ${error}`);
    console.error("Failed to save settings:", error);
  }
}

// --- Prompt templates ---------------------------------------------------------

async function loadPromptTemplates() {
  try {
    const templates = await invoke("list_prompt_templates");
    if (!templates || templates.length === 0) {
      templateListEl.innerHTML = '<p class="placeholder-text">No templates available.</p>';
      return;
    }
    templateListEl.innerHTML = templates
      .map(
        (t) => `
      <div class="template-item">
        <strong>${escapeHtml(t.name)}</strong>
        <p class="meeting-meta">Variables: ${t.variables.map((v) => escapeHtml(v.name)).join(", ") || "none"}</p>
      </div>`
      )
      .join("");
  } catch (error) {
    console.error("Failed to load templates:", error);
  }
}

async function createPromptTemplate() {
  const name = window.prompt("Template name:");
  if (!name) return;
  const content = window.prompt("Template content (use {{variable}} placeholders):");
  if (!content) return;
  try {
    await invoke("save_prompt_template", { name, content });
    setStatus(`Template "${name}" saved`);
    await loadPromptTemplates();
  } catch (error) {
    setStatus(`Failed to save template: ${error}`);
  }
}

// --- Utilities ----------------------------------------------------------------

function formatMs(ms) {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text == null ? "" : String(text);
  return div.innerHTML;
}

function setStatus(message) {
  statusEl.textContent = message;
}

// Initialize on DOM ready
window.addEventListener("DOMContentLoaded", init);
