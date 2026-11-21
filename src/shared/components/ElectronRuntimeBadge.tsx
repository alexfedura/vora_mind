import React, { useEffect, useState } from "react";
import { getRuntimeInfo, type RuntimeInfo } from "@/services/runtimeService";

export const ElectronRuntimeBadge: React.FC = () => {
  const [runtime, setRuntime] = useState<RuntimeInfo | null>(null);

  useEffect(() => {
    let active = true;
    void getRuntimeInfo().then((info) => {
      if (active) setRuntime(info);
    }).catch((error: unknown) => {
      console.error("Could not read the application runtime:", error);
    });
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
    </div>
  );
};
