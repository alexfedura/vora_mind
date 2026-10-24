import React, { useState } from "react";
import { X, Check, Settings, Bot, Key, Server, Cpu } from "lucide-react";
import { forgeLLMService } from "@/forge/services/llmService";

interface LLMSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const LLMSettingsModal: React.FC<LLMSettingsModalProps> = ({
  isOpen,
  onClose,
}) => {
  const currentConfig = forgeLLMService.getConfig();
  const [provider, setProvider] = useState<"ollama" | "openai" | "custom">(
    currentConfig.provider
  );
  const [baseUrl, setBaseUrl] = useState(currentConfig.baseUrl || "");
  const [apiKey, setApiKey] = useState(currentConfig.apiKey || "");
  const [model, setModel] = useState(currentConfig.model || "");

  if (!isOpen) return null;

  const handleSave = () => {
    forgeLLMService.updateConfig({
      provider,
      baseUrl: baseUrl.trim() || undefined,
      apiKey: apiKey.trim() || undefined,
      model: model.trim(),
    });
    onClose();
  };

  const getProviderLabel = (p: "ollama" | "openai" | "custom") => {
    switch (p) {
      case "ollama":
        return "Ollama";
      case "openai":
        return "OpenAI";
      case "custom":
        return "Custom";
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm animate-in fade-in duration-150">
      <div className="w-[30rem] bg-[#0e1015] border border-[#1f232e] rounded-2xl shadow-2xl flex flex-col overflow-hidden text-zinc-200">
        {/* Header */}
        <div className="h-13 px-5 py-3.5 flex items-center justify-between border-b border-[#1b1f2b] bg-[#0b0c10]">
          <div className="flex items-center gap-2.5">
            <Settings size={16} className="text-violet-400" />
            <span className="text-sm font-semibold text-zinc-100">
              Forge LLM Engine Settings
            </span>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-[#181b24] transition"
          >
            <X size={16} />
          </button>
        </div>

        {/* Form Body */}
        <div className="p-5 flex flex-col gap-3.5 text-xs font-sans">
          {/* Provider Selection */}
          <div className="flex flex-col gap-1.5">
            <label className="font-medium text-zinc-300 flex items-center gap-1.5">
              <Bot size={13.5} className="text-cyan-400" />
              <span>AI Provider</span>
            </label>
            <div className="grid grid-cols-3 gap-2">
              {(["ollama", "openai", "custom"] as const).map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => {
                    setProvider(p);
                    if (p === "ollama" && !baseUrl) {
                      setBaseUrl("http://localhost:11434");
                      setModel("qwen2.5-coder:7b");
                    } else if (p === "openai" && !model) {
                      setModel("gpt-4o-mini");
                    }
                  }}
                  className={`py-1.5 rounded-lg font-medium transition border text-xs ${
                    provider === p
                      ? "bg-violet-600/15 text-violet-300 border-violet-500/40 shadow-sm"
                      : "bg-[#131620] text-zinc-400 border-[#202534] hover:bg-[#1a1e2b] hover:text-zinc-200"
                  }`}
                >
                  {getProviderLabel(p)}
                </button>
              ))}
            </div>
          </div>

          {/* Base URL */}
          <div className="flex flex-col gap-1.5">
            <label className="font-medium text-zinc-300 flex items-center gap-1.5">
              <Server size={13.5} className="text-violet-400" />
              <span>Base URL</span>
            </label>
            <input
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder={
                provider === "ollama"
                  ? "http://localhost:11434"
                  : "https://api.openai.com/v1"
              }
              className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg px-3 py-1.5 font-mono text-zinc-200 focus:outline-none transition"
            />
          </div>

          {/* Model Name */}
          <div className="flex flex-col gap-1.5">
            <label className="font-medium text-zinc-300 flex items-center gap-1.5">
              <Cpu size={13.5} className="text-indigo-400" />
              <span>Model Identifier</span>
            </label>
            <input
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="e.g. qwen2.5-coder:7b, gpt-4o-mini, claude-3-5-sonnet"
              className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg px-3 py-1.5 font-mono text-zinc-200 focus:outline-none transition"
            />
          </div>

          {/* API Key (for cloud providers) */}
          {provider !== "ollama" && (
            <div className="flex flex-col gap-1.5 animate-in fade-in">
              <label className="font-medium text-zinc-300 flex items-center gap-1.5">
                <Key size={13.5} className="text-amber-400" />
                <span>API Key</span>
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-..."
                className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg px-3 py-1.5 font-mono text-zinc-200 focus:outline-none transition"
              />
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="h-13 px-5 py-3 bg-[#0b0c10] border-t border-[#1b1f2b] flex items-center justify-between">
          <button
            onClick={onClose}
            className="px-3.5 py-1.5 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-[#181b24] transition"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="px-3.5 py-1.5 bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-md shadow-violet-950/40"
          >
            <Check size={13.5} />
            <span>Save Configuration</span>
          </button>
        </div>
      </div>
    </div>
  );
};