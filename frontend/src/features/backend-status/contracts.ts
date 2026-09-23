export interface HealthResponse {
  status: "ok";
  service: string;
  api_version: string;
}

export type BackendHealth =
  | { state: "online"; service: string; apiVersion: string }
  | { state: "offline" };

export function isHealthResponse(value: unknown): value is HealthResponse {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    candidate.status === "ok" &&
    typeof candidate.service === "string" &&
    typeof candidate.api_version === "string"
  );
}
