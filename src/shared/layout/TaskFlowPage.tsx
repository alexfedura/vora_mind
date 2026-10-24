import React, { useEffect, useState } from "react";
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
  const setTaskCompleted = useAppStore((state) => state.setTaskFlowItemCompleted);
  const reorderTasks = useAppStore((state) => state.reorderTaskFlowItems);
  const deleteTask = useAppStore((state) => state.deleteTaskFlowItem);
  const [title, setTitle] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [draggedTaskId, setDraggedTaskId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

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

  const runAction = async (action: () => Promise<void>): Promise<void> => {
    setError(null);
    setIsBusy(true);
    try {
      await action();
    } catch (cause) {
      console.error("Task Flow action failed:", cause);
      setError(getErrorMessage(cause));
    } finally {
      setIsBusy(false);
    }
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
              <div className="pointer-events-none absolute bottom-7 left-[21px] top-7 w-px bg-[#30333b]" />
              <ol className="space-y-2">
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
                    className={`relative flex min-h-[68px] items-center gap-3 rounded-xl border px-3 py-2 transition ${
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
                        {task.completed ? <Check size={14} /> : index + 1}
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
                    <input
                      type="checkbox"
                      checked={task.completed}
                      disabled={isBusy}
                      aria-label={`Mark ${task.title} ${task.completed ? "incomplete" : "complete"}`}
                      onChange={(event) => void runAction(() =>
                        setTaskCompleted(task.id, event.target.checked)
                      )}
                      className="h-4 w-4 shrink-0 cursor-pointer accent-[#e8ad55] disabled:cursor-not-allowed"
                    />
                    <span className={`min-w-0 flex-1 break-words text-sm ${
                      task.completed ? "text-gray-600 line-through" : "text-gray-200"
                    }`}>
                      {task.title}
                    </span>
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
    </section>
  );
};
