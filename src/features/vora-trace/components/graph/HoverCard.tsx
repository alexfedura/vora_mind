import { useTraceStore } from "../../store/traceStore";

export function HoverCard() {
  const id = useTraceStore((s) => s.hoveredNodeId);
  const node = useTraceStore((s) => (id ? s.getNode(id) : undefined));
  if (!node) return null;
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-50
                    w-80 rounded-lg border border-trace-border
                    bg-trace-panel/95 p-4 shadow-2xl backdrop-blur">
      <div className="mb-2 flex items-center gap-2">
        <span className="text-trace-accent">
          {node.kind === "decision" ? "◉" :
           node.kind === "experiment" ? "◇" :
           node.kind === "rejected" ? "✕" : "●"}
        </span>
        <span className="text-xs uppercase tracking-wider text-trace-muted">
          {node.kind}
        </span>
        <span className="ml-auto text-xs text-trace-muted">
          {node.id}
        </span>
      </div>
      <div className="text-sm font-medium text-trace-text mb-1">
        {node.title}
      </div>
      {node.whyAmIHere && (
        <div className="mt-2 text-xs text-trace-muted">
          <span className="text-trace-accent">why:</span>{" "}
          {node.whyAmIHere}
        </div>
      )}
      {node.alternatives && node.alternatives.length > 0 && (
        <div className="mt-2 text-xs text-trace-muted">
          {node.alternatives.length} alternatives
        </div>
      )}
    </div>
  );
}
