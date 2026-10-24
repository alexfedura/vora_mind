import { FolderOpen, Video, Mic, Book, FileText, Globe, Image as ImageIcon } from "lucide-react";

export function formatDuration(sec: number | null): string {
  if (sec === null || sec === undefined) return "--:--";
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function getSourceIcon(kind: string, size = 16) {
  switch (kind) {
    case "video":
      return <Video size={size} className="text-amber-400" />;
    case "audio":
      return <Mic size={size} className="text-orange-400" />;
    case "book":
      return <Book size={size} className="text-emerald-400" />;
    case "doc":
      return <FileText size={size} className="text-blue-400" />;
    case "url":
      return <Globe size={size} className="text-cyan-400" />;
    case "image":
      return <ImageIcon size={size} className="text-purple-400" />;
    default:
      return <FolderOpen size={size} className="text-gray-400" />;
  }
}
