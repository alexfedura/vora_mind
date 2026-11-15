import assert from "node:assert/strict";
import test from "node:test";
import {
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
