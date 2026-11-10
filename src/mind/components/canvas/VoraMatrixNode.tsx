import { memo } from "react";
import { Handle, NodeResizer, Position, useReactFlow, type Node, type NodeProps } from "@xyflow/react";
import { Plus } from "lucide-react";
import type { VoraMatrixData } from "@/mind/types";

type VoraMatrixNodeModel = Node<VoraMatrixData, "voraMatrix">;

const makeId = (kind: "criterion" | "option"): string => `${kind}-${crypto.randomUUID()}`;

export const VoraMatrixNode = memo(({ id, data, selected }: NodeProps<VoraMatrixNodeModel>) => {
  const { updateNodeData } = useReactFlow<VoraMatrixNodeModel>();

  const updateData = (nextData: VoraMatrixData): void => {
    updateNodeData(id, nextData);
  };

  const updateColumn = (
    columnId: string,
    update: (column: VoraMatrixData["columns"][number]) => VoraMatrixData["columns"][number],
  ): void => {
    updateData({
      ...data,
      columns: data.columns.map((column) => column.id === columnId ? update(column) : column),
    });
  };

  const updateRow = (
    rowId: string,
    update: (row: VoraMatrixData["rows"][number]) => VoraMatrixData["rows"][number],
  ): void => {
    updateData({
      ...data,
      rows: data.rows.map((row) => row.id === rowId ? update(row) : row),
    });
  };

  const addColumn = (): void => {
    const columnId = makeId("criterion");
    updateData({
      ...data,
      columns: [...data.columns, { id: columnId, name: `Criterion ${data.columns.length + 1}`, weight: 1 }],
      rows: data.rows.map((row) => ({ ...row, scores: { ...row.scores, [columnId]: 5 } })),
    });
  };

  const addRow = (): void => {
    updateData({
      ...data,
      rows: [
        ...data.rows,
        {
          id: makeId("option"),
          name: `Option ${data.rows.length + 1}`,
          scores: Object.fromEntries(data.columns.map((column) => [column.id, 5])),
        },
      ],
    });
  };

  return (
    <div className="relative flex h-full w-full flex-col rounded-xl border border-[#303543] bg-[#171a22] text-zinc-200 shadow-xl">
      <NodeResizer
        nodeId={id}
        isVisible={selected}
        minWidth={520}
        minHeight={220}
        color="#a78bfa"
      />
      <Handle type="source" position={Position.Left} id="source-left" className={`!z-10 !h-3 !w-3 !-left-1.5 !border-2 !border-[#171a22] !bg-violet-400 ${selected ? "!opacity-100" : "!opacity-0"}`} />
      <Handle type="source" position={Position.Right} id="source-right" className={`!z-10 !h-3 !w-3 !-right-1.5 !border-2 !border-[#171a22] !bg-violet-400 ${selected ? "!opacity-100" : "!opacity-0"}`} />
      <Handle type="source" position={Position.Top} id="source-top" className={`!z-10 !h-3 !w-3 !-top-1.5 !border-2 !border-[#171a22] !bg-violet-400 ${selected ? "!opacity-100" : "!opacity-0"}`} />
      <Handle type="source" position={Position.Bottom} id="source-bottom" className={`!z-10 !h-3 !w-3 !-bottom-1.5 !border-2 !border-[#171a22] !bg-violet-400 ${selected ? "!opacity-100" : "!opacity-0"}`} />
      <Handle type="target" position={Position.Left} id="target-left" className="!z-10 !h-3 !w-3 !-left-1.5 !border-2 !border-[#171a22] !bg-amber-300" />
      <Handle type="target" position={Position.Right} id="target-right" className="!z-10 !h-3 !w-3 !-right-1.5 !border-2 !border-[#171a22] !bg-amber-300" />
      <Handle type="target" position={Position.Top} id="target-top" className="!z-10 !h-3 !w-3 !-top-1.5 !border-2 !border-[#171a22] !bg-amber-300" />
      <Handle type="target" position={Position.Bottom} id="target-bottom" className="!z-10 !h-3 !w-3 !-bottom-1.5 !border-2 !border-[#171a22] !bg-amber-300" />

      <div className="flex shrink-0 items-center gap-2 border-b border-[#303543] bg-[#111318] px-3 py-2">
        <span className="text-[10px] font-semibold uppercase tracking-wide text-violet-300">Decision Matrix</span>
        <input
          aria-label="Decision matrix title"
          value={data.title}
          onChange={(event) => updateData({ ...data, title: event.currentTarget.value })}
          className="nodrag nowheel min-w-0 flex-1 bg-transparent text-sm font-semibold text-zinc-100 outline-none placeholder:text-zinc-500 focus:text-white"
          placeholder="Matrix title"
        />
      </div>

      <div className="nowheel min-h-0 flex-1 overflow-auto">
        <table className="w-full border-collapse text-left text-xs">
          <thead className="sticky top-0 z-[1] bg-[#1d2029] text-[10px] uppercase tracking-wide text-zinc-400">
            <tr>
              <th scope="col" className="min-w-32 border-b border-r border-[#303543] px-2 py-2">Options</th>
              {data.columns.map((column) => (
                <th key={column.id} scope="col" className="min-w-28 border-b border-r border-[#303543] px-2 py-1.5">
                  <input
                    aria-label={`Criterion ${column.name}`}
                    value={column.name}
                    onChange={(event) => updateColumn(column.id, (current) => ({
                      ...current,
                      name: event.currentTarget.value,
                    }))}
                    className="nodrag nowheel mb-1 w-full bg-transparent text-[10px] font-semibold uppercase tracking-wide text-zinc-200 outline-none focus:text-white"
                  />
                  <label className="flex items-center gap-1 normal-case tracking-normal text-zinc-500">
                    Weight
                    <input
                      type="number"
                      min={0}
                      max={100}
                      step={1}
                      aria-label={`Weight for ${column.name}`}
                      value={column.weight}
                      onChange={(event) => {
                        const weight = event.currentTarget.valueAsNumber;
                        if (Number.isFinite(weight)) {
                          updateColumn(column.id, (current) => ({ ...current, weight: Math.min(100, Math.max(0, weight)) }));
                        }
                      }}
                      className="nodrag nowheel w-14 rounded border border-[#353a48] bg-[#111318] px-1 py-0.5 text-right text-zinc-100 outline-none focus:border-violet-400"
                    />
                  </label>
                </th>
              ))}
              <th scope="col" className="min-w-24 border-b border-[#303543] px-2 py-2 text-right">Total Score</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#292d37]">
            {data.rows.map((row) => {
              const total = data.columns.reduce(
                (sum, column) => sum + (row.scores[column.id] ?? 0) * column.weight,
                0,
              );
              return (
                <tr key={row.id} className="bg-[#171a22]">
                  <th scope="row" className="border-r border-[#292d37] p-1.5">
                    <input
                      aria-label={`Option ${row.name}`}
                      value={row.name}
                      onChange={(event) => updateRow(row.id, (current) => ({
                        ...current,
                        name: event.currentTarget.value,
                      }))}
                      className="nodrag nowheel w-full rounded bg-transparent px-1 py-1 text-left font-medium text-zinc-100 outline-none focus:bg-[#222632]"
                    />
                  </th>
                  {data.columns.map((column) => (
                    <td key={column.id} className="border-r border-[#292d37] p-1.5">
                      <input
                        type="number"
                        min={1}
                        max={10}
                        step={1}
                        aria-label={`${row.name} score for ${column.name}`}
                        value={row.scores[column.id] ?? 5}
                        onChange={(event) => {
                          const score = event.currentTarget.valueAsNumber;
                          if (Number.isInteger(score) && score >= 1 && score <= 10) {
                            updateRow(row.id, (current) => ({
                              ...current,
                              scores: { ...current.scores, [column.id]: score },
                            }));
                          }
                        }}
                        className="nodrag nowheel w-full rounded border border-[#353a48] bg-[#111318] px-1.5 py-1 text-center text-zinc-100 outline-none focus:border-violet-400"
                      />
                    </td>
                  ))}
                  <td className="px-2 py-1.5 text-right font-semibold tabular-nums text-emerald-300">
                    {total.toFixed(2)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="flex shrink-0 gap-2 border-t border-[#303543] px-2 py-1.5">
        <button
          type="button"
          onClick={addColumn}
          className="nodrag nowheel inline-flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium text-violet-300 transition hover:bg-violet-500/10 hover:text-violet-200"
        >
          <Plus size={12} aria-hidden="true" />
          Add Column
        </button>
        <button
          type="button"
          onClick={addRow}
          className="nodrag nowheel inline-flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium text-violet-300 transition hover:bg-violet-500/10 hover:text-violet-200"
        >
          <Plus size={12} aria-hidden="true" />
          Add Row
        </button>
      </div>
    </div>
  );
});

VoraMatrixNode.displayName = "VoraMatrixNode";
