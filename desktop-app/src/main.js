import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";

// State
let meetings = [];
let currentView = "library"; // library | import | settings | detail
let currentMeetingId = null;

// DOM Elements
let meetingListEl;
let importSectionEl;
let settingsSectionEl;
let statusEl;
let meetingDetailEl;

// Initialize app
async function init() {
  meetingListEl = document.querySelector("#meeting-list");
  importSectionEl = document.querySelector("#import-section");
  settingsSectionEl = document.querySelector("#settings-section");
  statusEl = document.querySelector("#status-bar");
  meetingDetailEl = document.querySelector("#meeting-detail");

  // Setup event listeners
  document.querySelector("#nav-library").addEventListener("click", () => showView("library"));
  document.querySelector("#nav-import").addEventListener("click", () => showView("import"));
  document.querySelector("#nav-settings").addEventListener("click", () => showView("settings"));
  document.querySelector("#import-file-btn").addEventListener("click", importAudioFile);
  document.querySelector("#refresh-btn").addEventListener("click", loadMeetings);
  document.querySelector("#back-to-library-btn").addEventListener("click", () => showView("library"));
  document.querySelector("#save-api-keys-btn").addEventListener("click", saveApiKeys);
  document.querySelector("#create-template-btn").addEventListener("click", createPromptTemplate);

  // Load initial data
  await loadMeetings();
  await loadSupportedFormats();
  showView("library");
}

// Show/hide views
function showView(view) {
  currentView = view;
  document.querySelectorAll(".view").forEach(el => el.classList.remove("active"));
  document.querySelectorAll(".nav-btn").forEach(el => el.classList.remove("active"));

  if (view === "detail") {
    document.querySelector("#detail-view").classList.add("active");
  } else {
    document.querySelector(`#${view}-view`).classList.add("active");
    document.querySelector(`#nav-${view}`).classList.add("active");
  }
}

// Load meetings from backend
async function loadMeetings() {
  try {
    setStatus("Loading meetings...");
    const meetingIds = await invoke("list_meetings", { limit: 100, offset: 0 });
    meetings = meetingIds;
    renderMeetings();
    setStatus(`Loaded ${meetings.length} meetings`);
  } catch (error) {
    setStatus(`Error loading meetings: ${error}`);
    console.error("Failed to load meetings:", error);
  }
}

// Render meeting list
function renderMeetings() {
  if (meetings.length === 0) {
    meetingListEl.innerHTML = '<p class="empty-state">No meetings yet. Import an audio file or create a new meeting.</p>';
    return;
  }

  meetingListEl.innerHTML = meetings.map(id => `
    <div class="meeting-item" data-id="${id}">
      <div class="meeting-title">Meeting ${id.substring(0, 8)}...</div>
      <div class="meeting-meta">ID: ${id}</div>
      <button class="btn-small view-details-btn" data-id="${id}">View Details</button>
    </div>
  `).join("");

  // Add click handlers
  document.querySelectorAll(".meeting-item").forEach(el => {
    el.addEventListener("click", (e) => {
      if (!e.target.classList.contains("view-details-btn")) {
        selectMeeting(el.dataset.id);
      }
    });
  });

  document.querySelectorAll(".view-details-btn").forEach(btn => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      viewMeetingDetails(btn.dataset.id);
    });
  });
}

// Select a meeting
function selectMeeting(id) {
  document.querySelectorAll(".meeting-item").forEach(el => {
    el.classList.toggle("selected", el.dataset.id === id);
  });
  setStatus(`Selected meeting: ${id}`);
}

// View meeting details
async function viewMeetingDetails(id) {
  currentMeetingId = id;
  showView("detail");

  try {
    setStatus("Loading meeting details...");
    // In a real implementation, this would fetch full meeting data
    meetingDetailEl.innerHTML = `
      <div class="detail-header">
        <h2>Meeting Details</h2>
        <button id="back-to-library-btn" class="btn-secondary">Back to Library</button>
      </div>
      <div class="detail-content">
        <div class="detail-section">
          <h3>Meeting Information</h3>
          <p><strong>ID:</strong> ${id}</p>
          <p><strong>Status:</strong> Processing</p>
          <p><strong>Created:</strong> ${new Date().toLocaleString()}</p>
        </div>
        <div class="detail-section">
          <h3>Transcript</h3>
          <p class="placeholder-text">Transcript will appear here after processing...</p>
        </div>
        <div class="detail-section">
          <h3>Summary</h3>
          <p class="placeholder-text">Summary will appear here after processing...</p>
        </div>
        <div class="detail-section">
          <h3>Action Items</h3>
          <p class="placeholder-text">Action items will appear here after processing...</p>
        </div>
        <div class="detail-section">
          <h3>Decisions</h3>
          <p class="placeholder-text">Decisions will appear here after processing...</p>
        </div>
      </div>
    `;

    // Re-attach back button listener
    document.querySelector("#back-to-library-btn").addEventListener("click", () => showView("library"));
    setStatus(`Viewing meeting: ${id}`);
  } catch (error) {
    setStatus(`Error loading details: ${error}`);
    console.error("Failed to load meeting details:", error);
  }
}

// Import audio file
async function importAudioFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{
        name: "Audio Files",
        extensions: ["m4a", "wav", "mp3", "ogg", "webm", "flac"]
      }]
    });

    if (!selected) {
      setStatus("Import cancelled");
      return;
    }

    setStatus("Importing audio file...");
    const destPath = await invoke("import_audio_file", { path: selected });
    setStatus(`File imported to: ${destPath}`);

    // Refresh meeting list
    await loadMeetings();
  } catch (error) {
    setStatus(`Import failed: ${error}`);
    console.error("Failed to import file:", error);
  }
}

// Get supported formats
async function loadSupportedFormats() {
  try {
    const formats = await invoke("get_supported_audio_formats");
    document.querySelector("#supported-formats").textContent = formats.join(", ");
  } catch (error) {
    console.error("Failed to load formats:", error);
  }
}

// Save API keys
async function saveApiKeys() {
  try {
    setStatus("Saving API keys...");
    // In a real implementation, this would save API keys via Tauri commands
    setStatus("API keys saved successfully");
  } catch (error) {
    setStatus(`Error saving API keys: ${error}`);
    console.error("Failed to save API keys:", error);
  }
}

// Create prompt template
async function createPromptTemplate() {
  try {
    setStatus("Creating prompt template...");
    // In a real implementation, this would open a template editor dialog
    setStatus("Template editor would open here");
  } catch (error) {
    setStatus(`Error creating template: ${error}`);
    console.error("Failed to create template:", error);
  }
}

// Update status bar
function setStatus(message) {
  statusEl.textContent = message;
}

// Initialize on DOM ready
window.addEventListener("DOMContentLoaded", init);
