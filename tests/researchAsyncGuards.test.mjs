import assert from "node:assert/strict";
import test from "node:test";
import {
  assignmentContext,
  LatestRequestGuard,
  ProjectOperationGuard,
} from "../src/shared/layout/researchAsyncGuards.ts";

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

test("a late project A mutation cannot update project B form, selection, or error state", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { form: "B input", selection: "B item", error: null };
  guard.setCurrentProject("project", "A");
  const token = guard.capture("project");
  const operation = pending.promise.then((value) => {
    guard.commit(token, () => {
      state.form = "";
      state.selection = value;
      state.error = "A completed";
    });
  });

  guard.setCurrentProject("project", "B");
  pending.resolve("A item");
  await operation;

  assert.deepEqual(state, { form: "B input", selection: "B item", error: null });
});

test("a project mutation still applies its UI update when its project remains current", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { form: "submitted", selection: "" };
  guard.setCurrentProject("project", "A");
  const token = guard.capture("project");
  const operation = pending.promise.then((value) => {
    guard.commit(token, () => {
      state.form = "";
      state.selection = value;
    });
  });

  pending.resolve("created item");
  await operation;

  assert.deepEqual(state, { form: "", selection: "created item" });
});

test("a current createProject failure publishes its error", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  guard.setCurrentProject("project", "origin");
  const token = guard.capture("project");
  const isCurrent = () => guard.isCurrent(token);
  const operation = pending.promise.catch((error) => {
    if (isCurrent()) state.error = error.message;
  });

  pending.reject(new Error("project creation failed"));
  await operation;

  assert.equal(state.error, "project creation failed");
});

test("a createProject failure after lifecycle invalidation publishes no error", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  guard.setCurrentProject("project", "origin");
  const token = guard.capture("project");
  const isCurrent = () => guard.isCurrent(token);
  const operation = pending.promise.catch((error) => {
    if (isCurrent()) state.error = error.message;
  });

  guard.invalidate();
  pending.reject(new Error("late project creation failure"));
  await operation;

  assert.equal(state.error, null);
});

test("a current createProject success retains its guarded follow-up updates", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = {
    projectId: "origin",
    questionProjectId: "question-origin",
    form: "submitted",
  };
  guard.setCurrentProject("project", "origin");
  const token = guard.capture("project");
  const operation = pending.promise.then((project) => {
    guard.commit(token, () => {
      state.projectId = project.id;
      state.questionProjectId = project.id;
      state.form = "";
    });
  });

  pending.resolve({ id: "created-project" });
  await operation;

  assert.deepEqual(state, {
    projectId: "created-project",
    questionProjectId: "created-project",
    form: "",
  });
});

test("a successful mutation cannot publish after the Research page unmounts", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { form: "submitted", selection: "" };
  guard.setCurrentProject("project", "A");
  const token = guard.capture("project");
  const operation = pending.promise.then((value) => {
    guard.commit(token, () => {
      state.form = "";
      state.selection = value;
    });
  });

  guard.invalidate();
  pending.resolve("created item");
  await operation;

  assert.deepEqual(state, { form: "submitted", selection: "" });
});

test("a failed mutation cannot publish an error after the Research page unmounts", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  const token = guard.capture("project");
  const operation = pending.promise.catch((error) => {
    guard.commit(token, () => {
      state.error = error.message;
    });
  });

  guard.invalidate();
  pending.reject(new Error("late failure"));
  await operation;

  assert.equal(state.error, null);
});

test("lifecycle invalidation rejects all outstanding tokens", () => {
  const guard = new ProjectOperationGuard();
  guard.setCurrentProject("project", "A");
  guard.setCurrentProject("questionProject", "B");
  const projectToken = guard.capture("project");
  const questionToken = guard.capture("questionProject");
  const workspaceToken = guard.captureForProject("workspace", "");
  assert.equal(guard.isCurrent(workspaceToken), true);

  guard.invalidate();

  assert.equal(guard.isCurrent(projectToken), false);
  assert.equal(guard.isCurrent(questionToken), false);
  assert.equal(guard.isCurrent(workspaceToken), false);
});

test("repeated lifecycle invalidation is idempotent", () => {
  const guard = new ProjectOperationGuard();
  const firstToken = guard.capture("project");
  guard.invalidate();
  const invalidatedToken = guard.capture("project");
  guard.invalidate();
  const stillInvalidatedToken = guard.capture("project");

  assert.equal(guard.isCurrent(firstToken), false);
  assert.equal(
    stillInvalidatedToken.lifecycleGeneration,
    invalidatedToken.lifecycleGeneration,
  );
});

test("activation after cleanup supports Strict Mode setup while rejecting old tokens", () => {
  const guard = new ProjectOperationGuard();
  guard.setCurrentProject("project", "A");
  const oldToken = guard.capture("project");
  guard.invalidate();
  guard.activate();
  const state = { selection: "" };
  const currentToken = guard.capture("project");

  assert.equal(guard.isCurrent(oldToken), false);
  assert.equal(guard.commit(currentToken, () => {
    state.selection = "current result";
  }), true);
  assert.equal(state.selection, "current result");
});

