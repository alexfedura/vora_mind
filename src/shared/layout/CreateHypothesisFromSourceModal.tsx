import React, { useState } from "react";
import { X } from "lucide-react";
import type { SourceHypothesisInput } from "@/types";

interface CreateHypothesisFromSourceModalProps {
  sourceTitle: string;
  sourceCardId: string;
  sourceHighlightId: string | null;
  onClose: () => void;
  onCreate: (input: Omit<SourceHypothesisInput, "sourceCardId" | "sourceHighlightId" | "sourceTitle" | "sourceContent">) => Promise<void>;
}

type HypothesisFields = Pick<
  SourceHypothesisInput,
  "statementH1" | "statementH0" | "independentVar" | "dependentVar" | "metric" | "successThreshold"
>;

const initialFields: HypothesisFields = {
  statementH1: "",
  statementH0: "",
  independentVar: "",
  dependentVar: "",
  metric: "",
  successThreshold: "",
};

const fieldLabels: Record<keyof HypothesisFields, string> = {
  statementH1: "H1 — Alternative hypothesis",
  statementH0: "H0 — Null hypothesis",
  independentVar: "Independent variable",
  dependentVar: "Dependent variable",
  metric: "Measurement metric",
  successThreshold: "Success threshold",
};

export const CreateHypothesisFromSourceModal: React.FC<CreateHypothesisFromSourceModalProps> = ({
  sourceTitle,
  sourceCardId,
  sourceHighlightId,
  onClose,
  onCreate,
}) => {
  const [fields, setFields] = useState<HypothesisFields>(initialFields);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    setIsSaving(true);
    setError(null);
    try {
      await onCreate(fields);
    } catch (cause) {
      console.error("Could not create hypothesis from canvas source:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-black/70 p-4"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !isSaving) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="create-source-hypothesis-title"
        className="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-[#343741] bg-[#17191f] p-5 shadow-2xl sm:p-7"
      >
        <div className="mb-5 flex items-start justify-between gap-4">
          <div className="min-w-0">
            <h2 id="create-source-hypothesis-title" className="text-lg font-semibold text-white">
              Create Hypothesis from this Source
            </h2>
            <p dir="auto" className="mt-1 truncate text-sm text-gray-400">
              Source: {sourceTitle}
            </p>
            {sourceHighlightId && (
              <p className="mt-1 text-xs text-gray-500">Linked highlight: {sourceHighlightId}</p>
            )}
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
        {error && <p role="alert" className="mb-4 text-sm text-rose-300">{error}</p>}
        <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
          {(Object.keys(fieldLabels) as Array<keyof HypothesisFields>).map((field) => (
            <label key={field} className="block">
              <span className="mb-1.5 block text-xs font-medium text-gray-400">{fieldLabels[field]}</span>
              <textarea
                dir="auto"
                required
                rows={field === "statementH1" || field === "statementH0" ? 2 : 1}
                value={fields[field]}
                onChange={(event) => setFields((current) => ({
                  ...current,
                  [field]: event.currentTarget.value,
                }))}
                disabled={isSaving}
                className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-3 py-2.5 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
              />
            </label>
          ))}
          <div className="flex justify-end gap-2 border-t border-[#292c34] pt-4">
            <button
              type="button"
              disabled={isSaving}
              onClick={onClose}
              className="rounded-lg border border-[#343741] px-4 py-2.5 text-sm font-medium text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSaving}
              className="rounded-lg bg-[#e8ad55] px-4 py-2.5 text-sm font-semibold text-[#17130d] transition hover:bg-[#f0bd6c] disabled:opacity-50"
            >
              {isSaving ? "Creating..." : "Create Hypothesis"}
            </button>
          </div>
        </form>
        <span className="sr-only">Source identifier: {sourceCardId}</span>
      </section>
    </div>
  );
};
