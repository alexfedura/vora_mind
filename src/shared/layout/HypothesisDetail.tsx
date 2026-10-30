import React, { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Trash2 } from "lucide-react";
import type {
  ChecklistField,
  ChecklistFieldType,
  DataEntry,
  Hypothesis,
  HypothesisInsight,
  HypothesisVerdict,
} from "@/types";
import type { VoraSnapshot } from "@/lib/snapshot/types";
import { listSnapshots } from "@/lib/snapshot/storage";
import { ChildHypothesisModal } from "@/shared/layout/ChildHypothesisModal";
import { useAppStore } from "@/shared/store/useAppStore";

interface HypothesisDetailProps {
  hypothesis: Hypothesis;
  onUpdated: (hypothesis: Hypothesis) => void;
  onChildCreated: () => void;
}

const verdictPresentation: Record<
  HypothesisVerdict,
  { label: string; className: string }
> = {
  supported: {
    label: "Supported",
    className: "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
  },
  partially_supported: {
    label: "Partially Supported",
    className: "border-amber-500/30 bg-amber-500/10 text-amber-200",
  },
  rejected: {
    label: "Rejected",
    className: "border-rose-500/30 bg-rose-500/10 text-rose-300",
  },
  inconclusive: {
    label: "Inconclusive",
    className: "border-gray-600 bg-gray-700/40 text-gray-300",
  },
};

function formatTraceTimestamp(timestamp: number | null | undefined): string {
  if (timestamp == null) return "Pending";
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? "Invalid timestamp" : date.toLocaleString();
}

const EMPTY_INSIGHTS: HypothesisInsight[] = [];
const EMPTY_DATA_ENTRIES: DataEntry[] = [];
let checklistDraftSequence = 0;

function createChecklistFieldDraft(): ChecklistFieldDraft {
  checklistDraftSequence += 1;
  return { id: `checklist-field-${checklistDraftSequence}`, label: "", type: "select", optionsText: "" };
}

interface ChecklistFieldDraft {
  id: string;
  label: string;
  type: ChecklistFieldType;
  optionsText: string;
}

interface ParsedDataEntry {
  entry: DataEntry;
  values: Record<string, unknown>;
}

interface ChecklistFieldAnalysis {
  label: string;
  counts: Array<{ value: string; count: number }>;
  numeric?: { count: number; min: number; average: number; max: number };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseChecklistSchema(
  schemaJson: string | null | undefined,
): { fields: ChecklistField[] | null; error: string | null } {
  if (schemaJson == null) return { fields: null, error: null };
  try {
    const parsed: unknown = JSON.parse(schemaJson);
    if (!Array.isArray(parsed) || parsed.length === 0) {
      return { fields: null, error: "The saved checklist schema is not a non-empty field list." };
    }
    const fields: ChecklistField[] = [];
    const labels = new Set<string>();
    for (const item of parsed) {
      if (!isRecord(item) || typeof item.label !== "string" || !item.label.trim()) {
        return { fields: null, error: "The saved checklist schema contains a field without a label." };
      }
      if (labels.has(item.label)) {
        return { fields: null, error: `The saved checklist contains duplicate field "${item.label}".` };
      }
      labels.add(item.label);
      if (item.type !== "select" && item.type !== "number" && item.type !== "text" && item.type !== "boolean") {
        return { fields: null, error: `The saved checklist field "${item.label}" has an unsupported type.` };
      }
      let options: string[] = [];
      if (item.type === "select") {
        if (!Array.isArray(item.options) || item.options.length === 0
          || !item.options.every((option): option is string => typeof option === "string" && option.trim().length > 0)) {
          return { fields: null, error: `The saved select field "${item.label}" has invalid options.` };
        }
        options = item.options;
      }
      fields.push({ label: item.label, type: item.type, options });
    }
    return { fields, error: null };
  } catch (cause) {
    return {
      fields: null,
      error: `Could not parse the saved checklist schema: ${cause instanceof Error ? cause.message : String(cause)}`,
    };
  }
}

function formatChecklistValue(value: unknown): string {
  if (value === null || value === undefined || value === "") return "—";
  if (typeof value === "boolean") return value ? "Yes" : "No";
  if (typeof value === "string" || typeof value === "number") return String(value);
  return JSON.stringify(value) ?? "—";
}

function analyzeChecklistEntries(
  fields: ChecklistField[],
  entries: ParsedDataEntry[],
): ChecklistFieldAnalysis[] {
  return fields.map((field) => {
    const counts = new Map<string, number>();
    const numericValues: number[] = [];
    for (const { values } of entries) {
      const value = values[field.label];
      const key = formatChecklistValue(value);
      counts.set(key, (counts.get(key) ?? 0) + 1);
      if (typeof value === "number" && Number.isFinite(value)) numericValues.push(value);
    }
    const result: ChecklistFieldAnalysis = {
      label: field.label,
      counts: [...counts.entries()]
        .map(([value, count]) => ({ value, count }))
        .sort((left, right) => right.count - left.count || left.value.localeCompare(right.value)),
    };
    if (field.type === "number" && numericValues.length > 0) {
      result.numeric = {
        count: numericValues.length,
        min: Math.min(...numericValues),
        average: numericValues.reduce((sum, value) => sum + value, 0) / numericValues.length,
        max: Math.max(...numericValues),
      };
    }
    return result;
  });
}

export const HypothesisDetail: React.FC<HypothesisDetailProps> = ({
  hypothesis,
  onUpdated,
  onChildCreated,
}) => {
  const navigate = useNavigate();
  const recordVerdict = useAppStore((state) => state.recordVerdict);
  const saveHypothesisProtocol = useAppStore((state) => state.saveHypothesisProtocol);
  const recordHypothesisExecution = useAppStore((state) => state.recordHypothesisExecution);
  const saveChecklistSchema = useAppStore((state) => state.saveChecklistSchema);
  const loadDataEntries = useAppStore((state) => state.loadDataEntries);
  const addDataEntry = useAppStore((state) => state.addDataEntry);
  const deleteDataEntry = useAppStore((state) => state.deleteDataEntry);
  const graduateToLibrary = useAppStore((state) => state.graduateToLibrary);
  const loadHypothesisInsights = useAppStore((state) => state.loadHypothesisInsights);
  const addHypothesisInsight = useAppStore((state) => state.addHypothesisInsight);
  const createQuestion = useAppStore((state) => state.createQuestion);
  const linkSnapshotToHypothesis = useAppStore((state) => state.linkSnapshotToHypothesis);
  const unlinkSnapshotFromHypothesis = useAppStore((state) => state.unlinkSnapshotFromHypothesis);
  const setLabReturnState = useAppStore((state) => state.setLabReturnState);
  const refinedFromHypothesis = useAppStore((state) =>
    hypothesis.refinedFromId
      ? state.hypotheses.find((item) => item.id === hypothesis.refinedFromId) ?? null
      : null
  );
  const insights = useAppStore(
    (state) => state.hypothesisInsights[hypothesis.id] ?? EMPTY_INSIGHTS,
  );
  const dataEntries = useAppStore(
    (state) => state.hypothesisDataEntries[hypothesis.id] ?? EMPTY_DATA_ENTRIES,
  );
  const checklistSchema = useMemo(
    () => parseChecklistSchema(hypothesis.checklistSchema),
    [hypothesis.checklistSchema],
  );
  const parsedDataEntries = useMemo(() => {
    try {
      const entries: ParsedDataEntry[] = dataEntries.map((entry) => {
        const values: unknown = JSON.parse(entry.dataJson);
        if (!isRecord(values)) throw new Error(`Entry ${entry.id} is not a JSON object.`);
        return { entry, values };
      });
      return { entries, error: null };
    } catch (cause) {
      return {
        entries: [] as ParsedDataEntry[],
        error: cause instanceof Error ? cause.message : String(cause),
      };
    }
  }, [dataEntries]);
  const checklistAnalysis = useMemo(
    () => checklistSchema.fields
      ? analyzeChecklistEntries(checklistSchema.fields, parsedDataEntries.entries)
      : [],
    [checklistSchema.fields, parsedDataEntries.entries],
  );
  const [evidenceNotes, setEvidenceNotes] = useState(hypothesis.evidenceNotes ?? "");
  const [protocolSteps, setProtocolSteps] = useState(hypothesis.protocolSteps ?? "");
  const [rawData, setRawData] = useState(hypothesis.rawData ?? "");
  const [dataAnalysis, setDataAnalysis] = useState(hypothesis.dataAnalysis ?? "");
  const [boundaryConditions, setBoundaryConditions] = useState(
    hypothesis.boundaryConditions ?? "",
  );
  const [isEditingVerdict, setIsEditingVerdict] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isCreatingChild, setIsCreatingChild] = useState(false);
  const [childInsight, setChildInsight] = useState<HypothesisInsight | null>(null);
  const [insightContent, setInsightContent] = useState("");
  const [isCapturingInsight, setIsCapturingInsight] = useState(false);
  const [sendingInsightId, setSendingInsightId] = useState<string | null>(null);
  const [queuedInsightIds, setQueuedInsightIds] = useState<string[]>([]);
  const [scratchpadError, setScratchpadError] = useState<string | null>(null);
  const [snapshots, setSnapshots] = useState<VoraSnapshot[]>([]);
  const [isSnapshotPickerOpen, setIsSnapshotPickerOpen] = useState(false);
  const [isLoadingSnapshots, setIsLoadingSnapshots] = useState(true);
  const [snapshotError, setSnapshotError] = useState<string | null>(null);
  const [isChecklistBuilderOpen, setIsChecklistBuilderOpen] = useState(false);
  const [checklistDraftFields, setChecklistDraftFields] = useState<ChecklistFieldDraft[]>([]);
  const [isSavingChecklistSchema, setIsSavingChecklistSchema] = useState(false);
  const [checklistValues, setChecklistValues] = useState<Record<string, string | boolean>>({});
  const [isSavingDataEntry, setIsSavingDataEntry] = useState(false);
  const [deletingDataEntryId, setDeletingDataEntryId] = useState<string | null>(null);
  const [dataCollectionError, setDataCollectionError] = useState<string | null>(null);
  const [showChecklistAnalysis, setShowChecklistAnalysis] = useState(false);
  const dataEntryFormRef = useRef<HTMLFormElement | null>(null);
  const isProtocolLocked = hypothesis.protocolLockedAt != null;
  const isExecuted = hypothesis.executedAt != null;

