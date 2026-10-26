import { useEffect } from "react";
import { useTraceStore } from "../../store/traceStore";

export function RecentList() {
  const recent = useTraceStore((s) => s.recent);
  const loadRecent = useTraceStore((s) => s.loadRecent);
  const selectNode = useTraceStore((s) => s.selectNode);

  useEffect(() => { loadRecent(); }, [loadRecent]);

  return (
    <div className="mt-6 border-t border-trace-border pt-4">
      <div className="mb-2 flex items-center justify-between">
        <div className="text-xs uppercase tracking-wider text-trace-muted">
          Recent
        </div>
        <button onClick={loadRecent}
                className="text-xs text-trace-accent hover:text-trace-accent/80">
          ↻
        </button>
      </div>
      <div className="space-y-1">
        {recent.length === 0 && (
          <div className="text-xs text-trace-muted">None yet.</div>
        )}
        {recent.map((r) => (
          <button
            key={r.id}
            onClick={() => selectNode(r.id)}
            className="block w-full rounded-md border border-trace-border
                       bg-trace-bg/50 p-2 text-left text-xs
                       hover:bg-trace-bg"
          >
            <div className="flex items-center gap-2">
              <span className="text-trace-accent">◉</span>
              <span className="font-medium text-trace-text">{r.id}</span>
            </div>
            <div className="mt-0.5 truncate text-trace-muted">
              {r.title}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
