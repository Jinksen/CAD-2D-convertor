import { isSectionResponse, type SectionResponse } from "./contracts";

const DEFAULT_BACKEND_URL = "http://127.0.0.1:8000";

export async function computeSection(
  importId: string, plane: "XY" | "XZ" | "YZ", offsetMm: number,
  fetcher: typeof fetch = fetch,
): Promise<SectionResponse> {
  const configuredUrl = import.meta.env.VITE_BACKEND_URL as string | undefined;
  const baseUrl = configuredUrl?.replace(/\/$/, "") ?? DEFAULT_BACKEND_URL;
  let response: Response;
  try {
    response = await fetcher(`${baseUrl}/api/v1/section`, {
      method: "POST", headers: { Accept: "application/json", "Content-Type": "application/json" },
      body: JSON.stringify({ import_id: importId, plane: { kind: plane, offset_mm: offsetMm } }),
    });
  } catch {
    throw new Error("The local geometry service is unavailable.");
  }
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
    throw new Error("The exact section could not be completed.");
  }
  if (!isSectionResponse(payload) || payload.import_id !== importId ||
    payload.plane.kind !== plane || payload.plane.offset_mm !== offsetMm) {
    throw new Error("The geometry service returned an invalid section response.");
  }
  return payload;
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
