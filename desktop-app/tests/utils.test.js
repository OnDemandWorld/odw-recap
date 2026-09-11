/** Unit tests for the shared frontend helpers in src/utils.js. */
const utils = require("../src/utils.js");

describe("escapeHtml", () => {
  test("escapes markup-significant characters", () => {
    expect(utils.escapeHtml('<script>alert("x")</script>')).toBe(
      "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"
    );
  });

  test("escapes ampersands and single quotes", () => {
    expect(utils.escapeHtml("Tom & Jerry's meeting")).toBe("Tom &amp; Jerry&#39;s meeting");
  });

  test("handles null, undefined and non-strings", () => {
    expect(utils.escapeHtml(null)).toBe("");
    expect(utils.escapeHtml(undefined)).toBe("");
    expect(utils.escapeHtml(42)).toBe("42");
  });
});

describe("formatDuration", () => {
  test("formats hours, minutes and seconds", () => {
    expect(utils.formatDuration(3675)).toBe("1h 1m");
    expect(utils.formatDuration(95)).toBe("1m 35s");
    expect(utils.formatDuration(42)).toBe("0m 42s");
  });

  test("rounds fractional seconds", () => {
    expect(utils.formatDuration(61.4)).toBe("1m 1s");
  });

  test("returns empty string for invalid input", () => {
    expect(utils.formatDuration(null)).toBe("");
    expect(utils.formatDuration(-5)).toBe("");
    expect(utils.formatDuration("not-a-number")).toBe("");
  });
});

describe("formatBytes", () => {
  test("formats bytes, KB and MB", () => {
    expect(utils.formatBytes(512)).toBe("512 B");
    expect(utils.formatBytes(2048)).toBe("2.0 KB");
    expect(utils.formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  test("returns empty string for invalid input", () => {
    expect(utils.formatBytes(null)).toBe("");
    expect(utils.formatBytes(-1)).toBe("");
  });
});

describe("formatTimestamp", () => {
  test("formats a millisecond epoch into a parseable locale string", () => {
    const formatted = utils.formatTimestamp(0);
    expect(formatted).toContain("1970");
  });

  test("returns empty string for invalid input", () => {
    expect(utils.formatTimestamp(null)).toBe("");
    expect(utils.formatTimestamp("abc")).toBe("");
  });
});

describe("statusLabel / isReady", () => {
  test("maps internal statuses to friendly labels", () => {
    expect(utils.statusLabel("completed")).toBe("Ready");
    expect(utils.statusLabel("recording")).toBe("Imported");
    expect(utils.statusLabel("failed")).toBe("Failed");
  });

  test("falls back to the raw status for unknown values", () => {
    expect(utils.statusLabel("mystery")).toBe("mystery");
  });

  test("isReady is only true for completed meetings", () => {
    expect(utils.isReady("completed")).toBe(true);
    expect(utils.isReady("recording")).toBe(false);
  });
});
