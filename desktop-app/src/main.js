import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";

// State
let meetings = [];
let currentView = "library"; // library | import | settings

// DOM Elements
let meetingListEl;
let importSectionEl;
let settingsSectionEl;
let statusEl;

// Initialize app
async function init() {
  meetingListEl = document.querySelector("#meeting-list");
  importSectionEl = document.querySelector("#import-section");
  settingsSectionEl = document.querySelector("#settings-section");
  statusEl = document.querySelector("#status-bar");

  // Setup event listeners
  document.querySelector("#nav-library").addEventListener("click", () => showView("library"));
  document.querySelector("#nav-import").addEventListener("click", () => showView("import"));
  document.querySelector("#nav-settings").addEventListener("click", () => showView("settings"));
  document.querySelector("#import-file-btn").addEventListener("click", importAudioFile);
  document.querySelector("#refresh-btn").addEventListener("click", loadMeetings);

  // Load initial data
  await loadMeetings();
  showView("library");
}

// Show/hide views
function showView(view) {
  currentView = view;
  document.querySelectorAll(".view").forEach(el => el.classList.remove("active"));
  document.querySelectorAll(".nav-btn").forEach(el => el.classList.remove("active"));

  document.querySelector(`#${view}-view`).classList.add("active");
  document.querySelector(`#nav-${view}`).classList.add("active");
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
    </div>
  `).join("");

  // Add click handlers
  document.querySelectorAll(".meeting-item").forEach(el => {
    el.addEventListener("click", () => selectMeeting(el.dataset.id));
  });
}

// Select a meeting
function selectMeeting(id) {
  document.querySelectorAll(".meeting-item").forEach(el => {
    el.classList.toggle("selected", el.dataset.id === id);
  });
  setStatus(`Selected meeting: ${id}`);
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

    // Refresh meeting list (would need backend to auto-create meeting from imported file)
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

// Update status bar
function setStatus(message) {
  statusEl.textContent = message;
}

// Initialize on DOM ready
window.addEventListener("DOMContentLoaded", init);
