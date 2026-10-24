import React, { useEffect, useRef, useState } from "react";
import { ArrowLeft, X } from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";

const isRtlLine = (line: string): boolean => {
  const trimmed = line.trim();
  const hasInvert = trimmed.startsWith("!");
  const clean = hasInvert ? trimmed.slice(1).trim() : trimmed;
  const rtlRegex = /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB50-\uFDFF\uFE70-\uFEFF]/;
  const naturalRtl = rtlRegex.test(clean.charAt(0) || "");
  return hasInvert ? !naturalRtl : naturalRtl;
};

interface FullscreenDocModalProps {
  cardId: string | null;
  onClose: () => void;
}

export const FullscreenDocModal: React.FC<FullscreenDocModalProps> = ({ cardId, onClose }) => {
  const { cards, updateCardContent, updateCardVersion } = useAppStore();
  const card = cardId ? cards[cardId] : null;

  const [isEditingContent, setIsEditingContent] = useState(false);
  const contentRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  useEffect(() => {
    if (isEditingContent && contentRef.current) {
      contentRef.current.focus();
    }
  }, [isEditingContent]);

  if (!card) return null;

  const isTitleRtl = isRtlLine(card.title || "");
  const isContentRtl = isRtlLine(card.content || "");

  return (
    <div className="fixed inset-0 z-50 bg-[#111317] flex flex-col select-text animate-in fade-in duration-150">
      <header className="h-12 px-6 flex items-center justify-between border-b border-[#20242f]/80 select-none bg-[#0e1014]">
        <div className="flex items-center gap-3">
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#1f232d] transition flex items-center gap-1.5 text-xs"
            title="Back to Canvas (Esc)"
          >
            <ArrowLeft size={16} />
            <span>Back to Canvas</span>
          </button>

          <div className="h-4 w-[1px] bg-[#252937]" />

          <input
            type="text"
            value={card.version || "v0.1.0"}
            onChange={(e) => updateCardVersion(card.id, e.target.value)}
            className="w-16 text-center text-xs font-mono text-gray-300 bg-[#1c202a] focus:bg-[#262b38] focus:text-white focus:outline-none rounded px-1.5 py-0.5 transition"
            title="Card Version"
          />
        </div>

        <button
          onClick={onClose}
          className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#1f232d] transition"
          title="Close (Esc)"
        >
          <X size={17} />
        </button>
      </header>

      <main className="flex-1 overflow-y-auto no-scrollbar py-12 px-6">
        <div className="max-w-3xl mx-auto flex flex-col gap-6">
          <textarea
            rows={1}
            value={card.title}
            onChange={(e) => {
              updateCardContent(card.id, e.target.value, card.content);
            }}
            placeholder={isTitleRtl ? "عنوان یادداشت..." : "Note title..."}
            className={`w-full bg-transparent text-3xl font-extrabold text-white placeholder-gray-600 focus:outline-none leading-snug resize-none pb-2 border-b border-[#222734] ${
              isTitleRtl ? "text-right" : "text-left"
            }`}
            dir={isTitleRtl ? "rtl" : "ltr"}
          />

          {isEditingContent ? (
            <textarea
              ref={contentRef}
              rows={24}
              value={card.content}
              onBlur={() => setIsEditingContent(false)}
              onChange={(e) => {
                updateCardContent(card.id, card.title, e.target.value);
              }}
              placeholder={isContentRtl ? "شروع به تایپ آزاد کنید..." : "Start typing note..."}
              className={`w-full bg-transparent text-base text-gray-200 placeholder-gray-600 focus:outline-none leading-relaxed resize-none font-sans ${
                isContentRtl ? "text-right" : "text-left"
              }`}
              dir={isContentRtl ? "rtl" : "ltr"}
            />
          ) : (
            <div
              onClick={() => setIsEditingContent(true)}
              className="min-h-[400px] flex flex-col gap-2 cursor-text"
            >
              {card.content ? (
                card.content.split("\n").map((line: string, idx: number) => {
                  const isRtl = isRtlLine(line);
                  const displayLine = line.trim().startsWith("!") ? line.replace(/^!\s?/, "") : line;
                  return (
                    <div
                      key={idx}
                      className={`text-base text-gray-200 leading-relaxed ${
                        isRtl ? "text-right" : "text-left"
                      }`}
                      dir={isRtl ? "rtl" : "ltr"}
                    >
                      {displayLine || "\u00A0"}
                    </div>
                  );
                })
              ) : (
                <span className="text-gray-600 text-base">Click to start typing notes...</span>
              )}
            </div>
          )}
        </div>
      </main>
    </div>
  );
};