import React, { useState } from "react";
import { Layers, X } from "lucide-react";

interface CreateItemGroupPanelProps {
  selectedCount: number;
  onCreate: (name: string, version: string) => Promise<void>;
  onClose: () => void;
}

export const CreateItemGroupPanel: React.FC<CreateItemGroupPanelProps> = ({
  selectedCount,
  onCreate,
  onClose,
}) => {
  const [name, setName] = useState("");
  const [version, setVersion] = useState("0.1.0");
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const submit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (isSaving) return;
    setError(null);
    setIsSaving(true);
    try {
      await onCreate(name.trim(), version.trim());
      onClose();
    } catch (cause) {
      console.error("Could not save item group:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <form
      onSubmit={(event) => void submit(event)}
      className="absolute left-1/2 top-3 z-50 w-80 -translate-x-1/2 rounded-xl border border-[#303543] bg-[#171a22]/95 p-3 text-zinc-200 shadow-2xl backdrop-blur"
      aria-label="Create item group"
    >
      <div className="mb-3 flex items-center gap-2">
        <Layers size={16} className="text-violet-300" />
        <span className="flex-1 text-xs font-semibold">Add selection to Library</span>
        <span className="text-[10px] text-zinc-500">
          {selectedCount} {selectedCount === 1 ? "item" : "items"}
        </span>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close group form"
          className="rounded p-1 text-zinc-400 hover:bg-[#282d3a] hover:text-white"
        >
          <X size={14} />
        </button>
      </div>
      <p className="mb-2 text-[10px] text-zinc-500">Open with Ctrl+G or Ctrl+ل</p>
      <label className="mb-2 block text-[10px] text-zinc-400">
        Group name
        <input
          autoFocus
          required
          maxLength={80}
          value={name}
          onChange={(event) => setName(event.currentTarget.value)}
          className="mt-1 w-full rounded-md border border-[#303644] bg-[#101218] px-2.5 py-2 text-xs text-white outline-none focus:border-violet-500"
          placeholder="Name this group"
        />
      </label>
      <label className="mb-3 block text-[10px] text-zinc-400">
        Version
        <input
          required
          value={version}
          onChange={(event) => setVersion(event.currentTarget.value)}
          pattern="(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
          title="Use a three-part version such as 0.1.0"
          className="mt-1 w-full rounded-md border border-[#303644] bg-[#101218] px-2.5 py-2 font-mono text-xs text-white outline-none focus:border-violet-500"
        />
      </label>
      {error && <p role="alert" className="mb-2 text-[10px] text-rose-300">{error}</p>}
      <button
        type="submit"
        disabled={!name.trim() || !version.trim() || isSaving}
        className="w-full rounded-md bg-violet-600 px-3 py-2 text-xs font-semibold text-white transition hover:bg-violet-500 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {isSaving ? "Saving…" : "Save group to Library"}
      </button>
    </form>
  );
};
