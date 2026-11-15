import assert from "node:assert/strict";
import test from "node:test";
import { ProjectWorkspaceController } from "../src/shared/layout/projectWorkspaceState.ts";

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

test("a late project A response cannot replace project B after a switch", async () => {
  const controller = new ProjectWorkspaceController();
  const projectA = deferred();
  const projectB = deferred();

  controller.activateProject("A");
  const requestA = controller.refresh("A", () => projectA.promise);
  assert.equal(controller.getSnapshot().status, "loading");

  controller.deactivateProject("A");
  controller.activateProject("B");
  const requestB = controller.refresh("B", () => projectB.promise);
  projectB.resolve({ projectId: "B", items: ["b"] });
  assert.equal(await requestB, true);

  projectA.resolve({ projectId: "A", items: ["a"] });
  assert.equal(await requestA, false);
  assert.deepEqual(controller.getSnapshot(), {
    projectId: "B",
    status: "loaded",
    workspace: { projectId: "B", items: ["b"] },
    error: null,
    isMutating: false,
  });
});

test("a late project A failure cannot replace loaded project B state", async () => {
  const controller = new ProjectWorkspaceController();
  const projectA = deferred();

  controller.activateProject("A");
  const requestA = controller.refresh("A", () => projectA.promise);
  controller.deactivateProject("A");
  controller.activateProject("B");
  await controller.refresh("B", async () => ({ projectId: "B", items: ["b"] }));

  projectA.reject(new Error("project A failed"));
  assert.equal(await requestA, false);
  assert.deepEqual(controller.getSnapshot().workspace, { projectId: "B", items: ["b"] });
  assert.equal(controller.getSnapshot().error, null);
});

for (const operationKind of ["create", "update"]) {
  test(`a project ${operationKind} resolving after a switch cannot mutate project B`, async () => {
    const controller = new ProjectWorkspaceController();
    const mutation = deferred();
    let projectALoads = 0;

    controller.activateProject("A");
    const operation = controller.runAction(
      "A",
      () => mutation.promise,
      async () => {
        projectALoads += 1;
        return { projectId: "A", items: [] };
      },
    );

    controller.deactivateProject("A");
    controller.activateProject("B");
    await controller.refresh("B", async () => ({ projectId: "B", items: ["b"] }));
    mutation.resolve({ id: "created-or-updated-in-A" });

    assert.deepEqual(await operation, { status: "stale" });
    assert.equal(projectALoads, 0);
    assert.deepEqual(controller.getSnapshot().workspace, { projectId: "B", items: ["b"] });
  });
}

test("only the latest refresh for one project can publish its workspace", async () => {
  const controller = new ProjectWorkspaceController();
  const first = deferred();
  const second = deferred();
  controller.activateProject("A");

  const firstRefresh = controller.refresh("A", () => first.promise);
  const secondRefresh = controller.refresh("A", () => second.promise);
  second.resolve({ projectId: "A", revision: 2 });
  assert.equal(await secondRefresh, true);
  first.resolve({ projectId: "A", revision: 1 });
  assert.equal(await firstRefresh, false);

  assert.deepEqual(controller.getSnapshot().workspace, { projectId: "A", revision: 2 });
});

test("a current failed load exposes an error instead of an empty successful workspace", async () => {
  const controller = new ProjectWorkspaceController();
  controller.activateProject("A");

  assert.equal(await controller.refresh("A", async () => {
    throw new Error("load unavailable");
  }), false);
  assert.deepEqual(controller.getSnapshot(), {
    projectId: "A",
    status: "error",
    workspace: null,
    error: "load unavailable",
    isMutating: false,
  });
});

test("cleanup prevents a pending response from restoring workspace state", async () => {
  const controller = new ProjectWorkspaceController();
  const request = deferred();
  controller.activateProject("A");
  const pending = controller.refresh("A", () => request.promise);

  controller.deactivateProject("A");
  request.resolve({ projectId: "A", items: ["obsolete"] });

  assert.equal(await pending, false);
  assert.deepEqual(controller.getSnapshot(), {
    projectId: "",
    status: "idle",
    workspace: null,
    error: null,
    isMutating: false,
  });
});
