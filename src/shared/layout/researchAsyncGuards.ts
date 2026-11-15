export interface ProjectOperationToken {
  context: string;
  projectId: string;
  generation: number;
  lifecycleGeneration: number;
}

interface ProjectContext {
  projectId: string;
  generation: number;
}

export class ProjectOperationGuard {
  private readonly contexts = new Map<string, ProjectContext>();
  private active = true;
  private lifecycleGeneration = 0;

  activate(): void {
    if (this.active) return;
    this.active = true;
    this.lifecycleGeneration += 1;
  }

  invalidate(): void {
    if (!this.active) return;
    this.active = false;
    this.lifecycleGeneration += 1;
  }

  setCurrentProject(context: string, projectId: string): void {
    const current = this.contexts.get(context);
    if (current?.projectId === projectId) return;
    this.contexts.set(context, {
      projectId,
      generation: (current?.generation ?? 0) + 1,
    });
  }

  capture(context: string): ProjectOperationToken {
    const current = this.contexts.get(context) ?? { projectId: "", generation: 0 };
    return { context, ...current, lifecycleGeneration: this.lifecycleGeneration };
  }

  captureForProject(context: string, projectId: string): ProjectOperationToken {
    this.setCurrentProject(context, projectId);
    return this.capture(context);
  }

  isCurrent(token: ProjectOperationToken): boolean {
    const current = this.contexts.get(token.context);
    return this.active
      && token.lifecycleGeneration === this.lifecycleGeneration
      && current !== undefined
      && current.projectId === token.projectId
      && current.generation === token.generation;
  }

  commit(token: ProjectOperationToken, update: () => void): boolean {
    if (!this.isCurrent(token)) return false;
    update();
    return true;
  }
}

export function assignmentContext(kind: "question" | "hypothesis", recordId: string): string {
  return `${kind}Assignment:${recordId}`;
}

export class LatestRequestGuard {
  private latestRequestId = 0;
  private active = false;

  activate(): void {
    this.active = true;
    this.latestRequestId += 1;
  }

  begin(): number {
    this.latestRequestId += 1;
    return this.latestRequestId;
  }

  isCurrent(requestId: number): boolean {
    return this.active && this.latestRequestId === requestId;
  }

  commit(requestId: number, update: () => void): boolean {
    if (!this.isCurrent(requestId)) return false;
    update();
    return true;
  }

  deactivate(): void {
    this.active = false;
    this.latestRequestId += 1;
  }
}
