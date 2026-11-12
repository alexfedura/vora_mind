import { convertFileSrc } from "@tauri-apps/api/core";

export function toAssetUrl(filePath: string): string {
  return convertFileSrc(filePath);
}
