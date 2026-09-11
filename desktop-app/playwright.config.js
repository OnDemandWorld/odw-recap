/** Playwright config for the (scaffolded) Tauri E2E tests. */
module.exports = {
  testDir: "tests/e2e",
  use: {
    // Tauri webview testing needs the built app; see DEVELOPMENT.md.
  },
};
