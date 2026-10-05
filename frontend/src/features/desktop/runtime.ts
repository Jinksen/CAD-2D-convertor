interface DesktopApi {
  core: { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> };
}

function desktopApi(): DesktopApi | undefined {
  return (window as Window & { __TAURI__?: DesktopApi }).__TAURI__;
}

export async function backendHeaders(headers: Record<string, string>): Promise<Record<string, string>> {
  const desktop = desktopApi();
  const token = desktop
    ? await desktop.core.invoke<string | null>("backend_session_token")
    : import.meta.env.VITE_SESSION_TOKEN as string | undefined;
  return token ? { ...headers, "X-Session-Token": token } : headers;
}

export async function saveSectionArchive(archive: Blob): Promise<string | null> {
  const desktop = desktopApi();
  if (desktop) {
    if (archive.size > 64 * 1024 * 1024) throw new Error("The DXF bundle exceeds the 64 MiB save limit.");
    const bytes = await new Promise<ArrayBuffer>((resolve, reject) => {
      const reader = new FileReader();
      reader.onerror = () => { reject(new Error("The DXF bundle could not be read.")); };
      reader.onload = () => {
        if (reader.result instanceof ArrayBuffer) resolve(reader.result);
        else reject(new Error("The DXF bundle could not be read."));
      };
      reader.readAsArrayBuffer(archive);
    });
    try {
      return await desktop.core.invoke<string | null>("save_section_archive", {
        bytes: Array.from(new Uint8Array(bytes)),
      });
    } catch (cause) {
      throw new Error(typeof cause === "string" ? cause : "The DXF bundle could not be saved.", { cause });
    }
  }
  const url = URL.createObjectURL(archive);
  const link = document.createElement("a");
  link.href = url;
  link.download = "section.zip";
  document.body.append(link);
  try { link.click(); }
  finally { link.remove(); URL.revokeObjectURL(url); }
  return "section.zip (browser download requested)";
}
