import { create } from "zustand";
import { persist } from "zustand/middleware";
import { setSetting } from "@/shared/store/useSettingsStore";
import { VoraCard } from "@/mind/types";
import { createTextChangeSet, dbService } from "@/mind/services/dbService";
import type { TaskFlowItem } from "@/types";
import {
  CanonicalSymbolRecord,
  LineageHistoryEntry,
  ImpactAnalysisReport,
  ExecutionPlan,
  forgeService,
  normalizeId,
} from "@/forge/services/forgeService";

export interface ExecutedTaskRecord {
  id: string;
  taskId?: string;
  timestamp: number;
  intent: string;
  targetSymbol: string;
  targetFile: string;
  canonicalEntityId?: string;
  snapshotId?: string | null;
  gitCommitHash?: string | null;
  success: boolean;
  patchedFilesCount: number;
  healingAttempts?: number;
}

interface AppState {
  // --- Mind & Canvas Existing State ---
  cards: Record<string, VoraCard>;
  isLibraryBoardEditing: boolean;
  libraryBoardReturnCards: Record<string, VoraCard> | null;
  setLibraryBoardEditing: (editing: boolean, returnCards?: Record<string, VoraCard> | null) => void;
  activeBoardId: string;
  setActiveBoardId: (boardId: string) => void;
  isRightDrawerOpen: boolean;
  isForgeAgentOpen: boolean;
  textDirection: "rtl" | "ltr";
  setTextDirection: (direction: "rtl" | "ltr") => void;
  toggleTextDirection: () => void;
  setCards: (cards: Record<string, VoraCard>) => void;
  addCard: (title?: string, content?: string) => string;
  updateCardContent: (id: string, title: string, content: string) => void;
  updateCardVersion: (id: string, version: string) => void;
  moveToBin: (id: string) => void;
  restoreFromBin: (id: string) => void;
  toggleRightDrawer: () => void;
  toggleForgeAgent: () => void;

  // --- Priority 10 & 11: Semantic Workspace State ---
  canonicalEntities: CanonicalSymbolRecord[];
  activeCanonicalEntity: CanonicalSymbolRecord | null;
  activeLineageHistory: LineageHistoryEntry[];
  activeImpactReport: ImpactAnalysisReport | null;
  activeExecutionPlan: ExecutionPlan | null;
  isIndexingWorkspace: boolean;

  // --- Priority 12 & A14: Execution History & Comprehensive Audit Trail ---
  executedTasks: ExecutedTaskRecord[];
  recordExecutedTask: (task: Omit<ExecutedTaskRecord, "id" | "timestamp">) => void;
  clearExecutedTasks: () => void;

  taskFlowItems: TaskFlowItem[];
  loadTaskFlowItems: () => Promise<void>;
  createTaskFlowItem: (title: string) => Promise<void>;
  setTaskFlowItemCompleted: (id: string, completed: boolean) => Promise<void>;
  reorderTaskFlowItems: (ids: string[]) => Promise<void>;
  deleteTaskFlowItem: (id: string) => Promise<void>;

  fetchCanonicalEntities: () => Promise<void>;
  selectCanonicalEntity: (entity: CanonicalSymbolRecord | null) => Promise<void>;
  inspectImpactForSymbol: (symbolId: string) => Promise<void>;
  setActiveExecutionPlan: (plan: ExecutionPlan | null) => void;
  setIsIndexingWorkspace: (indexing: boolean) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // --- Mind Defaults ---
      cards: {},
      isLibraryBoardEditing: false,
      libraryBoardReturnCards: null,
      activeBoardId: "",
      setActiveBoardId: (activeBoardId) => {
        set({ activeBoardId });
        setSetting("vora.canvas.active_board_id", activeBoardId);
      },
      isRightDrawerOpen: false,
      isForgeAgentOpen: false,
      textDirection: "rtl",

      setTextDirection: (direction) => {
        set({ textDirection: direction });
        setSetting("vora.ui.text_direction", direction);
      },
      toggleTextDirection: () =>
        get().setTextDirection(get().textDirection === "rtl" ? "ltr" : "rtl"),

      setCards: (cards) => set({ cards }),
      setLibraryBoardEditing: (editing, returnCards = null) =>
        set({ isLibraryBoardEditing: editing, libraryBoardReturnCards: editing ? returnCards : null }),

