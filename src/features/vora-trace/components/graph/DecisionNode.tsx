import { Handle, Position, type NodeProps } from "@xyflow/react";

export function DecisionNode({ data, selected }: NodeProps) {
  const status = String(data.status ?? "pending");
  const ring =
    status === "accepted" ? "ring-emerald-400" :
    status === "rejected" ? "ring-red-400" :
    "ring-violet-400";
  return (
    <div className={`rounded-xl border-2 border-violet-400 bg-violet-500/20
                     px-4 py-3 text-sm shadow-xl ring-2 ${ring}
                     ${selected ? "scale-105" : ""} transition-transform`}>
      <Handle type="target" position={Position.Left}
              className="!bg-violet-400" />
      <div className="flex items-center gap-2">
        <span className="text-violet-200 text-lg">◉</span>
        <div>
          <div className="font-semibold text-violet-50">
            {String(data.id)}
          </div>
          <div className="text-xs text-violet-200/80 max-w-[180px]
                          truncate">
            {String(data.title)}
          </div>
        </div>
      </div>
      <Handle type="source" position={Position.Right}
              className="!bg-violet-400" />
    </div>
  );
}
