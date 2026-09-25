import { describe, expect, it } from "vitest";

import { downloadSectionArchive } from "./exportClient";

describe("downloadSectionArchive", () => {
  it("returns the DXF bundle for matching sessions", async () => {
    const bytes = new Uint8Array([80, 75, 3, 4]);
    const fetcher: typeof fetch = (_url, init) => {
      expect(JSON.parse(init?.body as string)).toEqual({ import_id: "import-1", section_id: "section-1" });
      return Promise.resolve(new Response(bytes, { headers: { "Content-Type": "application/zip" } }));
    };

    const blob = await downloadSectionArchive("import-1", "section-1", fetcher);
    expect(blob.type).toBe("application/zip");
    expect(blob.size).toBe(4);
  });

  it("shows a backend export error", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      error: { code: "export_not_ready", message: "The section has open wires." },
    }), { status: 422 }));
    await expect(downloadSectionArchive("import-1", "section-1", fetcher))
      .rejects.toThrow("The section has open wires.");
  });
});
