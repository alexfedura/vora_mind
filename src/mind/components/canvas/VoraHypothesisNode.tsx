import { memo, type MouseEvent } from "react";
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { useAppStore } from "@/shared/store/useAppStore";
import type { VoraHypothesisData } from "@/mind/types";

type VoraHypothesisNodeModel = Node<VoraHypothesisData, "voraHypothesis">;

function getStatusPresentation(status: string): { label: string; className: string } {
  switch (status) {
    case "testing":
    case "preregistered":
      return {
        label: status === "testing" ? "Testing" : "Preregistered",
        className: "border-sky-400/30 bg-sky-500/10 text-sky-200",
      };
    case "supported":
    case "partially_supported":
      return { label: "Supported", className: "border-emerald-400/30 bg-emerald-500/10 text-emerald-200" };
    case "rejected":
      return { label: "Rejected", className: "border-rose-400/30 bg-rose-500/10 text-rose-200" };
    case "inconclusive":
      return { label: "Inconclusive", className: "border-amber-400/30 bg-amber-500/10 text-amber-200" };
    default:
      return { label: "Draft", className: "border-zinc-400/30 bg-zinc-500/10 text-zinc-300" };
  }
}

export const VoraHypothesisNode = memo(({ data, selected }: NodeProps<VoraHypothesisNodeModel>) => {
  const hypothesis = useAppStore((state) =>
    state.hypotheses.find((item) => item.id === data.hypothesisId),
  );
  const status = hypothesis?.verdict ??
    (hypothesis?.protocolLockedAt != null || data.protocolLockedAt != null
      ? "testing"
      : data.verdict ?? hypothesis?.status ?? data.status);
  const presentation = getStatusPresentation(status);
  const statement = hypothesis?.statementH1 ?? data.statementH1;

  const openLabDetail = (event: MouseEvent<HTMLDivElement>): void => {
    event.stopPropagation();
    window.dispatchEvent(new CustomEvent("vora:open-hypothesis-lab", {
      detail: { hypothesisId: data.hypothesisId },
    }));
  };

  return (
    <div
      className={`relative flex min-h-36 w-72 flex-col gap-3 rounded-xl border bg-[#171a22] p-4 text-zinc-200 shadow-xl ${
        selected ? "border-violet-300 ring-2 ring-violet-400/30" : "border-violet-500/40"
      }`}
      onDoubleClick={openLabDetail}
      title="Double-click to open Lab details"
    >
      {([Position.Left, Position.Right, Position.Top, Position.Bottom] as const).map((position) => {
        const side = position === Position.Left ? "left"
          : position === Position.Right ? "right"
            : position === Position.Top ? "top" : "bottom";
        const positionClass = position === Position.Left ? "!-left-1.5"
          : position === Position.Right ? "!-right-1.5"
            : position === Position.Top ? "!-top-1.5" : "!-bottom-1.5";
        return (
          <div key={side}>
            <Handle
              type="source"
              position={position}
              id={`source-${side}`}
              className={`!z-10 !h-3 !w-3 !border-2 !border-[#171a22] !bg-violet-400 ${positionClass}`}
              style={{ opacity: selected ? 1 : 0 }}
            />
            <Handle
              type="target"
              position={position}
              id={`target-${side}`}
              className={`!z-10 !h-3 !w-3 !border-2 !border-[#171a22] !bg-amber-300 ${positionClass}`}
            />
          </div>
        );
      })}
      <header className="flex items-center justify-between gap-2">
        <span className="text-[10px] font-semibold uppercase tracking-[0.16em] text-violet-300">
          Hypothesis
        </span>
        <span className={`rounded-full border px-2 py-1 text-[10px] font-semibold ${presentation.className}`}>
          {presentation.label}
        </span>
      </header>
      <p dir="auto" className="nowheel line-clamp-5 whitespace-pre-wrap text-sm font-medium leading-5 text-zinc-100">
        {statement || "Untitled hypothesis"}
      </p>
      <span className="text-[10px] text-zinc-500">Double-click to open Lab details</span>
    </div>
  );
});

VoraHypothesisNode.displayName = "VoraHypothesisNode";
