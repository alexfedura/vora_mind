import React, { FormEvent, useEffect, useMemo, useState } from "react";
import { PromoteHypothesisModal } from "@/shared/layout/PromoteHypothesisModal";
import { useAppStore } from "@/shared/store/useAppStore";
import type { QuestionItem } from "@/types";

interface TriageFormState {
  id: string | null;
  impact: number;
  feasibility: number;
  urgency: number;
  notes: string;
}

function sortByUpdatedAt(left: QuestionItem, right: QuestionItem): number {
  return right.updatedAt - left.updatedAt;
}

function sortByPriority(left: QuestionItem, right: QuestionItem): number {
  return (right.priorityScore ?? -1) - (left.priorityScore ?? -1) ||
    right.updatedAt - left.updatedAt;
}

export const InquiryTriagePanel: React.FC = () => {
  const questions = useAppStore((state) => state.questions);
  const questionSaveError = useAppStore((state) => state.questionSaveError);
  const loadQuestions = useAppStore((state) => state.loadQuestions);
  const triageQuestion = useAppStore((state) => state.triageQuestion);
  const untriageQuestion = useAppStore((state) => state.untriageQuestion);
  const selectQuestionForPromotion = useAppStore((state) =>
    state.selectQuestionForPromotion
  );
  const unselectQuestionForPromotion = useAppStore((state) =>
    state.unselectQuestionForPromotion
  );
  const [form, setForm] = useState<TriageFormState | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setLoadError(null);
    void loadQuestions()
      .catch((error: unknown) => {
        console.error("Could not load inquiries for Triage:", error);
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

  const awaitingTriage = useMemo(
    () => questions.filter((question) => question.status === "parked").sort(sortByUpdatedAt),
    [questions],
  );
  const rankedInquiries = useMemo(
    () => questions.filter((question) => question.status === "triaged").sort(sortByPriority),
    [questions],
  );
  const selectedInquiry = rankedInquiries.find((question) => question.isSelectedForPromotion);
  const [isPromotionModalOpen, setIsPromotionModalOpen] = useState(false);

  const handleTriage = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!form) return;
    const formData = form;
    const questionId = formData.id;
    if (questionId === null) return;
    setBusyId(questionId);
    setActionError(null);
    try {
      await triageQuestion(
        questionId,
        formData.impact,
        formData.feasibility,
        formData.urgency,
        formData.notes.trim() || undefined,
      );
      setForm(null);
    } catch (error) {
      console.error("Could not triage inquiry:", error);
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusyId(null);
    }
  };

  const runQuestionAction = async (
    id: string,
    action: (questionId: string) => Promise<QuestionItem>,
  ): Promise<void> => {
    setBusyId(id);
    setActionError(null);
    try {
      await action(id);
    } catch (error) {
      console.error("Could not update inquiry:", error);
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setBusyId(null);
    }
  };

  const renderTriageForm = (question: QuestionItem): React.ReactNode => {
    if (form?.id !== question.id) return null;

    return (
      <form onSubmit={(event) => void handleTriage(event)} className="mt-4 space-y-4">
        <div>
          <label className="mb-1 block text-xs text-gray-500">Inquiry</label>
          <p className="text-sm text-gray-300">{question.title}</p>
        </div>
        {([
          ["impact", "Impact"],
          ["feasibility", "Feasibility"],
          ["urgency", "Urgency"],
        ] as const).map(([field, label]) => (
          <label key={field} className="block">
            <span className="mb-1 flex justify-between text-xs text-gray-400">
              <span>{label}</span>
              <span>{form[field]}</span>
            </span>
            <input
              type="range"
              min={1}
              max={5}
              step={1}
              value={form[field]}
              onChange={(event) => setForm({
                ...form,
                [field]: Number(event.target.value),
              })}
              className="w-full accent-[#e8ad55]"
            />
          </label>
        ))}
        <p className="rounded-lg bg-[#101114] px-3 py-2 text-sm text-gray-300">
          Priority: {form.impact} × {form.feasibility} × {form.urgency} = {form.impact * form.feasibility * form.urgency}
        </p>
        <label className="block">
          <span className="mb-1 block text-xs text-gray-400">Notes (optional)</span>
          <textarea
            value={form.notes}
            onChange={(event) => setForm({ ...form, notes: event.target.value })}
            rows={3}
            className="w-full resize-y rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-gray-200 outline-none focus:border-[#e8ad55]"
          />
        </label>
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={() => setForm(null)}
            className="rounded-lg border border-[#343741] px-3 py-2 text-xs font-semibold text-gray-300 transition hover:bg-[#24262d]"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busyId === question.id}
            className="rounded-lg bg-[#e8ad55] px-3 py-2 text-xs font-semibold text-[#17130d] transition hover:bg-[#f0bd6c] disabled:opacity-50"
          >
            {busyId === question.id ? "Triaging..." : "Triage"}
          </button>
        </div>
      </form>
    );
  };

  const displayError = actionError || questionSaveError || loadError;
  const showUnifiedEmpty = !isLoading &&
    awaitingTriage.length === 0 &&
    rankedInquiries.length === 0;

  return (
    <div className="mx-auto w-full max-w-6xl">
      <div className="mb-5">
        <h2 className="text-xl font-semibold text-white">Inquiry Triage</h2>
        <p className="mt-1 text-sm text-gray-400">
          Prioritize parked inquiries and select one for future promotion.
        </p>
      </div>

      {displayError && (
        <div
          role="alert"
          className="mb-5 rounded-xl border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200"
        >
          {displayError}
        </div>
      )}

      {isLoading ? (
        <p className="py-12 text-center text-sm text-gray-500">Loading inquiries...</p>
      ) : showUnifiedEmpty ? (
        <div className="rounded-2xl border border-dashed border-[#343741] px-6 py-14 text-center">
          <p className="text-sm font-semibold text-gray-300">No inquiries to triage yet</p>
          <p className="mt-1 text-xs text-gray-500">
            Add inquiries in Inquiry Queue, then return here to prioritize them.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <section className="min-w-0 rounded-2xl border border-[#292c34] bg-[#14161b] p-4 sm:p-5">
            <div className="mb-4 border-b border-[#292c34] pb-3">
              <h3 className="text-sm font-semibold text-white">Awaiting Triage</h3>
            </div>
            {awaitingTriage.length === 0 ? (
              <p className="py-8 text-center text-sm text-gray-500">
                Nothing waiting to triage. Add inquiries in Inquiry Queue.
              </p>
            ) : (
              <ul className="space-y-3">
                {awaitingTriage.map((question) => (
                  <li
                    key={question.id}
                    dir="auto"
                    className="rounded-xl border border-[#292c34] bg-[#17191f] p-3"
                  >
                    <button
                      type="button"
                      aria-expanded={form?.id === question.id}
                      onClick={() => {
                        setActionError(null);
                        setForm(form?.id === question.id ? null : {
                          id: question.id,
                          impact: 3,
                          feasibility: 3,
                          urgency: 3,
                          notes: "",
                        });
                      }}
                      className="w-full text-start text-sm font-medium text-gray-200 transition hover:text-white"
                    >
                      {question.title}
                    </button>
                    {renderTriageForm(question)}
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="min-w-0 rounded-2xl border border-[#292c34] bg-[#14161b] p-4 sm:p-5">
            <div className="mb-4 border-b border-[#292c34] pb-3">
              <h3 className="text-sm font-semibold text-white">Ranked Inquiries</h3>
            </div>
            {rankedInquiries.length === 0 ? (
              <p className="py-8 text-center text-sm text-gray-500">
                Nothing triaged yet. Triage on the left to rank.
              </p>
            ) : (
              <ol className="space-y-3">
                {rankedInquiries.map((question, index) => (
                  <li
                    key={question.id}
                    dir="auto"
                    className="rounded-xl border border-[#292c34] bg-[#17191f] p-3"
                  >
                    <div className="flex flex-wrap items-center gap-3">
                      <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-blue-500/30 bg-blue-500/10 text-xs font-semibold text-blue-300">
                        {index + 1}
                      </span>
                      <div className="min-w-0 flex-1">
                        <p className="break-words text-sm font-medium text-gray-200">{question.title}</p>
                        <span className="mt-1 inline-flex rounded-full border border-blue-500/20 bg-blue-500/5 px-2.5 py-1 text-xs text-blue-200">
                          I{question.impact}·F{question.feasibility}·U{question.urgency}→{question.priorityScore}
                        </span>
                      </div>
                      {question.isSelectedForPromotion ? (
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="rounded-full border border-amber-500/30 bg-amber-500/10 px-2.5 py-1 text-xs font-medium text-amber-300">
                            ★ Selected
                          </span>
                          <button
                            type="button"
                            disabled={busyId === question.id}
                            onClick={() => void runQuestionAction(
                              question.id,
                              unselectQuestionForPromotion,
                            )}
                            className="rounded-lg border border-[#343741] px-3 py-2 text-xs font-semibold text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
                          >
                            Unselect
                          </button>
                        </div>
                      ) : (
                        <button
                          type="button"
                          disabled={busyId === question.id}
                          onClick={() => void runQuestionAction(
                            question.id,
                            selectQuestionForPromotion,
                          )}
                          className="rounded-lg bg-amber-500/15 px-3 py-2 text-xs font-semibold text-amber-300 transition hover:bg-amber-500/25 disabled:opacity-50"
                        >
                          Select for Promotion
                        </button>
                      )}
                      <button
                        type="button"
                        disabled={busyId === question.id}
                        onClick={() => {
                          setActionError(null);
                          setForm({
                            id: question.id,
                            impact: question.impact ?? 3,
                            feasibility: question.feasibility ?? 3,
                            urgency: question.urgency ?? 3,
                            notes: question.triageNotes ?? "",
                          });
                        }}
                        className="rounded-lg border border-[#343741] px-3 py-2 text-xs font-semibold text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
                      >
                        Edit Scores
                      </button>
                      <button
                        type="button"
                        disabled={busyId === question.id}
                        onClick={() => void runQuestionAction(question.id, untriageQuestion)}
                        className="rounded-lg border border-[#343741] px-3 py-2 text-xs font-semibold text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
                      >
                        Untriage
                      </button>
                    </div>
                    {renderTriageForm(question)}
                  </li>
                ))}
              </ol>
            )}
            <div className="mt-5 border-t border-[#292c34] pt-4">
              {selectedInquiry ? (
                <button
                  type="button"
                  onClick={() => setIsPromotionModalOpen(true)}
                  className="rounded-xl bg-[#e8ad55] px-4 py-3 text-sm font-semibold text-[#17130d] shadow-lg shadow-amber-950/20 transition hover:bg-[#f0bd6c]"
                >
                  🧪 Promote to Hypothesis
                </button>
              ) : (
                <p className="text-sm text-gray-500">Select an inquiry to promote.</p>
              )}
            </div>
          </section>
        </div>
      )}
      {isPromotionModalOpen && selectedInquiry && (
        <PromoteHypothesisModal
          inquiry={selectedInquiry}
          onClose={() => setIsPromotionModalOpen(false)}
          onPromoted={() => setIsPromotionModalOpen(false)}
        />
      )}
    </div>
  );
};
