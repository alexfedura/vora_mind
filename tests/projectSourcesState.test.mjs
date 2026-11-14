import assert from "node:assert/strict";
import test from "node:test";
import {
  projectSourcesReducer,
  selectProjectSourcesState,
} from "../src/shared/layout/projectSourcesState.ts";

function association(projectId, sourceId, useContext) {
  return {
    projectId,
    projectTitle: `Project ${projectId}`,
    projectLifecycleStatus: "active",
    sourceId,
    sourceTitle: `Source ${sourceId}`,
    sourceKind: "book",
    useContext,
    notes: "",
    createdAtUs: 1,
    updatedAtUs: 1,
  };
}

function startLoad(state, projectId, requestId) {
  return projectSourcesReducer(state, { type: "loadStarted", projectId, requestId });
}

test("switching projects makes the previous project's context ineligible for display", () => {
  const projectA = association("A", "source-a", "Context for A");
  let state = startLoad(null, "A", 1);
  state = projectSourcesReducer(state, {
    type: "loadSucceeded",
    projectId: "A",
    requestId: 1,
    sources: [projectA],
  });

  assert.equal(selectProjectSourcesState(state, "B"), null);
  state = startLoad(state, "B", 2);
  assert.deepEqual(selectProjectSourcesState(state, "B")?.sources, []);
});

test("a failed load for project B does not expose project A associations", () => {
  const projectA = association("A", "source-a", "Context for A");
  let state = startLoad(null, "A", 1);
  state = projectSourcesReducer(state, {
    type: "loadSucceeded",
    projectId: "A",
    requestId: 1,
    sources: [projectA],
  });
  state = startLoad(state, "B", 2);
  state = projectSourcesReducer(state, {
    type: "loadFailed",
    projectId: "B",
    requestId: 2,
    error: "load failed",
  });

  const projectBState = selectProjectSourcesState(state, "B");
  assert.equal(projectBState?.status, "error");
  assert.equal(projectBState?.error, "load failed");
  assert.deepEqual(projectBState?.sources, []);
  assert.equal(selectProjectSourcesState(state, "A"), null);
});

test("a late response for project A cannot overwrite project B's loaded associations", () => {
  let state = startLoad(null, "A", 1);
  state = startLoad(state, "B", 2);
  const projectB = association("B", "source-b", "Context for B");
  state = projectSourcesReducer(state, {
    type: "loadSucceeded",
    projectId: "B",
    requestId: 2,
    sources: [projectB],
  });
  const loadedBState = state;

  state = projectSourcesReducer(state, {
    type: "loadSucceeded",
    projectId: "A",
    requestId: 1,
    sources: [association("A", "source-a", "Context for A")],
  });

  assert.equal(state, loadedBState);
  assert.deepEqual(selectProjectSourcesState(state, "B")?.sources, [projectB]);
});

test("a successful load displays only the selected project's associations", () => {
  let state = startLoad(null, "A", 1);
  state = projectSourcesReducer(state, {
    type: "loadSucceeded",
    projectId: "A",
    requestId: 1,
    sources: [association("A", "source-a", "Context for A")],
  });
  state = startLoad(state, "B", 2);
  const projectB = association("B", "source-b", "Context for B");
  state = projectSourcesReducer(state, {
    type: "loadSucceeded",
    projectId: "B",
    requestId: 2,
    sources: [projectB],
  });

  assert.deepEqual(selectProjectSourcesState(state, "B")?.sources, [projectB]);
});
