import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { HISTORY_COALESCE_MS, useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import { applyProject, currentProject, useProjectStore } from "./projectActions";
import { parseProject, serializeProject } from "./projectFile";

const importStep = vi.hoisted(() => vi.fn());
vi.mock("../step-import/client", () => ({ importStep }));

const summary = (sha256: string) => ({ import_id: "i1", path: "C:\\m.step", sha256 }) as unknown as ImportSummary;
const saved = () => { const p = currentProject(); if (!p) throw new Error("no project"); return p; };
const state = () => useWorkspaceStore.getState();

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(1_000_000);
  state().setImported(summary("aa"));
  useWorkspaceStore.setState({ activePlane: "XY", sectionOffsetMm: null });
});
afterEach(() => { vi.useRealTimers(); });

it("round-trips a project and restores the plane", async () => {
  useWorkspaceStore.setState({ activePlane: "XZ", sectionOffsetMm: 4.5 });
  const project = parseProject(serializeProject(saved()));
  expect(project.plane).toEqual({ kind: "XZ", normal_xyz: [0, 0, 1], offset_mm: 4.5 });
  expect(JSON.stringify(project)).not.toContain("components");
  importStep.mockResolvedValue(summary("aa"));
  useWorkspaceStore.setState({ activePlane: "XY", sectionOffsetMm: null });
  await applyProject(project, "C:\\p.c2mproj");
  expect(state()).toMatchObject({ activePlane: "XZ", sectionOffsetMm: 4.5 });
  expect(useProjectStore.getState()).toMatchObject({ warning: null, path: "C:\\p.c2mproj" });
});

it("warns visibly when the STEP hash changed", async () => {
  const project = parseProject(serializeProject(saved()));
  importStep.mockResolvedValue(summary("bb"));
  await applyProject(project, null);
  expect(useProjectStore.getState().warning).toMatch(/SHA-256 mismatch/);
});

it("rejects future versions and malformed files", () => {
  const text = serializeProject(saved());
  expect(() => parseProject(text.replace('"version": 1', '"version": 2'))).toThrow(/newer/);
  expect(() => parseProject("{}")).toThrow(/not a valid/);
  expect(() => parseProject("nope")).toThrow(/JSON/);
});

it("undoes and redoes plane edits, coalescing typing into one step", () => {
  state().setSectionOffsetMm(1);
  vi.advanceTimersByTime(100);
  state().setSectionOffsetMm(12);
  vi.advanceTimersByTime(HISTORY_COALESCE_MS + 1);
  state().setActivePlane("YZ");
  state().undoPlane();
  expect(state()).toMatchObject({ activePlane: "XY", sectionOffsetMm: 12 });
  state().undoPlane();
  expect(state().sectionOffsetMm).toBeNull();
  state().redoPlane();
  expect(state().sectionOffsetMm).toBe(12);
  state().redoPlane();
  expect(state().activePlane).toBe("YZ");
  state().undoPlane();
  state().setImported(summary("aa"));
  expect(state().planeHistory).toEqual({ past: [], future: [] });
});
