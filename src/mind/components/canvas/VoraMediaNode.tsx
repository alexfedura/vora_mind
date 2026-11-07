import { memo } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { NodeResizer, type Node, type NodeProps } from "@xyflow/react";
import type { VoraMediaData } from "@/mind/types";

type VoraMediaNodeModel = Node<VoraMediaData, "voraMedia">;

export const VoraMediaNode = memo(({ id, data, selected }: NodeProps<VoraMediaNodeModel>) => {
  const source = data.filePath ? convertFileSrc(data.filePath) : null;

  return (
    <div className="relative h-full w-full overflow-hidden rounded-xl border border-white/10 bg-[#17191f] shadow-xl shadow-black/30">
      <NodeResizer
        nodeId={id}
        isVisible={selected}
        minWidth={data.type === "image" ? 160 : 240}
        minHeight={data.type === "image" ? 120 : 96}
        keepAspectRatio={data.type === "image"}
        color="#a78bfa"
      />
      {data.type === "image" ? (
        source ? (
          <img
            src={source}
            alt={data.fileName}
            draggable={false}
            className="h-full w-full select-none object-contain p-1"
          />
        ) : (
          <div className="flex h-full items-center justify-center p-4 text-center text-xs text-rose-200">
            Image path is unavailable.
          </div>
        )
      ) : (
        <div className="flex h-full flex-col justify-center gap-3 p-4">
          <div className="truncate text-xs font-medium text-zinc-200" title={data.fileName}>
            {data.fileName}
          </div>
          {source ? (
            <audio controls preload="metadata" src={source} className="w-full">
              Your browser does not support audio playback.
            </audio>
          ) : (
            <div className="text-xs text-rose-200">Audio path is unavailable.</div>
          )}
        </div>
      )}
    </div>
  );
});

VoraMediaNode.displayName = "VoraMediaNode";
