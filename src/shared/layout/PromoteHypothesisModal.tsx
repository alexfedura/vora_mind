import React, { useEffect, useState } from "react";
import { X } from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import type {
  HypothesisPromotionInput,
  LinkedDefinition,
  QuestionItem,
} from "@/types";

interface PromoteHypothesisModalProps {
  inquiry: QuestionItem;
  onClose: () => void;
  onPromoted: () => void;
}

type PromotionFields = Omit<HypothesisPromotionInput, "inquiryId" | "linkedDefinitions">;

const initialFields: PromotionFields = {
  statementH1: "",
  statementH0: "",
  independentVar: "",
  dependentVar: "",
  metric: "",
  successThreshold: "",
};

const fieldLabels: Record<keyof PromotionFields, string> = {
  statementH1: "H1 — Alternative hypothesis",
  statementH0: "H0 — Null hypothesis",
  independentVar: "Independent variable",
  dependentVar: "Dependent variable",
  metric: "Measurement metric",
  successThreshold: "Success threshold",
};

export const PromoteHypothesisModal: React.FC<PromoteHypothesisModalProps> = ({
  inquiry,
  onClose,
  onPromoted,
}) => {
  const promoteToHypothesis = useAppStore((state) => state.promoteToHypothesis);
  const definitions = useAppStore((state) => state.libraryDefinitionsForSelection);
  const loadDefinitions = useAppStore((state) => state.loadLibraryDefinitionsForSelection);
  const [fields, setFields] = useState<PromotionFields>(initialFields);
  const [selectedDefinitions, setSelectedDefinitions] = useState<LinkedDefinition[]>([]);
  const [isLoadingDefinitions, setIsLoadingDefinitions] = useState(true);
  const [definitionsError, setDefinitionsError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    void loadDefinitions()
      .then(() => {
        if (isMounted) setDefinitionsError(null);
      })
      .catch((cause: unknown) => {
        console.error("Could not load Library definitions:", cause);
        if (isMounted) {
          setDefinitionsError(cause instanceof Error ? cause.message : String(cause));
        }
      })
      .finally(() => {
        if (isMounted) setIsLoadingDefinitions(false);
      });
    return () => {
      isMounted = false;
    };
  }, [loadDefinitions]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    setIsSaving(true);
    setError(null);
    try {
      await promoteToHypothesis({
        inquiryId: inquiry.id,
        ...fields,
        linkedDefinitions: selectedDefinitions,
      });
      onPromoted();
    } catch (cause) {
      console.error("Could not promote inquiry to hypothesis:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !isSaving) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="promote-hypothesis-title"
        className="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-[#343741] bg-[#17191f] p-5 shadow-2xl sm:p-7"
      >
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <h2 id="promote-hypothesis-title" className="text-lg font-semibold text-white">
              Promote to Hypothesis
            </h2>
            <p dir="auto" className="mt-1 text-sm text-gray-400">
              Inquiry: {inquiry.title}
            </p>
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
          {(Object.keys(fieldLabels) as Array<keyof PromotionFields>).map((field) => (
            <label key={field} className="block">
              <span className="mb-1.5 block text-xs font-medium text-gray-400">
                {fieldLabels[field]}
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
          ))}
          <section dir="auto" className="border-t border-[#292c34] pt-4">
            <h3 className="text-sm font-semibold text-white">
              Linked Definitions (Optional)
            </h3>
            <label dir="auto" className="mt-3 block">
              <span className="mb-1.5 block text-xs font-medium text-gray-400">
                Select Library definitions and versions
              </span>
              <select
                dir="auto"
                multiple
                size={Math.min(Math.max(definitions.length, 3), 6)}
                value={selectedDefinitions.map((definition) => definition.libraryVersionId)}
                onChange={(event) => {
                  const selectedVersionIds = new Set(
                    Array.from(event.currentTarget.selectedOptions, (option) => option.value),
                  );
                  setSelectedDefinitions(
                    definitions
                      .filter((definition) => selectedVersionIds.has(definition.versionId))
                      .map((definition) => ({
                        libraryGroupId: definition.groupId,
                        libraryVersionId: definition.versionId,
                        definitionName: definition.name,
                        versionString: definition.versionString,
                      })),
                  );
                }}
                disabled={isSaving || isLoadingDefinitions || definitions.length === 0}
                className="w-full rounded-xl border border-[#343741] bg-[#101114] px-3 py-2.5 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
              >
                {definitions.map((definition) => (
                  <option key={definition.versionId} value={definition.versionId} dir="auto">
                    {definition.name} (v{definition.versionString})
                  </option>
                ))}
              </select>
            </label>
            {isLoadingDefinitions && (
              <p className="mt-2 text-xs text-gray-500">Loading Library definitions...</p>
            )}
            {!isLoadingDefinitions && !definitionsError && definitions.length === 0 && (
              <p dir="auto" className="mt-2 text-xs text-gray-500">
                No Library definitions are available.
              </p>
            )}
            {definitionsError && (
              <p role="alert" className="mt-2 text-sm text-rose-300">
                Could not load Library definitions: {definitionsError}
              </p>
            )}
          </section>
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
              className="rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2 text-sm font-semibold text-gray-950 shadow-lg shadow-amber-950/30 transition hover:bg-amber-400 hover:shadow-amber-950/40 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-amber-300 disabled:cursor-not-allowed disabled:opacity-70"
            >
              {isSaving ? "Promoting..." : "Create Hypothesis"}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
};
