import { act, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { useProjectStore } from "./projectActions";
import { NOTICE_MS, ProjectStatus } from "./ProjectStatus";

afterEach(() => { vi.useRealTimers(); useProjectStore.setState({ notice: null, warning: null, error: null }); });

it("hides a plain notice after a while but keeps warnings until dismissed", () => {
  vi.useFakeTimers();
  render(<ProjectStatus />);
  act(() => { useProjectStore.setState({ notice: "Project saved." }); });
  expect(screen.getByRole("status")).toHaveTextContent("Project saved.");
  act(() => { vi.advanceTimersByTime(NOTICE_MS); });
  expect(screen.queryByRole("status")).not.toBeInTheDocument();

  act(() => { useProjectStore.setState({ warning: "SHA-256 mismatch" }); });
  act(() => { vi.advanceTimersByTime(NOTICE_MS * 2); });
  expect(screen.getByRole("alert")).toHaveTextContent("SHA-256 mismatch");
  act(() => { screen.getByRole("button", { name: "Dismiss" }).click(); });
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
