import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import {
  ProjectWorkspaceController,
  type ProjectWorkspaceSnapshot,
} from "@/shared/layout/projectWorkspaceState";

export function useProjectWorkspace<T>(
  projectId: string,
  load: (projectId: string) => Promise<T>,
) {
  const [controller] = useState(() => new ProjectWorkspaceController<T>());
  const currentProjectId = useRef(projectId);
  currentProjectId.current = projectId;
  const loader = useRef(load);
  loader.current = load;

  const storedSnapshot = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
    controller.getSnapshot,
  );
  const snapshot: ProjectWorkspaceSnapshot<T> = storedSnapshot.projectId === projectId
    ? storedSnapshot
    : {
      projectId,
      status: projectId ? "loading" : "idle",
      workspace: null,
      error: null,
      isMutating: false,
    };

  useEffect(() => {
    const isProjectCurrent = (): boolean => currentProjectId.current === projectId;
    controller.activateProject(projectId);
    if (projectId) {
      void controller.refresh(projectId, (id) => loader.current(id), isProjectCurrent);
    }
    return () => controller.deactivateProject(projectId);
  }, [controller, projectId]);

  const refresh = useCallback(
    () => controller.refresh(
      projectId,
      (id) => loader.current(id),
      () => currentProjectId.current === projectId,
    ),
    [controller, projectId],
  );

  const perform = useCallback(
    <TResult,>(action: () => Promise<TResult>) => controller.runAction(
      projectId,
      action,
      (id) => loader.current(id),
      () => currentProjectId.current === projectId,
    ),
    [controller, projectId],
  );

  const setError = useCallback(
    (error: string) => controller.setError(
      projectId,
      error,
      () => currentProjectId.current === projectId,
    ),
    [controller, projectId],
  );

  const isCurrentProject = useCallback(
    () => controller.isCurrentProject(
      projectId,
      () => currentProjectId.current === projectId,
    ),
    [controller, projectId],
  );

  return {
    ...snapshot,
    isLoading: snapshot.status === "loading",
    busy: snapshot.status === "loading" || snapshot.isMutating,
    refresh,
    perform,
    setError,
    isCurrentProject,
  };
}
