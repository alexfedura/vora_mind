export type ProjectWorkspaceStatus = "idle" | "loading" | "loaded" | "error";

export interface ProjectWorkspaceSnapshot<T> {
  projectId: string;
  status: ProjectWorkspaceStatus;
  workspace: T | null;
  error: string | null;
  isMutating: boolean;
}

export type ProjectWorkspaceActionResult<T> =
  | { status: "completed"; value: T; workspaceRefreshed: boolean }
  | { status: "failed"; error: string }
  | { status: "stale" };

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class ProjectWorkspaceController<T> {
  private currentProjectId = "";
  private generation = 0;
  private latestLoadId = 0;
  private activeMutations = 0;
  private snapshot: ProjectWorkspaceSnapshot<T> = {
    projectId: "",
    status: "idle",
    workspace: null,
    error: null,
    isMutating: false,
  };
  private readonly listeners = new Set<() => void>();

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): ProjectWorkspaceSnapshot<T> => this.snapshot;

  activateProject(projectId: string): void {
    if (this.currentProjectId === projectId) return;
    this.currentProjectId = projectId;
    this.generation += 1;
    this.latestLoadId += 1;
    this.activeMutations = 0;
    this.publish({
      projectId,
      status: projectId ? "loading" : "idle",
      workspace: null,
      error: null,
      isMutating: false,
    });
  }

  deactivateProject(projectId: string): void {
    if (this.currentProjectId !== projectId) return;
    this.currentProjectId = "";
    this.generation += 1;
    this.latestLoadId += 1;
    this.activeMutations = 0;
    this.publish({
      projectId: "",
      status: "idle",
      workspace: null,
      error: null,
      isMutating: false,
    });
  }

  isCurrentProject(projectId: string, isProjectCurrent: () => boolean = () => true): boolean {
    return projectId !== ""
      && this.currentProjectId === projectId
      && isProjectCurrent();
  }

  async refresh(
    projectId: string,
    load: (projectId: string) => Promise<T>,
    isProjectCurrent: () => boolean = () => true,
  ): Promise<boolean> {
    if (!this.isCurrentProject(projectId, isProjectCurrent)) return false;

    const generation = this.generation;
    const loadId = ++this.latestLoadId;
    this.publish({
      projectId,
      status: "loading",
      workspace: null,
      error: null,
      isMutating: this.activeMutations > 0,
    });

    try {
      const workspace = await load(projectId);
      if (!this.isCurrentLoad(projectId, generation, loadId, isProjectCurrent)) return false;
      this.publish({
        projectId,
        status: "loaded",
        workspace,
        error: null,
        isMutating: this.activeMutations > 0,
      });
      return true;
    } catch (error) {
      if (!this.isCurrentLoad(projectId, generation, loadId, isProjectCurrent)) return false;
      this.publish({
        projectId,
        status: "error",
        workspace: null,
        error: errorMessage(error),
        isMutating: this.activeMutations > 0,
      });
      return false;
    }
  }

  async runAction<R>(
    projectId: string,
    action: () => Promise<R>,
    load: (projectId: string) => Promise<T>,
    isProjectCurrent: () => boolean = () => true,
  ): Promise<ProjectWorkspaceActionResult<R>> {
    if (!this.isCurrentProject(projectId, isProjectCurrent)) return { status: "stale" };

    const generation = this.generation;
    this.activeMutations += 1;
    this.publish({
      ...this.snapshot,
      error: null,
      isMutating: true,
    });

    try {
      const value = await action();
      if (!this.isCurrentGeneration(projectId, generation, isProjectCurrent)) {
        return { status: "stale" };
      }

      const workspaceRefreshed = await this.refresh(projectId, load, isProjectCurrent);
      if (!this.isCurrentGeneration(projectId, generation, isProjectCurrent)) {
        return { status: "stale" };
      }
      return { status: "completed", value, workspaceRefreshed };
    } catch (error) {
      if (!this.isCurrentGeneration(projectId, generation, isProjectCurrent)) {
        return { status: "stale" };
      }
      const message = errorMessage(error);
      this.publish({
        ...this.snapshot,
        error: message,
      });
      return { status: "failed", error: message };
    } finally {
      if (this.currentProjectId === projectId && this.generation === generation) {
        this.activeMutations = Math.max(0, this.activeMutations - 1);
        this.publish({
          ...this.snapshot,
          isMutating: this.activeMutations > 0,
        });
      }
    }
  }

  setError(
    projectId: string,
    error: string,
    isProjectCurrent: () => boolean = () => true,
  ): void {
    if (!this.isCurrentProject(projectId, isProjectCurrent)) return;
    this.publish({ ...this.snapshot, error });
  }

  private isCurrentGeneration(
    projectId: string,
    generation: number,
    isProjectCurrent: () => boolean,
  ): boolean {
    return this.generation === generation && this.isCurrentProject(projectId, isProjectCurrent);
  }

  private isCurrentLoad(
    projectId: string,
    generation: number,
    loadId: number,
    isProjectCurrent: () => boolean,
  ): boolean {
    return this.latestLoadId === loadId
      && this.isCurrentGeneration(projectId, generation, isProjectCurrent);
  }

  private publish(snapshot: ProjectWorkspaceSnapshot<T>): void {
    this.snapshot = snapshot;
    for (const listener of this.listeners) listener();
  }
}
