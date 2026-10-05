import { afterEach, describe, expect, it, vi } from "vitest";

import { backendHeaders, saveSectionArchive } from "./runtime";

afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); });

describe("desktop runtime", () => {
  it("reads the launch token through the desktop bridge", async () => {
    const invoke = vi.fn().mockResolvedValue("session-token");
    vi.stubGlobal("__TAURI__", { core: { invoke } });
    await expect(backendHeaders({ Accept: "application/json" })).resolves.toEqual({
      Accept: "application/json", "X-Session-Token": "session-token",
    });
    expect(invoke).toHaveBeenCalledWith("backend_session_token");
  });

  it("uses the development token in a browser harness", async () => {
    vi.stubEnv("VITE_SESSION_TOKEN", "browser-session");
    await expect(backendHeaders({ Accept: "application/json" })).resolves.toEqual({
      Accept: "application/json", "X-Session-Token": "browser-session",
    });
  });

  it("sends the ZIP bytes to a native save dialog and returns the saved path", async () => {
    const invoke = vi.fn().mockResolvedValue("C:\\Exports\\section.zip");
    vi.stubGlobal("__TAURI__", { core: { invoke } });
    const archive = new Blob([new Uint8Array([80, 75, 3, 4])]);
    await expect(saveSectionArchive(archive)).resolves.toBe("C:\\Exports\\section.zip");
    expect(invoke).toHaveBeenCalledWith("save_section_archive", { bytes: [80, 75, 3, 4] });
  });

  it("treats cancelling the native dialog as cancellation", async () => {
    vi.stubGlobal("__TAURI__", { core: { invoke: vi.fn().mockResolvedValue(null) } });
    await expect(saveSectionArchive(new Blob(["zip"]))).resolves.toBeNull();
  });
});
