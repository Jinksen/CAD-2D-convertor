import { describe, expect, it } from "vitest";

import { fetchBackendHealth } from "./client";

describe("fetchBackendHealth", () => {
  it("maps a valid response to online", async () => {
    const fetcher: typeof fetch = () =>
      Promise.resolve(new Response(
        JSON.stringify({ status: "ok", service: "cad2maxwell-backend", api_version: "v1" }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ));

    await expect(fetchBackendHealth(fetcher)).resolves.toEqual({
      state: "online",
      service: "cad2maxwell-backend",
      apiVersion: "v1",
    });
  });

  it.each([
    ["rejected request", () => Promise.reject(new Error("offline"))],
    ["malformed body", () => Promise.resolve(new Response(JSON.stringify({ status: "ok" })))],
    ["non-2xx response", () => Promise.resolve(new Response(null, { status: 503 }))],
  ])("maps %s to offline", async (_case, fetcher) => {
    await expect(fetchBackendHealth(fetcher as typeof fetch)).resolves.toEqual({
      state: "offline",
    });
  });
});