  useEffect(() => {
    setEvidenceNotes(hypothesis.evidenceNotes ?? "");
    setProtocolSteps(hypothesis.protocolSteps ?? "");
    setRawData(hypothesis.rawData ?? "");
    setDataAnalysis(hypothesis.dataAnalysis ?? "");
    setBoundaryConditions(hypothesis.boundaryConditions ?? "");
  }, [
    hypothesis.id,
    hypothesis.evidenceNotes,
    hypothesis.protocolSteps,
    hypothesis.rawData,
    hypothesis.dataAnalysis,
    hypothesis.boundaryConditions,
  ]);

  useEffect(() => {
    setScratchpadError(null);
    void loadHypothesisInsights(hypothesis.id).catch((cause: unknown) => {
      console.error("Could not load hypothesis scratchpad insights:", cause);
      setScratchpadError(cause instanceof Error ? cause.message : String(cause));
    });
  }, [hypothesis.id, loadHypothesisInsights]);

  useEffect(() => {
    if (!isProtocolLocked || hypothesis.checklistSchema == null) return;
    setDataCollectionError(null);
    void loadDataEntries(hypothesis.id).catch((cause: unknown) => {
      console.error("Could not load hypothesis data entries:", cause);
      setDataCollectionError(cause instanceof Error ? cause.message : String(cause));
    });
  }, [hypothesis.id, hypothesis.checklistSchema, isProtocolLocked, loadDataEntries]);

  useEffect(() => {
    let cancelled = false;
    void listSnapshots()
      .then((items) => {
        if (!cancelled) setSnapshots(items);
      })
      .catch((cause: unknown) => {
        console.error("Could not load chart snapshots:", cause);
        if (!cancelled) setSnapshotError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (!cancelled) setIsLoadingSnapshots(false);
      });
    return () => { cancelled = true; };
  }, []);

