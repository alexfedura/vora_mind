import { Handle, Position, type NodeProps } from "@xyflow/react";

export function RejectedNode({ data }: NodeProps) {
  return (
    <div className="rounded-lg border border-red-400/50 bg-red-500/10
                    px-3 py-2 text-sm text-red-200/80 shadow
                    opacity-70 line-through">
      <Handle type="target" position={Position.Left}
              className="!bg-red-400" />
      ✕ {String(data.id)} — {String(data.title)}
    </div>
  );
}
