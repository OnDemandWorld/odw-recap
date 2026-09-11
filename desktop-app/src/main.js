/**
 * Recap frontend. Runs inside the Tauri webview with `withGlobalTauri`, so
 * all backend access goes through the injected `window.__TAURI__` global —
 * bare `import "@tauri-apps/api/..."` specifiers cannot resolve in this
 * bundler-free setup (the app never loads without window.__TAURI__).
 */
(function () {
  "use strict";

  const { escapeHtml, formatDuration, formatTimestamp, formatBytes, statusLabel, isReady } =
    window.RecapUtils;

  // ---------------------------------------------------------------------------
  // Tauri bridge
  // ---------------------------------------------------------------------------

  const tauriAvailable = () =>
    typeof window.__TAURI__ !== "undefined" && window.__TAURI__.tauri;

  function invoke(command, args) {
    return window.__TAURI__.tauri.invoke(command, args);
  }

  async function openFilePicker(options) {
    return window.__TAURI__.dialog.open(options);
  }

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let meetings = [];
  let currentMeetingId = null;
  let processingMeetingId = null;

  // DOM Elements
  let meetingListEl;
  let statusEl;
  let meetingDetailEl;
  let templateListEl;

  // ---------------------------------------------------------------------------
  // Init
  // ---------------------------------------------------------------------------

  async function init() {
    meetingListEl = document.querySelector("#meeting-list");
    statusEl = document.querySelector("#status-bar");
    meetingDetailEl = document.querySelector("#meeting-detail");
    templateListEl = document.querySelector("#template-list");

    if (!tauriAvailable()) {
      setStatus("Fatal: Tauri bridge not found. This UI must run inside the ODW Recap desktop app.");
      return;
    }

    // Navigation
    document.querySelector("#nav-library").addEventListener("click", () => showView("library"));
    document.querySelector("#nav-import").addEventListener("click", () => showView("import"));
    document.querySelector("#nav-settings").addEventListener("click", () => showView("settings"));
    document.querySelector("#import-file-btn").addEventListener("click", importAudioFile);
    document.querySelector("#refresh-btn").addEventListener("click", loadMeetings);

    // Settings
    document.querySelector("#save-api-keys-btn").addEventListener("click", saveApiKeys);
    document.querySelector("#save-template-btn").addEventListener("click", savePromptTemplate);
    document.querySelector("#stt-provider").addEventListener("change", (e) =>
      persistSetting("stt_provider", e.target.value)
    );
    document.querySelector("#llm-provider").addEventListener("change", (e) =>
      persistSetting("llm_provider", e.target.value)
    );
    document.querySelector("#watch-folder-browse").addEventListener("click", pickWatchFolder);
    document.querySelector("#watch-folder-stop").addEventListener("click", stopWatchFolder);
    document.querySelector("#http-server-enabled").addEventListener("change", (e) =>
      toggleHttpServer(e.target.checked)
    );

    // Drag & drop import (Tauri file-drop event).
    window.__TAURI__.event.listen("tauri://file-drop", (event) => {
      const paths = event.payload || event.detail || [];
      if (Array.isArray(paths) && paths.length > 0) importAudioPath(paths[0]);
    });

    // Watch-folder imports arrive as backend events.
    window.__TAURI__.event.listen("meeting-imported", (event) => {
      const payload = event.payload || {};
      setStatus(`Imported from watch folder: ${payload.title || payload.meeting_id}`);
      loadMeetings();
    });

    await restoreSettings();
    await loadMeetings();
    await loadSupportedFormats();
    await loadTemplates();
    showView("library");
  }

  function showView(view) {
    document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
    document.querySelectorAll(".nav-btn").forEach((el) => el.classList.remove("active"));

    if (view === "detail") {
      document.querySelector("#detail-view").classList.add("active");
    } else {
      document.querySelector(`#${view}-view`).classList.add("active");
      document.querySelector(`#nav-${view}`).classList.add("active");
    }
  }

  // ---------------------------------------------------------------------------
  // Library
  // ---------------------------------------------------------------------------

  async function loadMeetings() {
    try {
      setStatus("Loading meetings...");
      meetings = await invoke("list_meetings", { limit: 100, offset: 0 });
      renderMeetings();
      setStatus(`Loaded ${meetings.length} meeting${meetings.length === 1 ? "" : "s"}`);
    } catch (error) {
      setStatus(`Error loading meetings: ${error}`);
      console.error("Failed to load meetings:", error);
    }
  }

  function renderMeetings() {
    if (!Array.isArray(meetings) || meetings.length === 0) {
      meetingListEl.innerHTML =
        '<p class="empty-state">No meetings yet. Import an audio file from the Import tab.</p>';
      return;
    }

    meetingListEl.innerHTML = meetings
      .map((m) => {
        const ready = isReady(m.status);
        return `
          <div class="meeting-item ${m.id === currentMeetingId ? "selected" : ""}" data-id="${escapeHtml(m.id)}">
            <div class="meeting-title">${escapeHtml(m.title)}</div>
            <div class="meeting-meta">
              <span class="status-badge status-${escapeHtml(m.status)}">${escapeHtml(statusLabel(m.status))}</span>
              ${m.duration_seconds ? `<span>${escapeHtml(formatDuration(m.duration_seconds))}</span>` : ""}
              <span>${escapeHtml(formatTimestamp(m.created_at))}</span>
            </div>
            <button class="btn-small view-details-btn" data-id="${escapeHtml(m.id)}">
              ${ready ? "View Details" : "Open"}
            </button>
          </div>`;
      })
      .join("");

    document.querySelectorAll(".view-details-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        viewMeetingDetails(btn.dataset.id);
      });
    });
    document.querySelectorAll(".meeting-item").forEach((el) => {
      el.addEventListener("click", () => viewMeetingDetails(el.dataset.id));
    });
  }

  // ---------------------------------------------------------------------------
  // Detail view
  // ---------------------------------------------------------------------------

  async function viewMeetingDetails(id) {
    currentMeetingId = id;
    showView("detail");
    renderDetailPlaceholder(id);

    try {
      const [meeting, transcript, summary, actionItems, decisions] = await Promise.all([
        invoke("get_meeting", { id }),
        invoke("get_transcript", { id }),
        invoke("get_summary", { id }),
        invoke("get_action_items", { id }),
        invoke("get_decisions", { id }),
      ]);
      renderDetail(meeting, transcript, summary, actionItems, decisions);
      setStatus(`Viewing: ${meeting.title || id}`);
    } catch (error) {
      setStatus(`Error loading details: ${error}`);
      console.error("Failed to load meeting details:", error);
      meetingDetailEl.innerHTML = `
        <div class="detail-header">
          <h2>Meeting Details</h2>
          <button id="back-to-library-btn" class="btn-secondary">Back to Library</button>
        </div>
        <p class="placeholder-text">Could not load this meeting: ${escapeHtml(String(error))}</p>`;
      bindBackButton();
    }
  }

  function renderDetailPlaceholder(id) {
    meetingDetailEl.innerHTML = `
      <div class="detail-header">
        <h2>Meeting Details</h2>
        <button id="back-to-library-btn" class="btn-secondary">Back to Library</button>
      </div>
      <p class="placeholder-text">Loading…</p>`;
    bindBackButton();
    void id;
  }

  function renderDetail(meeting, transcript, summary, actionItems, decisions) {
    const ready = isReady(meeting.status);
    const meta = [
      `<span class="status-badge status-${escapeHtml(meeting.status)}">${escapeHtml(statusLabel(meeting.status))}</span>`,
      meeting.duration_seconds ? `<strong>Duration:</strong> ${escapeHtml(formatDuration(meeting.duration_seconds))}` : "",
      `<strong>Size:</strong> ${meeting.audio_size_bytes ? escapeHtml(formatBytes(meeting.audio_size_bytes)) : "—"}`,
      `<strong>Created:</strong> ${escapeHtml(formatTimestamp(meeting.created_at))}`,
    ]
      .filter(Boolean)
      .join(" · ");

    const summaryHtml = summary
      ? `<p>${escapeHtml(summary.content)}</p>`
      : '<p class="placeholder-text">No summary yet. Click "Process Meeting" to generate one.</p>';

    const itemsHtml = (list, emptyText) =>
      list && list.length
        ? `<ul>${list.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>`
        : `<p class="placeholder-text">${escapeHtml(emptyText)}</p>`;

    const transcriptHtml =
      transcript && transcript.length
        ? transcript
            .map(
              (seg) =>
                `<p class="transcript-line"><span class="transcript-time">${escapeHtml(formatDuration(seg.start_ms / 1000))}</span> ${escapeHtml(seg.text)}</p>`
            )
            .join("")
        : '<p class="placeholder-text">No transcript yet. Click "Process Meeting" to transcribe.</p>';

    meetingDetailEl.innerHTML = `
      <div class="detail-header">
        <h2>${escapeHtml(meeting.title)}</h2>
        <div class="detail-actions">
          <button id="process-meeting-btn" class="btn-primary" ${ready ? "disabled" : ""}>
            ${ready ? "Processed" : "Process Meeting"}
          </button>
          <button id="delete-meeting-btn" class="btn-secondary">Delete</button>
          <button id="back-to-library-btn" class="btn-secondary">Back to Library</button>
        </div>
      </div>
      <div class="detail-content">
        <div class="detail-section">
          <h3>Meeting Information</h3>
          <p>${meta}</p>
          <p class="placeholder-text">${ready ? "" : "Processing transcribes the audio (cloud STT provider required) and generates a summary. Configure providers in Settings."}</p>
        </div>
        <div class="detail-section">
          <h3>Summary</h3>
          ${summaryHtml}
        </div>
        <div class="detail-section">
          <h3>Action Items</h3>
          ${itemsHtml(actionItems, "No action items detected.")}
        </div>
        <div class="detail-section">
          <h3>Decisions</h3>
          ${itemsHtml(decisions, "No decisions detected.")}
        </div>
        <div class="detail-section">
          <h3>Transcript</h3>
          ${transcriptHtml}
        </div>
      </div>`;

    bindBackButton();
    const processBtn = document.querySelector("#process-meeting-btn");
    if (processBtn && !ready) processBtn.addEventListener("click", () => processMeeting(meeting.id));
    document.querySelector("#delete-meeting-btn").addEventListener("click", () => deleteMeeting(meeting.id));
  }

  function bindBackButton() {
    const btn = document.querySelector("#back-to-library-btn");
    if (btn) btn.addEventListener("click", () => showView("library"));
  }

  async function processMeeting(id) {
    if (processingMeetingId) {
      setStatus("Another meeting is already processing. Please wait.");
      return;
    }
    processingMeetingId = id;
    setStatus("Processing meeting: transcribing and summarizing… (this can take a while)");
    const btn = document.querySelector("#process-meeting-btn");
    if (btn) btn.disabled = true;

    try {
      const outcome = await invoke("process_meeting", { id });
      setStatus(
        `Meeting processed: ${outcome.segments} transcript segments, summary by ${outcome.summary_provider}.`
      );
      await loadMeetings();
      await viewMeetingDetails(id);
    } catch (error) {
      setStatus(`Processing failed: ${error}`);
      console.error("Processing failed:", error);
      await viewMeetingDetails(id);
    } finally {
      processingMeetingId = null;
    }
  }

  async function deleteMeeting(id) {
    // Two-step confirm: window.confirm() is unreliable inside the Tauri
    // webview, so require a second click on the same button instead.
    const btn = document.querySelector("#delete-meeting-btn");
    if (btn && btn.dataset.armed !== "true") {
      btn.dataset.armed = "true";
      btn.textContent = "Confirm delete?";
      setStatus("Click again to permanently delete this meeting.");
      setTimeout(() => {
        if (btn.isConnected) {
          btn.dataset.armed = "false";
          btn.textContent = "Delete";
        }
      }, 5000);
      return;
    }

    try {
      await invoke("delete_meeting", { id });
      setStatus("Meeting deleted.");
      currentMeetingId = null;
      await loadMeetings();
      showView("library");
    } catch (error) {
      setStatus(`Delete failed: ${error}`);
      console.error("Delete failed:", error);
    }
  }

  // ---------------------------------------------------------------------------
  // Import
  // ---------------------------------------------------------------------------

  async function importAudioFile() {
    try {
      const selected = await openFilePicker({
        multiple: false,
        filters: [{ name: "Audio Files", extensions: ["m4a", "wav", "mp3", "ogg", "webm", "flac"] }],
      });
      if (!selected) {
        setStatus("Import cancelled");
        return;
      }
      await importAudioPath(selected);
    } catch (error) {
      setStatus(`Import failed: ${error}`);
      console.error("Failed to import file:", error);
    }
  }

  async function importAudioPath(path) {
    try {
      setStatus("Importing audio file...");
      const imported = await invoke("import_audio_file", { path });
      setStatus(`Imported. Meeting ${imported.meeting_id} is ready to process.`);
      await loadMeetings();
      await viewMeetingDetails(imported.meeting_id);
    } catch (error) {
      setStatus(`Import failed: ${error}`);
      console.error("Failed to import file:", error);
    }
  }

  async function loadSupportedFormats() {
    try {
      const formats = await invoke("get_supported_audio_formats");
      document.querySelector("#supported-formats").textContent = formats.join(", ");
    } catch (error) {
      console.error("Failed to load formats:", error);
    }
  }

  // ---------------------------------------------------------------------------
  // Settings
  // ---------------------------------------------------------------------------

  async function persistSetting(key, value) {
    try {
      await invoke("set_config", { key, value });
      setStatus("Setting saved.");
    } catch (error) {
      setStatus(`Failed to save setting: ${error}`);
      console.error("Failed to save setting:", error);
    }
  }

  async function restoreSettings() {
    try {
      const [stt, llm] = await Promise.all([
        invoke("get_config", { key: "stt_provider" }),
        invoke("get_config", { key: "llm_provider" }),
      ]);
      if (stt) document.querySelector("#stt-provider").value = stt;
      if (llm) document.querySelector("#llm-provider").value = llm;

      // Mark providers that already have a saved key.
      await Promise.all(
        [
          ["openai", "#openai-api-key"],
          ["anthropic", "#anthropic-api-key"],
          ["deepgram", "#deepgram-api-key"],
        ].map(async ([provider, selector]) => {
          const has = await invoke("has_api_key", { provider });
          const hint = document.querySelector(`${selector} + .key-status`);
          if (hint) {
            hint.textContent = has ? "Saved ✓" : "";
            hint.classList.toggle("saved", has);
          }
        })
      );
    } catch (error) {
      console.error("Failed to restore settings:", error);
    }
  }

  async function saveApiKeys() {
    const entries = [
      ["openai", "#openai-api-key"],
      ["anthropic", "#anthropic-api-key"],
      ["deepgram", "#deepgram-api-key"],
    ];
    let saved = 0;
    try {
      for (const [provider, selector] of entries) {
        const input = document.querySelector(selector);
        const key = (input.value || "").trim();
        if (!key) continue;
        await invoke("save_api_key", { provider, key });
        input.value = "";
        const hint = document.querySelector(`${selector} + .key-status`);
        if (hint) {
          hint.textContent = "Saved ✓";
          hint.classList.add("saved");
        }
        saved += 1;
      }
      setStatus(saved ? `Saved ${saved} API key${saved === 1 ? "" : "s"} (encrypted on this device).` : "No keys entered.");
    } catch (error) {
      setStatus(`Failed to save API keys: ${error}`);
      console.error("Failed to save API keys:", error);
    }
  }

  async function savePromptTemplate() {
    const nameInput = document.querySelector("#template-name");
    const contentInput = document.querySelector("#template-content");
    const name = (nameInput.value || "").trim();
    const content = (contentInput.value || "").trim();
    if (!name || !content) {
      setStatus("Template needs both a name and content.");
      return;
    }
    try {
      await invoke("save_prompt_template", { name, content, meetingType: null });
      setStatus(`Template "${name}" saved.`);
      nameInput.value = "";
      contentInput.value = "";
      await loadTemplates();
    } catch (error) {
      setStatus(`Failed to save template: ${error}`);
      console.error("Failed to save template:", error);
    }
  }

  async function loadTemplates() {
    try {
      const templates = await invoke("list_prompt_templates");
      if (!templateListEl) return;
      if (!Array.isArray(templates) || templates.length === 0) {
        templateListEl.innerHTML =
          '<p class="placeholder-text">No templates yet. The summarizer works without one.</p>';
        return;
      }
      templateListEl.innerHTML = templates
        .map(
          (t) => `
            <div class="template-item">
              <strong>${escapeHtml(t.name)}</strong>
              ${t.is_custom ? '<span class="badge">custom</span>' : '<span class="badge badge-muted">built-in</span>'}
              ${t.meeting_type ? `<span class="placeholder-text">${escapeHtml(t.meeting_type)}</span>` : ""}
            </div>`
        )
        .join("");
    } catch (error) {
      console.error("Failed to load templates:", error);
    }
  }

  // ---------------------------------------------------------------------------
  // Watch folder + HTTP upload
  // ---------------------------------------------------------------------------

  async function pickWatchFolder() {
    try {
      const selected = await openFilePicker({ multiple: false, directory: true });
      if (!selected) return;
      const active = await invoke("start_watch_folder", { path: selected });
      document.querySelector("#watch-folder-path").value = active;
      setStatus(`Watching folder: ${selected} — new recordings import automatically.`);
    } catch (error) {
      setStatus(`Failed to start watch folder: ${error}`);
      console.error("Watch folder failed:", error);
    }
  }

  async function stopWatchFolder() {
    try {
      const stopped = await invoke("stop_watch_folder");
      setStatus(stopped ? "Watch folder stopped." : "No watch folder was active.");
    } catch (error) {
      setStatus(`Failed to stop watch folder: ${error}`);
    }
  }

  async function toggleHttpServer(enable) {
    try {
      const urlEl = document.querySelector("#http-server-url");
      if (enable) {
        const info = await invoke("start_http_upload_server");
        urlEl.innerHTML = `Upload URL (open on your phone): <code>${escapeHtml(info.url)}</code>`;
        setStatus("HTTP upload server running on your local network.");
      } else {
        const stopped = await invoke("stop_http_upload_server");
        urlEl.textContent = stopped ? "Server stopped." : "";
        setStatus(stopped ? "HTTP upload server stopped." : "Server was not running.");
      }
    } catch (error) {
      setStatus(`HTTP upload server error: ${error}`);
      document.querySelector("#http-server-enabled").checked = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Status bar
  // ---------------------------------------------------------------------------

  function setStatus(message) {
    if (statusEl) statusEl.textContent = message;
  }

  // Initialize on DOM ready
  window.addEventListener("DOMContentLoaded", init);
})();
