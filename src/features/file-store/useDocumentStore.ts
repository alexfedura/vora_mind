import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { dbService, type CardDto } from "@/mind/services/dbService";
import type { TextChangeSet, VoraCard } from "@/mind/types";
import { useAppStore } from "@/shared/store/useAppStore";

export interface DocumentState {
  currentDocId: string | null;
  currentBoardId: string | null;
  currentTitle: string;
  currentVersion: string;
  content: string;
  pendingTextChangeSets: TextChangeSet[];
  isDirty: boolean;
  isLoading: boolean;
  lastSavedAt: number | null;
  lastError: string | null;
  loadDoc: (id: string) => Promise<void>;
  setContent: (content: string, changeSet?: TextChangeSet) => void;
  setTitle: (title: string, changeSet?: TextChangeSet) => void;
  setVersion: (version: string) => void;
  saveDoc: () => Promise<void>;
  closeDoc: () => Promise<void>;
  unloadDoc: () => void;
  clearError: () => void;
}

const SAVE_DEBOUNCE_MS = 1000;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let saveInFlight: Promise<void> | null = null;
let loadRequest = 0;

function cancelPendingSave(): void {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
}

function scheduleSave(get: () => DocumentState): void {
  cancelPendingSave();
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void get().saveDoc().catch((error: unknown) => {
      console.error("Failed to save the current card:", error);
    });
  }, SAVE_DEBOUNCE_MS);
}

function toCard(boardId: string, card: CardDto, existing?: VoraCard): VoraCard {
  return {
    ...existing,
    id: card.id,
    title: card.title,
    content: card.content,
    boardId,
    version: card.version,
    deleted_at: card.deleted_at ?? undefined,
  };
}

function syncCard(boardId: string, card: CardDto): void {
  const appState = useAppStore.getState();
  if (appState.activeBoardId !== boardId) return;
  appState.setCards({
    ...appState.cards,
    [card.id]: toCard(boardId, card, appState.cards[card.id]),
  });
}

