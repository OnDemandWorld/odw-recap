/**
 * Recap frontend helpers. Loaded as a plain script before main.js so no
 * bundler is needed; it also exports itself CommonJS-style so Jest can test
 * it without a transpile step.
 */
(function (global) {
  "use strict";

  /**
   * Escape a string for safe interpolation into innerHTML templates.
   * Meeting titles are user input and must never reach the DOM raw.
   */
  function escapeHtml(value) {
    return String(value == null ? "" : value)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  /** Human-readable duration: 3675 -> "1h 1m", 95 -> "1m 35s", 42 -> "0m 42s". */
  function formatDuration(seconds) {
    if (seconds == null || isNaN(seconds) || seconds < 0) return "";
    const total = Math.round(seconds);
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    if (h > 0) return h + "h " + m + "m";
    if (m > 0) return m + "m " + s + "s";
    return "0m " + s + "s";
  }

  /** Human-readable file size: 1024 -> "1.0 KB". */
  function formatBytes(bytes) {
    if (bytes == null || isNaN(bytes) || bytes < 0) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return (unit === 0 ? value : value.toFixed(1)) + " " + units[unit];
  }

  /** Locale timestamp for a millisecond epoch value, "" when invalid. */
  function formatTimestamp(ms) {
    if (ms == null || isNaN(ms)) return "";
    const date = new Date(ms);
    if (isNaN(date.getTime())) return "";
    return date.toLocaleString();
  }

  /** Friendly label for internal meeting status values. */
  function statusLabel(status) {
    const labels = {
      recording: "Imported",
      processing: "Processing",
      completed: "Ready",
      failed: "Failed",
      archived: "Archived",
      deleted: "Deleted",
    };
    return labels[status] || status;
  }

  /** True when the status means the meeting is usable. */
  function isReady(status) {
    return status === "completed";
  }

  const api = {
    escapeHtml: escapeHtml,
    formatDuration: formatDuration,
    formatBytes: formatBytes,
    formatTimestamp: formatTimestamp,
    statusLabel: statusLabel,
    isReady: isReady,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
  global.RecapUtils = api;
})(typeof window !== "undefined" ? window : globalThis);
