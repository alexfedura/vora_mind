import { Handle, Position, type NodeProps } from "@xyflow/react";

export function ExperimentNode({ data }: NodeProps) {
  return (
    <div className="rounded-lg border-2 border-amber-400 bg-amber-500/20
                    px-3 py-2 text-sm text-amber-100 shadow-lg">
      <Handle type="target" position={Position.Left}
              className="!bg-amber-400" />
      <span className="text-amber-200">◇</span>{" "}
      {String(data.id)} — {String(data.title)}
      <Handle type="source" position={Position.Right}
              className="!bg-amber-400" />
    </div>
  );
}
