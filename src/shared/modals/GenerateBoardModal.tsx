import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { KeyRound, LoaderCircle, Sparkles, X } from "lucide-react";

interface GenerateBoardModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const GenerateBoardModal: React.FC<GenerateBoardModalProps> = ({ isOpen, onClose }) => {
  const [prompt, setPrompt] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [streamedText, setStreamedText] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSavingKey, setIsSavingKey] = useState(false);
  const [isDone, setIsDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape" && !isGenerating) onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [isGenerating, isOpen, onClose]);

  if (!isOpen) return null;

  const saveKey = async (): Promise<boolean> => {
    if (!apiKey.trim()) {
      setError("Enter your DeepSeek API key first.");
      return false;
    }
    setIsSavingKey(true);
    setError(null);
    try {
      await invoke("save_deepseek_key", { apiKey: apiKey.trim() });
      setApiKey("");
      return true;
    } catch (cause) {
      console.error("Could not save the DeepSeek API key:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
      return false;
    } finally {
      setIsSavingKey(false);
    }
  };

  const generate = async (): Promise<void> => {
    if (!prompt.trim() || isGenerating) return;
    setIsGenerating(true);
    setIsDone(false);
    setError(null);
    setStreamedText("");
    let unlistenChunk: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    try {
      if (apiKey.trim() && !(await saveKey())) return;
      unlistenChunk = await listen<string>("deepseek-chunk", (event) => {
        setStreamedText((previous) => previous + event.payload);
      });
      unlistenDone = await listen<string>("deepseek-done", () => setIsDone(true));
      await invoke("generate_board_from_prompt", { prompt: prompt.trim() });
      setIsDone(true);
    } catch (cause) {
      console.error("Could not generate a Board with DeepSeek:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      unlistenChunk?.();
      unlistenDone?.();
      setIsGenerating(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="generate-board-title"
        className="w-full max-w-xl overflow-hidden rounded-2xl border border-[#303648] bg-[#171a22] text-zinc-100 shadow-2xl"
      >
        <header className="flex items-center justify-between border-b border-[#292e3a] px-5 py-4">
          <h2 id="generate-board-title" className="flex items-center gap-2 text-sm font-semibold">
            <Sparkles size={16} className="text-violet-300" />
            Generate Board with DeepSeek
          </h2>
          <button
            type="button"
            disabled={isGenerating}
            onClick={onClose}
            aria-label="Close Board generation"
            className="rounded-md p-1.5 text-zinc-500 hover:bg-[#292e39] hover:text-white disabled:opacity-40"
          >
            <X size={16} />
          </button>
        </header>
        <div className="space-y-4 p-5">
          <label className="block space-y-1.5 text-xs font-medium text-zinc-300">
            Board prompt
            <textarea
              autoFocus
              rows={5}
              maxLength={20_000}
              value={prompt}
              disabled={isGenerating}
              onChange={(event) => setPrompt(event.currentTarget.value)}
              placeholder="Describe the ideas, structure, and connections you want on your Board..."
              className="w-full resize-y rounded-xl border border-[#303648] bg-[#101217] px-3 py-2.5 text-sm leading-relaxed text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-violet-500/70 disabled:opacity-60"
            />
          </label>
          <details className="rounded-xl border border-[#2b303c] bg-[#13161d] p-3">
            <summary className="flex cursor-pointer list-none items-center gap-2 text-xs font-medium text-zinc-300">
              <KeyRound size={14} className="text-amber-300" />
              Set or replace DeepSeek API key
            </summary>
            <div className="mt-3 space-y-2">
              <input
                type="password"
                autoComplete="new-password"
                value={apiKey}
                disabled={isGenerating || isSavingKey}
                onChange={(event) => setApiKey(event.currentTarget.value)}
                placeholder="DeepSeek API key"
                className="w-full rounded-lg border border-[#303648] bg-[#101217] px-3 py-2 text-xs text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-violet-500/70"
              />
              <p className="text-[10px] leading-relaxed text-zinc-500">
                Stored with this workspace's local app settings; it is not shown again or sent to the frontend after saving.
              </p>
              <button
                type="button"
                disabled={isSavingKey || isGenerating || !apiKey.trim()}
                onClick={() => void saveKey()}
                className="rounded-lg border border-[#3b4050] px-3 py-1.5 text-[11px] font-medium text-zinc-200 hover:bg-[#252a35] disabled:opacity-40"
              >
                {isSavingKey ? "Saving key..." : "Save API key"}
              </button>
            </div>
          </details>
          {isGenerating && (
            <div className="rounded-xl border border-violet-500/20 bg-violet-950/15 p-3">
              <p className="flex items-center gap-2 text-xs text-violet-200">
                <LoaderCircle size={14} className="animate-spin" />
                {isDone ? "Board generated." : "Generating and streaming response..."}
              </p>
              {streamedText && (
                <pre className="mt-2 max-h-32 overflow-auto whitespace-pre-wrap break-words text-[10px] leading-relaxed text-zinc-400">
                  {streamedText}
                </pre>
              )}
            </div>
          )}
          {error && <p role="alert" className="rounded-lg bg-rose-950/50 px-3 py-2 text-xs text-rose-200">{error}</p>}
        </div>
        <footer className="flex justify-end gap-2 border-t border-[#292e3a] bg-[#13161d] px-5 py-3">
          <button
            type="button"
            disabled={isGenerating}
            onClick={onClose}
            className="rounded-lg px-3 py-2 text-xs text-zinc-400 hover:bg-[#252a35] hover:text-zinc-100 disabled:opacity-40"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={isGenerating || !prompt.trim()}
            onClick={() => void generate()}
            className="flex items-center gap-2 rounded-lg bg-violet-600 px-4 py-2 text-xs font-semibold text-white hover:bg-violet-500 disabled:cursor-not-allowed disabled:opacity-45"
          >
            {isGenerating ? <LoaderCircle size={14} className="animate-spin" /> : <Sparkles size={14} />}
            {isGenerating ? "Generating..." : "Generate Board"}
          </button>
        </footer>
      </section>
    </div>
  );
};
