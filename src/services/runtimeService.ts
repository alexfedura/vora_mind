export interface ElectronRuntimeInfo {
  kind: "electron";
  electron: string;
  node: string;
  platform: string;
  arch: string;
  packaged: boolean;
  appVersion: string;
}

export interface TauriRuntimeInfo {
  kind: "tauri";
  platform: string;
  arch: string;
  appVersion: string | null;
}

export type RuntimeInfo = ElectronRuntimeInfo | TauriRuntimeInfo;

export function isElectronRuntime(): boolean {
  return typeof window !== "undefined" &&
    typeof window.voraElectron?.getRuntimeInfo === "function";
}

export async function getRuntimeInfo(): Promise<RuntimeInfo> {
  if (isElectronRuntime()) {
    const electronApi = window.voraElectron;
    if (!electronApi) throw new Error("Electron runtime bridge is unavailable");
    const result = await electronApi.getRuntimeInfo();
    if (!result.ok) {
      throw new Error(result.error?.message ?? "Electron runtime information unavailable");
    }
    return {
      kind: "electron",
      ...result.result
    };
  }

  return {
    kind: "tauri",
    platform: navigator.platform,
    arch: navigator.userAgent,
    appVersion: null
  };
}
