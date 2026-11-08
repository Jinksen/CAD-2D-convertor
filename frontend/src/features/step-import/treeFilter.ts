import type { ImportComponent } from "./contracts";

/** Case-insensitive match on display name, export name, or any hierarchy segment. */
export function bodyMatchesFilter(component: ImportComponent, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) return true;
  const fields = [component.display_name, component.export_name, ...component.hierarchy_path];
  return fields.some((field) => field.toLowerCase().includes(needle));
}
