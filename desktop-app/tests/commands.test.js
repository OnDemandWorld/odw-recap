/**
 * Command contract test: every command the frontend invokes must be
 * registered in the Rust backend's invoke_handler. This prevents the classic
 * drift bug where the UI calls `invoke("x")` for a command that was renamed
 * or never registered (which only surfaces at runtime in the packaged app).
 */
const fs = require("fs");
const path = require("path");

const mainJs = fs.readFileSync(path.join(__dirname, "..", "src", "main.js"), "utf8");
const mainRs = fs.readFileSync(
  path.join(__dirname, "..", "src-tauri", "src", "main.rs"),
  "utf8"
);

describe("frontend/backend command contract", () => {
  const invoked = new Set();
  for (const match of mainJs.matchAll(/invoke\(\s*["']([a-z_]+)["']/g)) {
    invoked.add(match[1]);
  }

  const handlerMatch = mainRs.match(/generate_handler!\s*\[([^\]]*)\]/s);
  const registered = new Set();
  if (handlerMatch) {
    for (const entry of handlerMatch[1].split(",")) {
      const name = entry.trim().replace(/\s+$/, "");
      if (name) registered.add(name);
    }
  }

  test("generate_handler! block was found in main.rs", () => {
    expect(handlerMatch).not.toBeNull();
    expect(registered.size).toBeGreaterThan(10);
  });

  test("frontend was found", () => {
    expect(invoked.size).toBeGreaterThan(10);
  });

  test("every invoked command is registered in the backend", () => {
    const missing = [...invoked].filter((cmd) => !registered.has(cmd));
    expect(missing).toEqual([]);
  });

  test("commands the UI depends on exist", () => {
    for (const critical of [
      "import_audio_file",
      "list_meetings",
      "get_meeting",
      "get_transcript",
      "get_summary",
      "get_action_items",
      "get_decisions",
      "process_meeting",
      "save_api_key",
      "start_watch_folder",
      "start_http_upload_server",
    ]) {
      expect(registered.has(critical)).toBe(true);
    }
  });
});

describe("frontend loads without a bundler", () => {
  const indexHtml = fs.readFileSync(
    path.join(__dirname, "..", "src", "index.html"),
    "utf8"
  );

  test("main.js is loaded as a plain script, not a bare-import module", () => {
    // Bare ESM imports cannot resolve without a bundler; the app uses the
    // injected window.__TAURI__ global instead.
    expect(indexHtml).toContain('<script src="main.js"');
    expect(indexHtml).not.toMatch(/<script[^>]*type=["']module["'][^>]*src=["']\/?main\.js["']/);
  });

  test("main.js does not use bare module imports", () => {
    expect(mainJs).not.toMatch(/^\s*import\s+.*from\s+["']@tauri-apps\//m);
  });
});
