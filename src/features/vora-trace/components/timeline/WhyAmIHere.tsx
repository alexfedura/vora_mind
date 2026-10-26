import { useTraceStore } from "../../store/traceStore";

export function WhyAmIHere() {
  const selectedId = useTraceStore((s) => s.selectedNodeId);
  const getNode = useTraceStore((s) => s.getNode);

  if (!selectedId) {
    return (
      <div className="rounded-lg border border-trace-border
                      bg-trace-panel p-4 text-sm text-trace-muted">
        Select a node to see the causal chain.
      </div>
    );
  }

  const chain: string[] = [];
  let cur = getNode(selectedId);
  while (cur) {
    chain.push(`${cur.id} — ${cur.title}`);
    cur = cur.parentId ? getNode(cur.parentId) : undefined;
  }

  return (
    <div className="rounded-lg border border-trace-border
                    bg-trace-panel p-4">
      <div className="mb-3 text-xs uppercase tracking-wider
                      text-trace-accent">
        Why am I here?
      </div>
      <div className="space-y-2">
        {chain.map((line, i) => (
          <div key={i} className="flex items-start gap-2 text-sm">
            <span className="text-trace-accent">▼</span>
            <span className="text-trace-text/80">{line}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
