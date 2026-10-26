import { useEffect } from "react";
import { useTraceStore } from "../../store/traceStore";

export function ProjectSidebar() {
  const projects = useTraceStore((s) => s.projects);
  const loading = useTraceStore((s) => s.projectsLoading);
  const loadProjects = useTraceStore((s) => s.loadProjects);
  const selectedProjectId = useTraceStore((s) => s.selectedProjectId);
  const selectProject = useTraceStore((s) => s.selectProject);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  return (
    <div className="flex flex-col h-full">
      <div className="mb-3 flex items-center justify-between">
        <div className="text-xs uppercase tracking-wider text-trace-muted">
          Ecosystem
        </div>
        <button
          onClick={loadProjects}
          disabled={loading}
          className="text-xs text-trace-accent hover:text-trace-accent/80
                     disabled:opacity-40"
        >
          {loading ? "…" : "refresh"}
        </button>
      </div>

      <div className="space-y-1 overflow-y-auto">
        {projects.length === 0 && !loading && (
          <div className="text-xs text-trace-muted">
            No projects found.
          </div>
        )}
        {projects.map((p) => (
          <button
            key={p.id}
            type="button"
            disabled={!p.exists}
            onClick={() => selectProject(p.id)}
            aria-pressed={selectedProjectId === p.id}
            className={`w-full rounded-md border p-2 text-left text-xs transition ${
              selectedProjectId === p.id
                ? "border-trace-accent bg-trace-accent/10"
                : "border-trace-border bg-trace-bg/50 hover:border-trace-accent/50"
            } disabled:cursor-not-allowed disabled:opacity-50`}
          >
            <div className="flex items-center gap-2">
              <span className={
                p.exists ? "text-emerald-400" : "text-red-400"
              }>
                {p.exists ? "●" : "○"}
              </span>
              <span className="font-medium text-trace-text">
                {p.id}
              </span>
              <span className="ml-auto text-trace-muted">
                {p.role}
              </span>
            </div>
            {p.git_branch && (
              <div className="mt-1 flex items-center gap-1 text-trace-muted">
                <span>⎇</span>
                <span>{p.git_branch}</span>
                <span className="opacity-60">·</span>
                <span>{p.git_head}</span>
                {p.git_dirty && (
                  <span className="text-amber-400 ml-1">*</span>
                )}
              </div>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
