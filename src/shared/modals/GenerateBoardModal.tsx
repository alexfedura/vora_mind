import React, { useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { LoaderCircle, Sparkles, X } from "lucide-react";
import { boardGenerationService } from "@/services/boardGenerationService";

interface GenerateBoardModalProps {
  isOpen: boolean;
  onClose: () => void;
  onGenerated: (graphJson: string) => void;
}

export const GenerateBoardModal: React.FC<GenerateBoardModalProps> = ({ isOpen, onClose, onGenerated }) => {
  const [prompt, setPrompt] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [streamPreview, setStreamPreview] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const generate = async (): Promise<void> => {
    if (!prompt.trim() || isGenerating) return;
    setError(null);
    setStreamPreview("");
    setIsGenerating(true);
    let unlistenChunk: (() => void) | null = null;
    let unlistenDone: (() => void) | null = null;
    try {
      if (apiKey.trim()) {
        await boardGenerationService.saveApiKey(apiKey.trim());
        setApiKey("");
      }
      let resolveGenerated: ((value: string) => void) | null = null;
      const generated = new Promise<string>((resolve) => {
        resolveGenerated = resolve;
      });
      [unlistenChunk, unlistenDone] = await Promise.all([
        listen<string>("deepseek-chunk", (event) => {
          setStreamPreview((current) => `${current}${event.payload}`.slice(-4_000));
        }),
        listen<string>("deepseek-done", (event) => {
          resolveGenerated?.(event.payload);
        }),
      ]);
      await boardGenerationService.generate(prompt.trim());
      const graphJson = await generated;
      onGenerated(graphJson);
      setPrompt("");
      onClose();
    } catch (cause) {
      console.error("Could not generate Board with DeepSeek:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      unlistenChunk?.();
      unlistenDone?.();
      setIsGenerating(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[110] flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !isGenerating) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="generate-board-title"
        className="flex max-h-[85vh] w-full max-w-xl flex-col overflow-hidden rounded-2xl border border-[#303644] bg-[#171a22] text-zinc-100 shadow-2xl"
      >
        <header className="flex items-center justify-between border-b border-[#2a2f3b] px-5 py-4">
          <div className="flex items-center gap-2.5">
            <Sparkles size={17} className="text-violet-300" />
            <h2 id="generate-board-title" className="text-sm font-semibold">Generate Board with AI</h2>
          </div>
          <button
            type="button"
            disabled={isGenerating}
            onClick={onClose}
            aria-label="Close generator"
            className="rounded-lg p-1.5 text-zinc-500 transition hover:bg-[#292e38] hover:text-white disabled:opacity-40"
          >
            <X size={16} />
          </button>
        </header>

        <div className="flex flex-col gap-4 overflow-y-auto p-5">
          <label className="flex flex-col gap-2 text-xs font-medium text-zinc-300">
            Describe the board you want
            <textarea
              autoFocus
              rows={5}
              maxLength={8_000}
              value={prompt}
              onChange={(event) => setPrompt(event.currentTarget.value)}
              disabled={isGenerating}
              placeholder="For example: Create a project launch plan with research, design, development, and review stages."
              className="resize-y rounded-xl border border-[#343a48] bg-[#101217] px-3 py-2.5 text-sm leading-relaxed text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-violet-500/70 disabled:opacity-60"
            />
          </label>

          <label className="flex flex-col gap-2 text-xs font-medium text-zinc-300">
            DeepSeek API key
            <input
              type="password"
              autoComplete="new-password"
              value={apiKey}
              onChange={(event) => setApiKey(event.currentTarget.value)}
              disabled={isGenerating}
              placeholder="Leave blank to use the saved key"
              className="rounded-xl border border-[#343a48] bg-[#101217] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-violet-500/70 disabled:opacity-60"
            />
            <span className="text-[10px] font-normal text-zinc-500">The key is saved in this workspace’s settings and sent only by the Rust backend.</span>
          </label>

          {isGenerating && (
            <div className="rounded-xl border border-violet-500/20 bg-violet-500/5 p-3">
              <p className="flex items-center gap-2 text-xs text-violet-200">
                <LoaderCircle size={14} className="animate-spin" />
                Generating and streaming the graph…
              </p>
              {streamPreview && (
                <pre className="mt-2 max-h-28 overflow-auto whitespace-pre-wrap break-words text-[10px] leading-relaxed text-zinc-500">
                  {streamPreview}
                </pre>
              )}
            </div>
          )}
          {error && <p role="alert" className="rounded-lg bg-rose-950/50 px-3 py-2 text-xs text-rose-200">{error}</p>}
        </div>

        <footer className="flex justify-end gap-2 border-t border-[#2a2f3b] px-5 py-4">
          <button
            type="button"
            disabled={isGenerating}
            onClick={onClose}
            className="rounded-lg px-3 py-2 text-xs text-zinc-400 transition hover:bg-[#292e38] hover:text-white disabled:opacity-40"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={isGenerating || !prompt.trim()}
            onClick={() => void generate()}
            className="flex items-center gap-2 rounded-lg bg-violet-600 px-4 py-2 text-xs font-semibold text-white transition hover:bg-violet-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isGenerating ? <LoaderCircle size={14} className="animate-spin" /> : <Sparkles size={14} />}
            {isGenerating ? "Generating…" : "Generate"}
          </button>
        </footer>
      </section>
    </div>
  );
};
