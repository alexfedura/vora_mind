import { useEffect, useState } from "react";
import { useTraceStore } from "../../store/traceStore";
import { findCommitsForFile } from "@/mind/services/traceService";
import type { TraceGitLogEntry, TraceNodeKind, TraceNodeStatus } from "@/types";

const KINDS: TraceNodeKind[] = ["state", "decision", "experiment", "rejected"];
const STATUSES: TraceNodeStatus[] =
  ["pending", "accepted", "rejected", "current", "abandoned"];

export function DecisionEditor() {
  const selectedId = useTraceStore((s) => s.selectedNodeId);
  const node = useTraceStore((s) => (selectedId ? s.getNode(selectedId) : undefined));
  const update = useTraceStore((s) => s.updateNode);
  const del = useTraceStore((s) => s.deleteNode);
  const clear = useTraceStore((s) => s.selectNode);

  const [draft, setDraft] = useState(node);
  const [commitSearch, setCommitSearch] = useState("");
  const [commitResults, setCommitResults] = useState<TraceGitLogEntry[]>([]);
  const [searchingCommits, setSearchingCommits] = useState(false);
  useEffect(() => { setDraft(node); setCommitResults([]); setCommitSearch(""); }, [node]);

  if (!draft || !selectedId) return null;

  const onSave = () => {
    update(selectedId, draft);
    clear(null);
  };

  const onDelete = () => {
    if (!confirm(`Delete ${selectedId}?`)) return;
    del(selectedId);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center
                    bg-black/70 backdrop-blur-sm"
         onClick={() => clear(null)}>
      <div className="max-h-[85vh] w-[720px] overflow-y-auto rounded-xl
                      border border-trace-border bg-trace-panel p-6 shadow-2xl"
           onClick={(e) => e.stopPropagation()}>
        <div className="mb-4 flex items-center justify-between">
          <div>
            <div className="text-xs uppercase tracking-wider text-trace-muted">
              Editing · {selectedId}
            </div>
            <div className="text-xl font-semibold text-trace-text">
              Decision Editor
            </div>
          </div>
          <button onClick={() => clear(null)}
                  className="text-trace-muted hover:text-trace-text">✕</button>
        </div>

        <Field label="Title">
          <input
            value={draft.title}
            onChange={(e) => setDraft({ ...draft, title: e.target.value })}
            className="w-full rounded-md border border-trace-border
                       bg-trace-bg px-3 py-2 text-sm text-trace-text
                       outline-none focus:border-trace-accent"
          />
        </Field>

        <div className="grid grid-cols-2 gap-4">
          <Field label="Kind">
            <select
              value={draft.kind}
              onChange={(e) => {
                const kind = KINDS.find((value) => value === e.target.value);
                if (kind) setDraft({ ...draft, kind });
              }}
              className="w-full rounded-md border border-trace-border
                         bg-trace-bg px-3 py-2 text-sm text-trace-text">
              {KINDS.map((k) => <option key={k} value={k}>{k}</option>)}
            </select>
          </Field>
          <Field label="Status">
            <select
              value={draft.status}
              onChange={(e) => {
                const status = STATUSES.find((value) => value === e.target.value);
                if (status) setDraft({ ...draft, status });
              }}
              className="w-full rounded-md border border-trace-border
                         bg-trace-bg px-3 py-2 text-sm text-trace-text">
              {STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}
            </select>
          </Field>
        </div>

        <Field label="Why am I here?">
          <textarea
            rows={2}
            value={draft.whyAmIHere ?? ""}
            onChange={(e) => setDraft({ ...draft, whyAmIHere: e.target.value })}
            className="w-full rounded-md border border-trace-border
                       bg-trace-bg px-3 py-2 text-sm text-trace-text"
          />
        </Field>

        <Field label="Objective">
          <input
            value={draft.objective ?? ""}
            onChange={(e) => setDraft({ ...draft, objective: e.target.value })}
            className="w-full rounded-md border border-trace-border
                       bg-trace-bg px-3 py-2 text-sm text-trace-text"
          />
        </Field>

        <Field label="Reason">
          <textarea
            rows={3}
            value={draft.reason ?? ""}
            onChange={(e) => setDraft({ ...draft, reason: e.target.value })}
            className="w-full rounded-md border border-trace-border
                       bg-trace-bg px-3 py-2 text-sm text-trace-text"
          />
        </Field>

        <Field label="Linked Commits">
          <div className="space-y-1">
            {(draft.linkedCommits ?? []).map((hash) => (
              <div key={hash}
                   className="flex items-center justify-between rounded
                              bg-trace-bg px-2 py-1 text-xs font-mono
                              text-trace-text/80">
                <span>{hash.slice(0, 8)}</span>
                <button
                  onClick={() => setDraft({
                    ...draft,
                    linkedCommits: (draft.linkedCommits ?? []).filter((h) => h !== hash),
                  })}
                  className="ml-2 text-trace-muted hover:text-red-400">✕</button>
              </div>
            ))}
          </div>
          <div className="mt-2 flex gap-2">
            <input
              placeholder="file pattern (e.g. src/main.rs)"
              value={commitSearch}
              onChange={(e) => setCommitSearch(e.target.value)}
              className="flex-1 rounded-md border border-trace-border
                         bg-trace-bg px-2 py-1 text-xs text-trace-text"
            />
            <button
              disabled={searchingCommits || !commitSearch}
              onClick={async () => {
                if (!draft) return;
                setSearchingCommits(true);
                try {
                  const res = await findCommitsForFile(
                    "D:/Workspace", commitSearch, 10
                  );
                  setCommitResults(res);
                } catch (e) { console.warn(e); }
                setSearchingCommits(false);
              }}
              className="rounded-md border border-trace-border
                         bg-trace-bg px-2 py-1 text-xs text-trace-text/80
                         hover:bg-trace-border disabled:opacity-40">
              {searchingCommits ? "…" : "Find"}
            </button>
          </div>
          {commitResults.length > 0 && (
            <div className="mt-2 max-h-32 overflow-y-auto rounded-md
                            border border-trace-border bg-trace-bg">
              {commitResults.map((c) => (
                <label key={c.hash}
                       className="flex cursor-pointer items-start gap-2
                                  px-2 py-1.5 hover:bg-trace-border">
                  <input
                    type="checkbox"
                    checked={(draft.linkedCommits ?? []).includes(c.hash)}
                    onChange={(e) => {
                      const cur = draft.linkedCommits ?? [];
                      setDraft({
                        ...draft,
                        linkedCommits: e.target.checked
                          ? [...cur, c.hash]
                          : cur.filter((h) => h !== c.hash),
                      });
                    }}
                    className="mt-0.5"
                  />
                  <div>
                    <span className="font-mono text-xs text-trace-accent">
                      {c.short}
                    </span>
                    <span className="ml-2 text-xs text-trace-text/70">
                      {c.message}
                    </span>
                  </div>
                </label>
              ))}
            </div>
          )}
        </Field>

        <div className="mt-4 flex items-center justify-between">
          <button
            onClick={onDelete}
            className="rounded-md border border-red-500/40
                       bg-red-500/10 px-3 py-1.5 text-xs text-red-300
                       hover:bg-red-500/20">
            Delete
          </button>
          <div className="flex gap-2">
            <button
              onClick={() => clear(null)}
              className="rounded-md border border-trace-border
                         bg-trace-bg px-3 py-1.5 text-xs text-trace-text/80
                         hover:bg-trace-border">
              Cancel
            </button>
            <button
              onClick={onSave}
              className="rounded-md border border-trace-accent/60
                         bg-trace-accent/20 px-3 py-1.5 text-xs text-trace-text
                         hover:bg-trace-accent/30">
              Save changes
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="mt-3">
      <div className="mb-1 text-xs uppercase tracking-wider text-trace-muted">
        {label}
      </div>
      {children}
    </div>
  );
}
