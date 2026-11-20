import assert from "node:assert/strict";
import test, { mock } from "node:test";

const calls = [];
let response;
let failure;

mock.module("@tauri-apps/api/core", {
  exports: {
    invoke: async (command, args) => {
      calls.push({ command, args });
      if (failure) throw failure;
      return response;
    },
  },
});

const { learningService } = await import("../src/mind/services/learningService.ts");

test("learning service loads the workspace and preserves exact pinned IDs", async () => {
  calls.length = 0;
  failure = null;
  response = {
    projects: [{
      id: "learning-project-a",
      frameworkId: "framework-a",
      frameworkVersionId: "framework-a-v1",
    }],
    attempts: [{
      id: "attempt-a",
      learningProjectId: "learning-project-a",
      assessmentVersionId: "assessment-a-v1",
      frameworkVersionId: "framework-a-v1",
    }],
  };

  const result = await learningService.getWorkspace();

  assert.deepEqual(calls, [{ command: "learning_get_workspace", args: undefined }]);
  assert.equal(result.projects[0].frameworkVersionId, "framework-a-v1");
  assert.equal(result.attempts[0].assessmentVersionId, "assessment-a-v1");
  assert.equal(result.attempts[0].frameworkVersionId, "framework-a-v1");
});

test("learning mutation service forwards exact assessment and competency requests", async () => {
  calls.length = 0;
  failure = null;
  response = {};
  const attemptRequest = {
    learningProjectId: "learning-project-a",
    learnerId: "learner-a",
    assessmentVersionId: "assessment-a-v1",
  };
  const competencyRequest = {
    competencyId: "competency-a",
    learnerId: "learner-a",
    masteryRuleVersionId: "mastery-rule-a-v1",
  };

  await learningService.createAttempt(attemptRequest);
  await learningService.evaluateCompetency(competencyRequest);

  assert.deepEqual(calls, [
    { command: "learning_create_attempt", args: { request: attemptRequest } },
    { command: "learning_evaluate_competency", args: { request: competencyRequest } },
  ]);
});

test("learning service surfaces persistence and lifecycle failures", async () => {
  calls.length = 0;
  failure = new Error("submitted responses cannot be rewritten");

  await assert.rejects(
    learningService.saveAttemptResponse({
      attemptId: "attempt-a",
      assessmentQuestionId: "question-a",
      response: "changed answer",
    }),
    /submitted responses cannot be rewritten/,
  );

  assert.deepEqual(calls, [{
    command: "learning_save_attempt_response",
    args: {
      request: {
        attemptId: "attempt-a",
        assessmentQuestionId: "question-a",
        response: "changed answer",
      },
    },
  }]);
});
