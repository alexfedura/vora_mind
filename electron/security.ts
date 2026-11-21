import type { IpcMainInvokeEvent, WebContents } from "electron";

export function isAllowedFrameUrl(frameUrl: string | undefined, allowedUrls: readonly string[]): boolean {
  return typeof frameUrl === "string" && allowedUrls.includes(frameUrl);
}

export function authorizeInvoke(
  event: IpcMainInvokeEvent,
  allowedWebContents: WebContents,
  allowedUrls: readonly string[],
): void {
  if (event.sender !== allowedWebContents) {
    throw Object.assign(new Error("Unauthorized IPC sender"), { code: "IPC_SENDER_UNAUTHORIZED" });
  }
  if (!isAllowedFrameUrl(event.senderFrame?.url, allowedUrls)) {
    throw Object.assign(new Error("Unauthorized IPC frame"), { code: "IPC_FRAME_UNAUTHORIZED" });
  }
}

export function validateShellStatusPayload(input: unknown): Record<string, never> {
  if (input !== undefined) {
    throw Object.assign(new Error("Invalid shell status payload"), { code: "INVALID_REQUEST" });
  }
  return {};
}
