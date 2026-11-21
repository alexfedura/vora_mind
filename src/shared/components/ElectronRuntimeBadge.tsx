import React, { useEffect, useState } from "react";
import { getRuntimeInfo, type RuntimeInfo } from "@/services/runtimeService";

export const ElectronRuntimeBadge: React.FC = () => {
  const [runtime, setRuntime] = useState<RuntimeInfo | null>(null);
  const [sidecar, setSidecar] = useState<"checking" | "ready" | "blocked">("checking");
  const [engine, setEngine] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void getRuntimeInfo().then((info) => {
      if (active) setRuntime(info);
    }).catch((error: unknown) => {
      console.error("Could not read the application runtime:", error);
    });
    if (window.voraElectron?.getSidecarStatus) {
      void window.voraElectron.getSidecarStatus().then((response) => {
        if (active) setSidecar(response.ok ? "ready" : "blocked");
      }).catch(() => {
        if (active) setSidecar("blocked");
      });
      void window.voraElectron.getSidecarEngineInfo().then((response) => {
        if (!active || !response.ok) return;
        const result = response.result as { engine?: string } | undefined;
        setEngine(result?.engine ?? null);
      }).catch(() => undefined);
    } else if (active) {
      setSidecar("blocked");
    }
    return () => {
      active = false;
    };
  }, []);

  if (!runtime) return null;

  return (
    <div
      className="px-2 py-1 text-[10px] text-zinc-500"
      title={runtime.kind === "electron"
        ? `Electron ${runtime.electron} / ${runtime.platform} / ${runtime.arch}`
        : "Tauri runtime"}
    >
      {runtime.kind === "electron"
        ? `Electron ${runtime.appVersion}`
        : "Tauri"}
      {runtime.kind === "electron" && (
        <span className="ml-1 text-[9px]">
          / Sidecar {sidecar}
          {engine ? ` / ${engine}` : ""}
        </span>
      )}
    </div>
  );
};
