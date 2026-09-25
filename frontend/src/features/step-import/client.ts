import { isImportSummary, type ImportSummary } from "./contracts";

const DEFAULT_BACKEND_URL = "http://127.0.0.1:8000";

export async function importStep(path: string, fetcher: typeof fetch = fetch): Promise<ImportSummary> {
  const configuredUrl = import.meta.env.VITE_BACKEND_URL as string | undefined;
  const baseUrl = configuredUrl?.replace(/\/$/, "") ?? DEFAULT_BACKEND_URL;
  let response: Response;
  try {
    response = await fetcher(`${baseUrl}/api/v1/imports/step`, {
      method: "POST",
      headers: { Accept: "application/json", "Content-Type": "application/json" },
      body: JSON.stringify({ path }),
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
    if (typeof payload === "object" && payload !== null && "error" in payload &&
      typeof payload.error === "object" && payload.error !== null && "message" in payload.error &&
      typeof payload.error.message === "string") {
      throw new Error(payload.error.message);
    }
    throw new Error("The STEP file could not be imported.");
  }
  if (!isImportSummary(payload)) throw new Error("The geometry service returned an invalid import response.");
  return payload;
}
