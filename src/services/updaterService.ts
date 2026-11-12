import { invoke } from "@tauri-apps/api/core";
import type { UpdaterFailureMarkerDto } from "@/types";

export const updaterService = {
  install(requestedVersion: string): Promise<string | null> {
    return invoke<string | null>("install_update", { requestedVersion });
  },

  getLastFailureMarker(): Promise<UpdaterFailureMarkerDto | null> {
    return invoke<UpdaterFailureMarkerDto | null>("get_last_update_failure");
  },
};
