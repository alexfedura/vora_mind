import React, { memo, useState, useRef, useLayoutEffect, useEffect } from "react";
import { NodeProps, Handle, Position, useViewport } from "@xyflow/react";
import {
  Type,
  Bold,
  Italic,
  Underline,
  AlignLeft,
  AlignCenter,
  AlignRight,
  List,
  Link,
  Trash2,
  ChevronDown,
} from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import { useCanvasZoomScales } from "./CanvasZoomContext";

export const VoraTextNode = memo(({ id, selected, data }: NodeProps) => {
  const { textDirection } = useAppStore();
  const { zoom } = useViewport();
  const canvasZoom = useCanvasZoomScales();
  const defaultRtl = textDirection === "rtl";

  const [text, setText] = useState((data?.text as string) || "");
  const [fontSize, setFontSize] = useState<number>((data?.fontSize as number) || 16);
  const [isBold, setIsBold] = useState<boolean>((data?.isBold as boolean) || false);
  const [isItalic, setIsItalic] = useState<boolean>((data?.isItalic as boolean) || false);
  const [isUnderline, setIsUnderline] = useState<boolean>((data?.isUnderline as boolean) || false);
  const [alignment, setAlignment] = useState<"left" | "center" | "right">(
    (data?.alignment as "left" | "center" | "right") || (defaultRtl ? "right" : "left")
  );
  const [color, setColor] = useState<string>((data?.color as string) || "#d6dbe2");

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const mirrorRef = useRef<HTMLDivElement>(null);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const [toolbarPosition, setToolbarPosition] = useState<{ left: number; top: number } | null>(null);

  useEffect(() => {
    if (!selected) return;
    const centerToolbarAtPointer = (event: Event): void => {
      if (!(event instanceof CustomEvent)) return;
      const detail: unknown = event.detail;
      if (typeof detail !== "object" || detail === null || !("x" in detail) || !("y" in detail)) return;
      if (typeof detail.x !== "number" || !Number.isFinite(detail.x) ||
        typeof detail.y !== "number" || !Number.isFinite(detail.y)) return;
      const toolbar = toolbarRef.current;
      const nodeElement = toolbar?.closest(".react-flow__node");
      if (!toolbar || !nodeElement) return;
      const nodeBounds = nodeElement.getBoundingClientRect();
      const scaleX = zoom * canvasZoom.x;
      const scaleY = zoom * canvasZoom.y;
      setToolbarPosition({
        left: (detail.x - nodeBounds.left - toolbar.offsetWidth / 2) / scaleX,
        top: (detail.y - nodeBounds.top - toolbar.offsetHeight / 2) / scaleY,
      });
    };
    window.addEventListener("vora:center-floating-toolbar", centerToolbarAtPointer);
    return () => window.removeEventListener("vora:center-floating-toolbar", centerToolbarAtPointer);
  }, [canvasZoom.x, canvasZoom.y, selected, zoom]);

  // فوکوس خودکار هنگام ساخت نود جدید
  useEffect(() => {
    if (data?.autoFocus && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [data?.autoFocus]);

  // تنظیم پویا و بی‌نهایت عرض و ارتفاع بر اساس طول متن و تعداد خطوط (شکستن خط فقط با Enter)
  useLayoutEffect(() => {
    if (textareaRef.current && mirrorRef.current) {
      const mirrorWidth = mirrorRef.current.offsetWidth;
      textareaRef.current.style.width = `${Math.max(140, mirrorWidth + 24)}px`;
      textareaRef.current.style.height = "0px";
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
    }
  }, [text, fontSize, isBold]);

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.dispatchEvent(new CustomEvent("vora:remove-from-canvas", { detail: { nodeId: id } }));
  };

  const isRtl = /[\u0600-\u06FF\u0750-\u077F]/.test(text.trim().charAt(0)) || defaultRtl;

  return (
    <div className="relative group/text will-change-transform">
      <Handle
        type="source"
        position={Position.Left}
        id="source-left"
        className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#13151b] !-left-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="source-right"
        className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#13151b] !-right-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`}
      />
      <Handle type="source" position={Position.Top} id="source-top" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#13151b] !-top-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Bottom} id="source-bottom" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#13151b] !-bottom-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="target" position={Position.Left} id="target-left" className="!w-3 !h-3 !-left-1.5 !opacity-0" />
      <Handle type="target" position={Position.Right} id="target-right" className="!w-3 !h-3 !-right-1.5 !opacity-0" />
      <Handle type="target" position={Position.Top} id="target-top" className="!w-3 !h-3 !-top-1.5 !opacity-0" />
      <Handle type="target" position={Position.Bottom} id="target-bottom" className="!w-3 !h-3 !-bottom-1.5 !opacity-0" />

      {/* منوی ابزار شناور متنی Miro */}
      {selected && (
        <div
          ref={toolbarRef}
          className={`vora-text-toolbar absolute flex items-center gap-1 px-2 py-1.5 rounded-xl bg-[#181a22] border border-[#2b303e] shadow-2xl shadow-black/80 z-50 select-none animate-in fade-in zoom-in-95 duration-100 whitespace-nowrap ${toolbarPosition ? "" : "left-1/2 -translate-x-1/2"}`}
          style={{
            left: toolbarPosition?.left,
            top: toolbarPosition?.top ?? -52 / (zoom * canvasZoom.y),
            scale: `${1 / (zoom * canvasZoom.x)} ${1 / (zoom * canvasZoom.y)}`,
            transformOrigin: toolbarPosition ? "top left" : "top center",
          }}
        >
          <button className="flex items-center gap-1.5 px-2 py-1 rounded-lg text-xs font-semibold text-gray-200 hover:bg-[#252936] transition">
            <Type size={14} className="text-purple-400" />
            <span>Text</span>
            <ChevronDown size={12} className="text-gray-400" />
          </button>

          <div className="h-4 w-[1px] bg-[#292e3c] mx-0.5" />

          {/* اندازه فونت */}
          <div className="flex items-center">
            <button
              onClick={() => setFontSize((s) => Math.max(12, s - 2))}
              className="w-5 h-6 flex items-center justify-center text-xs text-gray-400 hover:text-white hover:bg-[#252936] rounded transition"
            >
              -
            </button>
            <span className="w-6 text-center text-xs font-mono font-medium text-gray-200">
              {fontSize}
            </span>
            <button
              onClick={() => setFontSize((s) => Math.min(64, s + 2))}
              className="w-5 h-6 flex items-center justify-center text-xs text-gray-400 hover:text-white hover:bg-[#252936] rounded transition"
            >
              +
            </button>
          </div>

          <div className="h-4 w-[1px] bg-[#292e3c] mx-0.5" />

          <button
            onClick={() => setIsBold(!isBold)}
            className={`p-1.5 rounded-lg transition ${
              isBold ? "bg-purple-600/30 text-purple-400" : "text-gray-400 hover:text-white hover:bg-[#252936]"
            }`}
            title="Bold"
          >
            <Bold size={14} />
          </button>

          <button
            onClick={() => setIsItalic(!isItalic)}
            className={`p-1.5 rounded-lg transition ${
              isItalic ? "bg-purple-600/30 text-purple-400" : "text-gray-400 hover:text-white hover:bg-[#252936]"
            }`}
            title="Italic"
          >
            <Italic size={14} />
          </button>

          <button
            onClick={() => setIsUnderline(!isUnderline)}
            className={`p-1.5 rounded-lg transition ${
              isUnderline ? "bg-purple-600/30 text-purple-400" : "text-gray-400 hover:text-white hover:bg-[#252936]"
            }`}
            title="Underline"
          >
            <Underline size={14} />
          </button>

          <div className="h-4 w-[1px] bg-[#292e3c] mx-0.5" />

          <button
            onClick={() =>
              setAlignment((a) => (a === "left" ? "center" : a === "center" ? "right" : "left"))
            }
            className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#252936] transition"
            title="Alignment"
          >
            {alignment === "left" ? (
              <AlignLeft size={14} />
            ) : alignment === "center" ? (
              <AlignCenter size={14} />
            ) : (
              <AlignRight size={14} />
            )}
          </button>

          <button
            className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#252836] transition"
            title="Bullet list"
          >
            <List size={14} />
          </button>

          <button
            className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#252836] transition"
            title="Insert Link"
          >
            <Link size={14} />
          </button>

          {/* پالت رنگ */}
          <div className="flex items-center gap-1 px-1">
            {["#d6dbe2", "#e8ad55", "#7f8997", "#30b58a", "#e36b74"].map((c) => (
              <button
                key={c}
                onClick={() => setColor(c)}
                style={{ backgroundColor: c }}
                className={`w-3.5 h-3.5 rounded-full transition-transform ${
                  color === c ? "scale-125 ring-2 ring-purple-400" : "hover:scale-110"
                }`}
              />
            ))}
          </div>

          <div className="h-4 w-[1px] bg-[#292e3c] mx-0.5" />

          <button
            onClick={handleDelete}
            className="p-1.5 rounded-lg text-gray-400 hover:text-rose-400 hover:bg-rose-500/20 transition"
            title="Delete Text"
          >
            <Trash2 size={14} />
          </button>
        </div>
      )}

      {/* المنت محاسباتی نامرئی برای سنجش دقیق عرض متن */}
      <div
        ref={mirrorRef}
        aria-hidden="true"
        style={{
          fontSize: `${fontSize}px`,
          fontWeight: isBold ? "bold" : "normal",
          fontStyle: isItalic ? "italic" : "normal",
        }}
        className="absolute left-0 top-0 opacity-0 pointer-events-none whitespace-pre select-none font-sans px-2"
      >
        {text || "Type something..."}
      </div>

      {/* کانتینر متن بدون محدودیت عرض */}
      <div
        className={`p-1 rounded-xl transition-all border ${
          selected
            ? "border-purple-500/70 ring-2 ring-purple-500/10 bg-[#161820]/30"
            : "border-transparent"
        }`}
      >
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Type something..."
          rows={1}
          style={{
            fontSize: `${fontSize}px`,
            fontWeight: isBold ? "bold" : "normal",
            fontStyle: isItalic ? "italic" : "normal",
            textDecoration: isUnderline ? "underline" : "none",
            textAlign: alignment,
            color: color,
            whiteSpace: "pre",
          }}
          className={`nodrag bg-transparent resize-none overflow-hidden focus:outline-none leading-relaxed placeholder-gray-600 font-sans block ${
            isRtl ? "text-right" : "text-left"
          }`}
          dir={isRtl ? "rtl" : "ltr"}
        />
      </div>
    </div>
  );
});

VoraTextNode.displayName = "VoraTextNode";
export default VoraTextNode;
