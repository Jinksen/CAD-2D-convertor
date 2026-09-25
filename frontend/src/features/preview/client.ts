import { isPreviewMeshResponse, type PreviewMeshResponse } from "./contracts";

export async function fetchPreview(importId: string, fetcher: typeof fetch = fetch): Promise<PreviewMeshResponse> {
  const configuredUrl = import.meta.env.VITE_BACKEND_URL as string | undefined;
  const baseUrl = configuredUrl?.replace(/\/$/, "") ?? "http://127.0.0.1:8000";
  let response: Response;
  try {
    response = await fetcher(`${baseUrl}/api/v1/imports/${encodeURIComponent(importId)}/preview`);
  } catch {
    throw new Error("The local geometry service is unavailable.");
  }
  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new Error("The geometry service returned an invalid preview response.");
  }
  if (!response.ok) {
    if (typeof payload === "object" && payload !== null && "error" in payload &&
        typeof payload.error === "object" && payload.error !== null && "message" in payload.error &&
        typeof payload.error.message === "string") throw new Error(payload.error.message);
    throw new Error("The 3D preview could not be loaded.");
  }
  if (!isPreviewMeshResponse(payload) || payload.import_id !== importId) {
    throw new Error("The geometry service returned an invalid preview response.");
  }
  return payload;
}
