/**
 * E2E scaffolding: drives the packaged Tauri app. Requires the app bundle
 * (tauri build) and Playwright browsers (npx playwright install). Not run in
 * CI yet — see DEVELOPMENT.md.
 */
// @ts-check
const { test, expect } = require("@playwright/test");

test.describe("ODW Recap smoke", () => {
  test.skip(process.env.RECAP_E2E_APP_PATH === undefined, "set RECAP_E2E_APP_PATH to the built app binary");

  test("library view renders", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#nav-library")).toBeVisible();
    await expect(page.locator("#status-bar")).toBeVisible();
  });
});
