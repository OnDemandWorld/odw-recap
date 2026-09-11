/** Jest runs pure frontend helpers only — no DOM, no Tauri runtime needed. */
module.exports = {
  testEnvironment: "node",
  testMatch: ["**/tests/**/*.test.js"],
  testPathIgnorePatterns: ["/node_modules/"],
};
