import { type BackendHealth, isHealthResponse } from "./contracts";
import { backendHeaders } from "../desktop/runtime";

const DEFAULT_BACKEND_URL = "http://127.0.0.1:8000";

export async function fetchBackendHealth(fetcher: typeof fetch = fetch): Promise<BackendHealth> {
  const configuredUrl = import.meta.env.VITE_BACKEND_URL as string | undefined;
  const baseUrl = configuredUrl?.replace(/\/$/, "") ?? DEFAULT_BACKEND_URL;

  try {
    const response = await fetcher(`${baseUrl}/api/v1/health`, {
      headers: await backendHeaders({ Accept: "application/json" }),
    });
    if (!response.ok) return { state: "offline" };

    const payload: unknown = await response.json();
    if (!isHealthResponse(payload)) return { state: "offline" };

    return {
      state: "online",
      service: payload.service,
      apiVersion: payload.api_version,
    };
  } catch {
    return { state: "offline" };
  }
}
