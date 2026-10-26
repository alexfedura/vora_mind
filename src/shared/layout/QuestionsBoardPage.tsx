import React, { FormEvent, useEffect, useState } from "react";
import { ArrowLeft, CircleHelp, House, Plus, Trash2 } from "lucide-react";
import { MarkdownTextEditor } from "@/features/markdown-editor/MarkdownTextEditor";
import { useAppStore } from "@/shared/store/useAppStore";
import type { QuestionItem } from "@/types";

interface QuestionsBoardPageProps {
  onGoHome: () => void;
}

export const QuestionsBoardPage: React.FC<QuestionsBoardPageProps> = ({ onGoHome }) => {
  const questions = useAppStore((state) => state.questions);
  const saveStatus = useAppStore((state) => state.questionSaveStatus);
  const saveError = useAppStore((state) => state.questionSaveError);
  const loadQuestions = useAppStore((state) => state.loadQuestions);
  const createQuestion = useAppStore((state) => state.createQuestion);
  const updateQuestion = useAppStore((state) => state.updateQuestion);
  const flushQuestion = useAppStore((state) => state.flushQuestion);
  const deleteQuestion = useAppStore((state) => state.deleteQuestion);
  const [selectedQuestionId, setSelectedQuestionId] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingNew, setIsSavingNew] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const selectedQuestion = questions.find((question) => question.id === selectedQuestionId);
  const selectedSaveStatus = selectedQuestionId ? saveStatus[selectedQuestionId] : undefined;

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setLoadError(null);
    void loadQuestions()
      .catch((error: unknown) => {
        console.error("Could not load Questions Board items:", error);
        if (!cancelled) {
          setLoadError(error instanceof Error ? error.message : String(error));
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [loadQuestions]);

  const handleCreate = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    setIsSavingNew(true);
    setActionError(null);
    try {
      const question = await createQuestion(newTitle, newDescription);
      setSelectedQuestionId(question.id);
      setNewTitle("");
      setNewDescription("");
      setIsCreating(false);
    } catch (error) {
      console.error("Could not create question:", error);
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSavingNew(false);
    }
  };

  const handleReturnToList = async (): Promise<void> => {
    if (selectedQuestionId) {
      try {
        await flushQuestion(selectedQuestionId);
      } catch {
        return;
      }
    }
    setSelectedQuestionId(null);
  };

  const handleDelete = async (): Promise<void> => {
    if (!selectedQuestion || !window.confirm(`Delete "${selectedQuestion.title}"?`)) return;
    setIsDeleting(true);
    setActionError(null);
    try {
      await deleteQuestion(selectedQuestion.id);
      setSelectedQuestionId(null);
    } catch (error) {
      console.error("Could not delete question:", error);
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsDeleting(false);
    }
  };

  const handleGoHome = (): void => {
    if (selectedQuestionId) {
      void flushQuestion(selectedQuestionId).catch(() => undefined);
    }
    onGoHome();
  };

  const renderErrors = (): React.ReactNode => {
    if (!loadError && !actionError && !(saveError && selectedSaveStatus === "error")) return null;
    return (
      <div
        role="alert"
        className="mb-5 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200"
      >
        <span>
          {loadError
            ? `Could not load Questions Board: ${loadError}`
            : actionError
              ? `Could not complete this action: ${actionError}`
              : `Could not save this question: ${saveError}`}
        </span>
        {loadError ? (
          <button
            type="button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              void loadQuestions()
                .catch((error: unknown) => {
                  console.error("Could not load Questions Board items:", error);
                  setLoadError(error instanceof Error ? error.message : String(error));
                })
                .finally(() => setIsLoading(false));
            }}
            className="font-semibold underline underline-offset-2"
          >
            Retry
          </button>
        ) : selectedQuestionId && saveError ? (
          <button
            type="button"
            onClick={() => void flushQuestion(selectedQuestionId).catch(() => undefined)}
            className="font-semibold underline underline-offset-2"
          >
            Retry
          </button>
        ) : null}
      </div>
    );
  };

  return (
    <section className="absolute inset-0 z-20 flex min-h-0 flex-col overflow-hidden bg-[#101114] text-gray-200">
      <header className="border-b border-[#292c34] bg-[#14161b] px-6 py-5 sm:px-10">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-4">
          <div className="flex min-w-0 items-center gap-3 pl-12 sm:pl-14">
            {selectedQuestion ? (
              <button
                type="button"
                onClick={() => void handleReturnToList()}
                aria-label="Back to Questions Board"
                className="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-xl border border-[#343741] bg-[#1a1c22] text-gray-300 transition hover:border-purple-500/40 hover:text-white"
              >
                <ArrowLeft size={17} />
              </button>
            ) : (
              <CircleHelp size={25} className="flex-shrink-0 text-purple-300" />
            )}
            <div className="min-w-0">
              <h1 className="truncate text-2xl font-semibold tracking-tight text-white">
                {selectedQuestion?.title || "Questions Board"}
              </h1>
              <p className="mt-1 text-sm text-purple-300">
                {selectedQuestion ? "Question details" : "Keep track of questions worth exploring"}
              </p>
            </div>
          </div>

          <div className="flex flex-shrink-0 items-center gap-2">
            {selectedQuestion ? (
              <button
                type="button"
                onClick={() => void handleDelete()}
                disabled={isDeleting}
                aria-label="Delete question"
                title="Delete question"
                className="flex h-9 w-9 items-center justify-center rounded-xl border border-[#343741] bg-[#1a1c22] text-gray-400 transition hover:border-rose-500/40 hover:text-rose-300 disabled:opacity-50"
              >
                <Trash2 size={16} />
              </button>
            ) : (
              <button
                type="button"
                onClick={() => {
                  setIsCreating((value) => !value);
                  setActionError(null);
                }}
                className="flex items-center gap-2 rounded-xl border border-purple-500/40 bg-purple-500/15 px-3 py-2 text-xs font-semibold text-purple-100 transition hover:bg-purple-500/25"
              >
                <Plus size={15} />
                New question
              </button>
            )}
            <button
              type="button"
              onClick={handleGoHome}
              aria-label="Go to Home"
              title="Home"
              className="flex h-9 w-9 items-center justify-center rounded-xl border border-[#343741] bg-[#1a1c22] text-gray-300 transition hover:border-purple-500/40 hover:bg-purple-500/10 hover:text-white"
            >
              <House size={17} />
            </button>
          </div>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-7 sm:px-10">
        <div className="mx-auto w-full max-w-6xl">
          {renderErrors()}

          {selectedQuestion ? (
            <div className="mx-auto w-full md:w-[70%]">
              <label htmlFor="question-title" className="mb-2 block text-xs font-semibold uppercase tracking-wide text-gray-500">
                Question summary
              </label>
              <input
                id="question-title"
                type="text"
                maxLength={200}
                value={selectedQuestion.title}
                onChange={(event) =>
                  updateQuestion(selectedQuestion.id, event.target.value, selectedQuestion.description)
                }
                onBlur={() => void flushQuestion(selectedQuestion.id).catch(() => undefined)}
                className="mb-6 w-full border-b border-[#292c34] bg-transparent pb-3 text-xl font-semibold text-white outline-none placeholder:text-gray-600 focus:border-purple-500/60"
                placeholder="Summarize your question"
              />
              <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500">
                Full question
              </p>
              <MarkdownTextEditor
                key={selectedQuestion.id}
                value={selectedQuestion.description}
                onChange={(description) =>
                  updateQuestion(selectedQuestion.id, selectedQuestion.title, description)
                }
                onBlur={() => void flushQuestion(selectedQuestion.id).catch(() => undefined)}
                placeholder="Write the full question here... (Markdown supported)"
                direction="auto"
                ariaLabel={`Full question: ${selectedQuestion.title}`}
                className="vm-journal-markdown-editor min-h-[55vh] w-full bg-transparent text-gray-200"
              />
              <p className="mt-3 text-right text-xs text-gray-500" aria-live="polite">
                {selectedSaveStatus === "saving"
                  ? "Saving..."
                  : selectedSaveStatus === "saved"
                    ? "Saved"
                    : ""}
              </p>
            </div>
          ) : (
            <>
              {isCreating && (
                <form
                  onSubmit={(event) => void handleCreate(event)}
                  className="mb-6 rounded-2xl border border-purple-500/30 bg-[#171920] p-5"
                >
                  <label htmlFor="new-question-title" className="mb-2 block text-sm font-semibold text-gray-200">
                    Question summary
                  </label>
                  <input
                    id="new-question-title"
                    type="text"
                    required
                    maxLength={200}
                    value={newTitle}
                    onChange={(event) => setNewTitle(event.target.value)}
                    autoFocus
                    placeholder="A short title for your question"
                    className="mb-4 w-full rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-white outline-none placeholder:text-gray-600 focus:border-purple-500/60"
                  />
                  <label htmlFor="new-question-description" className="mb-2 block text-sm font-semibold text-gray-200">
                    Full question
                  </label>
                  <textarea
                    id="new-question-description"
                    value={newDescription}
                    onChange={(event) => setNewDescription(event.target.value)}
                    rows={5}
                    dir="auto"
                    placeholder="Add the complete question or context (optional)"
                    className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none placeholder:text-gray-600 focus:border-purple-500/60"
                  />
                  <div className="mt-4 flex justify-end gap-2">
                    <button
                      type="button"
                      onClick={() => setIsCreating(false)}
                      className="rounded-xl px-4 py-2 text-xs font-semibold text-gray-400 transition hover:bg-[#22242c] hover:text-white"
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      disabled={isSavingNew || !newTitle.trim()}
                      className="rounded-xl bg-purple-600 px-4 py-2 text-xs font-semibold text-white transition hover:bg-purple-500 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {isSavingNew ? "Creating..." : "Create question"}
                    </button>
                  </div>
                </form>
              )}

              {isLoading ? (
                <p className="py-12 text-center text-sm text-gray-500">Loading questions...</p>
              ) : questions.length === 0 ? (
                <div className="rounded-2xl border border-dashed border-[#343741] px-6 py-14 text-center">
                  <CircleHelp size={32} className="mx-auto text-gray-600" />
                  <p className="mt-4 text-sm font-semibold text-gray-300">No questions yet</p>
                  <p className="mt-1 text-xs text-gray-500">Create a question to start your board.</p>
                  {!isCreating && (
                    <button
                      type="button"
                      onClick={() => setIsCreating(true)}
                      className="mt-5 rounded-xl bg-purple-600 px-4 py-2 text-xs font-semibold text-white transition hover:bg-purple-500"
                    >
                      Add your first question
                    </button>
                  )}
                </div>
              ) : (
                <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
                  {questions.map((question: QuestionItem) => (
                    <button
                      key={question.id}
                      type="button"
                      onClick={() => {
                        setActionError(null);
                        setSelectedQuestionId(question.id);
                      }}
                      className="flex min-h-44 flex-col rounded-2xl border border-[#2d3038] bg-[#191b20] p-5 text-left transition hover:border-purple-500/40 hover:bg-[#1d1f26]"
                    >
                      <span className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-purple-300">
                        <CircleHelp size={14} />
                        Question
                      </span>
                      <span className="mt-4 line-clamp-2 break-words text-base font-semibold text-white">
                        {question.title}
                      </span>
                      <span
                        dir="auto"
                        className={`mt-2 line-clamp-4 whitespace-pre-wrap break-words text-sm ${
                          question.description ? "text-gray-400" : "text-gray-600"
                        }`}
                      >
                        {question.description || "No details added yet"}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </section>
  );
};
