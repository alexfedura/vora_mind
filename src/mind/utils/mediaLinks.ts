import type { MediaLinkPayload, MediaSlice, MediaSource } from "@/mind/types";

const MEDIA_LINK_PREFIX = "#vora-media-v1?";
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const WINDOWS_DRIVE_PATH_PATTERN = /^[a-z]:[\\/]/i;
const WINDOWS_UNC_PATH_PATTERN = /^\\\\[^\\]+\\[^\\]+/;
const REQUIRED_KEYS = [
  "v",
  "source",
  "slice",
  "start",
  "end",
  "path",
  "size",
  "modified",
] as const;

export function formatMediaTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "00:00:00";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const wholeSeconds = Math.floor(seconds % 60);
  const milliseconds = Math.floor((seconds % 1) * 1000);
  const base = [
    String(hours).padStart(2, "0"),
    String(minutes).padStart(2, "0"),
    String(wholeSeconds).padStart(2, "0"),
  ].join(":");
  return milliseconds === 0 ? base : `${base}.${String(milliseconds).padStart(3, "0")}`;
}

function escapeMarkdownLabel(label: string): string {
  return label.replace(/[\\`*_{}\[\]()#+.!|>~-]/g, "\\$&");
}

export function createMediaMarkdownLink(slice: MediaSlice, source: MediaSource): string {
  if (
    slice.file_id !== source.id ||
    !Number.isFinite(slice.start_seconds) ||
    !Number.isFinite(slice.end_seconds) ||
    slice.start_seconds < 0 ||
    slice.end_seconds <= slice.start_seconds
  ) {
    throw new Error("Cannot create a media link from an invalid source or time range.");
  }

  const params = new URLSearchParams({
    v: "1",
    source: source.id,
    slice: slice.id,
    start: String(slice.start_seconds),
    end: String(slice.end_seconds),
    path: source.path,
    size: String(source.fingerprint_size),
    modified: String(source.fingerprint_modified_ms),
  });
  const label = escapeMarkdownLabel(
    `${slice.title} (${formatMediaTime(slice.start_seconds)}-${formatMediaTime(slice.end_seconds)})`,
  );
  return `[${label}](${MEDIA_LINK_PREFIX}${params.toString()})`;
}

export function parseMediaLinkHash(hash: string): MediaLinkPayload | null {
  if (!hash.startsWith(MEDIA_LINK_PREFIX)) return null;

  const params = new URLSearchParams(hash.slice(MEDIA_LINK_PREFIX.length));
  const keys = Array.from(params.keys());
  if (
    keys.length !== REQUIRED_KEYS.length ||
    new Set(keys).size !== REQUIRED_KEYS.length ||
    REQUIRED_KEYS.some((key) => !params.has(key)) ||
    params.get("v") !== "1"
  ) {
    return null;
  }

  const sourceId = params.get("source");
  const sliceId = params.get("slice");
  const path = params.get("path");
  const startSeconds = Number(params.get("start"));
  const endSeconds = Number(params.get("end"));
  const fingerprintSize = Number(params.get("size"));
  const fingerprintModifiedMs = Number(params.get("modified"));

  if (
    sourceId === null ||
    sliceId === null ||
    path === null ||
    !UUID_PATTERN.test(sourceId) ||
    !UUID_PATTERN.test(sliceId) ||
    (!WINDOWS_DRIVE_PATH_PATTERN.test(path) && !WINDOWS_UNC_PATH_PATTERN.test(path)) ||
    path.includes("\0") ||
    !Number.isFinite(startSeconds) ||
    !Number.isFinite(endSeconds) ||
    startSeconds < 0 ||
    endSeconds <= startSeconds ||
    !Number.isSafeInteger(fingerprintSize) ||
    fingerprintSize < 0 ||
    !Number.isSafeInteger(fingerprintModifiedMs) ||
    fingerprintModifiedMs < 0
  ) {
    return null;
  }

  return {
    sourceId,
    sliceId,
    startSeconds,
    endSeconds,
    path,
    fingerprintSize,
    fingerprintModifiedMs,
  };
}
