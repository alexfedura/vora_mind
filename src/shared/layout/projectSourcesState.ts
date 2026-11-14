import type { ResearchProjectSourceDto } from "@/types";

export interface ProjectSourcesState {
  projectId: string;
  requestId: number;
  status: "loading" | "loaded" | "error";
  sources: ResearchProjectSourceDto[];
  error: string | null;
}

export type ProjectSourcesAction =
  | { type: "loadStarted"; projectId: string; requestId: number }
  | {
      type: "loadSucceeded";
      projectId: string;
      requestId: number;
      sources: ResearchProjectSourceDto[];
    }
  | { type: "loadFailed"; projectId: string; requestId: number; error: string }
  | { type: "clearError"; projectId: string }
  | { type: "associationAdded"; projectId: string; association: ResearchProjectSourceDto }
  | { type: "associationUpdated"; projectId: string; association: ResearchProjectSourceDto }
  | { type: "operationFailed"; projectId: string; error: string };

export function projectSourcesReducer(
  state: ProjectSourcesState | null,
  action: ProjectSourcesAction,
): ProjectSourcesState | null {
  switch (action.type) {
    case "loadStarted":
      return {
        projectId: action.projectId,
        requestId: action.requestId,
        status: "loading",
        sources: state?.projectId === action.projectId ? state.sources : [],
        error: null,
      };
    case "loadSucceeded":
      if (!isCurrentLoad(state, action.projectId, action.requestId)) return state;
      return {
        ...state,
        status: "loaded",
        sources: action.sources,
        error: null,
      };
    case "loadFailed":
      if (!isCurrentLoad(state, action.projectId, action.requestId)) return state;
      return {
        ...state,
        status: "error",
        error: action.error,
      };
    case "clearError":
      if (state?.projectId !== action.projectId || state.status !== "loaded") return state;
      return { ...state, error: null };
    case "associationAdded":
      if (state?.projectId !== action.projectId || state.status !== "loaded") return state;
      return {
        ...state,
        sources: [...state.sources, action.association].sort((left, right) =>
          left.sourceTitle.localeCompare(right.sourceTitle)
        ),
        error: null,
      };
    case "associationUpdated":
      if (state?.projectId !== action.projectId || state.status !== "loaded") return state;
      return {
        ...state,
        sources: state.sources.map((item) =>
          item.sourceId === action.association.sourceId ? action.association : item
        ),
        error: null,
      };
    case "operationFailed":
      if (state?.projectId !== action.projectId || state.status !== "loaded") return state;
      return { ...state, error: action.error };
  }
}

export function selectProjectSourcesState(
  state: ProjectSourcesState | null,
  projectId: string,
): ProjectSourcesState | null {
  return projectId !== "" && state?.projectId === projectId ? state : null;
}

function isCurrentLoad(
  state: ProjectSourcesState | null,
  projectId: string,
  requestId: number,
): state is ProjectSourcesState {
  return state?.projectId === projectId
    && state.requestId === requestId
    && state.status === "loading";
}