      addCard: (title = "New card", content = "") => {
        const boardId = get().activeBoardId;
        const id = `card-${boardId}-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
        const newCard: VoraCard = {
          id,
          title,
          content,
          boardId,
          version: "v0.1.0",
          created_at: Date.now(),
        };
        set((state) => ({
          cards: { ...state.cards, [id]: newCard },
        }));
        if (!get().isLibraryBoardEditing) {
          void dbService
            .saveCardDirect(boardId, id, title, content, newCard.version ?? "v0.1.0")
            .catch((error: unknown) => console.error("Failed to save new card:", error));
        }
        return id;
      },

      updateCardContent: (id, title, content) =>
        set((state) => {
          const card = state.cards[id];
          if (!card) return state;
          const updatedCard = { ...card, title, content };
          const textChangeSets = [
            createTextChangeSet("title", card.title, title),
            createTextChangeSet("content", card.content, content),
          ].filter((changeSet) => changeSet !== null);
          if (!state.isLibraryBoardEditing) {
            void dbService
              .saveCardDirect(
                card.boardId ?? state.activeBoardId,
                id,
                title,
                content,
                card.version ?? "v0.1.0",
                textChangeSets,
              )
              .then(() => {
                window.dispatchEvent(new CustomEvent("vora:text-links-updated", {
                  detail: { cardId: id },
                }));
              })
              .catch((error: unknown) => console.error("Failed to save card content:", error));
          }
          return { cards: { ...state.cards, [id]: updatedCard } };
        }),

      updateCardVersion: (id, version) =>
        set((state) => {
          const card = state.cards[id];
          if (!card) return state;
          const updatedCard = { ...card, version };
          if (!state.isLibraryBoardEditing) {
            void dbService
              .saveCardDirect(card.boardId ?? state.activeBoardId, id, card.title, card.content, version)
              .catch((error: unknown) => console.error("Failed to save card version:", error));
          }
          return { cards: { ...state.cards, [id]: updatedCard } };
        }),

      moveToBin: (id) =>
        set((state) => {
          const card = state.cards[id];
          if (!card) return state;
          if (!state.isLibraryBoardEditing) {
            void dbService
              .moveToBin(card.boardId ?? state.activeBoardId, id)
              .catch((error: unknown) => console.error("Failed to move card to trash:", error));
          }
          return { cards: { ...state.cards, [id]: { ...card, deleted_at: Date.now() } } };
        }),

      restoreFromBin: (id) =>
        set((state) => {
          const card = state.cards[id];
          if (!card) return state;
          if (!state.isLibraryBoardEditing) {
            void dbService
              .restoreFromBin(card.boardId ?? state.activeBoardId, id)
              .catch((error: unknown) => console.error("Failed to restore card:", error));
          }
          return { cards: { ...state.cards, [id]: { ...card, deleted_at: undefined } } };
        }),

      toggleRightDrawer: () =>
        set((state) => ({ isRightDrawerOpen: !state.isRightDrawerOpen })),

      toggleForgeAgent: () =>
        set((state) => ({ isForgeAgentOpen: !state.isForgeAgentOpen })),

      // --- Priority 10 & 11: Semantic Workspace Defaults & Actions ---
      canonicalEntities: [],
      activeCanonicalEntity: null,
      activeLineageHistory: [],
      activeImpactReport: null,
      activeExecutionPlan: null,
      isIndexingWorkspace: false,

      // --- Priority 12 & A14: Execution History Defaults & Actions ---
      executedTasks: [],

      recordExecutedTask: (task) =>
        set((state) => ({
          executedTasks: [
            {
              ...task,
              id: `task_${Date.now()}`,
              timestamp: Date.now(),
            },
            ...state.executedTasks,
          ],
        })),

      clearExecutedTasks: () => set({ executedTasks: [] }),

      taskFlowItems: [],
      loadTaskFlowItems: async () => {
        const taskFlowItems = await dbService.getTaskFlowItems();
        set({ taskFlowItems });
      },
      createTaskFlowItem: async (title) => {
        const task = await dbService.createTaskFlowItem(title);
        set((state) => ({
          taskFlowItems: [...state.taskFlowItems, task].sort((left, right) =>
            left.position - right.position
          ),
        }));
      },
      setTaskFlowItemCompleted: async (id, completed) => {
        const task = await dbService.setTaskFlowItemCompleted(id, completed);
        set((state) => ({
          taskFlowItems: state.taskFlowItems.map((item) => item.id === id ? task : item),
        }));
      },
      reorderTaskFlowItems: async (ids) => {
        const taskFlowItems = await dbService.reorderTaskFlowItems(ids);
        set({ taskFlowItems });
      },
      deleteTaskFlowItem: async (id) => {
        await dbService.deleteTaskFlowItem(id);
        set((state) => ({
          taskFlowItems: state.taskFlowItems.filter((item) => item.id !== id),
        }));
      },

      setIsIndexingWorkspace: (isIndexingWorkspace) => set({ isIndexingWorkspace }),

      fetchCanonicalEntities: async () => {
        try {
          const entities = await forgeService.getCanonicalEntities();
          set({ canonicalEntities: entities });
        } catch (error) {
          console.error("Failed to fetch canonical entities:", error);
        }
      },

      selectCanonicalEntity: async (entity) => {
        set({
          activeCanonicalEntity: entity,
          activeLineageHistory: [],
          activeImpactReport: null,
        });

        if (!entity) return;

        const lineageIdStr = normalizeId(entity.lineage_id);
        const symbolIdStr = normalizeId(entity.symbol_id);

        try {
          if (lineageIdStr) {
            const history = await forgeService.getLineageHistory(lineageIdStr);
            set({ activeLineageHistory: history });
          }

          if (symbolIdStr) {
            const impact = await forgeService.getImpactRadius(symbolIdStr);
            set({ activeImpactReport: impact });
          }
        } catch (error) {
          console.error("Failed to load entity context details:", error);
        }
      },

      inspectImpactForSymbol: async (symbolId: string) => {
        try {
          const report = await forgeService.getImpactRadius(symbolId);
          set({ activeImpactReport: report });
        } catch (error) {
          console.error("Failed to inspect impact radius:", error);
        }
      },

      setActiveExecutionPlan: (activeExecutionPlan) => set({ activeExecutionPlan }),
    }),
    {
      name: "vora-app-store",
      partialize: (state) => ({
        cards: state.isLibraryBoardEditing
          ? state.libraryBoardReturnCards ?? state.cards
          : state.cards,
        executedTasks: state.executedTasks,
      }),
    }
  )
);