import React from "react";
import { Link } from "react-router-dom";
import {
  Plus,
  Search,
  Layout,
  FolderKanban,
  BookOpen,
  CircleHelp,
  ListTodo,
  Sparkles,
  GitBranch,
  Library as LibraryIcon,
  Tag,
  Highlighter,
  Sliders,
  FolderOpen,
} from "lucide-react";
import { OpenPanel } from "@/mind/types";
import { WorkspaceMenuFooter } from "@/shared/layout/WorkspaceMenuFooter";

interface LeftSidebarProps {
  activeOpenPanel: OpenPanel;
  hasActiveBoard: boolean;
  isLibraryOpen: boolean;
  isTaskFlowOpen: boolean;
  isAiTasksOpen: boolean;
  isVoraTraceOpen: boolean;
  isJournalOpen: boolean;
  isQuestionsBoardOpen: boolean;
  onAddNewCard: () => void;
  onOpenBoards: () => void;
  onOpenTaskFlow: () => void;
  onOpenAiTasks: () => void;
  onOpenVoraTrace: () => void;
  onOpenJournal: () => void;
  onOpenQuestionsBoard: () => void;
  onToggleLibrary: () => void;
}

export const LeftSidebar: React.FC<LeftSidebarProps> = ({
  activeOpenPanel,
  hasActiveBoard,
  isLibraryOpen,
  isTaskFlowOpen,
  isAiTasksOpen,
  isVoraTraceOpen,
  isJournalOpen,
  isQuestionsBoardOpen,
  onAddNewCard,
  onOpenBoards,
  onOpenTaskFlow,
  onOpenAiTasks,
  onOpenVoraTrace,
  onOpenJournal,
  onOpenQuestionsBoard,
  onToggleLibrary,
}) => {
  return (
    <aside className="flex h-full w-56 flex-shrink-0 flex-col justify-between border-r border-[#2a2a2a] bg-background p-3 select-none">
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2 px-2 pt-1 pb-2 border-b border-[#242424]">
          <div className="w-6 h-6 rounded-lg bg-gradient-to-br from-purple-500 to-indigo-600 flex items-center justify-center font-bold text-white text-xs shadow-md">
            V
          </div>
          <span className="font-bold text-sm text-gray-100 tracking-wider">VORA</span>
        </div>

        <div className="flex items-center gap-1.5">
          <button
            type="button"
            disabled={!hasActiveBoard}
            onClick={onAddNewCard}
            title={hasActiveBoard ? "New content" : "Create a Board first"}
            className="flex-1 flex items-center justify-center gap-2 py-1.5 px-3 bg-[#1e222c] hover:bg-purple-600 text-gray-200 hover:text-white rounded-xl text-xs font-semibold transition shadow-sm disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Plus size={15} />
            <span>New content</span>
          </button>
          <button className="p-2 rounded-xl bg-[#1e222c] hover:bg-[#272c38] text-gray-400 hover:text-white transition">
            <Search size={15} />
          </button>
        </div>

        <nav className="flex flex-col gap-1 text-xs font-medium text-gray-400">
          <button
            onClick={onOpenBoards}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              activeOpenPanel === "boards"
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <Layout size={16} />
            <span>My desk</span>
          </button>
          <button className="flex items-center gap-2.5 px-3 py-2 rounded-xl hover:bg-[#1b1e27] hover:text-gray-200 transition">
            <FolderKanban size={16} />
            <span>Projects</span>
          </button>
          <button
            type="button"
            onClick={onOpenTaskFlow}
            aria-current={isTaskFlowOpen ? "page" : undefined}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              isTaskFlowOpen
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <ListTodo size={16} />
            <span>Task Flow</span>
          </button>
          <button
            type="button"
            onClick={onOpenAiTasks}
            aria-current={isAiTasksOpen ? "page" : undefined}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              isAiTasksOpen
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <Sparkles size={16} />
            <span>AI Tasks</span>
          </button>
          <button
            type="button"
            onClick={onOpenVoraTrace}
            aria-current={isVoraTraceOpen ? "page" : undefined}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              isVoraTraceOpen
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <GitBranch size={16} />
            <span>Vora Trace</span>
          </button>
          <button
            type="button"
            onClick={onOpenJournal}
            aria-current={isJournalOpen ? "page" : undefined}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              isJournalOpen
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <BookOpen size={16} />
            <span>Journal</span>
          </button>
          <button
            type="button"
            onClick={onOpenQuestionsBoard}
            aria-current={isQuestionsBoardOpen ? "page" : undefined}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              isQuestionsBoardOpen
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <CircleHelp size={16} />
            <span>Inquiry Queue</span>
          </button>
          <button
            onClick={onToggleLibrary}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-xl transition ${
              isLibraryOpen
                ? "bg-[#1d212d] text-purple-400 font-semibold"
                : "hover:bg-[#1b1e27] hover:text-gray-200"
            }`}
          >
            <LibraryIcon size={16} />
            <span>Library</span>
          </button>
          <button className="flex items-center gap-2.5 px-3 py-2 rounded-xl hover:bg-[#1b1e27] hover:text-gray-200 transition">
            <Tag size={16} />
            <span>Tags</span>
          </button>
          <button className="flex items-center gap-2.5 px-3 py-2 rounded-xl hover:bg-[#1b1e27] hover:text-gray-200 transition">
            <Highlighter size={16} />
            <span>Highlights</span>
          </button>
          <button className="flex items-center gap-2.5 px-3 py-2 rounded-xl hover:bg-[#1b1e27] hover:text-gray-200 transition">
            <Sliders size={16} />
            <span>Properties</span>
          </button>
          <Link
            to="/sources"
            className="flex items-center gap-2.5 px-3 py-2 rounded-xl hover:bg-[#1b1e27] hover:text-amber-300 text-gray-400 transition"
          >
            <FolderOpen size={16} />
            <span>Sources</span>
          </Link>
        </nav>
      </div>

      <WorkspaceMenuFooter />
    </aside>
  );
};