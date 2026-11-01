import { DEFAULT_NUMBER_FORMAT, type NumberFormat } from "../../lib/formatting";
import type { SectionPlaneKind, Vec3 } from "../section/plane";

export const PROJECT_VERSION = 1;
export const PROJECT_EXTENSION = ".c2mproj";

/** `.c2mproj` content: settings and a reference to the STEP file, never geometry. */
export interface ProjectFile {
  app: "CAD2Maxwell";
  version: number;
  step: { path: string; sha256: string };
  plane: { kind: SectionPlaneKind; normal_xyz: Vec3; offset_mm: number | null };
  number_format: NumberFormat;
}

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const isVec3 = (v: unknown): v is Vec3 => Array.isArray(v) && v.length === 3 && v.every((n) => typeof n === "number" && Number.isFinite(n));

export function isProjectFile(value: unknown): value is ProjectFile {
  if (!isRecord(value) || value.app !== "CAD2Maxwell" || value.version !== PROJECT_VERSION) return false;
  const { step, plane, number_format: format } = value;
  return isRecord(step) && typeof step.path === "string" && typeof step.sha256 === "string" &&
    isRecord(plane) && ["XY", "XZ", "YZ", "custom"].includes(plane.kind as string) && isVec3(plane.normal_xyz) &&
    (plane.offset_mm === null || (typeof plane.offset_mm === "number" && Number.isFinite(plane.offset_mm))) &&
    isRecord(format);
}

/** Parses project text; throws an Error with a user-readable message. */
export function parseProject(text: string): ProjectFile {
  let value: unknown;
  try { value = JSON.parse(text); } catch { throw new Error("The project file is not valid JSON."); }
  if (isRecord(value) && value.app === "CAD2Maxwell" && typeof value.version === "number" && value.version > PROJECT_VERSION) {
    throw new Error(`The project file version ${String(value.version)} is newer than this application supports (${String(PROJECT_VERSION)}). Update CAD2Maxwell to open it.`);
  }
  if (!isProjectFile(value)) throw new Error("The file is not a valid CAD2Maxwell project.");
  return { ...value, number_format: { ...DEFAULT_NUMBER_FORMAT, ...value.number_format } };
}

export const serializeProject = (project: ProjectFile): string => `${JSON.stringify(project, null, 2)}\n`;
