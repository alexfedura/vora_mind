import { memo, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Handle, NodeResizer, Position, type Node, type NodeProps } from "@xyflow/react";
import { Play } from "lucide-react";
import type { VoraMediaData } from "@/mind/types";

type VoraMediaNodeModel = Node<VoraMediaData, "voraMedia">;

export const VoraMediaNode = memo(({ id, data, selected }: NodeProps<VoraMediaNodeModel>) => {
  const source = data.filePath ? convertFileSrc(data.filePath) : null;
  const videoRef = useRef<HTMLVideoElement | null>(null);

  const handleVideoClick = (): void => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) {
      void video.play().catch((playError: unknown) => {
        console.error("Could not play canvas video:", playError);
      });
    } else {
      video.pause();
    }
  };

  const handleVideoDoubleClick = (event: React.MouseEvent<HTMLVideoElement>): void => {
    event.stopPropagation();
    videoRef.current?.pause();
    window.dispatchEvent(new CustomEvent("vora:open-player-with-media", {
      detail: { filePath: data.filePath, mediaType: data.type },
    }));
  };

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
      <Handle type="source" position={Position.Left} id="source-left" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-left-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Right} id="source-right" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-right-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Top} id="source-top" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-top-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Bottom} id="source-bottom" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-bottom-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="target" position={Position.Left} id="target-left" className="!w-3 !h-3 !-left-1.5 !opacity-0" />
      <Handle type="target" position={Position.Right} id="target-right" className="!w-3 !h-3 !-right-1.5 !opacity-0" />
      <Handle type="target" position={Position.Top} id="target-top" className="!w-3 !h-3 !-top-1.5 !opacity-0" />
      <Handle type="target" position={Position.Bottom} id="target-bottom" className="!w-3 !h-3 !-bottom-1.5 !opacity-0" />
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
            <div className="group relative min-h-0 flex-1">
              <video
                ref={videoRef}
                preload="metadata"
                src={source}
                onClick={handleVideoClick}
                onDoubleClick={handleVideoDoubleClick}
                className="nodrag nowheel h-full w-full object-contain"
              >
                Your browser does not support video playback.
              </video>
              <div className="pointer-events-none absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover:opacity-100">
                <span className="rounded-full bg-black/45 p-3 text-white/80">
                  <Play size={24} fill="currentColor" aria-hidden="true" />
                </span>
              </div>
            </div>
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
