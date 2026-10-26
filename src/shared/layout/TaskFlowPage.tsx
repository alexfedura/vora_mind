import React, { useEffect, useRef, useState } from "react";
import { Check, CheckCircle2, GripVertical, ListTodo, Plus, Trash2 } from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import type { TaskFlowItem } from "@/types";

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const TaskFlowPage: React.FC = () => {
  const tasks = useAppStore((state) => state.taskFlowItems);
  const loadTasks = useAppStore((state) => state.loadTaskFlowItems);
  const createTask = useAppStore((state) => state.createTaskFlowItem);
  const updateTask = useAppStore((state) => state.updateTaskFlowItem);
  const setTaskCompleted = useAppStore((state) => state.setTaskFlowItemCompleted);
  const reorderTasks = useAppStore((state) => state.reorderTaskFlowItems);
  const deleteTask = useAppStore((state) => state.deleteTaskFlowItem);
  const [title, setTitle] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [draggedTaskId, setDraggedTaskId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);
  const [inlineEdit, setInlineEdit] = useState<{ id: string; value: string } | null>(null);
  const [detailsEdit, setDetailsEdit] = useState<{
    id: string;
    title: string;
    description: string;
  } | null>(null);
  const [isSavingInline, setIsSavingInline] = useState(false);
  const inlineInputRef = useRef<HTMLInputElement>(null);
  const skipInlineBlurRef = useRef(false);

  useEffect(() => {
    if (inlineEdit) {
      inlineInputRef.current?.focus();
      inlineInputRef.current?.select();
    }
  }, [inlineEdit?.id]);

  useEffect(() => {
    if (!detailsEdit) return;
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setDetailsEdit(null);
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [detailsEdit]);

  useEffect(() => {
    let cancelled = false;
    const load = async (): Promise<void> => {
      setIsLoading(true);
      setError(null);
      try {
        await loadTasks();
      } catch (cause) {
        console.error("Could not load Task Flow items:", cause);
        if (!cancelled) setError(getErrorMessage(cause));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };
    void load();
    return () => { cancelled = true; };
  }, [loadTasks]);

  const runAction = async (action: () => Promise<void>): Promise<boolean> => {
    setError(null);
    setIsBusy(true);
    try {
      await action();
      return true;
    } catch (cause) {
      console.error("Task Flow action failed:", cause);
      setError(getErrorMessage(cause));
      return false;
    } finally {
      setIsBusy(false);
    }
  };

  const finishInlineEdit = async (task: TaskFlowItem, value: string): Promise<void> => {
    const nextTitle = value.trim();
    if (!nextTitle) {
      setError("Task title must contain between 1 and 200 characters");
      inlineInputRef.current?.focus();
      return;
    }
    if (nextTitle.length > 200) {
      setError("Task title must contain between 1 and 200 characters");
      inlineInputRef.current?.focus();
      return;
    }
    setInlineEdit(null);
    if (nextTitle !== task.title) {
      await runAction(() => updateTask(task.id, nextTitle, task.description));
    }
  };

  const openDetailsEdit = (task: TaskFlowItem): void => {
    const title = inlineEdit?.id === task.id ? inlineEdit.value : task.title;
    skipInlineBlurRef.current = inlineEdit?.id === task.id;
    setInlineEdit(null);
    setDetailsEdit({ id: task.id, title, description: task.description });
  };

  const saveDetailsEdit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!detailsEdit) return;
    const nextTitle = detailsEdit.title.trim();
    if (!nextTitle || nextTitle.length > 200) {
      setError("Task title must contain between 1 and 200 characters");
      return;
    }
    const saved = await runAction(() =>
      updateTask(detailsEdit.id, nextTitle, detailsEdit.description)
    );
    if (saved) setDetailsEdit(null);
  };

  const handleCreate = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const nextTitle = title.trim();
    if (!nextTitle) return;
    await runAction(async () => {
      await createTask(nextTitle);
      setTitle("");
    });
  };

  const handleDrop = async (targetTask: TaskFlowItem): Promise<void> => {
    if (!draggedTaskId || draggedTaskId === targetTask.id || isBusy) return;
    const sourceIndex = tasks.findIndex((task) => task.id === draggedTaskId);
    const targetIndex = tasks.findIndex((task) => task.id === targetTask.id);
    if (sourceIndex < 0 || targetIndex < 0) return;
    const nextTasks = [...tasks];
    const [movedTask] = nextTasks.splice(sourceIndex, 1);
    if (!movedTask) return;
    nextTasks.splice(targetIndex, 0, movedTask);
    await runAction(() => reorderTasks(nextTasks.map((task) => task.id)));
  };

  const completedCount = tasks.filter((task) => task.completed).length;

  return (
    <section className="absolute inset-0 z-20 flex min-h-0 flex-col overflow-hidden bg-[#101114] text-gray-200">
      <div className="border-b border-[#292c34] bg-[#14161b] px-6 py-6 sm:px-10">
        <div className="mx-auto flex w-full max-w-4xl items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-[#e8ad55]">
              <ListTodo size={15} />
              Workspace
            </div>
            <h1 className="mt-2 text-2xl font-semibold tracking-tight text-white">Task Flow</h1>
            <p className="mt-1 text-sm text-gray-400">
              Keep your priorities in order. Drag any task to move it through your timeline.
            </p>
          </div>
          <div className="shrink-0 rounded-xl border border-[#30333c] bg-[#1a1c22] px-3 py-2 text-right">
            <p className="text-lg font-semibold leading-none text-white">{completedCount}/{tasks.length}</p>
            <p className="mt-1 text-[10px] uppercase tracking-wider text-gray-500">completed</p>
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-7 sm:px-10">
        <div className="mx-auto w-full max-w-4xl">
          <form onSubmit={(event) => void handleCreate(event)} className="flex gap-2">
            <label className="sr-only" htmlFor="task-flow-title">New task</label>
            <input
              id="task-flow-title"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              maxLength={200}
              placeholder="What needs to get done?"
              disabled={isBusy}
              className="min-w-0 flex-1 rounded-xl border border-[#343741] bg-[#17191f] px-4 py-3 text-sm text-white outline-none transition placeholder:text-gray-600 focus:border-[#e8ad55]/70 disabled:opacity-60"
            />
            <button
              type="submit"
              disabled={isBusy || !title.trim()}
              className="flex shrink-0 items-center gap-2 rounded-xl bg-[#e8ad55] px-4 py-3 text-sm font-semibold text-[#17130d] transition hover:bg-[#f0bd6c] disabled:cursor-not-allowed disabled:opacity-40"
            >
              <Plus size={16} />
              Add task
            </button>
          </form>

          <div className="mt-7 flex items-center justify-between border-b border-[#292c34] pb-3">
            <h2 className="text-xs font-semibold uppercase tracking-[0.15em] text-gray-400">
              Priority timeline
            </h2>
            <p className="text-xs text-gray-600">Drag a row to change its priority</p>
          </div>

          {error && (
            <div role="alert" className="mt-4 flex items-start justify-between gap-4 rounded-lg border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200">
              <span>{error}</span>
              <button
                type="button"
                onClick={() => void runAction(loadTasks)}
                className="shrink-0 font-semibold underline underline-offset-2"
              >
                Reload list
              </button>
            </div>
          )}

          {isLoading ? (
            <p className="py-12 text-center text-sm text-gray-500">Loading tasks...</p>
          ) : tasks.length === 0 ? (
            <div className="flex flex-col items-center py-16 text-center">
              <div className="flex h-12 w-12 items-center justify-center rounded-2xl border border-[#343741] bg-[#1a1c22] text-[#e8ad55]">
                <CheckCircle2 size={22} />
              </div>
              <p className="mt-4 text-sm font-medium text-gray-300">Your timeline is clear</p>
              <p className="mt-1 text-xs text-gray-600">Add your first task above to start setting priorities.</p>
            </div>
          ) : (
            <div className="relative mt-4">
              <div className="pointer-events-none absolute bottom-7 right-[21px] top-7 w-px bg-[#30333b]" />
              <ol className="space-y-2" dir="rtl">
                {tasks.map((task, index) => (
                  <li
                    key={task.id}
                    draggable={!isBusy}
                    onDragStart={(event) => {
                      setDraggedTaskId(task.id);
                      event.dataTransfer.effectAllowed = "move";
                      event.dataTransfer.setData("text/plain", task.id);
                    }}
                    onDragOver={(event) => {
                      event.preventDefault();
                      event.dataTransfer.dropEffect = "move";
                      setDropTargetId(task.id);
                    }}
                    onDrop={(event) => {
                      event.preventDefault();
                      void handleDrop(task);
                      setDraggedTaskId(null);
                      setDropTargetId(null);
                    }}
                    onDragEnd={() => {
                      setDraggedTaskId(null);
                      setDropTargetId(null);
                    }}
                    dir="rtl"
                    className={`relative flex min-h-[68px] flex-row items-center gap-3 rounded-xl border px-3 py-2 transition ${
                      dropTargetId === task.id
                        ? "border-[#e8ad55]/70 bg-[#25221d]"
                        : "border-[#292c34] bg-[#17191f] hover:border-[#3c404a]"
                    } ${draggedTaskId === task.id ? "opacity-45" : ""}`}
                  >
                    <div className="relative z-[1] flex w-8 shrink-0 justify-center">
                      <span className={`flex h-7 w-7 items-center justify-center rounded-full border text-[11px] font-semibold ${
                        task.completed
                          ? "border-emerald-700/60 bg-emerald-950/70 text-emerald-400"
                          : "border-[#48413a] bg-[#211e19] text-[#e8ad55]"
                      }`}>
                        {index + 1}
                      </span>
                    </div>
                    <button
                      type="button"
                      aria-label={`Drag to reorder ${task.title}`}
                      title="Drag to change priority"
                      className="cursor-grab touch-none text-gray-600 transition hover:text-gray-300 active:cursor-grabbing"
                    >
                      <GripVertical size={18} />
                    </button>
                    <button
                      type="button"
                      aria-label={`Mark ${task.title} ${task.completed ? "incomplete" : "complete"}`}
                      aria-pressed={task.completed}
                      disabled={isBusy}
                      onClick={() => void runAction(() =>
                        setTaskCompleted(task.id, !task.completed)
                      )}
                      className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-[4px] border transition disabled:cursor-not-allowed disabled:opacity-50 ${
                        task.completed
                          ? "border-[#e8ad55] bg-[#e8ad55] text-[#17130d]"
                          : "border-gray-500 text-transparent hover:border-[#e8ad55]"
                      }`}
                    >
                      {task.completed && <Check size={14} strokeWidth={3} />}
                    </button>
                    <div className="min-w-0 flex-1 text-right" onDoubleClick={() => openDetailsEdit(task)}>
                      {inlineEdit?.id === task.id ? (
                        <input
                          ref={inlineInputRef}
                          value={inlineEdit.value}
                          maxLength={200}
                          disabled={isBusy || isSavingInline}
                          aria-label={`Edit ${task.title}`}
                          onChange={(event) => {
                            setError(null);
                            setInlineEdit({ id: task.id, value: event.target.value });
                          }}
                          onBlur={() => {
                            if (skipInlineBlurRef.current) {
                              skipInlineBlurRef.current = false;
                              return;
                            }
                            setIsSavingInline(true);
                            void finishInlineEdit(task, inlineEdit.value).finally(() => {
                              setIsSavingInline(false);
                            });
                          }}
                          onKeyDown={(event) => {
                            if (event.key === "Enter") {
                              event.preventDefault();
                              skipInlineBlurRef.current = true;
                              void finishInlineEdit(task, inlineEdit.value);
                            } else if (event.key === "Escape") {
                              skipInlineBlurRef.current = true;
                              setInlineEdit(null);
                            }
                          }}
                          className="w-full rounded-md border border-[#e8ad55]/60 bg-[#101114] px-2 py-1 text-right text-sm text-white outline-none focus:border-[#e8ad55]"
                        />
                      ) : (
                        <button
                          type="button"
                          onClick={() => {
                            setError(null);
                            setInlineEdit({ id: task.id, value: task.title });
                          }}
                          className={`block w-full overflow-hidden text-ellipsis whitespace-nowrap rounded-md px-2 py-1 text-right text-sm transition hover:bg-[#202229] ${
                            task.completed ? "text-gray-600 line-through" : "text-gray-200"
                          }`}
                          title="Click to edit title; double-click to edit details"
                        >
                          {task.title}
                        </button>
                      )}
                    </div>
                    <button
                      type="button"
                      aria-label={`Delete ${task.title}`}
                      title="Delete task"
                      disabled={isBusy}
                      onClick={() => void runAction(() => deleteTask(task.id))}
                      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-gray-600 transition hover:bg-rose-950/40 hover:text-rose-300 disabled:cursor-not-allowed disabled:opacity-40"
                    >
                      <Trash2 size={15} />
                    </button>
                  </li>
                ))}
              </ol>
            </div>
          )}
        </div>
      </div>
      {detailsEdit && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setDetailsEdit(null);
          }}
        >
          <form
            role="dialog"
            aria-modal="true"
            aria-labelledby="task-flow-details-title"
            onSubmit={(event) => void saveDetailsEdit(event)}
            className="w-full max-w-xl rounded-2xl border border-[#343741] bg-[#17191f] p-5 shadow-2xl sm:p-6"
          >
            <h2 id="task-flow-details-title" className="mb-5 text-lg font-semibold text-white">
              Edit task
            </h2>
            <label htmlFor="task-flow-details-name" className="mb-2 block text-sm text-gray-300">
              Title
            </label>
            <input
              id="task-flow-details-name"
              autoFocus
              value={detailsEdit.title}
              onChange={(event) => {
                setError(null);
                setDetailsEdit({ ...detailsEdit, title: event.target.value });
              }}
              maxLength={200}
              required
              className="mb-4 w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2.5 text-sm text-white outline-none focus:border-[#e8ad55]"
            />
            <label htmlFor="task-flow-details-description" className="mb-2 block text-sm text-gray-300">
              Description
            </label>
            <textarea
              id="task-flow-details-description"
              value={detailsEdit.description}
              onChange={(event) => {
                setError(null);
                setDetailsEdit({ ...detailsEdit, description: event.target.value });
              }}
              rows={8}
              maxLength={50000}
              placeholder="Add task details..."
              className="w-full resize-y rounded-lg border border-[#343741] bg-[#101114] px-3 py-2.5 text-sm text-white outline-none focus:border-[#e8ad55]"
            />
            {error && (
              <p role="alert" className="mt-3 text-sm text-rose-300">{error}</p>
            )}
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setDetailsEdit(null)}
                className="rounded-lg border border-[#343741] px-4 py-2 text-sm text-gray-300 transition hover:bg-[#24262d]"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isBusy}
                className="rounded-lg bg-[#e8ad55] px-4 py-2 text-sm font-semibold text-[#17130d] transition hover:bg-[#f0bd6c] disabled:cursor-not-allowed disabled:opacity-50"
              >
                Save task
              </button>
            </div>
          </form>
        </div>
      )}
    </section>
  );
};
