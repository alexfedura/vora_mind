import React, { useState } from "react";
import type { Hypothesis, HypothesisVerdict } from "@/types";
import { ChildHypothesisModal } from "@/shared/layout/ChildHypothesisModal";
import { useAppStore } from "@/shared/store/useAppStore";

interface HypothesisDetailProps {
  hypothesis: Hypothesis;
  onUpdated: (hypothesis: Hypothesis) => void;
  onChildCreated: () => void;
}

const verdictPresentation: Record<
  HypothesisVerdict,
  { label: string; className: string }
> = {
  supported: {
    label: "Supported",
    className: "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
  },
  rejected: {
    label: "Rejected",
    className: "border-rose-500/30 bg-rose-500/10 text-rose-300",
  },
  inconclusive: {
    label: "Inconclusive",
    className: "border-gray-600 bg-gray-700/40 text-gray-300",
  },
};

export const HypothesisDetail: React.FC<HypothesisDetailProps> = ({
  hypothesis,
  onUpdated,
  onChildCreated,
}) => {
  const recordVerdict = useAppStore((state) => state.recordVerdict);
  const graduateToLibrary = useAppStore((state) => state.graduateToLibrary);
  const [evidenceNotes, setEvidenceNotes] = useState(hypothesis.evidenceNotes ?? "");
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isCreatingChild, setIsCreatingChild] = useState(false);
  const [isRetesting, setIsRetesting] = useState(false);

  const handleGraduate = async (): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const updated = await graduateToLibrary(hypothesis.id);
      onUpdated(updated);
    } catch (cause) {
      console.error("Could not graduate hypothesis to Library:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  const handleRecordVerdict = async (verdict: HypothesisVerdict): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const updated = await recordVerdict(hypothesis.id, evidenceNotes, verdict);
      onUpdated(updated);
      setIsRetesting(false);
    } catch (cause) {
      console.error("Could not record hypothesis verdict:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <article className="mx-auto w-full max-w-4xl rounded-2xl border border-[#292c34] bg-[#14161b] p-5 sm:p-7">
      <h2 className="text-xl font-semibold text-white">Hypothesis</h2>
      {hypothesis.parentHypothesisId && (
        <p className="mt-2 text-xs text-gray-500">
          This is an iteration of a previous hypothesis.
        </p>
      )}
      <dl className="mt-5 grid gap-4 sm:grid-cols-2">
        <div className="rounded-xl bg-[#101114] p-4 sm:col-span-2">
          <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">H1</dt>
          <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
            {hypothesis.statementH1}
          </dd>
        </div>
        <div className="rounded-xl bg-[#101114] p-4 sm:col-span-2">
          <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">H0</dt>
          <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
            {hypothesis.statementH0}
          </dd>
        </div>
        <div className="rounded-xl bg-[#101114] p-4">
          <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">
            Independent variable
          </dt>
          <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
            {hypothesis.independentVar}
          </dd>
        </div>
        <div className="rounded-xl bg-[#101114] p-4">
          <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">
            Dependent variable
          </dt>
          <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
            {hypothesis.dependentVar}
          </dd>
        </div>
        <div className="rounded-xl bg-[#101114] p-4">
          <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">Metric</dt>
          <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
            {hypothesis.metric}
          </dd>
        </div>
        <div className="rounded-xl bg-[#101114] p-4">
          <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">
            Success threshold
          </dt>
          <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
            {hypothesis.successThreshold}
          </dd>
        </div>
      </dl>

      {hypothesis.verdict === "supported" && (
        <section className="mt-6 border-t border-[#292c34] pt-5">
          {hypothesis.promotedToLibraryId ? (
            <span className="inline-flex rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-sm font-semibold text-emerald-300">
              ✅ Saved to Library
            </span>
          ) : (
            <button
              type="button"
              disabled={isSaving}
              onClick={() => void handleGraduate()}
              className="rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2.5 text-sm font-bold text-gray-950 shadow-lg shadow-amber-950/30 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-60"
            >
              📚 Graduate to Library
            </button>
          )}
          {error && !isCreatingChild && (
            <p role="alert" className="mt-3 text-sm text-rose-300">
              {error}
            </p>
          )}
        </section>
      )}

      {hypothesis.verdict === "rejected" && (
        <section className="mt-6 border-t border-[#292c34] pt-5">
          <button
            type="button"
            disabled={isSaving}
            onClick={() => {
              setError(null);
              setIsCreatingChild(true);
            }}
            className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-4 py-2.5 text-sm font-semibold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50"
          >
            🌱 Create Child Hypothesis
          </button>
        </section>
      )}

      {hypothesis.verdict && (
        <section className="mt-7 border-t border-[#292c34] pt-5">
          <h3 className="text-sm font-semibold text-white">Test Verdict</h3>
          <span
            className={`mt-3 inline-flex rounded-full border px-3 py-1.5 text-sm font-semibold ${
              verdictPresentation[hypothesis.verdict].className
            }`}
          >
            {verdictPresentation[hypothesis.verdict].label}
          </span>
          <p dir="auto" className="mt-4 whitespace-pre-wrap text-sm text-gray-300">
            {hypothesis.evidenceNotes || "No evidence notes recorded."}
          </p>
          {hypothesis.verdict === "inconclusive" && !isRetesting && (
            <button
              type="button"
              onClick={() => {
                setEvidenceNotes(hypothesis.evidenceNotes ?? "");
                setError(null);
                setIsRetesting(true);
              }}
              className="mt-4 rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-2.5 text-sm font-semibold text-amber-200 transition hover:bg-amber-500/20"
            >
              🔄 Re-test / Update Evidence
            </button>
          )}
        </section>
      )}

      {(!hypothesis.verdict || isRetesting) && (
        <section className="mt-7 border-t border-[#292c34] pt-5">
          <h3 className="text-sm font-semibold text-white">
            {isRetesting ? "Re-test Hypothesis" : "Log Test Results"}
          </h3>
          <label className="mt-4 block">
            <span className="mb-2 block text-xs font-medium text-gray-400">
              Evidence notes
            </span>
            <textarea
              dir="auto"
              value={evidenceNotes}
              onChange={(event) => setEvidenceNotes(event.target.value)}
              rows={5}
              disabled={isSaving}
              className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
            />
          </label>
          {error && (
            <p role="alert" className="mt-3 text-sm text-rose-300">
              {error}
            </p>
          )}
          <div className="mt-4 flex flex-wrap gap-2">
            <button
              type="button"
              disabled={isSaving}
              onClick={() => void handleRecordVerdict("supported")}
              className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm font-semibold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50"
            >
              ✅ Supported
            </button>
            <button
              type="button"
              disabled={isSaving}
              onClick={() => void handleRecordVerdict("rejected")}
              className="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-sm font-semibold text-rose-200 transition hover:bg-rose-500/20 disabled:opacity-50"
            >
              ❌ Rejected
            </button>
            <button
              type="button"
              disabled={isSaving}
              onClick={() => void handleRecordVerdict("inconclusive")}
              className="rounded-lg border border-gray-600 bg-gray-700/40 px-3 py-2 text-sm font-semibold text-gray-200 transition hover:bg-gray-700/70 disabled:opacity-50"
            >
              ⚠️ Inconclusive
            </button>
            {isRetesting && (
              <button
                type="button"
                disabled={isSaving}
                onClick={() => setIsRetesting(false)}
                className="rounded-lg border border-[#343741] px-3 py-2 text-sm font-medium text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
              >
                Cancel
              </button>
            )}
          </div>
        </section>
      )}

      {isCreatingChild && (
        <ChildHypothesisModal
          hypothesis={hypothesis}
          onClose={() => setIsCreatingChild(false)}
          onCreated={onChildCreated}
        />
      )}
    </article>
  );
};
