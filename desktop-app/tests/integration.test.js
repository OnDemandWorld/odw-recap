// Integration tests for Recap desktop app Tauri commands
// These tests would be run with @tauri-apps/api in a Tauri testing environment

import { invoke } from "@tauri-apps/api/tauri";

describe("Desktop App Integration", () => {
  test("should list meetings", async () => {
    const meetings = await invoke("list_meetings", { limit: 10, offset: 0 });
    expect(Array.isArray(meetings)).toBe(true);
  });

  test("should get supported audio formats", async () => {
    const formats = await invoke("get_supported_audio_formats");
    expect(formats).toContain("m4a");
    expect(formats).toContain("mp3");
    expect(formats).toContain("wav");
  });

  test("should create a meeting", async () => {
    const id = await invoke("create_meeting", { title: "Integration Test Meeting" });
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(0);
  });

  test("should get config", async () => {
    const value = await invoke("get_config", { key: "nonexistent" });
    expect(value).toBeNull();
  });
});
