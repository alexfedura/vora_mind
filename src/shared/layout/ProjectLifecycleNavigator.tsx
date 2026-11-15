import React, { useMemo } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import {
  buildProjectLifecycleConnections,
  type ProjectLifecycleConnection,
  type ProjectLifecycleEndpoint,
} from "@/shared/layout/projectLifecycleModel";
import type { ProjectLifecycleSnapshot } from "@/hooks/useProjectLifecycleWorkbench";
import type { ProjectKnowledgeOverview } from "@/shared/layout/projectKnowledgeModel";
import type { ResearchWorkspaceDto } from "@/types";

interface ProjectLifecycleNavigatorProps {
  projectId: string;
  overview: ProjectKnowledgeOverview;
  research: ResearchWorkspaceDto;
  snapshot: ProjectLifecycleSnapshot;
  onRefresh: () => void;
  onOpenProject: (projectId: string) => void;
}

const panelClass = "rounded-xl border border-[#292c34] bg-[#17191f] p-4";
const mutedClass = "text-xs text-gray-500";

function Endpoint({
  value,
  projects,
  onOpenProject,
}: {
  value: ProjectLifecycleEndpoint;
  projects: ReadonlyMap<string, string>;
  onOpenProject: (projectId: string) => void;
}): React.JSX.Element {
  if (value.availability === "outside_project") {
    const projectId = value.projectId;
    const projectTitle = projectId ? projects.get(projectId) : null;
    return (
      <span className="inline-flex flex-wrap items-center gap-1">
        <span className="font-medium text-sky-200">{value.label}</span>
        <code className="break-all text-[10px] text-gray-400">{value.id}</code>
        <span className="text-[10px] text-amber-300">owned by another project</span>
        {projectId && projectTitle && (
          <button
            type="button"
            onClick={() => onOpenProject(projectId)}
            className="text-[10px] text-sky-300 underline"
          >
            Open {projectTitle}
          </button>
        )}
      </span>
    );
  }
  if (value.availability === "available" && value.href) {
    return (
      <a
        href={value.href}
        onClick={() => {
          if (!value.href?.startsWith("#")) return;
          let parent = document.getElementById(value.href.slice(1))?.parentElement?.closest("details") ?? null;
          while (parent) {
            parent.open = true;
            parent = parent.parentElement?.closest("details") ?? null;
          }
        }}
        className="inline-flex flex-wrap items-center gap-1 text-sky-200 underline decoration-sky-900 underline-offset-2"
      >
        <span>{value.label}</span>
        <code className="break-all text-[10px] text-gray-400">{value.id}</code>
      </a>
    );
  }
  if (value.availability === "not_verified" && value.href) {
    return (
      <span className="inline-flex flex-wrap items-center gap-1">
        <a href={value.href} className="text-sky-200 underline">{value.label}</a>
        <code className="break-all text-[10px] text-gray-400">{value.id}</code>
        <span className="text-[10px] text-amber-300">exact reference not verified in this workspace</span>
      </span>
    );
  }
  return (
    <span className="inline-flex flex-wrap items-center gap-1">
      <span className="text-gray-300">{value.label}</span>
      <code className="break-all text-[10px] text-gray-500">{value.id}</code>
      <span className="text-[10px] text-rose-300">
        {value.availability === "unavailable"
          ? "record unavailable; no substitute selected"
          : "not verified"}
      </span>
    </span>
  );
}

function ConnectionRow({
  connection,
  projects,
  onOpenProject,
}: {
  connection: ProjectLifecycleConnection;
  projects: ReadonlyMap<string, string>;
  onOpenProject: (projectId: string) => void;
}): React.JSX.Element {
  return (
    <li className="rounded-lg border border-[#292c34] bg-[#101114] p-3 text-xs">
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <Endpoint value={connection.from} projects={projects} onOpenProject={onOpenProject} />
        <span className="text-amber-200">{connection.relationship}</span>
        <Endpoint value={connection.to} projects={projects} onOpenProject={onOpenProject} />
      </div>
      <p className="mt-2 text-[10px] text-gray-500">
        Recorded by {connection.recordKind} <code>{connection.recordId}</code>
        {connection.note ? ` · ${connection.note}` : ""}
      </p>
    </li>
  );
}

export const ProjectLifecycleNavigator: React.FC<ProjectLifecycleNavigatorProps> = ({
  projectId,
  overview,
  research,
  snapshot,
  onRefresh,
  onOpenProject,
}) => {
  const connections = useMemo(
    () => buildProjectLifecycleConnections({
      projectId,
      overview,
      analysis: snapshot.analysis,
      experiment: snapshot.experiment,
      model: snapshot.model,
      transfer: snapshot.transfer,
    }),
    [overview, projectId, research, snapshot],
  );
  const projectTitles = useMemo(
    () => new Map(research.projects.map((project) => [project.id, project.title])),
    [research.projects],
  );
  const categories = [...new Set(connections.map((connection) => connection.category))];
  const hasLoading = [snapshot.analysis, snapshot.experiment, snapshot.model, snapshot.transfer]
    .some((domain) => domain.status === "loading");
  const errors = [
    ["Analysis", snapshot.analysis],
    ["Experiment", snapshot.experiment],
    ["Model", snapshot.model],
    ["Knowledge Transfer", snapshot.transfer],
  ] as const;

  return (
    <section className={`${panelClass} space-y-3`} aria-labelledby="project-lifecycle-heading">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 id="project-lifecycle-heading" className="text-sm font-semibold text-white">
            Project lifecycle connections
          </h3>
          <p className={`mt-1 max-w-3xl ${mutedClass}`}>
            Exact recorded references across Research, Experiment, Analysis, Model, and explicit Knowledge Transfer. Missing references are never replaced with a different version.
          </p>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={hasLoading}
          className="inline-flex items-center gap-1.5 rounded border border-[#343741] px-2.5 py-1.5 text-xs text-gray-300 disabled:opacity-50"
        >
          <RefreshCw size={12} className={hasLoading ? "animate-spin" : ""} />
          Refresh links
        </button>
      </div>

      {errors.filter(([, domain]) => domain.status === "error").map(([label, domain]) => (
        <p key={label} role="alert" className="flex items-start gap-2 text-xs text-rose-300">
          <AlertCircle size={14} className="mt-0.5 shrink-0" />
          {label} references are unavailable: {domain.error ?? "workspace load failed."}
        </p>
      ))}
      {hasLoading && <p className={mutedClass}>Loading lifecycle references…</p>}
      {connections.length === 0 && !hasLoading && (
        <p className={mutedClass}>
          No explicit cross-domain lifecycle links are recorded for this project.
        </p>
      )}
      <div className="space-y-4">
        {categories.map((category) => (
          <div key={category} className="space-y-2">
            <h4 className="text-xs font-semibold text-gray-300">{category}</h4>
            <ul className="space-y-2">
              {connections.filter((connection) => connection.category === category).map((connection) => (
                <ConnectionRow
                  key={connection.id}
                  connection={connection}
                  projects={projectTitles}
                  onOpenProject={onOpenProject}
                />
              ))}
            </ul>
          </div>
        ))}
      </div>
    </section>
  );
};
