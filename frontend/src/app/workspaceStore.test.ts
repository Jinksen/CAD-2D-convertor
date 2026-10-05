import { expect, it } from "vitest";
import { useWorkspaceStore } from "./workspaceStore";

it("keeps the computed offset when the active plane is clicked again", () => {
  useWorkspaceStore.setState({ activePlane: "XY", sectionOffsetMm: 12.5 });
  useWorkspaceStore.getState().setActivePlane("XY");
  expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(12.5);
  useWorkspaceStore.getState().setActivePlane("XZ");
  expect(useWorkspaceStore.getState().sectionOffsetMm).toBeNull();
});