test("returning to project A does not reactivate an older A operation", () => {
  const guard = new ProjectOperationGuard();
  const state = { selection: "" };
  guard.setCurrentProject("project", "A");
  const oldToken = guard.capture("project");

  guard.setCurrentProject("project", "B");
  guard.setCurrentProject("project", "A");

  assert.equal(guard.commit(oldToken, () => {
    state.selection = "obsolete A selection";
  }), false);
  assert.equal(state.selection, "");
});

test("a late project A failure cannot publish an error after switching to B", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  guard.setCurrentProject("project", "A");
  const token = guard.capture("project");
  const operation = pending.promise.catch((error) => {
    guard.commit(token, () => {
      state.error = error.message;
    });
  });

  guard.setCurrentProject("project", "B");
  pending.reject(new Error("project A failed"));
  await operation;

  assert.equal(state.error, null);
});

test("a late Question assignment failure is suppressed after its destination changes", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  guard.setCurrentProject("questionProject", "question-project");
  const context = assignmentContext("question", "question-1");
  const token = guard.captureForProject(context, "destination-A");
  const operation = pending.promise.catch((error) => {
    guard.commit(token, () => {
      state.error = error.message;
    });
  });

  guard.setCurrentProject(context, "destination-B");
  pending.reject(new Error("destination A assignment failed"));
  await operation;

  assert.equal(state.error, null);
  assert.equal(guard.isCurrent(guard.capture("questionProject")), true);
});

test("a late Hypothesis assignment failure is suppressed after its destination changes", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  guard.setCurrentProject("project", "main-project");
  const context = assignmentContext("hypothesis", "hypothesis-1");
  const token = guard.captureForProject(context, "destination-A");
  const operation = pending.promise.catch((error) => {
    guard.commit(token, () => {
      state.error = error.message;
    });
  });

  guard.setCurrentProject(context, "destination-B");
  pending.reject(new Error("destination A assignment failed"));
  await operation;

  assert.equal(state.error, null);
  assert.equal(guard.isCurrent(guard.capture("project")), true);
});

test("an assignment failure remains visible while its destination is unchanged", async () => {
  const guard = new ProjectOperationGuard();
  const pending = deferred();
  const state = { error: null };
  const context = assignmentContext("question", "question-1");
  const token = guard.captureForProject(context, "destination-A");
  const operation = pending.promise.catch((error) => {
    guard.commit(token, () => {
      state.error = error.message;
    });
  });

  pending.reject(new Error("assignment failed"));
  await operation;

  assert.equal(state.error, "assignment failed");
});

test("the latest global workspace refresh wins when requests resolve out of order", async () => {
  const guard = new LatestRequestGuard();
  const first = deferred();
  const second = deferred();
  const state = { workspace: null, error: null, isLoading: true };
  guard.activate();
  const firstId = guard.begin();
  const secondId = guard.begin();

  second.resolve({ revision: 2 });
  const latest = await second.promise;
  guard.commit(secondId, () => {
    state.workspace = latest;
    state.error = null;
    state.isLoading = false;
  });
  first.resolve({ revision: 1 });
  const older = await first.promise;
  guard.commit(firstId, () => {
    state.workspace = older;
    state.isLoading = false;
  });

  assert.deepEqual(state, { workspace: { revision: 2 }, error: null, isLoading: false });
});

test("an older refresh failure cannot replace a newer successful workspace", async () => {
  const guard = new LatestRequestGuard();
  const older = deferred();
  const newer = deferred();
  const state = { workspace: null, error: null, isLoading: true };
  guard.activate();
  const olderId = guard.begin();
  const newerId = guard.begin();

  newer.resolve({ revision: 2 });
  guard.commit(newerId, () => {
    state.workspace = { revision: 2 };
    state.error = null;
    state.isLoading = false;
  });
  older.reject(new Error("old refresh failed"));
  await older.promise.catch((error) => {
    guard.commit(olderId, () => {
      state.error = error.message;
      state.isLoading = false;
    });
  });

  assert.deepEqual(state, { workspace: { revision: 2 }, error: null, isLoading: false });
});

test("cleanup prevents an obsolete global refresh from publishing", async () => {
  const guard = new LatestRequestGuard();
  const pending = deferred();
  const state = { workspace: null, isLoading: true };
  guard.activate();
  const requestId = guard.begin();
  guard.deactivate();

  pending.resolve({ revision: 1 });
  const workspace = await pending.promise;
  guard.commit(requestId, () => {
    state.workspace = workspace;
    state.isLoading = false;
  });

  assert.deepEqual(state, { workspace: null, isLoading: true });
});

test("closing Provenance invalidates its pending result but permits an intentional reopen", () => {
  const guard = new LatestRequestGuard();
  const state = { provenance: null };
  guard.activate();
  const pendingRequestId = guard.begin();

  guard.begin();
  state.provenance = null;
  const reopenedRequestId = guard.begin();

  assert.equal(guard.commit(pendingRequestId, () => {
    state.provenance = "stale result";
  }), false);
  assert.equal(guard.commit(reopenedRequestId, () => {
    state.provenance = "reopened result";
  }), true);
  assert.equal(state.provenance, "reopened result");
});
