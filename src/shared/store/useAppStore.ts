import { create } from "zustand";
import { persist } from "zustand/middleware";
import { setSetting } from "@/shared/store/useSettingsStore";
import { VoraCard } from "@/mind/types";
import { createTextChangeSet, dbService } from "@/mind/services/dbService";
import type {
  ChildHypothesisInput,
  Hypothesis,
  HypothesisPromotionInput,
  HypothesisVerdict,
  JournalEntry,
  QuestionItem,
  TaskFlowItem,
  TaskFlowType,
} from "@/types";
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

const journalSaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
const journalSaveQueues = new Map<string, Promise<void>>();
const questionSaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
const questionSaveQueues = new Map<string, Promise<void>>();

function sortQuestions(questions: QuestionItem[]): QuestionItem[] {
  return [...questions].sort((left, right) =>
    right.updatedAt - left.updatedAt ||
    right.createdAt - left.createdAt ||
    left.id.localeCompare(right.id)
  );
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
  addCard: (title?: string, content?: string, librarySourceCardId?: string) => string;
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
  loadTaskFlowItems: (taskType: TaskFlowType) => Promise<void>;
  createTaskFlowItem: (title: string, taskType: TaskFlowType) => Promise<void>;
  updateTaskFlowItem: (id: string, title: string, description: string) => Promise<void>;
  setTaskFlowItemCompleted: (id: string, completed: boolean) => Promise<void>;
  reorderTaskFlowItems: (ids: string[], taskType: TaskFlowType) => Promise<void>;
  deleteTaskFlowItem: (id: string) => Promise<void>;

  journalEntries: Record<string, JournalEntry>;
  journalSaveStatus: Record<string, "saving" | "saved" | "error">;
  journalSaveError: string | null;
  loadJournalEntries: (startDate: string, endDate: string) => Promise<void>;
  updateJournalEntryContent: (date: string, content: string) => void;
  flushJournalEntry: (date: string) => Promise<void>;

  questions: QuestionItem[];
  questionSaveStatus: Record<string, "saving" | "saved" | "error">;
  questionSaveError: string | null;
  loadQuestions: () => Promise<void>;
  createQuestion: (title: string, description: string) => Promise<QuestionItem>;
  updateQuestion: (id: string, title: string, description: string) => void;
  flushQuestion: (id: string) => Promise<void>;
  deleteQuestion: (id: string) => Promise<void>;
  triageQuestion: (
    id: string,
    impact: number,
    feasibility: number,
    urgency: number,
    triageNotes?: string,
  ) => Promise<QuestionItem>;
  untriageQuestion: (id: string) => Promise<QuestionItem>;
  archiveQuestion: (id: string) => Promise<QuestionItem>;
  selectQuestionForPromotion: (id: string) => Promise<QuestionItem>;
  unselectQuestionForPromotion: (id: string) => Promise<QuestionItem>;

  hypotheses: Hypothesis[];
  loadHypotheses: () => Promise<void>;
  promoteToHypothesis: (input: HypothesisPromotionInput) => Promise<Hypothesis>;
  graduateToLibrary: (hypothesisId: string, libraryGroupId?: string | null) => Promise<Hypothesis>;
  createChildHypothesis: (input: ChildHypothesisInput) => Promise<Hypothesis>;
  recordVerdict: (
    id: string,
    evidenceNotes: string,
    verdict: HypothesisVerdict,
  ) => Promise<Hypothesis>;

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

      addCard: (title = "New card", content = "", librarySourceCardId) => {
        const boardId = get().activeBoardId;
        const id = `card-${boardId}-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
        const newCard: VoraCard = {
          id,
          title,
          content,
          boardId,
          librarySourceCardId,
          version: "v0.1.0",
          created_at: Date.now(),
        };
        set((state) => ({
          cards: { ...state.cards, [id]: newCard },
        }));
        if (!get().isLibraryBoardEditing) {
          void dbService
            .saveCardDirect(
              boardId,
              id,
              title,
              content,
              newCard.version ?? "v0.1.0",
              [],
              librarySourceCardId,
            )
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
      loadTaskFlowItems: async (taskType) => {
        const taskFlowItems = await dbService.getTaskFlowItems(taskType);
        set({ taskFlowItems });
      },
      createTaskFlowItem: async (title, taskType) => {
        const task = await dbService.createTaskFlowItem(title, taskType);
        set((state) => ({
          taskFlowItems: [...state.taskFlowItems, task].sort((left, right) =>
            left.position - right.position
          ),
        }));
      },
      updateTaskFlowItem: async (id, title, description) => {
        const task = await dbService.updateTaskFlowItem(id, title, description);
        set((state) => ({
          taskFlowItems: state.taskFlowItems.map((item) => item.id === id ? task : item),
        }));
      },
      setTaskFlowItemCompleted: async (id, completed) => {
        const task = await dbService.setTaskFlowItemCompleted(id, completed);
        set((state) => ({
          taskFlowItems: state.taskFlowItems.map((item) => item.id === id ? task : item),
        }));
      },
      reorderTaskFlowItems: async (ids, taskType) => {
        const taskFlowItems = await dbService.reorderTaskFlowItems(ids, taskType);
        set({ taskFlowItems });
      },
      deleteTaskFlowItem: async (id) => {
        await dbService.deleteTaskFlowItem(id);
        set((state) => ({
          taskFlowItems: state.taskFlowItems.filter((item) => item.id !== id),
        }));
      },

      journalEntries: {},
      journalSaveStatus: {},
      journalSaveError: null,
      loadJournalEntries: async (startDate, endDate) => {
        const entries = await dbService.getJournalEntries(startDate, endDate);
        set((state) => {
          const journalEntries = { ...state.journalEntries };
          for (const entry of entries) {
            const saveStatus = state.journalSaveStatus[entry.date];
            const existingEntry = state.journalEntries[entry.date];
            if (
              saveStatus === "saving" ||
              saveStatus === "error" ||
              (existingEntry && existingEntry.updatedAt > entry.updatedAt)
            ) {
              continue;
            }
            journalEntries[entry.date] = entry;
          }
          return { journalEntries };
        });
      },
      updateJournalEntryContent: (date, content) => {
        const now = Date.now();
        set((state) => ({
          journalEntries: {
            ...state.journalEntries,
            [date]: {
              date,
              content,
              createdAt: state.journalEntries[date]?.createdAt ?? now,
              updatedAt: now,
            },
          },
          journalSaveStatus: { ...state.journalSaveStatus, [date]: "saving" },
          journalSaveError: null,
        }));
        const previousTimer = journalSaveTimers.get(date);
        if (previousTimer) clearTimeout(previousTimer);
        journalSaveTimers.set(date, setTimeout(() => {
          journalSaveTimers.delete(date);
          void get().flushJournalEntry(date).catch(() => undefined);
        }, 500));
      },
      flushJournalEntry: async (date) => {
        const timer = journalSaveTimers.get(date);
        if (timer) {
          clearTimeout(timer);
          journalSaveTimers.delete(date);
        }
        const entry = get().journalEntries[date];
        if (!entry) return;

        const previousSave = journalSaveQueues.get(date) ?? Promise.resolve();
        const save = previousSave.catch(() => undefined).then(async () => {
          set({ journalSaveStatus: { ...get().journalSaveStatus, [date]: "saving" } });
          try {
            const saved = await dbService.saveJournalEntry(date, entry.content);
            set((state) => {
              const latestEntry = state.journalEntries[date];
              return {
                journalEntries: latestEntry?.content === entry.content
                  ? { ...state.journalEntries, [date]: saved }
                  : state.journalEntries,
                journalSaveStatus: {
                  ...state.journalSaveStatus,
                  [date]: latestEntry?.content === entry.content ? "saved" : "saving",
                },
                journalSaveError: null,
              };
            });
          } catch (error) {
            console.error("Failed to save journal entry:", error);
            set({
              journalSaveStatus: { ...get().journalSaveStatus, [date]: "error" },
              journalSaveError: error instanceof Error ? error.message : String(error),
            });
            throw error;
          }
        });
        journalSaveQueues.set(date, save.catch(() => undefined));
        await save;
      },

      questions: [],
      questionSaveStatus: {},
      questionSaveError: null,
      loadQuestions: async () => {
        const questions = await dbService.getQuestions();
        set((state) => {
          const loadedQuestions = new Map(questions.map((question) => [question.id, question]));
          for (const question of state.questions) {
            const saveStatus = state.questionSaveStatus[question.id];
            const loadedQuestion = loadedQuestions.get(question.id);
            if (
              saveStatus === "saving" ||
              saveStatus === "error" ||
              (loadedQuestion && question.updatedAt > loadedQuestion.updatedAt)
            ) {
              loadedQuestions.set(question.id, question);
            }
          }
          return { questions: sortQuestions([...loadedQuestions.values()]) };
        });
      },
      createQuestion: async (title, description) => {
        const question = await dbService.createQuestion(title, description);
        set((state) => ({
          questions: sortQuestions([
            ...state.questions.filter((item) => item.id !== question.id),
            question,
          ]),
          questionSaveError: null,
        }));
        return question;
      },
      updateQuestion: (id, title, description) => {
        const currentQuestion = get().questions.find((question) => question.id === id);
        if (!currentQuestion) return;

        set((state) => ({
          questions: sortQuestions(state.questions.map((question) =>
            question.id === id
              ? { ...question, title, description, updatedAt: Date.now() }
              : question
          )),
          questionSaveStatus: { ...state.questionSaveStatus, [id]: "saving" },
          questionSaveError: null,
        }));
        const previousTimer = questionSaveTimers.get(id);
        if (previousTimer) clearTimeout(previousTimer);
        questionSaveTimers.set(id, setTimeout(() => {
          questionSaveTimers.delete(id);
          void get().flushQuestion(id).catch(() => undefined);
        }, 500));
      },
      flushQuestion: async (id) => {
        const timer = questionSaveTimers.get(id);
        if (timer) {
          clearTimeout(timer);
          questionSaveTimers.delete(id);
        }

        const previousSave = questionSaveQueues.get(id) ?? Promise.resolve();
        const save = previousSave.catch(() => undefined).then(async () => {
          const question = get().questions.find((item) => item.id === id);
          if (!question) return;

          set({ questionSaveStatus: { ...get().questionSaveStatus, [id]: "saving" } });
          try {
            const saved = await dbService.saveQuestion(
              id,
              question.title,
              question.description,
            );
            set((state) => {
              const latestQuestion = state.questions.find((item) => item.id === id);
              const isCurrent = latestQuestion?.title === question.title &&
                latestQuestion.description === question.description;
              return {
                questions: isCurrent
                  ? sortQuestions(state.questions.map((item) => item.id === id ? saved : item))
                  : state.questions,
                questionSaveStatus: {
                  ...state.questionSaveStatus,
                  [id]: isCurrent ? "saved" : "saving",
                },
                questionSaveError: null,
              };
            });
          } catch (error) {
            console.error("Failed to save question:", error);
            set({
              questionSaveStatus: { ...get().questionSaveStatus, [id]: "error" },
              questionSaveError: error instanceof Error ? error.message : String(error),
            });
            throw error;
          }
        });
        questionSaveQueues.set(id, save.catch(() => undefined));
        await save;
      },
      deleteQuestion: async (id) => {
        const timer = questionSaveTimers.get(id);
        if (timer) {
          clearTimeout(timer);
          questionSaveTimers.delete(id);
        }
        await get().flushQuestion(id);
        await dbService.deleteQuestion(id);
        set((state) => {
          const questionSaveStatus = { ...state.questionSaveStatus };
          delete questionSaveStatus[id];
          return {
            questions: state.questions.filter((question) => question.id !== id),
            questionSaveStatus,
            questionSaveError: null,
          };
        });
      },
      triageQuestion: async (id, impact, feasibility, urgency, triageNotes) => {
        const previousQuestion = get().questions.find((question) => question.id === id);
        if (!previousQuestion) throw new Error(`Question not found: ${id}`);
        const optimisticQuestion: QuestionItem = {
          ...previousQuestion,
          status: "triaged",
          impact,
          feasibility,
          urgency,
          priorityScore: impact * feasibility * urgency,
          triageNotes: triageNotes ?? null,
          triagedAt: Date.now(),
          updatedAt: Date.now(),
        };
        set((state) => ({
          questions: sortQuestions(state.questions.map((question) =>
            question.id === id ? optimisticQuestion : question
          )),
        }));
        try {
          const question = await dbService.triageQuestion(
            id,
            impact,
            feasibility,
            urgency,
            triageNotes,
          );
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) =>
              item.id === id ? question : item
            )),
          }));
          return question;
        } catch (error) {
          set((state) => ({
            questions: sortQuestions(state.questions.map((question) =>
              question.id === id ? previousQuestion : question
            )),
          }));
          throw error;
        }
      },
      untriageQuestion: async (id) => {
        const previousQuestion = get().questions.find((question) => question.id === id);
        if (!previousQuestion) throw new Error(`Question not found: ${id}`);
        const optimisticQuestion: QuestionItem = {
          ...previousQuestion,
          status: "parked",
          impact: null,
          feasibility: null,
          urgency: null,
          priorityScore: null,
          triageNotes: null,
          triagedAt: null,
          isSelectedForPromotion: false,
          updatedAt: Date.now(),
        };
        set((state) => ({
          questions: sortQuestions(state.questions.map((question) =>
            question.id === id ? optimisticQuestion : question
          )),
        }));
        try {
          const question = await dbService.untriageQuestion(id);
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) =>
              item.id === id ? question : item
            )),
          }));
          return question;
        } catch (error) {
          set((state) => ({
            questions: sortQuestions(state.questions.map((question) =>
              question.id === id ? previousQuestion : question
            )),
          }));
          throw error;
        }
      },
      archiveQuestion: async (id) => {
        const previousQuestion = get().questions.find((question) => question.id === id);
        if (!previousQuestion) throw new Error(`Question not found: ${id}`);
        const optimisticQuestion: QuestionItem = {
          ...previousQuestion,
          status: "archived",
          impact: null,
          feasibility: null,
          urgency: null,
          priorityScore: null,
          triageNotes: null,
          triagedAt: null,
          isSelectedForPromotion: false,
          updatedAt: Date.now(),
        };
        set((state) => ({
          questions: sortQuestions(state.questions.map((question) =>
            question.id === id ? optimisticQuestion : question
          )),
        }));
        try {
          const question = await dbService.archiveQuestion(id);
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) =>
              item.id === id ? question : item
            )),
          }));
          return question;
        } catch (error) {
          set((state) => ({
            questions: sortQuestions(state.questions.map((question) =>
              question.id === id ? previousQuestion : question
            )),
          }));
          throw error;
        }
      },
      selectQuestionForPromotion: async (id) => {
        const currentQuestions = get().questions;
        const previousSelectedId = currentQuestions.find(
          (question) => question.isSelectedForPromotion,
        )?.id ?? null;
        const targetQuestion = currentQuestions.find((question) => question.id === id);
        if (!targetQuestion) throw new Error(`Question not found: ${id}`);
        const previousSelection = currentQuestions
          .filter((question) => question.isSelectedForPromotion)
          .map((question) => question.id);
        set((state) => ({
          questions: sortQuestions(state.questions.map((question) => ({
            ...question,
            isSelectedForPromotion: question.id === id,
          }))),
        }));
        try {
          const question = await dbService.selectQuestionForPromotion(id);
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) =>
              item.id === id
                ? question
                : item.id === previousSelectedId
                  ? { ...item, isSelectedForPromotion: false }
                  : item
            )),
          }));
          return question;
        } catch (error) {
          const previousSelectedIds = new Set(previousSelection);
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) => ({
              ...item,
              isSelectedForPromotion: previousSelectedIds.has(item.id),
            }))),
          }));
          throw error;
        }
      },
      unselectQuestionForPromotion: async (id) => {
        const previousQuestion = get().questions.find((question) => question.id === id);
        if (!previousQuestion) throw new Error(`Question not found: ${id}`);
        set((state) => ({
          questions: sortQuestions(state.questions.map((question) =>
            question.id === id
              ? { ...question, isSelectedForPromotion: false, updatedAt: Date.now() }
              : question
          )),
        }));
        try {
          const question = await dbService.unselectQuestionForPromotion(id);
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) =>
              item.id === id ? question : item
            )),
          }));
          return question;
        } catch (error) {
          set((state) => ({
            questions: sortQuestions(state.questions.map((item) =>
              item.id === id ? previousQuestion : item
            )),
          }));
          throw error;
        }
      },

      hypotheses: [],
      loadHypotheses: async () => {
        const hypotheses = await dbService.getHypotheses();
        set({ hypotheses });
      },
      promoteToHypothesis: async (input) => {
        const hypothesis = await dbService.promoteToHypothesis(input);
        set((state) => ({
          hypotheses: [hypothesis, ...state.hypotheses.filter(
            (item) => item.id !== hypothesis.id
          )],
          questions: sortQuestions(state.questions.map((question) =>
            question.id === input.inquiryId
              ? {
                  ...question,
                  status: "promoted",
                  isSelectedForPromotion: false,
                  promotedToHypothesisId: hypothesis.id,
                  updatedAt: hypothesis.updatedAt,
                }
              : question
          )),
        }));
        return hypothesis;
      },
      graduateToLibrary: async (hypothesisId, libraryGroupId) => {
        const hypothesis = await dbService.graduateToLibrary(hypothesisId, libraryGroupId);
        set((state) => ({
          hypotheses: state.hypotheses.map((item) =>
            item.id === hypothesis.id ? hypothesis : item
          ),
        }));
        return hypothesis;
      },
      createChildHypothesis: async (input) => {
        const hypothesis = await dbService.createChildHypothesis(input);
        set((state) => ({
          hypotheses: [hypothesis, ...state.hypotheses.filter(
            (item) => item.id !== hypothesis.id
          )],
        }));
        return hypothesis;
      },
      recordVerdict: async (id, evidenceNotes, verdict) => {
        const hypothesis = await dbService.recordVerdict(id, evidenceNotes, verdict);
        set((state) => ({
          hypotheses: state.hypotheses.map((item) =>
            item.id === hypothesis.id ? hypothesis : item
          ),
        }));
        return hypothesis;
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