import React, { useState } from "react";
import { X } from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import type { ChildHypothesisInput, Hypothesis } from "@/types";

interface ChildHypothesisModalProps {
  hypothesis: Hypothesis;
  onClose: () => void;
  onCreated: () => void;
}

type ChildHypothesisFields = Omit<ChildHypothesisInput, "parentId" | "linkedDefinitions">;

const childFieldLabels: Record<keyof ChildHypothesisFields, string> = {
  statementH1: "H1 — Alternative hypothesis",
  statementH0: "H0 — Null hypothesis",
  independentVar: "Independent variable",
  dependentVar: "Dependent variable",
  metric: "Measurement metric",
  successThreshold: "Success threshold",
};

export const ChildHypothesisModal: React.FC<ChildHypothesisModalProps> = ({
  hypothesis,
  onClose,
  onCreated,
}) => {
  const createChildHypothesis = useAppStore((state) => state.createChildHypothesis);
  const [fields, setFields] = useState<ChildHypothesisFields>(() => ({
    statementH1: hypothesis.statementH1,
    statementH0: hypothesis.statementH0,
    independentVar: hypothesis.independentVar,
    dependentVar: hypothesis.dependentVar,
    metric: hypothesis.metric,
    successThreshold: hypothesis.successThreshold,
  }));
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    setIsSaving(true);
    setError(null);
    try {
      await createChildHypothesis({
        parentId: hypothesis.id,
        ...fields,
        linkedDefinitions: hypothesis.linkedDefinitions,
      });
      onCreated();
    } catch (cause) {
      console.error("Could not create child hypothesis:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      dir="auto"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !isSaving) onClose();
      }}
    >
      <section
        dir="auto"
        role="dialog"
        aria-modal="true"
        aria-labelledby="child-hypothesis-title"
        className="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-[#343741] bg-[#17191f] p-5 shadow-2xl sm:p-7"
      >
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <h2 id="child-hypothesis-title" className="text-lg font-semibold text-white">
              Create Child Hypothesis (Iteration)
            </h2>
            <p className="mt-1 text-sm text-gray-400">
              Adjust the hypothesis for another iteration.
            </p>
            <p className="mt-1 text-xs text-gray-500">Parent ID: {hypothesis.id}</p>
          </div>
          <button
            type="button"
            aria-label="Close dialog"
            disabled={isSaving}
            onClick={onClose}
            className="rounded-lg p-2 text-gray-400 transition hover:bg-[#292c34] hover:text-white disabled:opacity-50"
          >
            <X size={18} />
          </button>
        </div>
        <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
          {(Object.keys(childFieldLabels) as Array<keyof ChildHypothesisFields>).map(
            (field) => (
              <label key={field} className="block">
                <span className="mb-1.5 block text-xs font-medium text-gray-400">
                  {childFieldLabels[field]}
                </span>
                <textarea
                  dir="auto"
                  required
                  rows={field === "statementH1" || field === "statementH0" ? 2 : 1}
                  value={fields[field]}
                  onChange={(event) => setFields((current) => ({
                    ...current,
                    [field]: event.target.value,
                  }))}
                  disabled={isSaving}
                  className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-3 py-2.5 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
                />
              </label>
            ),
          )}
          {error && (
            <p role="alert" className="text-sm text-rose-300">
              {error}
            </p>
          )}
          <div className="flex justify-end gap-2 border-t border-[#292c34] pt-4">
            <button
              type="button"
              disabled={isSaving}
              onClick={onClose}
              className="rounded-lg border border-[#343741] px-4 py-2 text-sm font-medium text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSaving}
              className="rounded-lg border border-emerald-500/30 bg-emerald-500 px-4 py-2 text-sm font-semibold text-gray-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-70"
            >
              {isSaving ? "Creating..." : "Create Iteration"}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
};
