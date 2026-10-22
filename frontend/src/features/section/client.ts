import { isComponentPlane, isSectionResponse, type ComponentPlane, type SectionResponse, type SectionResponsePlane } from "./contracts";
import type { SectionPlaneRequest } from "./plane";
import { backendHeaders } from "../desktop/runtime";

const DEFAULT_BACKEND_URL = "http://127.0.0.1:8000";

function backendUrl(): string {
  const configuredUrl = import.meta.env.VITE_BACKEND_URL as string | undefined;
  return configuredUrl?.replace(/\/$/, "") ?? DEFAULT_BACKEND_URL;
}

export async function computeSection(
  importId: string, plane: SectionPlaneRequest,
  fetcher: typeof fetch = fetch,
): Promise<SectionResponse> {
  let response: Response;
  try {
    response = await fetcher(`${backendUrl()}/api/v1/section`, {
      method: "POST", headers: await backendHeaders({ Accept: "application/json", "Content-Type": "application/json" }),
      body: JSON.stringify({ import_id: importId, plane }),
    });
  } catch {
    throw new Error("The local geometry service is unavailable.");
  }
  const payload = await readPayload(response, "The exact section could not be completed.");
  if (!isSectionResponse(payload) || payload.import_id !== importId || !samePlane(payload.plane, plane)) {
    throw new Error("The geometry service returned an invalid section response.");
  }
  return payload;
}

export async function fetchComponentPlane(
  importId: string, componentId: string, fetcher: typeof fetch = fetch,
): Promise<ComponentPlane> {
  let response: Response;
  try {
    response = await fetcher(
      `${backendUrl()}/api/v1/imports/${encodeURIComponent(importId)}/components/${encodeURIComponent(componentId)}/plane`,
      { headers: await backendHeaders({ Accept: "application/json" }) },
    );
  } catch {
    throw new Error("The local geometry service is unavailable.");
  }
  const payload = await readPayload(response, "The body plane could not be determined.");
  if (!isComponentPlane(payload) || payload.component_id !== componentId) {
    throw new Error("The geometry service returned an invalid plane response.");
  }
  return payload;
}

async function readPayload(response: Response, failure: string): Promise<unknown> {
  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new Error("The geometry service returned an invalid response.");
  }
  if (!response.ok) {
    if (record(payload) && record(payload.error) && typeof payload.error.message === "string") {
      throw new Error(payload.error.message);
    }
    throw new Error(failure);
  }
  return payload;
}

function samePlane(actual: SectionResponsePlane, requested: SectionPlaneRequest): boolean {
  if (requested.kind !== "custom") return actual.kind === requested.kind && "offset_mm" in actual &&
    actual.offset_mm === requested.offset_mm;
  if (actual.kind !== "custom") return false;
  const same = (a: number[], b: number[]) => a.every((value, index) => value === b[index]);
  return same(actual.origin_xyz, requested.origin_xyz) && same(actual.normal_xyz, requested.normal_xyz) &&
    same(actual.x_dir_xyz, requested.x_dir_xyz);
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
