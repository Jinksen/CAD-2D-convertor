import { act, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { fetchBackendHealth } from "../features/backend-status/client";
import { App, HEALTH_RETRY_MS } from "./App";

vi.mock("../features/backend-status/client", () => ({ fetchBackendHealth: vi.fn() }));

afterEach(() => { vi.useRealTimers(); });

it("keeps checking the geometry service until it comes online, then stops", async () => {
  vi.useFakeTimers();
  vi.mocked(fetchBackendHealth)
    .mockResolvedValueOnce({ state: "offline" })
    .mockResolvedValue({ state: "online", service: "cad2maxwell-backend", apiVersion: "v1" });
  render(<App />);
  await act(async () => { await Promise.resolve(); });
  expect(screen.getByText(/Backend Offline/)).toBeInTheDocument();
  await act(async () => { await vi.advanceTimersByTimeAsync(HEALTH_RETRY_MS); });
  expect(screen.getByText(/Backend Online/)).toBeInTheDocument();
  await act(async () => { await vi.advanceTimersByTimeAsync(HEALTH_RETRY_MS * 5); });
  expect(fetchBackendHealth).toHaveBeenCalledTimes(2);
});
