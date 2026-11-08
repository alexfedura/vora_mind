import { memo } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { NodeResizer, type Node, type NodeProps } from "@xyflow/react";
import type { VoraMediaData } from "@/mind/types";

type VoraMediaNodeModel = Node<VoraMediaData, "voraMedia">;

export const VoraMediaNode = memo(({ id, data, selected }: NodeProps<VoraMediaNodeModel>) => {
  const source = data.filePath ? convertFileSrc(data.filePath) : null;

  return (
    <div className="relative h-full w-full rounded-xl border border-white/10 bg-[#17191f] shadow-xl shadow-black/30">
      <NodeResizer
        nodeId={id}
        isVisible={selected}
        minWidth={data.type === "audio" ? 240 : 160}
        minHeight={data.type === "audio" ? 96 : 120}
        maxWidth={960}
        maxHeight={720}
        keepAspectRatio={data.type !== "audio"}
        color="#a78bfa"
      />
      {data.type === "image" ? (
        source ? (
          <div className="h-full w-full overflow-hidden rounded-xl">
            <img
              src={source}
              alt={data.fileName}
              draggable={false}
              className="h-full w-full select-none object-contain p-1"
            />
          </div>
        ) : (
          <div className="flex h-full items-center justify-center p-4 text-center text-xs text-rose-200">
            Image path is unavailable.
          </div>
        )
      ) : data.type === "video" ? (
        source ? (
          <div className="flex h-full w-full flex-col gap-1 overflow-hidden rounded-xl p-2">
            <div className="truncate text-xs font-medium text-zinc-200" title={data.fileName}>
              {data.fileName}
            </div>
            <video
              controls
              preload="metadata"
              src={source}
              className="min-h-0 w-full flex-1 object-contain"
            >
              Your browser does not support video playback.
            </video>
          </div>
        ) : (
          <div className="flex h-full items-center justify-center p-4 text-center text-xs text-rose-200">
            Video path is unavailable.
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
