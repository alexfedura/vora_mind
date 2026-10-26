import { Handle, Position, type NodeProps } from "@xyflow/react";

export function StateNode({ data }: NodeProps) {
  return (
    <div className="rounded-full border-2 border-blue-400 bg-blue-500/20
                    px-4 py-2 text-sm text-blue-100 shadow-lg">
      <Handle type="target" position={Position.Left}
              className="!bg-blue-400" />
      <span className="font-medium">● {String(data.label)}</span>
      <Handle type="source" position={Position.Right}
              className="!bg-blue-400" />
    </div>
  );
}
