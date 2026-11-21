import { z } from "zod";

export const SidecarRequestSchema = z.object({
  protocolVersion: z.literal(1),
  requestId: z.string().min(1).max(128),
  method: z.enum([
    "health",
    "create_synthetic_database",
    "execute_approved_query",
    "create_parquet_fixture",
    "read_parquet_fixture",
    "shutdown"
  ]),
  params: z.record(z.string(), z.unknown()).default({})
});

export type SidecarRequest = z.infer<typeof SidecarRequestSchema>;

export const SidecarResponseSchema = z.object({
  protocolVersion: z.literal(1),
  requestId: z.string().min(1).max(128),
  ok: z.boolean(),
  result: z.unknown().optional(),
  error: z.object({
    code: z.string().min(1).max(128),
    message: z.string().min(1).max(4096)
  }).optional()
}).strict().refine((response) =>
  response.ok ? response.result !== undefined : response.error !== undefined,
  { message: "Response must contain result or error" }
);

export interface SidecarResponse {
  protocolVersion: number;
  requestId: string;
  ok: boolean;
  result?: unknown;
  error?: { code: string; message: string };
}
