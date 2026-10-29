import React, { useEffect, useState } from "react";
import { ArrowLeft } from "lucide-react";
import { HypothesisDetail } from "@/shared/layout/HypothesisDetail";
import { useAppStore } from "@/shared/store/useAppStore";
import type { Hypothesis } from "@/types";

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const HypothesesList: React.FC = () => {
  const hypotheses = useAppStore((state) => state.hypotheses);
  const loadHypotheses = useAppStore((state) => state.loadHypotheses);
  const [selectedHypothesis, setSelectedHypothesis] = useState<Hypothesis | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);
    void loadHypotheses()
      .catch((cause: unknown) => {
        console.error("Could not load hypotheses:", cause);
        if (!cancelled) setError(getErrorMessage(cause));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [loadHypotheses]);

  const updatedHypothesis = (updated: Hypothesis): void => {
    setSelectedHypothesis(updated);
  };

  if (selectedHypothesis) {
    return (
      <div className="mx-auto w-full max-w-4xl">
        <button
          type="button"
          onClick={() => setSelectedHypothesis(null)}
          className="mb-4 inline-flex items-center gap-2 rounded-lg border border-[#343741] px-3 py-2 text-sm font-medium text-gray-300 transition hover:bg-[#22242b] hover:text-white"
        >
          <ArrowLeft size={16} />
          Back
        </button>
        <HypothesisDetail
          hypothesis={selectedHypothesis}
          onUpdated={updatedHypothesis}
        />
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-4xl">
      <div className="mb-5">
        <h2 className="text-xl font-semibold text-white">Lab Hypotheses</h2>
        <p className="mt-1 text-sm text-gray-400">
          Review test evidence and record a verdict for each hypothesis.
        </p>
      </div>
      {error && (
        <div role="alert" className="mb-4 rounded-xl border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200">
          {error}
          <button
            type="button"
            onClick={() => void loadHypotheses().catch((cause: unknown) => {
              console.error("Could not reload hypotheses:", cause);
              setError(getErrorMessage(cause));
            })}
            className="ms-3 underline underline-offset-2"
          >
            Retry
          </button>
        </div>
      )}
      {isLoading ? (
        <p className="py-12 text-center text-sm text-gray-500">Loading hypotheses...</p>
      ) : hypotheses.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-[#343741] px-6 py-14 text-center">
          <p className="text-sm font-semibold text-gray-300">No hypotheses yet</p>
          <p className="mt-1 text-xs text-gray-500">
            Promote a selected inquiry to begin testing a hypothesis.
          </p>
        </div>
      ) : (
        <ul className="space-y-3">
          {hypotheses.map((hypothesis) => (
            <li key={hypothesis.id}>
              <button
                type="button"
                dir="auto"
                onClick={() => setSelectedHypothesis(hypothesis)}
                className="w-full rounded-xl border border-[#292c34] bg-[#14161b] p-4 text-start transition hover:border-[#e8ad55]/40 hover:bg-[#191b20]"
              >
                <span className="flex items-start justify-between gap-3">
                  <span className="line-clamp-2 text-sm font-semibold text-white">
                    {hypothesis.statementH1}
                  </span>
                  {hypothesis.verdict && (
                    <span className="shrink-0 rounded-full border border-[#343741] px-2.5 py-1 text-[11px] font-semibold text-gray-300">
                      {hypothesis.verdict}
                    </span>
                  )}
                </span>
                <span className="mt-2 block line-clamp-1 text-xs text-gray-500">
                  {hypothesis.metric} · Success: {hypothesis.successThreshold}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
