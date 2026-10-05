import { backendHeaders } from "../desktop/runtime";

const DEFAULT_BACKEND_URL = "http://127.0.0.1:8000";

export async function downloadSectionArchive(
  importId: string, sectionId: string, fetcher: typeof fetch = fetch,
): Promise<Blob> {
  const configuredUrl = import.meta.env.VITE_BACKEND_URL as string | undefined;
  const baseUrl = configuredUrl?.replace(/\/$/, "") ?? DEFAULT_BACKEND_URL;
  let response: Response;
  try {
    response = await fetcher(`${baseUrl}/api/v1/export/dxf`, {
      method: "POST", headers: await backendHeaders({ Accept: "application/zip", "Content-Type": "application/json" }),
      body: JSON.stringify({ import_id: importId, section_id: sectionId }),
    });
  } catch {
    throw new Error("The local geometry service is unavailable.");
  }
  if (!response.ok) {
    const payload: unknown = await response.json().catch(() => null);
    if (record(payload) && record(payload.error) && typeof payload.error.message === "string") {
      throw new Error(payload.error.message);
    }
    throw new Error("The DXF could not be exported.");
  }
  if (!response.headers.get("Content-Type")?.includes("application/zip")) {
    throw new Error("The geometry service returned an invalid DXF download.");
  }
  return response.blob();
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
