import { useEffect, useState } from "react";
import { DecisionGraph } from "./components/graph/DecisionGraph";
import { HoverCard } from "./components/graph/HoverCard";
import { FilterBar } from "./components/graph/FilterBar";
import { DecisionEditor } from "./components/decision/DecisionEditor";
import { WhyAmIHere } from "./components/timeline/WhyAmIHere";
import { ProjectSidebar } from "./components/ecosystem/ProjectSidebar";
import { AboutDialog } from "./components/app/AboutDialog";
import { RecentList } from "./components/app/RecentList";
import { useTraceStore } from "./store/traceStore";

export default function App() {
  const loadFromDisk = useTraceStore((s) => s.loadFromDisk);
  const saveToDisk = useTraceStore((s) => s.saveToDisk);
  const syncFromDisk = useTraceStore((s) => s.syncFromDisk);
  const loadProjects = useTraceStore((s) => s.loadProjects);
  const exportGraph = useTraceStore((s) => s.exportGraph);
  const importGraph = useTraceStore((s) => s.importGraph);
  const loadRecent = useTraceStore((s) => s.loadRecent);
  const addNode = useTraceStore((s) => s.addNode);
  const selectNode = useTraceStore((s) => s.selectNode);
  const dirty = useTraceStore((s) => s.dirty);
  const saving = useTraceStore((s) => s.saving);
  const syncing = useTraceStore((s) => s.syncing);
  const lastError = useTraceStore((s) => s.lastError);
  const [aboutOpen, setAboutOpen] = useState(false);

  useEffect(() => {
    loadFromDisk();
    loadProjects();
  }, [loadFromDisk, loadProjects]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        saveToDisk();
      }
      if (e.key === "Escape") {
        selectNode(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [saveToDisk, selectNode]);

  const onNewDecision = () => {
    const id = addNode("decision", "New decision");
    selectNode(id);
  };

  return (
    <div className="flex h-screen w-screen flex-col bg-trace-bg">
      <header className="flex items-center gap-3 border-b border-trace-border bg-trace-panel px-6 py-3">
        <div className="text-lg font-semibold text-trace-text">
          <span className="text-trace-accent">◆</span> Vora Trace
        </div>
        <div className="text-xs text-trace-muted">Engineering Memory Graph</div>

        <div className="ml-4 flex items-center gap-2">
          <button
            onClick={onNewDecision}
            className="rounded-md border border-trace-accent/60 bg-trace-accent/20 px-3 py-1.5 text-xs text-trace-text hover:bg-trace-accent/30">
            + New Decision
          </button>
          <button
            onClick={syncFromDisk}
            disabled={syncing}
            className="rounded-md border border-trace-border bg-trace-bg px-3 py-1.5 text-xs text-trace-text/80 hover:bg-trace-border disabled:opacity-40">
            {syncing ? "Syncing…" : "Sync"}
          </button>
          <button
            onClick={saveToDisk}
            disabled={!dirty || saving}
            className="rounded-md border border-trace-border bg-trace-bg px-3 py-1.5 text-xs text-trace-text/80 hover:bg-trace-border disabled:opacity-40 disabled:cursor-not-allowed">
            {saving ? "Saving…" : dirty ? "Save *" : "Saved"}
          </button>
          <button
            onClick={async () => {
              const dest = await exportGraph();
              if (dest) console.info("exported to", dest);
            }}
            className="rounded-md border border-trace-border bg-trace-bg px-3 py-1.5 text-xs text-trace-text/80 hover:bg-trace-border">
            Export
          </button>
          <button
            onClick={async () => {
              await importGraph();
              loadRecent();
            }}
            className="rounded-md border border-trace-border bg-trace-bg px-3 py-1.5 text-xs text-trace-text/80 hover:bg-trace-border">
            Import
          </button>
        </div>

        <FilterBar />

        <div className="ml-auto flex items-center gap-3 text-xs">
          {lastError && <span className="text-red-400">ERR: {lastError}</span>}
          <span className="text-trace-muted">
            {dirty ? "unsaved changes" : "all saved"}
          </span>
          <button
            onClick={() => setAboutOpen(true)}
            className="rounded-full border border-trace-border bg-trace-bg w-6 h-6 text-xs text-trace-muted hover:text-trace-text hover:bg-trace-border"
            title="About Vora Trace">
            ?
          </button>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <aside className="w-64 border-r border-trace-border bg-trace-panel/30 p-4 overflow-y-auto">
          <ProjectSidebar />
          <RecentList />
        </aside>
        <main className="flex-1">
          <DecisionGraph />
        </main>
        <aside className="w-96 border-l border-trace-border bg-trace-panel/40 p-4 overflow-y-auto">
          <WhyAmIHere />
        </aside>
      </div>

      <HoverCard />
      <DecisionEditor />
      <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} />
    </div>
  );
}