  const handleLinkSnapshot = async (snapshotId: string): Promise<void> => {
    setSnapshotError(null);
    try {
      await linkSnapshotToHypothesis(hypothesis.id, snapshotId);
      const linkedSnapshots = hypothesis.linkedSnapshots.includes(snapshotId)
        ? hypothesis.linkedSnapshots
        : [...hypothesis.linkedSnapshots, snapshotId];
      onUpdated({ ...hypothesis, linkedSnapshots });
      setIsSnapshotPickerOpen(false);
    } catch (cause) {
      console.error("Could not link chart snapshot:", cause);
      setSnapshotError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const handleUnlinkSnapshot = async (snapshotId: string): Promise<void> => {
    setSnapshotError(null);
    try {
      await unlinkSnapshotFromHypothesis(hypothesis.id, snapshotId);
      onUpdated({
        ...hypothesis,
        linkedSnapshots: hypothesis.linkedSnapshots.filter((id) => id !== snapshotId),
      });
    } catch (cause) {
      console.error("Could not unlink chart snapshot:", cause);
      setSnapshotError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const handleOpenSnapshot = (snapshotId: string): void => {
    setLabReturnState({ hypothesisId: hypothesis.id, tab: "lab" });
    navigate(`/chart?snapshot=${encodeURIComponent(snapshotId)}`, {
      state: { snapshotId },
    });
  };

  const handleCaptureInsight = async (): Promise<void> => {
    if (!insightContent.trim()) return;
    setIsCapturingInsight(true);
    setScratchpadError(null);
    try {
      await addHypothesisInsight(hypothesis.id, insightContent);
      setInsightContent("");
    } catch (cause) {
      console.error("Could not capture hypothesis insight:", cause);
      setScratchpadError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsCapturingInsight(false);
    }
  };

  const handleSendInsightToInquiry = async (
    insight: HypothesisInsight,
  ): Promise<void> => {
    const content = insight.content.trim();
    if (!content) return;
    const firstLine = content.split(/\r?\n/, 1)[0].trim();
    const title = Array.from(firstLine || content).slice(0, 200).join("");
    setSendingInsightId(insight.id);
    setScratchpadError(null);
    try {
      await createQuestion(title, content);
      setQueuedInsightIds((current) =>
        current.includes(insight.id) ? current : [...current, insight.id]
      );
    } catch (cause) {
      console.error("Could not send hypothesis insight to Inquiry Queue:", cause);
      setScratchpadError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSendingInsightId(null);
    }
  };

  const handleSaveProtocol = async (): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const updated = await saveHypothesisProtocol(hypothesis.id, protocolSteps);
      setProtocolSteps(updated.protocolSteps ?? "");
      onUpdated(updated);
    } catch (cause) {
      console.error("Could not save hypothesis protocol:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  const handleRecordExecution = async (): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const updated = await recordHypothesisExecution(hypothesis.id, rawData);
      setRawData(updated.rawData ?? "");
      onUpdated(updated);
    } catch (cause) {
      console.error("Could not record hypothesis test execution:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveChecklistSchema = async (): Promise<void> => {
    const labels = new Set<string>();
    const fields: ChecklistField[] = [];
    for (const draft of checklistDraftFields) {
      const label = draft.label.trim();
      if (!label) {
        setDataCollectionError("Give every checklist field a label.");
        return;
      }
      if (labels.has(label)) {
        setDataCollectionError(`Checklist field labels must be unique: ${label}`);
        return;
      }
      labels.add(label);
      const options = draft.type === "select"
        ? draft.optionsText.split(",").map((option) => option.trim()).filter(Boolean)
        : [];
      if (draft.type === "select" && options.length === 0) {
        setDataCollectionError(`Add at least one option for "${label}".`);
        return;
      }
      fields.push({ label, type: draft.type, options });
    }
    if (fields.length === 0) {
      setDataCollectionError("Add at least one checklist field.");
      return;
    }

    setIsSavingChecklistSchema(true);
    setDataCollectionError(null);
    try {
      const updated = await saveChecklistSchema(hypothesis.id, fields);
      onUpdated(updated);
      setChecklistValues({});
      setShowChecklistAnalysis(false);
      setIsChecklistBuilderOpen(false);
    } catch (cause) {
      console.error("Could not save hypothesis checklist schema:", cause);
      setDataCollectionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSavingChecklistSchema(false);
    }
  };

  const handleAddDataEntry = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const fields = checklistSchema.fields;
    if (!fields) return;
    const values: Record<string, string | number | boolean> = {};
    for (const field of fields) {
      const value = checklistValues[field.label];
      if (field.type === "boolean") {
        if (value !== undefined && typeof value !== "boolean") {
          setDataCollectionError(`Choose a value for "${field.label}".`);
          return;
        }
        values[field.label] = value === true;
      } else if (field.type === "number") {
        if (typeof value !== "string" || value.trim() === "" || !Number.isFinite(Number(value))) {
          setDataCollectionError(`Enter a valid number for "${field.label}".`);
          return;
        }
        values[field.label] = Number(value);
      } else {
        if (typeof value !== "string") {
          setDataCollectionError(`Enter a value for "${field.label}".`);
          return;
        }
        if (field.type === "select" && !field.options?.includes(value)) {
          setDataCollectionError(`Choose an option for "${field.label}".`);
          return;
        }
        values[field.label] = value;
      }
    }

    setIsSavingDataEntry(true);
    setDataCollectionError(null);
    try {
      await addDataEntry(hypothesis.id, JSON.stringify(values));
      setChecklistValues({});
      setShowChecklistAnalysis(false);
      window.requestAnimationFrame(() => {
        dataEntryFormRef.current?.querySelector<HTMLElement>("input, select, textarea")?.focus();
      });
    } catch (cause) {
      console.error("Could not save hypothesis checklist entry:", cause);
      setDataCollectionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSavingDataEntry(false);
    }
  };

  const handleDeleteDataEntry = async (id: string): Promise<void> => {
    setDeletingDataEntryId(id);
    setDataCollectionError(null);
    try {
      await deleteDataEntry(hypothesis.id, id);
    } catch (cause) {
      console.error("Could not delete hypothesis data entry:", cause);
      setDataCollectionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDeletingDataEntryId(null);
    }
  };

  const handleGraduate = async (): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const updated = await graduateToLibrary(hypothesis.id);
      onUpdated(updated);
    } catch (cause) {
      console.error("Could not graduate hypothesis to Library:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  const handleRecordVerdict = async (verdict: HypothesisVerdict): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const updated = await recordVerdict(
        hypothesis.id,
        evidenceNotes,
        verdict,
        rawData,
        dataAnalysis,
        boundaryConditions,
      );
      onUpdated(updated);
      setIsEditingVerdict(false);
    } catch (cause) {
      console.error("Could not record hypothesis verdict:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <article className="mx-auto w-full max-w-4xl rounded-2xl border border-[#292c34] bg-[#14161b] p-5 sm:p-7">
      <h2 className="text-xl font-semibold text-white">Hypothesis</h2>
      {hypothesis.parentHypothesisId && (
        <p className="mt-2 text-xs text-gray-500">
          This is an iteration of a previous hypothesis.
        </p>
      )}
      {hypothesis.refinedFromId && (
        <p
          dir="auto"
          className="mt-2 inline-flex max-w-full rounded-full border border-sky-500/30 bg-sky-500/10 px-3 py-1.5 text-xs text-sky-200"
          title={refinedFromHypothesis?.statementH1 ?? hypothesis.refinedFromId}
        >
          🔗 Refined from: {refinedFromHypothesis?.statementH1 ?? hypothesis.refinedFromId}
        </p>
      )}

      <section dir="auto" aria-label="Hypothesis trace" className="mt-5 rounded-xl border border-[#292c34] bg-[#101114] p-4">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-400">
          Trace
        </h3>
        <ol className="mt-3 grid gap-3 sm:grid-cols-4">
          {[
            { label: "Created", timestamp: hypothesis.createdAt },
            { label: "Protocol Locked", timestamp: hypothesis.protocolLockedAt },
            { label: "Test Executed", timestamp: hypothesis.executedAt },
            { label: "Verdict Recorded", timestamp: hypothesis.evaluatedAt },
          ].map((step) => (
            <li key={step.label} className="min-w-0 border-l-2 border-[#343741] pl-3">
              <span className="block text-xs font-medium text-gray-300">{step.label}</span>
              <time
                className={`mt-1 block text-xs ${step.timestamp == null ? "text-gray-600" : "text-gray-400"}`}
                dateTime={step.timestamp == null ? undefined : new Date(step.timestamp).toISOString()}
              >
                {formatTraceTimestamp(step.timestamp)}
              </time>
            </li>
          ))}
        </ol>
      </section>

      {error && (
        <p role="alert" className="mt-4 text-sm text-rose-300">
          {error}
        </p>
      )}

      <section dir="auto" className="mt-6 border-t border-[#292c34] pt-5">
        <h3 className="text-base font-semibold text-white">1. Design &amp; Protocol</h3>
        <dl className="mt-4 grid gap-4 sm:grid-cols-2">
          <div className="rounded-xl bg-[#101114] p-4 sm:col-span-2">
            <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">H1</dt>
            <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
              {hypothesis.statementH1}
            </dd>
          </div>
          <div className="rounded-xl bg-[#101114] p-4 sm:col-span-2">
            <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">H0</dt>
            <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
              {hypothesis.statementH0}
            </dd>
          </div>
          <div className="rounded-xl bg-[#101114] p-4">
            <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">
              Independent variable
            </dt>
            <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
              {hypothesis.independentVar}
            </dd>
          </div>
          <div className="rounded-xl bg-[#101114] p-4">
            <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">
              Dependent variable
            </dt>
            <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
              {hypothesis.dependentVar}
            </dd>
          </div>
          <div className="rounded-xl bg-[#101114] p-4">
            <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">Metric</dt>
            <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
              {hypothesis.metric}
            </dd>
          </div>
          <div className="rounded-xl bg-[#101114] p-4">
            <dt className="text-xs font-semibold uppercase tracking-wide text-gray-500">
              Success threshold
            </dt>
            <dd dir="auto" className="mt-2 whitespace-pre-wrap text-sm text-gray-200">
              {hypothesis.successThreshold}
            </dd>
          </div>
        </dl>

        <div className="mt-5">
          <h4 className="text-sm font-semibold text-white">Linked Definitions</h4>
          {hypothesis.linkedDefinitions.length > 0 ? (
            <ul className="mt-3 flex flex-wrap gap-2">
              {hypothesis.linkedDefinitions.map((definition) => (
                <li
                  key={definition.libraryGroupId}
                  dir="auto"
                  className="rounded-full border border-[#343741] bg-[#101114] px-3 py-1.5 text-sm text-gray-300"
                >
                  📚 {definition.definitionName} (v{definition.versionString})
                </li>
              ))}
            </ul>
          ) : (
            <p className="mt-2 text-sm text-gray-500">
              No definitions linked to this hypothesis.
            </p>
          )}
        </div>

        <label dir="auto" className="mt-5 block">
          <span className="mb-2 block text-xs font-medium text-gray-400">Protocol Steps</span>
          <textarea
            dir="auto"
            value={protocolSteps}
            onChange={(event) => setProtocolSteps(event.target.value)}
            rows={5}
            readOnly={isProtocolLocked}
            disabled={isSaving}
            placeholder="Write the exact procedure before collecting data..."
            className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 read-only:text-gray-400 disabled:opacity-60"
          />
        </label>
        {isProtocolLocked ? (
          <p className="mt-2 text-xs text-emerald-300">
            🔒 Protocol locked on {formatTraceTimestamp(hypothesis.protocolLockedAt)}.
            It cannot be changed.
          </p>
        ) : (
          <button
            type="button"
            disabled={isSaving || protocolSteps.trim().length === 0}
            onClick={() => void handleSaveProtocol()}
            className="mt-3 rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2.5 text-sm font-semibold text-gray-950 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isSaving ? "Saving..." : "💾 Save & Lock Protocol"}
          </button>
        )}
        {hypothesis.verdict && !isExecuted && (
          <div className="mt-4 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
            <p className="text-xs font-medium text-amber-200">
              A historical verdict was recorded before execution tracking was introduced.
            </p>
            <p dir="auto" className="mt-2 text-sm text-gray-300">
              {verdictPresentation[hypothesis.verdict].label}: {hypothesis.evidenceNotes || "No evidence notes recorded."}
            </p>
          </div>
        )}
      </section>

      {isProtocolLocked && (
        <section dir="auto" className="mt-6 border-t border-[#292c34] pt-5">
          <h3 className="text-base font-semibold text-white">2. Execution &amp; Data</h3>
          <label dir="auto" className="mt-4 block">
            <span className="mb-2 block text-xs font-medium text-gray-400">Raw Data</span>
            <textarea
              dir="auto"
              value={rawData}
              onChange={(event) => setRawData(event.target.value)}
              rows={6}
              readOnly={isExecuted}
              disabled={isSaving}
              placeholder="Paste raw CSV, JSON, observations, or a data link..."
              className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 font-mono text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 read-only:text-gray-400 disabled:opacity-60"
            />
          </label>
          {isExecuted ? (
            <p className="mt-2 text-xs text-emerald-300">
              📊 Execution recorded on {formatTraceTimestamp(hypothesis.executedAt)}. Raw data is locked.
            </p>
          ) : (
            <button
              type="button"
              disabled={isSaving}
              onClick={() => void handleRecordExecution()}
              className="mt-3 rounded-lg border border-sky-500/30 bg-sky-500/10 px-4 py-2.5 text-sm font-semibold text-sky-200 transition hover:bg-sky-500/20 disabled:opacity-50"
            >
              {isSaving ? "Recording..." : "📊 Record Test Execution"}
            </button>
          )}

          <div dir="auto" className="mt-6 border-t border-[#292c34] pt-5">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h4 className="text-base font-semibold text-white">Data Collection (Checklist)</h4>
                <p className="mt-1 text-sm text-gray-400">
                  Log observations while the locked protocol is in progress.
                </p>
              </div>
              {checklistSchema.fields && (
                <span className="rounded-full border border-sky-500/30 bg-sky-500/10 px-3 py-1.5 text-sm font-semibold text-sky-200">
                  {dataEntries.length} {dataEntries.length === 1 ? "entry" : "entries"} collected
                </span>
              )}
            </div>

            {hypothesis.checklistSchema == null ? (
              <button
                type="button"
                dir="auto"
                onClick={() => {
                  setChecklistDraftFields([createChecklistFieldDraft()]);
                  setDataCollectionError(null);
                  setIsChecklistBuilderOpen(true);
                }}
                className="mt-4 rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2.5 text-sm font-semibold text-gray-950 transition hover:bg-amber-400"
              >
                ⚙️ Define Checklist
              </button>
            ) : checklistSchema.error ? (
              <p role="alert" className="mt-4 text-sm text-rose-300">{checklistSchema.error}</p>
            ) : checklistSchema.fields ? (
              <>
                <form
                  ref={dataEntryFormRef}
                  dir="auto"
                  onSubmit={(event) => void handleAddDataEntry(event)}
                  className="mt-4 rounded-xl border border-[#292c34] bg-[#101114] p-4"
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    {checklistSchema.fields.map((field, index) => {
                      const inputId = `hypothesis-entry-${hypothesis.id}-${index}`;
                      return (
                        <label key={field.label} htmlFor={inputId} dir="auto" className="block">
                          <span className="mb-1.5 block text-xs font-medium text-gray-400">{field.label}</span>
                          {field.type === "select" ? (
                            <select
                              id={inputId}
                              dir="auto"
                              required
                              value={typeof checklistValues[field.label] === "string" ? checklistValues[field.label] : ""}
                              onChange={(event) => setChecklistValues((current) => ({
                                ...current,
                                [field.label]: event.currentTarget.value,
                              }))}
                              disabled={isSavingDataEntry}
                              className="w-full rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2 text-sm text-gray-200 outline-none focus:border-amber-400/70 disabled:opacity-60"
                            >
                              <option value="">Select…</option>
                              {(field.options ?? []).map((option) => (
                                <option key={option} dir="auto" value={option}>{option}</option>
                              ))}
                            </select>
                          ) : field.type === "boolean" ? (
                            <span className="flex min-h-10 items-center gap-2 rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2">
                              <input
                                id={inputId}
                                dir="auto"
                                type="checkbox"
                                checked={checklistValues[field.label] === true}
                                onChange={(event) => setChecklistValues((current) => ({
                                  ...current,
                                  [field.label]: event.currentTarget.checked,
                                }))}
                                disabled={isSavingDataEntry}
                                className="accent-amber-400"
                              />
                              <span className="text-sm text-gray-300">Yes</span>
                            </span>
                          ) : (
                            <input
                              id={inputId}
                              dir="auto"
                              type={field.type === "number" ? "number" : "text"}
                              step={field.type === "number" ? "any" : undefined}
                              required={field.type === "number"}
                              value={typeof checklistValues[field.label] === "string" ? checklistValues[field.label] : ""}
                              onChange={(event) => setChecklistValues((current) => ({
                                ...current,
                                [field.label]: event.currentTarget.value,
                              }))}
                              disabled={isSavingDataEntry}
                              className="w-full rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2 text-sm text-gray-200 outline-none focus:border-amber-400/70 disabled:opacity-60"
                            />
                          )}
                        </label>
                      );
                    })}
                  </div>
                  <button
                    type="submit"
                    dir="auto"
                    disabled={isSavingDataEntry}
                    className="mt-4 rounded-lg border border-emerald-400/50 bg-emerald-400 px-5 py-3 text-base font-bold text-gray-950 shadow-lg shadow-emerald-950/20 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {isSavingDataEntry ? "Saving…" : "💾 Save & Add Next"}
                  </button>
                </form>

                <div className="mt-5">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <h5 className="text-sm font-semibold text-white">Collected Data</h5>
                    <button
                      type="button"
                      dir="auto"
                      disabled={dataEntries.length === 0 || parsedDataEntries.error !== null}
                      onClick={() => {
                        setDataCollectionError(null);
                        setShowChecklistAnalysis((visible) => !visible);
                      }}
                      className="rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-2 text-sm font-semibold text-sky-200 transition hover:bg-sky-500/20 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {showChecklistAnalysis ? "Hide Analysis" : "📊 Analyze Data"}
                    </button>
                  </div>

                  {showChecklistAnalysis && checklistAnalysis.length > 0 && (
                    <div className="mt-3 rounded-xl border border-sky-500/20 bg-sky-500/5 p-4">
                      <h6 className="text-sm font-semibold text-sky-100">Data Summary</h6>
                      <ul className="mt-3 space-y-3">
                        {checklistAnalysis.map((summary) => (
                          <li key={summary.label} dir="auto" className="text-sm text-gray-300">
                            <span className="font-semibold text-gray-100">{summary.label}:</span>{" "}
                            {summary.counts.length > 0
                              ? summary.counts.map(({ value, count }) => `${value} (${count})`).join(" · ")
                              : "No values"}
                            {summary.numeric && (
                              <span className="ml-2 text-xs text-gray-400">
                                n={summary.numeric.count}, min={summary.numeric.min}, avg={summary.numeric.average.toFixed(2)}, max={summary.numeric.max}
                              </span>
                            )}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {parsedDataEntries.error && (
                    <p role="alert" className="mt-3 text-sm text-rose-300">
                      Could not read collected entry data: {parsedDataEntries.error}
                    </p>
                  )}

                  {parsedDataEntries.entries.length > 0 ? (
                    <div className="mt-3 overflow-x-auto rounded-xl border border-[#292c34]">
                      <table className="w-full min-w-[640px] border-collapse text-left text-sm">
                        <thead className="bg-[#101114] text-xs uppercase text-gray-400">
                          <tr>
                            {checklistSchema.fields.map((field) => (
                              <th key={field.label} dir="auto" scope="col" className="px-3 py-2.5 font-semibold">{field.label}</th>
                            ))}
                            <th scope="col" className="px-3 py-2.5 font-semibold">Collected</th>
                            <th scope="col" className="px-3 py-2.5"><span className="sr-only">Actions</span></th>
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-[#292c34]">
                          {parsedDataEntries.entries.map(({ entry, values }) => (
                            <tr key={entry.id} className="bg-[#14161b] text-gray-300">
                              {checklistSchema.fields?.map((field) => (
                                <td key={field.label} dir="auto" className="max-w-[18rem] truncate px-3 py-2.5" title={formatChecklistValue(values[field.label])}>
                                  {formatChecklistValue(values[field.label])}
                                </td>
                              ))}
                              <td className="whitespace-nowrap px-3 py-2.5 text-xs text-gray-500">{formatTraceTimestamp(entry.createdAt)}</td>
                              <td className="px-3 py-2.5 text-right">
                                <button
                                  type="button"
                                  dir="auto"
                                  aria-label="Delete data entry"
                                  title="Delete data entry"
                                  disabled={deletingDataEntryId === entry.id}
                                  onClick={() => void handleDeleteDataEntry(entry.id)}
                                  className="rounded p-1.5 text-gray-500 transition hover:bg-rose-500/10 hover:text-rose-300 disabled:opacity-50"
                                >
                                  <Trash2 size={14} />
                                </button>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  ) : (
                    <p className="mt-3 text-sm text-gray-500">No data entries collected yet.</p>
                  )}
                </div>
              </>
            ) : null}
            {dataCollectionError && (
              <p role="alert" className="mt-3 text-sm text-rose-300">{dataCollectionError}</p>
            )}
          </div>

          <div dir="auto" className="mt-5 border-t border-[#292c34] pt-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h4 className="text-sm font-semibold text-white">
                Linked Chart Snapshots (Visual Evidence)
              </h4>
              <button
                type="button"
                onClick={() => setIsSnapshotPickerOpen(true)}
                className="rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-2 text-sm font-semibold text-sky-200 transition hover:bg-sky-500/20"
              >
                + Link Snapshot
              </button>
            </div>
            {snapshotError && (
              <p role="alert" className="mt-3 text-sm text-rose-300">{snapshotError}</p>
            )}
            {hypothesis.linkedSnapshots.length > 0 ? (
              <ul className="mt-3 flex flex-wrap gap-2">
                {hypothesis.linkedSnapshots.map((snapshotId) => {
                  const snapshot = snapshots.find((item) => item.id === snapshotId);
                  return (
                    <li
                      key={snapshotId}
                      dir="auto"
                      className="inline-flex max-w-full items-center gap-1 rounded-full border border-sky-500/30 bg-sky-500/10 pl-1.5 pr-1 text-sm text-sky-100"
                    >
                      <button
                        type="button"
                        dir="auto"
                        onClick={() => handleOpenSnapshot(snapshotId)}
                        className="max-w-[min(60vw,28rem)] truncate px-2 py-1.5 text-start hover:text-white"
                        title={snapshot?.name ?? snapshotId}
                      >
                        ðŸ“Š {snapshot?.name ?? snapshotId}
                      </button>
                      <button
                        type="button"
                        dir="auto"
                        onClick={() => void handleUnlinkSnapshot(snapshotId)}
                        className="rounded-full p-1.5 text-sky-200 transition hover:bg-rose-500/20 hover:text-rose-200"
                        aria-label={`Remove ${snapshot?.name ?? snapshotId}`}
                        title="Remove snapshot link"
                      >
                        <Trash2 size={14} />
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : (
              <p className="mt-2 text-sm text-gray-500">No chart snapshots linked.</p>
            )}
          </div>
        </section>
      )}

      {isExecuted && (
        <section dir="auto" className="mt-6 border-t border-[#292c34] pt-5">
          <h3 className="text-base font-semibold text-white">3. Analysis &amp; Verdict</h3>
          <label dir="auto" className="mt-4 block">
            <span className="mb-2 block text-xs font-medium text-gray-400">Data Analysis</span>
            {hypothesis.verdict && !isEditingVerdict ? (
              <p dir="auto" className="whitespace-pre-wrap rounded-xl bg-[#101114] px-4 py-3 text-sm text-gray-300">
                {hypothesis.dataAnalysis || "No data analysis recorded."}
              </p>
            ) : (
              <textarea
                dir="auto"
                value={dataAnalysis}
                onChange={(event) => setDataAnalysis(event.target.value)}
                rows={4}
                disabled={isSaving}
                placeholder="Analyze the collected data before deciding on a verdict..."
                className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
              />
            )}
          </label>

          {hypothesis.verdict && !isEditingVerdict && (
            <div className="mt-4 flex flex-wrap items-center gap-3">
              <span
                className={`inline-flex rounded-full border px-3 py-1.5 text-sm font-semibold ${
                  verdictPresentation[hypothesis.verdict].className
                }`}
              >
                {verdictPresentation[hypothesis.verdict].label}
              </span>
              <button
                type="button"
                onClick={() => {
                  if (
                    window.confirm(
                      "Are you sure you want to change the verdict? This will allow you to update the evidence and re-evaluate the test.",
                    )
                  ) {
                    setEvidenceNotes(hypothesis.evidenceNotes ?? "");
                    setDataAnalysis(hypothesis.dataAnalysis ?? "");
                    setError(null);
                    setIsEditingVerdict(true);
                  }
                }}
                className="rounded-lg border border-[#343741] px-3 py-1.5 text-sm font-medium text-gray-300 transition hover:bg-[#24262d]"
              >
                ✏️ Edit Verdict
              </button>
              <p dir="auto" className="w-full whitespace-pre-wrap text-sm text-gray-300">
                {hypothesis.evidenceNotes || "No evidence notes recorded."}
              </p>
              {hypothesis.boundaryConditions && (
                <p dir="auto" className="w-full whitespace-pre-wrap text-sm text-amber-200">
                  Boundary conditions: {hypothesis.boundaryConditions}
                </p>
              )}
            </div>
          )}

          {(!hypothesis.verdict || isEditingVerdict) && (
            <>
              <label dir="auto" className="mt-4 block">
                <span className="mb-2 block text-xs font-medium text-gray-400">Evidence Notes</span>
                <textarea
                  dir="auto"
                  value={evidenceNotes}
                  onChange={(event) => setEvidenceNotes(event.target.value)}
                  rows={4}
                  disabled={isSaving}
                  className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
                />
              </label>
              <label dir="auto" className="mt-4 block">
                <span className="mb-2 block text-xs font-medium text-gray-400">
                  Boundary Conditions (e.g., Only in Trending Markets, RSI &gt; 50)
                </span>
                <textarea
                  dir="auto"
                  value={boundaryConditions}
                  onChange={(event) => setBoundaryConditions(event.target.value)}
                  rows={3}
                  disabled={isSaving}
                  placeholder="Describe the conditions where the result applies..."
                  className="w-full resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
                />
              </label>
              <div className="mt-4 flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={isSaving}
                  onClick={() => void handleRecordVerdict("supported")}
                  className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm font-semibold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50"
                >
                  ✅ {isEditingVerdict ? "Mark Supported" : "Supported"}
                </button>
                <button
                  type="button"
                  disabled={isSaving || !boundaryConditions.trim()}
                  onClick={() => void handleRecordVerdict("partially_supported")}
                  className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm font-semibold text-amber-200 transition hover:bg-amber-500/20 disabled:opacity-50"
                >
                  ⚖️ Partially Supported
                </button>
                <button
                  type="button"
                  disabled={isSaving}
                  onClick={() => void handleRecordVerdict("rejected")}
                  className="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-sm font-semibold text-rose-200 transition hover:bg-rose-500/20 disabled:opacity-50"
                >
                  ❌ {isEditingVerdict ? "Mark Rejected" : "Rejected"}
                </button>
                <button
                  type="button"
                  disabled={isSaving}
                  onClick={() => void handleRecordVerdict("inconclusive")}
                  className="rounded-lg border border-gray-600 bg-gray-700/40 px-3 py-2 text-sm font-semibold text-gray-200 transition hover:bg-gray-700/70 disabled:opacity-50"
                >
                  ⚠️ Inconclusive
                </button>
                {isEditingVerdict && (
                  <button
                    type="button"
                    disabled={isSaving}
                    onClick={() => {
                      setEvidenceNotes(hypothesis.evidenceNotes ?? "");
                      setDataAnalysis(hypothesis.dataAnalysis ?? "");
                      setBoundaryConditions(hypothesis.boundaryConditions ?? "");
                      setIsEditingVerdict(false);
                    }}
                    className="rounded-lg border border-[#343741] px-3 py-2 text-sm font-medium text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
                  >
                    Cancel
                  </button>
                )}
              </div>
            </>
          )}

          {(hypothesis.verdict === "supported" ||
            hypothesis.verdict === "partially_supported") &&
            !isEditingVerdict && (
            <div className="mt-5 border-t border-[#292c34] pt-4">
              {hypothesis.boundaryConditions && (
                <p dir="auto" className="mb-3 text-sm text-amber-200">
                  Boundary conditions: {hypothesis.boundaryConditions}
                </p>
              )}
              {hypothesis.promotedToLibraryId ? (
                <span className="inline-flex rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-sm font-semibold text-emerald-300">
                  ✅ Saved to Library
                </span>
              ) : (
                <button
                  type="button"
                  disabled={isSaving}
                  onClick={() => void handleGraduate()}
                  className="rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2.5 text-sm font-bold text-gray-950 shadow-lg shadow-amber-950/30 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  📚 Graduate to Library
                </button>
              )}
            </div>
          )}

          {hypothesis.verdict &&
            !isEditingVerdict && (
            <div className="mt-5 border-t border-[#292c34] pt-4">
              <button
                type="button"
                disabled={isSaving}
                onClick={() => {
                  setError(null);
                  setIsCreatingChild(true);
                }}
                className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-4 py-2.5 text-sm font-semibold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50"
              >
                🌱 Create Refined Hypothesis (Child)
              </button>
            </div>
          )}
        </section>
      )}

      <section dir="auto" className="mt-7 border-t border-[#292c34] pt-5">
        <h3 className="text-base font-semibold text-white">
          Lab Scratchpad (Mid-Test Insights)
        </h3>
        <p className="mt-1 text-sm text-gray-400">
          Capture new ideas without changing the locked protocol or test data.
        </p>
        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <textarea
            dir="auto"
            value={insightContent}
            onChange={(event) => setInsightContent(event.target.value)}
            rows={2}
            disabled={isCapturingInsight}
            placeholder="Got a new idea mid-test? Record it here..."
            className="min-h-11 flex-1 resize-y rounded-xl border border-[#343741] bg-[#101114] px-4 py-3 text-sm text-gray-200 outline-none transition focus:border-[#e8ad55]/70 disabled:opacity-60"
          />
          <button
            type="button"
            disabled={isCapturingInsight || !insightContent.trim()}
            onClick={() => void handleCaptureInsight()}
            className="self-start rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2.5 text-sm font-semibold text-gray-950 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50 sm:self-stretch"
          >
            {isCapturingInsight ? "Capturing..." : "+ Capture"}
          </button>
        </div>
        {scratchpadError && (
          <p role="alert" className="mt-3 text-sm text-rose-300">
            {scratchpadError}
          </p>
        )}
        {insights.length > 0 ? (
          <ul className="mt-4 space-y-3">
            {insights.map((insight) => (
              <li
                key={insight.id}
                dir="auto"
                className="rounded-xl border border-[#292c34] bg-[#101114] p-4"
              >
                <p className="whitespace-pre-wrap text-sm text-gray-200">
                  {insight.content}
                </p>
                <time
                  className="mt-2 block text-xs text-gray-500"
                  dateTime={new Date(insight.createdAt).toISOString()}
                >
                  {formatTraceTimestamp(insight.createdAt)}
                </time>
                <div className="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    disabled={isSaving}
                    onClick={() => {
                      setError(null);
                      setChildInsight(insight);
                    }}
                    className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-xs font-semibold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50"
                  >
                    🌱 Create Child Hypothesis
                  </button>
                  <button
                    type="button"
                    disabled={
                      sendingInsightId === insight.id ||
                      queuedInsightIds.includes(insight.id)
                    }
                    onClick={() => void handleSendInsightToInquiry(insight)}
                    className="rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-1.5 text-xs font-semibold text-sky-200 transition hover:bg-sky-500/20 disabled:opacity-50"
                  >
                    {queuedInsightIds.includes(insight.id)
                      ? "✓ Sent to Inquiry Queue"
                      : sendingInsightId === insight.id
                        ? "Sending..."
                        : "📥 Send to Inquiry Queue"}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p className="mt-4 text-sm text-gray-500">
            No insights captured yet.
          </p>
        )}
      </section>

      {(isCreatingChild || childInsight) && (
        <ChildHypothesisModal
          hypothesis={hypothesis}
          initialStatementH1={childInsight?.content}
          onClose={() => {
            setIsCreatingChild(false);
            setChildInsight(null);
          }}
          onCreated={() => {
            setIsCreatingChild(false);
            setChildInsight(null);
            onChildCreated();
          }}
        />
      )}

      {isChecklistBuilderOpen && (
        <div
          dir="auto"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
          role="presentation"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget && !isSavingChecklistSchema) {
              setIsChecklistBuilderOpen(false);
            }
          }}
        >
          <section
            dir="auto"
            role="dialog"
            aria-modal="true"
            aria-labelledby="hypothesis-checklist-builder-title"
            className="max-h-[85vh] w-full max-w-2xl overflow-y-auto rounded-xl border border-[#343741] bg-[#17191f] p-5 shadow-2xl"
          >
            <div className="flex items-start justify-between gap-4">
              <div>
                <h3 id="hypothesis-checklist-builder-title" className="text-lg font-semibold text-white">
                  Define Checklist
                </h3>
                <p className="mt-1 text-sm text-gray-400">
                  Choose the fields to record for each test observation.
                </p>
              </div>
              <button
                type="button"
                dir="auto"
                disabled={isSavingChecklistSchema}
                onClick={() => setIsChecklistBuilderOpen(false)}
                className="rounded-lg border border-[#343741] px-3 py-1.5 text-sm text-gray-300 hover:bg-[#24262d] disabled:opacity-50"
              >
                Close
              </button>
            </div>
            <div className="mt-4 space-y-3">
              {checklistDraftFields.map((draft, index) => (
                <div key={draft.id} dir="auto" className="rounded-xl border border-[#343741] bg-[#101114] p-3">
                  <div className="grid gap-3 sm:grid-cols-[1fr_12rem_auto]">
                    <label dir="auto" className="block">
                      <span className="mb-1.5 block text-xs font-medium text-gray-400">Field label</span>
                      <input
                        dir="auto"
                        type="text"
                        value={draft.label}
                        onChange={(event) => setChecklistDraftFields((current) => current.map((field) =>
                          field.id === draft.id ? { ...field, label: event.currentTarget.value } : field
                        ))}
                        placeholder="e.g. Trend Direction"
                        disabled={isSavingChecklistSchema}
                        className="w-full rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2 text-sm text-gray-200 outline-none focus:border-amber-400/70 disabled:opacity-60"
                      />
                    </label>
                    <label dir="auto" className="block">
                      <span className="mb-1.5 block text-xs font-medium text-gray-400">Field type</span>
                      <select
                        dir="auto"
                        value={draft.type}
                        onChange={(event) => setChecklistDraftFields((current) => current.map((field) =>
                          field.id === draft.id
                            ? {
                                ...field,
                                type: event.currentTarget.value === "number"
                                  || event.currentTarget.value === "text"
                                  || event.currentTarget.value === "boolean"
                                  ? event.currentTarget.value
                                  : "select",
                              }
                            : field
                        ))}
                        disabled={isSavingChecklistSchema}
                        className="w-full rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2 text-sm text-gray-200 outline-none focus:border-amber-400/70 disabled:opacity-60"
                      >
                        <option value="select">Select</option>
                        <option value="number">Number</option>
                        <option value="text">Text</option>
                        <option value="boolean">Boolean</option>
                      </select>
                    </label>
                    <button
                      type="button"
                      dir="auto"
                      aria-label={`Remove checklist field ${index + 1}`}
                      title="Remove field"
                      disabled={isSavingChecklistSchema || checklistDraftFields.length === 1}
                      onClick={() => setChecklistDraftFields((current) =>
                        current.filter((field) => field.id !== draft.id)
                      )}
                      className="mt-5 self-end rounded-lg border border-[#343741] p-2 text-gray-400 transition hover:border-rose-500/40 hover:text-rose-300 disabled:cursor-not-allowed disabled:opacity-40"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                  {draft.type === "select" && (
                    <label dir="auto" className="mt-3 block">
                      <span className="mb-1.5 block text-xs font-medium text-gray-400">Options (comma-separated)</span>
                      <input
                        dir="auto"
                        type="text"
                        value={draft.optionsText}
                        onChange={(event) => setChecklistDraftFields((current) => current.map((field) =>
                          field.id === draft.id ? { ...field, optionsText: event.currentTarget.value } : field
                        ))}
                        placeholder="Bullish, Bearish, Neutral"
                        disabled={isSavingChecklistSchema}
                        className="w-full rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2 text-sm text-gray-200 outline-none focus:border-amber-400/70 disabled:opacity-60"
                      />
                    </label>
                  )}
                </div>
              ))}
            </div>
            {dataCollectionError && (
              <p role="alert" className="mt-3 text-sm text-rose-300">{dataCollectionError}</p>
            )}
            <div className="mt-4 flex flex-wrap justify-between gap-2">
              <button
                type="button"
                dir="auto"
                disabled={isSavingChecklistSchema}
                onClick={() => setChecklistDraftFields((current) => [...current, createChecklistFieldDraft()])}
                className="rounded-lg border border-[#343741] px-3 py-2 text-sm font-medium text-gray-300 transition hover:bg-[#24262d] disabled:opacity-50"
              >
                + Add Field
              </button>
              <button
                type="button"
                dir="auto"
                disabled={isSavingChecklistSchema}
                onClick={() => void handleSaveChecklistSchema()}
                className="rounded-lg border border-amber-300/50 bg-amber-500 px-4 py-2 text-sm font-semibold text-gray-950 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isSavingChecklistSchema ? "Saving..." : "Save Checklist"}
              </button>
            </div>
          </section>
        </div>
      )}

      {isSnapshotPickerOpen && (
        <div
          dir="auto"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
          role="presentation"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setIsSnapshotPickerOpen(false);
          }}
        >
          <section
            dir="auto"
            role="dialog"
            aria-modal="true"
            aria-labelledby="hypothesis-snapshot-picker-title"
            className="max-h-[85vh] w-full max-w-2xl overflow-y-auto rounded-xl border border-[#343741] bg-[#17191f] p-5 shadow-2xl"
          >
            <div className="flex items-start justify-between gap-4">
              <div>
                <h3 id="hypothesis-snapshot-picker-title" className="text-lg font-semibold text-white">
                  Link Chart Snapshot
                </h3>
                <p className="mt-1 text-sm text-gray-400">Choose visual evidence for this hypothesis.</p>
              </div>
              <button
                type="button"
                dir="auto"
                onClick={() => setIsSnapshotPickerOpen(false)}
                className="rounded-lg border border-[#343741] px-3 py-1.5 text-sm text-gray-300 hover:bg-[#24262d]"
              >
                Close
              </button>
            </div>
            {isLoadingSnapshots ? (
              <p className="py-8 text-center text-sm text-gray-400">Loading snapshots...</p>
            ) : snapshots.filter((snapshot) => !hypothesis.linkedSnapshots.includes(snapshot.id)).length > 0 ? (
              <ul className="mt-4 max-h-[60vh] space-y-2 overflow-y-auto">
                {snapshots
                  .filter((snapshot) => !hypothesis.linkedSnapshots.includes(snapshot.id))
                  .map((snapshot) => (
                    <li key={snapshot.id}>
                      <button
                        type="button"
                        dir="auto"
                        onClick={() => void handleLinkSnapshot(snapshot.id)}
                        className="w-full rounded-lg border border-[#343741] bg-[#101114] p-3 text-start transition hover:border-sky-500/50 hover:bg-sky-500/5"
                      >
                        <span className="block truncate text-sm font-semibold text-gray-100">{snapshot.name}</span>
                        <span className="mt-1 block text-xs text-gray-500">
                          {snapshot.chart.symbol} Â· {snapshot.chart.interval} Â· {new Date(snapshot.createdAt).toLocaleString()}
                        </span>
                      </button>
                    </li>
                  ))}
              </ul>
            ) : (
              <p className="py-8 text-center text-sm text-gray-400">
                {snapshots.length === 0 ? "No saved snapshots found." : "All snapshots are already linked."}
              </p>
            )}
          </section>
        </div>
      )}
    </article>
  );
};