export const useDocumentStore = create<DocumentState>()(
  subscribeWithSelector((set, get) => ({
    currentDocId: null,
    currentBoardId: null,
    currentTitle: "",
    currentVersion: "0.1.0",
    content: "",
    pendingTextChangeSets: [],
    isDirty: false,
    isLoading: false,
    lastSavedAt: null,
    lastError: null,

    loadDoc: async (id: string) => {
      const requestId = ++loadRequest;
      const boardId = useAppStore.getState().activeBoardId;
      const current = get();

      if (
        current.currentDocId === id &&
        current.currentBoardId === boardId &&
        !current.lastError &&
        (current.isLoading || !current.isDirty)
      ) {
        return;
      }

      if (current.currentDocId !== null && current.isDirty) {
        while (get().currentDocId !== null && get().isDirty) {
          await get().saveDoc();
          if (requestId !== loadRequest) return;
        }
      }

      if (requestId !== loadRequest) return;
      cancelPendingSave();
      set({
        currentDocId: id,
        currentBoardId: boardId,
        currentTitle: "",
        currentVersion: "0.1.0",
        content: "",
        pendingTextChangeSets: [],
        isLoading: true,
        lastError: null,
      });

      try {
        const cards = await dbService.getAllCards(boardId);
        if (requestId !== loadRequest) return;
        const card = cards.find((item) => item.id === id && item.deleted_at === null);
        if (!card) {
          throw new Error(`Card "${id}" was not found in board "${boardId}".`);
        }

        syncCard(boardId, card);
        set({
          content: card.content,
          pendingTextChangeSets: [],
          currentTitle: card.title,
          currentVersion: card.version,
          isDirty: false,
          isLoading: false,
          lastSavedAt: null,
        });
      } catch (error) {
        if (requestId !== loadRequest) return;
        const message = error instanceof Error ? error.message : String(error);
        set({
          content: "",
          pendingTextChangeSets: [],
          isDirty: false,
          isLoading: false,
          lastError: `Failed to load card "${id}": ${message}`,
        });
        throw error;
      }
    },

    setContent: (content: string, changeSet?: TextChangeSet) => {
      const state = get();
      if (state.currentDocId === null || state.isLoading || state.content === content) return;

      set({
        content,
        isDirty: true,
        pendingTextChangeSets: changeSet
          ? [...state.pendingTextChangeSets, changeSet]
          : state.pendingTextChangeSets,
      });
      scheduleSave(get);
    },

    setTitle: (currentTitle: string, changeSet?: TextChangeSet) => {
      const state = get();
      if (state.currentDocId === null || state.isLoading || state.currentTitle === currentTitle) return;

      set({
        currentTitle,
        isDirty: true,
        pendingTextChangeSets: changeSet
          ? [...state.pendingTextChangeSets, changeSet]
          : state.pendingTextChangeSets,
      });
      scheduleSave(get);
    },

    setVersion: (currentVersion: string) => {
      const state = get();
      if (state.currentDocId === null || state.isLoading || state.currentVersion === currentVersion) return;

      set({ currentVersion, isDirty: true });
      scheduleSave(get);
    },

    saveDoc: async () => {
      cancelPendingSave();
      if (saveInFlight) {
        await saveInFlight;
        if (get().isDirty) await get().saveDoc();
        return;
      }

      const state = get();
      if (state.currentDocId === null || state.currentBoardId === null || !state.isDirty) return;

      const {
        currentDocId: id,
        currentBoardId: boardId,
        currentTitle,
        currentVersion,
        content,
        pendingTextChangeSets,
      } = state;
      const save = (async () => {
        try {
          await dbService.saveCardDirect(
            boardId,
            id,
            currentTitle,
            content,
            currentVersion,
            pendingTextChangeSets,
          );
          const latest = get();
          if (latest.currentDocId !== id || latest.currentBoardId !== boardId) return;
          const remainingTextChangeSets = latest.pendingTextChangeSets.slice(
            pendingTextChangeSets.length,
          );
          const card: CardDto = {
            id,
            title: latest.currentTitle,
            content: latest.content,
            version: latest.currentVersion,
            deleted_at: null,
          };
          syncCard(boardId, card);
          window.dispatchEvent(new CustomEvent("vora:text-links-updated", {
            detail: { cardId: id },
          }));
          if (
            latest.content === content &&
            latest.currentTitle === currentTitle &&
            latest.currentVersion === currentVersion &&
            remainingTextChangeSets.length === 0
          ) {
            set({
              isDirty: false,
              pendingTextChangeSets: [],
              lastSavedAt: Date.now(),
              lastError: null,
            });
          } else {
            set({
              pendingTextChangeSets: remainingTextChangeSets,
              lastSavedAt: Date.now(),
              lastError: null,
            });
            scheduleSave(get);
          }
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          set({ lastError: `Failed to save card "${id}": ${message}` });
          throw error;
        }
      })();

      let guardedSave: Promise<void>;
      guardedSave = save.finally(() => {
        if (saveInFlight === guardedSave) saveInFlight = null;
      });
      saveInFlight = guardedSave;
      await guardedSave;
    },

    closeDoc: async () => {
      const id = get().currentDocId;
      if (id === null || get().isLoading) return;

      while (get().currentDocId === id && get().isDirty) {
        await get().saveDoc();
      }

      if (get().currentDocId !== id) return;
      loadRequest += 1;
      cancelPendingSave();
      set({
        currentDocId: null,
        currentBoardId: null,
        currentTitle: "",
        currentVersion: "0.1.0",
        content: "",
        pendingTextChangeSets: [],
        isDirty: false,
        isLoading: false,
      });
    },

    unloadDoc: () => {
      loadRequest += 1;
      cancelPendingSave();
      set({
        currentDocId: null,
        currentBoardId: null,
        currentTitle: "",
        currentVersion: "0.1.0",
        content: "",
        pendingTextChangeSets: [],
        isDirty: false,
        isLoading: false,
        lastError: null,
      });
    },

    clearError: () => set({ lastError: null }),
  })),
);

export const selectCurrentDocId = (state: DocumentState): string | null =>
  state.currentDocId;
export const selectContent = (state: DocumentState): string => state.content;
export const selectIsDirty = (state: DocumentState): boolean => state.isDirty;
export const selectIsLoading = (state: DocumentState): boolean => state.isLoading;
export const selectLastError = (state: DocumentState): string | null =>
  state.lastError;
