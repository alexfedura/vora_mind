import React, { useState } from "react";
import { Settings } from "lucide-react";
import { SettingsModal } from "@/shared/modals/SettingsModal";
import { UpdateCheckButton } from "@/shared/components/UpdateCheckButton";

export const WorkspaceMenuFooter: React.FC = () => {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  return (
    <>
      <div className="flex items-center gap-1 border-t border-[#1d212b] pt-1">
        <button
          type="button"
          onClick={() => setIsSettingsOpen(true)}
          className="flex flex-1 items-center gap-2.5 rounded-xl px-3 py-2.5 text-xs font-medium text-gray-400 transition hover:bg-[#1b1e27] hover:text-white"
        >
          <Settings size={16} />
          <span>Settings</span>
        </button>
        <UpdateCheckButton className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-gray-400 transition hover:bg-[#1b1e27] hover:text-white disabled:cursor-wait" />
      </div>
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
      />
    </>
  );
};
