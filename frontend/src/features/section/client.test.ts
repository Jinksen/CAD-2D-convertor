import { describe, expect, it } from "vitest";

import { computeSection } from "./client";

describe("computeSection", () => {
  it("rejects a section returned for a different plane", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      section_id: "section-1", import_id: "import-1",
      plane: { kind: "XZ", offset_mm: 10 }, components: [], diagnostics: [],
    })));
    await expect(computeSection("import-1", "XY", 10, fetcher))
      .rejects.toThrow("invalid section response");
  });

  it("reports an expired import session from the backend", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      error: { code: "unknown_import", message: "Import the file again." },
    }), { status: 404 }));
    await expect(computeSection("expired", "XY", 10, fetcher))
      .rejects.toThrow("Import the file again.");
  });
});
