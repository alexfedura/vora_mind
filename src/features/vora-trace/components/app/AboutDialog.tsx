import { useEffect, useState } from "react";
import { appVersion, isTauri } from "../../services/traceApi";

export function AboutDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [version, setVersion] = useState("--");

  useEffect(() => {
    if (!open) return;
    if (!isTauri()) { setVersion("dev"); return; }
    appVersion().then(setVersion).catch(() => setVersion("?"));
  }, [open]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center
                    bg-black/70 backdrop-blur-sm" onClick={onClose}>
      <div className="w-[480px] rounded-xl border border-trace-border
                      bg-trace-panel p-6 shadow-2xl"
           onClick={(e) => e.stopPropagation()}>
        <div className="mb-4 text-center">
          <div className="text-3xl text-trace-accent mb-2">◆</div>
          <div className="text-xl font-semibold text-trace-text">
            Vora Trace
          </div>
          <div className="text-sm text-trace-muted">
            Engineering Memory Graph
          </div>
          <div className="mt-2 text-xs text-trace-muted">
            v{version}
          </div>
        </div>

        <div className="mt-6 space-y-2 text-xs text-trace-text/80">
          <p>
            A decision graph for tracking <span className="text-trace-accent">
            why</span> engineering choices were made -- not just what.
          </p>
          <p className="text-trace-muted">
            Part of the VORA ecosystem.
          </p>
        </div>

        <div className="mt-6 flex justify-end">
          <button onClick={onClose}
                  className="rounded-md border border-trace-accent/60
                             bg-trace-accent/20 px-4 py-1.5 text-xs
                             text-trace-text hover:bg-trace-accent/30">
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
