import type { VoraSnapshot } from "./types";
import { SNAPSHOT_FORMAT, SNAPSHOT_SCHEMA_VERSION } from "./types";

export async function computeChecksum(snapshot: VoraSnapshot): Promise<string> {
  const { checksum, ...payload } = snapshot;
  void checksum;
  const json = JSON.stringify(payload);
  const buf = new TextEncoder().encode(json);
  const hash = await crypto.subtle.digest("SHA-256", buf);
  return Array.from(new Uint8Array(hash))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export function serialize(snapshot: VoraSnapshot): string {
  return JSON.stringify(snapshot, null, 2);
}

export function deserialize(json: string): VoraSnapshot {
  const raw = JSON.parse(json) as Record<string, unknown>;
  if (raw["format"] !== SNAPSHOT_FORMAT) {
    throw new Error(`Invalid snapshot format: ${String(raw["format"])}`);
  }
  if (typeof raw["schemaVersion"] !== "number") {
    throw new Error("Missing schemaVersion");
  }
  if ((raw["schemaVersion"] as number) > SNAPSHOT_SCHEMA_VERSION) {
    throw new Error(
      `Snapshot version ${String(raw["schemaVersion"])} is newer than supported ${SNAPSHOT_SCHEMA_VERSION}.`
    );
  }
  return raw as unknown as VoraSnapshot;
}

export async function verify(snapshot: VoraSnapshot): Promise<boolean> {
  const expected = await computeChecksum(snapshot);
  return expected === snapshot.checksum;
}
