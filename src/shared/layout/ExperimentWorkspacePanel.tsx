import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { FormEvent, ReactElement, ReactNode } from "react";
import { experimentService } from "@/mind/services/experimentService";
import { researchService } from "@/mind/services/researchService";
import type {
  AddDatasetSplitGroupReq,
  ComparisonGroupDto,
  CreateExperimentDesignVersionReq,
  CreateDatasetTransformationReq,
  DatasetFieldDefinitionDto,
  DatasetRole,
  DatasetRolePinDto,
  DatasetSampleDto,
  DatasetSamplePageDto,
  DatasetSampleSchemaDto,
  DatasetSampleValue,
  DatasetSplitAssignmentReq,
  DatasetTransformationInputReq,
  DataSampleDto,
  ExperimentDatasetVersionDto,
  ExperimentDesignSpecificationDto,
  ExperimentDesignVersionDto,
  ExperimentRunDto,
  ExperimentRunStatus,
  ExperimentStudyType,
  OperationalDefinitionDto,
  ResearchWorkspaceDto,
  TradeSampleDto,
  MarketOrderType,
  ExperimentWorkspaceDto,
} from "@/types";

const inputClass = "w-full rounded border border-[#343741] bg-[#15171b] px-2 py-1.5 text-xs text-gray-200";
const buttonClass = "rounded border border-[#414550] bg-[#1a1c22] px-2.5 py-1.5 text-xs text-gray-200 hover:bg-[#23262d] disabled:opacity-40";
const mutedClass = "text-[11px] text-gray-500";
const roles: DatasetRole[] = ["training", "validation", "test", "exploratory", "prospective", "forward"];

interface ExperimentWorkspacePanelProps {
  projectId: string;
  research: ResearchWorkspaceDto;
}

interface FieldProps {
  label: string;
  children: ReactNode;
}

function Field({ label, children }: FieldProps): ReactElement {
  return (
    <label className="block space-y-1">
      <span className="text-[11px] text-gray-400">{label}</span>
      {children}
    </label>
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isNullableNumber(value: unknown): value is number | null {
  return value === null || (typeof value === "number" && Number.isSafeInteger(value));
}

function isDatasetRole(value: unknown): value is DatasetRole {
  return typeof value === "string" && roles.some((role) => role === value);
}

function isSampleValue(value: unknown): value is DatasetSampleValue {
  if (!isRecord(value) || typeof value.kind !== "string") return false;
  switch (value.kind) {
    case "decimal":
      return typeof value.value === "string" && typeof value.unit === "string"
        && isNullableString(value.uncertainty);
    case "integer":
      return typeof value.value === "number" && Number.isSafeInteger(value.value)
        && isNullableString(value.unit);
    case "text":
    case "category":
      return typeof value.value === "string";
    case "boolean":
      return typeof value.value === "boolean";
    case "missing":
      return ["not_observed", "not_applicable", "censored", "unknown"].includes(String(value.state))
        && isNullableString(value.unit);
    default:
      return false;
  }
}

function isDataSample(value: unknown): value is DataSampleDto {
  if (!isRecord(value)
    || typeof value.sampleId !== "string"
    || typeof value.datasetVersionId !== "string"
    || !isNullableNumber(value.occurredAtUs)
    || !isNullableString(value.entityRef)
    || !isRecord(value.values)
    || !Array.isArray(value.provenance)) return false;
  if (!Object.values(value.values).every(isSampleValue)) return false;
  return value.provenance.every((entry) => isRecord(entry)
    && typeof entry.referenceKind === "string"
    && typeof entry.referenceId === "string"
    && typeof entry.rationale === "string");
}

function isMeasurement(value: unknown): value is TradeSampleDto["orderQuantity"] {
  return value === null || (isRecord(value)
    && typeof value.value === "string"
    && typeof value.unit === "string");
}

function isMarketOrderType(value: unknown): value is MarketOrderType | null {
  return value === null
    || value === "market"
    || value === "limit"
    || value === "stop"
    || value === "stop_limit"
    || value === "other";
}

function parseDatasetSamples(raw: string, versionId: string): DatasetSampleDto[] {
  const parsed: unknown = JSON.parse(raw);
  if (!Array.isArray(parsed)) throw new Error("Sample input must be a JSON array.");
  return parsed.map((entry): DatasetSampleDto => {
    if (!isRecord(entry) || !isRecord(entry.sample)) {
      throw new Error("Every sample needs a sampleKind and sample object.");
    }
    const sampleValue: unknown = entry.sample;
    if (!isDataSample(sampleValue) || sampleValue.datasetVersionId !== versionId) {
      throw new Error("Each sample must have a valid base shape and the selected exact dataset version ID.");
    }
    if (entry.sampleKind === "data_sample") {
      return { sampleKind: "data_sample", sample: sampleValue };
    }
    if (entry.sampleKind !== "trade_sample") {
      throw new Error("sampleKind must be data_sample or trade_sample.");
    }
    if (!isRecord(sampleValue)) throw new Error("Trade sample must be an object.");
    const market = sampleValue;
    if (typeof market.instrument !== "string"
      || !(market.orderSide === null || market.orderSide === "buy" || market.orderSide === "sell")
      || !isMarketOrderType(market.orderType)
      || !isMeasurement(market.orderQuantity)
      || !isMeasurement(market.executionPrice)
      || !isMeasurement(market.fees)
      || !isMeasurement(market.slippage)
      || !isNullableString(market.venue)
      || !isNullableString(market.marketSession)
      || !Array.isArray(market.sessionAssumptions)
      || !market.sessionAssumptions.every((item) => typeof item === "string")
      || !isNullableNumber(market.decisionAtUs)
      || !isNullableNumber(market.executionAtUs)) {
      throw new Error("Trade sample market fields are invalid.");
    }
    return {
      sampleKind: "trade_sample",
      sample: {
        ...sampleValue,
        instrument: market.instrument,
        orderSide: market.orderSide,
        orderType: market.orderType,
        orderQuantity: market.orderQuantity,
        executionPrice: market.executionPrice,
        fees: market.fees,
        slippage: market.slippage,
        venue: market.venue,
        marketSession: market.marketSession,
        sessionAssumptions: market.sessionAssumptions,
        decisionAtUs: market.decisionAtUs,
        executionAtUs: market.executionAtUs,
      },
    };
  });
}

function parseSplitAssignments(raw: string): DatasetSplitAssignmentReq[] {
  const parsed: unknown = JSON.parse(raw);
  if (!Array.isArray(parsed)) throw new Error("Split assignments must be a JSON array.");
  return parsed.map((entry): DatasetSplitAssignmentReq => {
    if (!isRecord(entry) || typeof entry.sampleId !== "string"
      || !isDatasetRole(entry.role) || typeof entry.splitKey !== "string") {
      throw new Error("Each split assignment needs sampleId, a supported role, and splitKey.");
    }
    return { sampleId: entry.sampleId, role: entry.role, splitKey: entry.splitKey };
  });
}

function parseTransformationInputs(raw: string): DatasetTransformationInputReq[] {
  const parsed: unknown = JSON.parse(raw);
  if (!Array.isArray(parsed)) throw new Error("Transformation inputs must be a JSON array.");
  return parsed.map((entry): DatasetTransformationInputReq => {
    if (!isRecord(entry) || typeof entry.datasetVersionId !== "string" || !isDatasetRole(entry.role)) {
      throw new Error("Each transformation input needs an exact datasetVersionId and role.");
    }
    return { datasetVersionId: entry.datasetVersionId, role: entry.role };
  });
}

function parseJsonValue(raw: string, fieldName: string): unknown {
  try {
    return JSON.parse(raw) as unknown;
  } catch (reason) {
    throw new Error(`${fieldName} must be valid JSON: ${reason instanceof Error ? reason.message : String(reason)}`);
  }
}

function isDatasetFieldKind(value: string): value is DatasetFieldDefinitionDto["valueKind"] {
  return ["decimal", "integer", "text", "category", "boolean"].some((kind) => kind === value);
}

function parseFieldDefinitions(raw: string): DatasetFieldDefinitionDto[] {
  return raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).map((line) => {
    const [key = "", valueKind = "", required = "", unit = ""] = line.split("|").map((part) => part.trim());
    if (!key || !isDatasetFieldKind(valueKind)) {
      throw new Error("Schema fields use: key|decimal/integer/text/category/boolean|required/optional|unit.");
    }
    return {
      key,
      valueKind,
      required: required === "required",
      unit: unit || null,
    };
  });
}

function lines(raw: FormDataEntryValue | null): string[] {
  return String(raw ?? "").split(/\r?\n/).map((value) => value.trim()).filter(Boolean);
}

function parseDefinitions(raw: FormDataEntryValue | null): OperationalDefinitionDto[] {
  return lines(raw).map((line) => {
    const [name = "", definition = "", unit = "", measurementMethod = ""] =
      line.split("|").map((part) => part.trim());
    if (!name || !definition) {
      throw new Error("Definitions use one line each: name|definition|unit|measurement method.");
    }
    return { name, definition, unit: unit || null, measurementMethod: measurementMethod || null };
  });
}

function parseComparisonGroups(raw: FormDataEntryValue | null): ComparisonGroupDto[] {
  return lines(raw).map((line) => {
    const [name = "", description = "", kind = "control"] = line.split("|").map((part) => part.trim());
    if (!name || !description || !["intervention", "control"].includes(kind)) {
      throw new Error("Groups use one line each: name|description|intervention-or-control.");
    }
    return { name, description, intervention: kind === "intervention" };
  });
}

function isStudyType(value: string): value is ExperimentStudyType {
  return [
    "randomized_controlled", "quasi_experimental", "observational",
    "market_backtest", "market_forward", "case_study", "other",
  ].some((studyType) => studyType === value);
}

function parseStudyType(value: FormDataEntryValue | null): ExperimentStudyType {
  const selected = String(value ?? "other");
  if (!isStudyType(selected)) throw new Error("Unsupported study type.");
  return selected;
}

function parseSpecification(form: FormData): ExperimentDesignSpecificationDto {
  const targetQuestion = String(form.get("targetResearchQuestionId") ?? "");
  const targetHypothesis = String(form.get("targetHypothesisVersionId") ?? "");
  const targetModel = String(form.get("targetModelVersionId") ?? "");
  const datasetVersionId = String(form.get("datasetVersionId") ?? "");
  const datasetRole = String(form.get("datasetRole") ?? "training");
  if (datasetVersionId && !isDatasetRole(datasetRole)) throw new Error("Select a supported dataset role.");
  const plannedSampleSizeRaw = String(form.get("plannedSampleSize") ?? "");
  const plannedSampleSize = plannedSampleSizeRaw ? Number(plannedSampleSizeRaw) : null;
  if (plannedSampleSize !== null && (!Number.isSafeInteger(plannedSampleSize) || plannedSampleSize <= 0)) {
    throw new Error("Planned sample size must be a positive whole number.");
  }
  const studyType = parseStudyType(form.get("studyType"));
  const marketStudy = studyType === "market_backtest" || studyType === "market_forward";
  const outcomeName = String(form.get("outcomeName") ?? "").trim();
  const outcomeDefinition = String(form.get("outcomeDefinition") ?? "").trim();
  const outcomeUnit = String(form.get("outcomeUnit") ?? "").trim();
  if (!outcomeName || !outcomeDefinition) throw new Error("Define the primary outcome and how it is measured.");
  const pin: DatasetRolePinDto[] = datasetVersionId
    ? [{
      datasetVersionId,
      role: datasetRole,
      rationale: String(form.get("datasetPinRationale") ?? ""),
    }]
    : [];
  return {
    targetResearchQuestionId: targetQuestion || null,
    targetHypothesisVersionId: targetHypothesis || null,
    targetModelVersionId: targetModel || null,
    estimand: String(form.get("estimand") ?? ""),
    studyType,
    intervention: String(form.get("intervention") ?? "") || null,
    control: String(form.get("control") ?? "") || null,
    comparisonGroups: parseComparisonGroups(form.get("comparisonGroups")),
    factors: parseDefinitions(form.get("factors")),
    outcomes: [{
      name: outcomeName,
      definition: outcomeDefinition,
      unit: outcomeUnit || null,
      measurementMethod: String(form.get("outcomeMethod") ?? "") || null,
    }],
    covariates: parseDefinitions(form.get("covariates")),
    unitOfAnalysis: String(form.get("unitOfAnalysis") ?? ""),
    samplingFrame: String(form.get("samplingFrame") ?? ""),
    eligibilityRules: lines(form.get("eligibilityRules")),
    exclusionRules: lines(form.get("exclusionRules")),
    plannedSampleSize,
    sampleSizeRationale: String(form.get("sampleSizeRationale") ?? ""),
    stoppingRule: String(form.get("stoppingRule") ?? ""),
    randomization: String(form.get("randomization") ?? "") || null,
    blinding: String(form.get("blinding") ?? "") || null,
    instruments: lines(form.get("instruments")),
    operationalDefinitions: parseDefinitions(form.get("operationalDefinitions")),
    dataCollectionSchedule: lines(form.get("dataCollectionSchedule")).map((line) => {
      const [stage = "", schedule = "", instrumentIds = ""] = line.split("|").map((part) => part.trim());
      if (!stage || !schedule) throw new Error("Schedules use: stage|schedule|comma-separated instrument IDs.");
      return { stage, schedule, instrumentIds: instrumentIds ? instrumentIds.split(",").map((part) => part.trim()).filter(Boolean) : [] };
    }),
    primaryOutcomes: [outcomeName],
    secondaryOutcomes: lines(form.get("secondaryOutcomes")),
    preregisteredAnalysisPlan: String(form.get("preregisteredAnalysisPlan") ?? ""),
    multiplicityStrategy: String(form.get("multiplicityStrategy") ?? ""),
    deviationsPolicy: String(form.get("deviationsPolicy") ?? ""),
    datasets: pin,
    datasetPlanRationale: String(form.get("datasetPlanRationale") ?? ""),
    codeEnvironment: {
      codeUri: String(form.get("codeUri") ?? ""),
      codeRevision: String(form.get("codeRevision") ?? "") || null,
      environment: String(form.get("environment") ?? ""),
      dependencyLockSha256: String(form.get("dependencyLockSha256") ?? "") || null,
    },
    randomSeeds: lines(form.get("randomSeeds")),
    reproducibilityInstructions: String(form.get("reproducibilityInstructions") ?? ""),
    marketContext: marketStudy ? {
      instrument: String(form.get("marketInstrument") ?? ""),
      orderExecutionContext: String(form.get("orderExecutionContext") ?? ""),
      feeAssumptions: String(form.get("feeAssumptions") ?? ""),
      slippageAssumptions: String(form.get("slippageAssumptions") ?? ""),
      marketSessionAssumptions: lines(form.get("marketSessionAssumptions")),
      decisionTimestampPolicy: String(form.get("decisionTimestampPolicy") ?? ""),
      executionTimestampPolicy: String(form.get("executionTimestampPolicy") ?? ""),
    } : null,
  };
}

function parseSamplesJsonTemplate(versionId: string): string {
  return JSON.stringify([{
    sampleKind: "data_sample",
    sample: {
      sampleId: "sample-001",
      datasetVersionId: versionId,
      occurredAtUs: null,
      entityRef: null,
      values: { price: { kind: "decimal", value: "12.5", unit: "USD", uncertainty: null } },
      provenance: [],
    },
  }], null, 2);
}

export function ExperimentWorkspacePanel({
  projectId,
  research,
}: ExperimentWorkspacePanelProps): ReactElement {
  const [workspace, setWorkspace] = useState<ExperimentWorkspaceDto | null>(null);
  const [samplePage, setSamplePage] = useState<DatasetSamplePageDto | null>(null);
  const [sampleVersionId, setSampleVersionId] = useState("");
  const [sampleJson, setSampleJson] = useState("");
  const [selectedFilePath, setSelectedFilePath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const refresh = useCallback(async (): Promise<void> => {
    if (!projectId) {
      setWorkspace(null);
      return;
    }
    setIsLoading(true);
    try {
      setWorkspace(await experimentService.getWorkspace(projectId));
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setIsLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const act = useCallback(async <T,>(action: () => Promise<T>): Promise<T | null> => {
    setError(null);
    try {
      const result = await action();
      await refresh();
      return result;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      return null;
    }
  }, [refresh]);

  const allDatasetVersions = useMemo(
    () => workspace?.datasets.flatMap((dataset) => dataset.versions.map((version) => ({
      ...version,
      datasetTitle: dataset.title,
      projectId: dataset.projectId,
    }))) ?? [],
    [workspace],
  );
  const sealedDatasetVersions = allDatasetVersions.filter((version) => version.lifecycleStatus === "sealed");
  const projectQuestions = research.questions.filter((question) => question.projectId === projectId);
  const projectHypotheses = research.hypotheses.filter((hypothesis) => hypothesis.projectId === projectId);
  const projectDesigns = workspace?.designVersions ?? [];
  const preregisteredDesigns = projectDesigns.filter(
    (design) => design.lifecycleStatus === "preregistered" && design.preregistration && design.specification,
  );
  const designById = useMemo(
    () => new Map(projectDesigns.map((design) => [design.id, design])),
    [projectDesigns],
  );

  const createDesignVersion = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    try {
      const form = new FormData(target);
      const parentVersionId = String(form.get("parentVersionId") ?? "") || null;
      const parent = parentVersionId ? designById.get(parentVersionId) : undefined;
      const request: CreateExperimentDesignVersionReq = {
        projectId,
        designId: parent?.designId ?? null,
        parentVersionId,
        title: String(form.get("title") ?? ""),
        description: String(form.get("description") ?? ""),
        protocol: String(form.get("protocol") ?? ""),
        changeRationale: String(form.get("changeRationale") ?? ""),
        specification: parseSpecification(form),
        createdBy: null,
      };
      const created = await act(() => experimentService.createDesignVersion(request));
      if (created) target.reset();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const createRun = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const design = designById.get(String(form.get("designVersionId") ?? ""));
    if (!design?.specification) {
      setError("Select a preregistered typed design version.");
      return;
    }
    const created = await act(() => experimentService.createRun({
      designVersionId: design.id,
      datasetPins: design.specification?.datasets ?? [],
      createdBy: null,
    }));
    if (created) target.reset();
  };

  const transitionRun = async (
    run: ExperimentRunDto,
    nextStatus: ExperimentRunStatus,
    options: {
      details: string;
      completionSummary?: string | null;
      failureDetails?: string | null;
      outputDatasetPins?: ExperimentDesignSpecificationDto["datasets"];
    },
  ): Promise<void> => {
    await act(() => experimentService.transitionRun({
      runId: run.id,
      nextStatus,
      actorId: null,
      details: options.details,
      completionSummary: options.completionSummary ?? null,
      failureDetails: options.failureDetails ?? null,
      outputDatasetPins: options.outputDatasetPins ?? [],
    }));
  };

  const createDataset = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const created = await act(() => experimentService.createDataset({
      projectId,
      title: String(form.get("title") ?? ""),
      description: String(form.get("description") ?? ""),
      createdBy: null,
    }));
    if (created) target.reset();
  };

  const createDatasetVersion = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    try {
      const form = new FormData(target);
      const rawCount = Number(form.get("expectedSampleCount"));
      if (!Number.isSafeInteger(rawCount) || rawCount < 0) throw new Error("Sample count must be a non-negative whole number.");
      const fields = parseFieldDefinitions(String(form.get("fields") ?? ""));
      const schema: DatasetSampleSchemaDto = {
        schemaId: String(form.get("schemaId") ?? ""),
        schemaVersion: 1,
        fields,
        allowAdditionalFields: form.get("allowAdditionalFields") === "on",
      };
      const version = await act(() => experimentService.createDatasetVersion({
        datasetId: String(form.get("datasetId") ?? ""),
        schema,
        expectedSampleCount: rawCount,
        createdBy: null,
      }));
      if (version) {
        setSampleVersionId(version.id);
        setSampleJson(parseSamplesJsonTemplate(version.id));
        target.reset();
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const addDatasetSamples = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const versionId = String(new FormData(target).get("datasetVersionId") ?? "");
    try {
      const samples = parseDatasetSamples(sampleJson, versionId);
      await act(() => experimentService.addDatasetSamples({ datasetVersionId: versionId, samples }));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const pickDatasetAsset = async (): Promise<void> => {
    try {
      const selected = await open({ multiple: false, directory: false });
      if (typeof selected === "string") setSelectedFilePath(selected);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const registerAsset = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    if (!selectedFilePath) {
      setError("Select a dataset asset file first.");
      return;
    }
    const pathParts = selectedFilePath.split(/[\\/]/);
    const name = pathParts[pathParts.length - 1] ?? "dataset-asset";
    const asset = await act(() => experimentService.registerDatasetAsset({
      datasetVersionId: String(form.get("datasetVersionId") ?? ""),
      sourcePath: selectedFilePath,
      originalName: name,
      mediaType: String(form.get("mediaType") ?? "application/octet-stream"),
      expectedSha256: String(form.get("expectedSha256") ?? "") || null,
    }));
    if (asset) {
      setSelectedFilePath("");
      target.reset();
    }
  };

  const addSplitGroup = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    try {
      const request: AddDatasetSplitGroupReq = {
        datasetVersionId: String(form.get("datasetVersionId") ?? ""),
        splitGroupKey: String(form.get("splitGroupKey") ?? ""),
        requireDisjoint: form.get("requireDisjoint") === "on",
        assignments: parseSplitAssignments(String(form.get("assignments") ?? "")),
      };
      const created = await act(() => experimentService.addDatasetSplitGroup(request));
      if (created) target.reset();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const inspectSamples = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const versionId = String(form.get("datasetVersionId") ?? "");
    setSampleVersionId(versionId);
    const result = await act(() => experimentService.getDatasetSamples(
      versionId,
      String(form.get("query") ?? "") || null,
      0,
      100,
    ));
    if (result) setSamplePage(result);
  };

  const reviewDesign = async (design: ExperimentDesignVersionDto): Promise<void> => {
    await act(() => researchService.transitionDesign(design.id, "review", null));
  };

  const preregisterDesign = async (event: FormEvent<HTMLFormElement>, design: ExperimentDesignVersionDto): Promise<void> => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    await act(() => experimentService.preregisterDesign({
      designVersionId: design.id,
      reviewerId: String(form.get("reviewerId") ?? "") || null,
      rationale: String(form.get("rationale") ?? ""),
    }));
  };

  const [compareLeft, setCompareLeft] = useState("");
  const [compareRight, setCompareRight] = useState("");
  const leftVersion = designById.get(compareLeft);
  const rightVersion = designById.get(compareRight);

  if (!projectId) {
    return <p className={mutedClass}>Select a research project to manage experiment designs and datasets.</p>;
  }

  return (
    <section className="space-y-5 border-t border-[#343741] pt-5">
      <div>
        <h3 className="text-sm font-semibold text-gray-200">Experimental architecture</h3>
        <p className={mutedClass}>
          Design versions are immutable snapshots. Runs record execution only; this workspace does not perform analysis or interpret results.
        </p>
      </div>
      {error && <p role="alert" className="rounded border border-rose-900 bg-rose-950/30 p-2 text-xs text-rose-300">{error}</p>}
      {isLoading && <p className={mutedClass}>Refreshing persisted experiment state…</p>}

      <div className="grid gap-4 xl:grid-cols-2">
        <form onSubmit={(event) => void createDesignVersion(event)} className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
          <h4 className="text-xs font-semibold text-gray-200">Create design version</h4>
          <Field label="Revise from an existing version (optional)">
            <select className={inputClass} name="parentVersionId" defaultValue="">
              <option value="">New design identity</option>
              {projectDesigns.map((design) => (
                <option key={design.id} value={design.id}>{design.title} · v{design.versionNo} · {design.lifecycleStatus}</option>
              ))}
            </select>
          </Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Design title"><input className={inputClass} name="title" required maxLength={200} /></Field>
            <Field label="Study type">
              <select className={inputClass} name="studyType" defaultValue="observational">
                <option value="randomized_controlled">Randomized controlled</option>
                <option value="quasi_experimental">Quasi-experimental</option>
                <option value="observational">Observational</option>
                <option value="market_backtest">Market backtest</option>
                <option value="market_forward">Market forward</option>
                <option value="case_study">Case study</option>
                <option value="other">Other</option>
              </select>
            </Field>
          </div>
          <Field label="Description"><textarea className={inputClass} name="description" rows={2} required /></Field>
          <Field label="Intended procedure / protocol"><textarea className={inputClass} name="protocol" rows={3} required /></Field>
          <Field label="Revision rationale (required when revising)"><input className={inputClass} name="changeRationale" /></Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Target research question">
              <select className={inputClass} name="targetResearchQuestionId" defaultValue="">
                <option value="">No selected question</option>
                {projectQuestions.map((question) => <option key={question.id} value={question.id}>{question.title}</option>)}
              </select>
            </Field>
            <Field label="Target hypothesis version">
              <select className={inputClass} name="targetHypothesisVersionId" defaultValue="">
                <option value="">No selected hypothesis</option>
                {projectHypotheses.flatMap((hypothesis) => hypothesis.definitions.map((definition) => (
                  <option key={definition.id} value={definition.id}>{definition.statementH1} · {definition.id}</option>
                )))}
              </select>
            </Field>
          </div>
          <Field label="Target model version reference (optional; model domain not implemented)">
            <input className={inputClass} name="targetModelVersionId" />
          </Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Estimand"><input className={inputClass} name="estimand" required /></Field>
            <Field label="Unit of analysis"><input className={inputClass} name="unitOfAnalysis" required /></Field>
          </div>
          <Field label="Intervention and control (when applicable)">
            <div className="grid gap-2 sm:grid-cols-2">
              <input className={inputClass} name="intervention" placeholder="Intervention" />
              <input className={inputClass} name="control" placeholder="Control" />
            </div>
          </Field>
          <Field label="Comparison groups (one per line: name|description|intervention-or-control)">
            <textarea className={inputClass} name="comparisonGroups" rows={2} />
          </Field>
          <Field label="Factors / covariates / other definitions (name|definition|unit|measurement method)">
            <textarea className={inputClass} name="factors" rows={2} placeholder="Factor name|Operational definition|unit|measurement method" />
            <textarea className={inputClass} name="covariates" rows={2} placeholder="Covariate name|Operational definition|unit|measurement method" />
            <textarea className={inputClass} name="operationalDefinitions" rows={2} placeholder="Name|Definition|unit|measurement method" />
          </Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Primary outcome name"><input className={inputClass} name="outcomeName" required /></Field>
            <Field label="Outcome unit"><input className={inputClass} name="outcomeUnit" /></Field>
          </div>
          <Field label="Primary outcome operational definition"><textarea className={inputClass} name="outcomeDefinition" rows={2} required /></Field>
          <Field label="Outcome measurement method"><input className={inputClass} name="outcomeMethod" /></Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Sampling frame"><textarea className={inputClass} name="samplingFrame" rows={2} required /></Field>
            <Field label="Planned sample size"><input className={inputClass} name="plannedSampleSize" type="number" min={1} required /></Field>
          </div>
          <Field label="Sample-size rationale"><input className={inputClass} name="sampleSizeRationale" required /></Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Eligibility rules (one per line)"><textarea className={inputClass} name="eligibilityRules" rows={2} /></Field>
            <Field label="Exclusion rules (one per line)"><textarea className={inputClass} name="exclusionRules" rows={2} /></Field>
          </div>
          <Field label="Stopping rule"><textarea className={inputClass} name="stoppingRule" rows={2} required /></Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Randomization"><input className={inputClass} name="randomization" /></Field>
            <Field label="Blinding"><input className={inputClass} name="blinding" /></Field>
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Instruments (one per line)"><textarea className={inputClass} name="instruments" rows={2} /></Field>
            <Field label="Data collection schedule (stage|schedule|instrument IDs)">
              <textarea className={inputClass} name="dataCollectionSchedule" rows={2} />
            </Field>
          </div>
          <Field label="Secondary outcomes (one per line)"><textarea className={inputClass} name="secondaryOutcomes" rows={2} /></Field>
          <Field label="Preregistered analysis plan"><textarea className={inputClass} name="preregisteredAnalysisPlan" rows={3} required /></Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Multiplicity strategy"><textarea className={inputClass} name="multiplicityStrategy" rows={2} required /></Field>
            <Field label="Deviations policy"><textarea className={inputClass} name="deviationsPolicy" rows={2} required /></Field>
          </div>
          <Field label="Pinned sealed dataset version (optional for prospective collection)">
            <select className={inputClass} name="datasetVersionId" defaultValue="">
              <option value="">No existing dataset version</option>
              {sealedDatasetVersions.map((version) => <option key={version.id} value={version.id}>{version.datasetTitle} · v{version.versionNo} · {version.id}</option>)}
            </select>
          </Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Dataset role">
              <select className={inputClass} name="datasetRole" defaultValue="training">
                {roles.map((role) => <option key={role} value={role}>{role.toUpperCase()}</option>)}
              </select>
            </Field>
            <Field label="Dataset pin rationale"><input className={inputClass} name="datasetPinRationale" /></Field>
          </div>
          <Field label="Prospective/no-existing-dataset rationale"><input className={inputClass} name="datasetPlanRationale" /></Field>
          <div className="grid gap-2 sm:grid-cols-2">
            <Field label="Code URI / repository identity"><input className={inputClass} name="codeUri" required /></Field>
            <Field label="Code revision"><input className={inputClass} name="codeRevision" /></Field>
            <Field label="Environment identity"><input className={inputClass} name="environment" required /></Field>
            <Field label="Dependency lock SHA-256"><input className={inputClass} name="dependencyLockSha256" /></Field>
          </div>
          <Field label="Random seeds (one per line)"><textarea className={inputClass} name="randomSeeds" rows={2} /></Field>
          <Field label="Reproducibility instructions"><textarea className={inputClass} name="reproducibilityInstructions" rows={2} required /></Field>
          <details className="rounded border border-[#292c34] p-2">
            <summary className="cursor-pointer text-xs text-gray-300">Market execution context (required for market study types)</summary>
            <div className="mt-2 grid gap-2 sm:grid-cols-2">
              <Field label="Instrument"><input className={inputClass} name="marketInstrument" /></Field>
              <Field label="Order and execution context"><input className={inputClass} name="orderExecutionContext" /></Field>
              <Field label="Fees"><input className={inputClass} name="feeAssumptions" /></Field>
              <Field label="Slippage"><input className={inputClass} name="slippageAssumptions" /></Field>
              <Field label="Market/session assumptions"><textarea className={inputClass} name="marketSessionAssumptions" rows={2} /></Field>
              <Field label="Decision timestamp policy"><input className={inputClass} name="decisionTimestampPolicy" /></Field>
              <Field label="Execution timestamp policy"><input className={inputClass} name="executionTimestampPolicy" /></Field>
            </div>
          </details>
          <button className={buttonClass} type="submit">Save immutable design version</button>
        </form>

        <div className="space-y-4">
          <section className="space-y-2 rounded-lg border border-[#292c34] bg-[#101114] p-3">
            <h4 className="text-xs font-semibold text-gray-200">Design version history and review</h4>
            {projectDesigns.length === 0 && <p className={mutedClass}>No design versions are registered.</p>}
            {projectDesigns.map((design) => (
              <article key={design.id} className="space-y-2 rounded border border-[#292c34] p-2 text-xs">
                <p className="font-medium text-gray-200">{design.title} · v{design.versionNo} · {design.lifecycleStatus}</p>
                <p className={mutedClass}>ID {design.id} · {design.specification ? `SHA-256 ${design.specificationSha256}` : "legacy/unconfigured; not eligible for runs"}</p>
                {design.parentVersionId && <p className={mutedClass}>Revises {design.parentVersionId}: {design.changeRationale}</p>}
                        {design.preregistration && <p className="text-emerald-300">Preregistered by {design.preregistration.reviewerId ?? "unspecified reviewer"} at {new Date(design.preregistration.preregisteredAtUs / 1000).toISOString()}</p>}
                <div className="flex flex-wrap gap-2">
                  {design.lifecycleStatus === "draft" && design.specification && (
                    <button className={buttonClass} type="button" onClick={() => void reviewDesign(design)}>Submit version for review</button>
                  )}
                  {design.lifecycleStatus === "review" && design.specification && (
                    <form onSubmit={(event) => void preregisterDesign(event, design)} className="grid gap-2 sm:grid-cols-3">
                      <input className={inputClass} name="reviewerId" placeholder="Reviewer ID" />
                      <input className={inputClass} name="rationale" placeholder="Review rationale" required />
                      <button className={buttonClass} type="submit">Preregister and lock</button>
                    </form>
                  )}
                </div>
              </article>
            ))}
            <div className="grid gap-2 sm:grid-cols-2">
              <Field label="Compare version A"><select className={inputClass} value={compareLeft} onChange={(event) => setCompareLeft(event.target.value)}><option value="">Select</option>{projectDesigns.map((design) => <option key={design.id} value={design.id}>{design.title} · v{design.versionNo}</option>)}</select></Field>
              <Field label="Compare version B"><select className={inputClass} value={compareRight} onChange={(event) => setCompareRight(event.target.value)}><option value="">Select</option>{projectDesigns.map((design) => <option key={design.id} value={design.id}>{design.title} · v{design.versionNo}</option>)}</select></Field>
            </div>
            {leftVersion && rightVersion && (
              <div className="grid gap-2 md:grid-cols-2">
                <pre className="max-h-64 overflow-auto whitespace-pre-wrap rounded bg-[#0b0c0e] p-2 text-[10px] text-gray-300">{JSON.stringify(leftVersion.specification ?? {legacy: true}, null, 2)}</pre>
                <pre className="max-h-64 overflow-auto whitespace-pre-wrap rounded bg-[#0b0c0e] p-2 text-[10px] text-gray-300">{JSON.stringify(rightVersion.specification ?? {legacy: true}, null, 2)}</pre>
              </div>
            )}
          </section>

          <section className="space-y-2 rounded-lg border border-[#292c34] bg-[#101114] p-3">
            <h4 className="text-xs font-semibold text-gray-200">Create and monitor runs</h4>
            <form onSubmit={(event) => void createRun(event)} className="flex flex-wrap items-end gap-2">
              <Field label="Preregistered design version">
                <select className={inputClass} name="designVersionId" required defaultValue="">
                  <option value="">Select immutable design</option>
                  {preregisteredDesigns.map((design) => <option key={design.id} value={design.id}>{design.title} · v{design.versionNo}</option>)}
                </select>
              </Field>
              <button className={buttonClass} type="submit" disabled={preregisteredDesigns.length === 0}>Create run</button>
            </form>
            {workspace?.runs.map((run) => (
              <RunMonitor key={run.id} run={run} datasets={sealedDatasetVersions} onTransition={transitionRun} onDeviation={(request) => act(() => experimentService.addRunDeviation(request))} onLog={(request) => act(() => experimentService.addRunLog(request))} />
            ))}
          </section>
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
          <h4 className="text-xs font-semibold text-gray-200">Dataset identities, immutable versions, and assets</h4>
          <form onSubmit={(event) => void createDataset(event)} className="grid gap-2 sm:grid-cols-3">
            <Field label="Dataset title"><input className={inputClass} name="title" required /></Field>
            <Field label="Description"><input className={inputClass} name="description" /></Field>
            <button className={buttonClass} type="submit">Create dataset identity</button>
          </form>
          <form onSubmit={(event) => void createDatasetVersion(event)} className="space-y-2 border-t border-[#292c34] pt-3">
            <h5 className="text-xs text-gray-300">Stage a dataset version</h5>
            <div className="grid gap-2 sm:grid-cols-3">
              <Field label="Dataset"><select className={inputClass} name="datasetId" required defaultValue=""><option value="">Select dataset</option>{workspace?.datasets.map((dataset) => <option key={dataset.id} value={dataset.id}>{dataset.title}</option>)}</select></Field>
              <Field label="Schema ID"><input className={inputClass} name="schemaId" required placeholder="study.observation" /></Field>
              <Field label="Expected sample count"><input className={inputClass} name="expectedSampleCount" type="number" min={0} required /></Field>
            </div>
            <Field label="Typed fields (one per line: key|kind|required-or-optional|unit)">
              <textarea className={inputClass} name="fields" rows={3} placeholder={"price|decimal|required|USD\ninstrument|text|required|"} />
            </Field>
            <label className="flex items-center gap-2 text-[11px] text-gray-400"><input type="checkbox" name="allowAdditionalFields" /> Allow additional typed sample fields</label>
            <button className={buttonClass} type="submit" disabled={!workspace?.datasets.length}>Create staged version</button>
          </form>
          <form onSubmit={(event) => void addDatasetSamples(event)} className="space-y-2 border-t border-[#292c34] pt-3">
            <Field label="Staged dataset version">
              <select className={inputClass} name="datasetVersionId" required value={sampleVersionId} onChange={(event) => { setSampleVersionId(event.target.value); setSampleJson(parseSamplesJsonTemplate(event.target.value)); }}>
                <option value="">Select staged version</option>
                {allDatasetVersions.filter((version) => version.lifecycleStatus === "staged").map((version) => <option key={version.id} value={version.id}>{version.datasetTitle} · v{version.versionNo}</option>)}
              </select>
            </Field>
            <Field label="Queryable sample rows (JSON array; stable IDs and exact version are required)">
              <textarea className={`${inputClass} font-mono`} value={sampleJson} onChange={(event) => setSampleJson(event.target.value)} rows={7} placeholder="[]" />
            </Field>
            <button className={buttonClass} type="submit" disabled={!sampleVersionId}>Add typed samples</button>
          </form>
          <form onSubmit={(event) => void registerAsset(event)} className="space-y-2 border-t border-[#292c34] pt-3">
            <Field label="Attach an immutable workspace-managed asset">
              <select className={inputClass} name="datasetVersionId" required defaultValue=""><option value="">Select staged dataset version</option>{allDatasetVersions.filter((version) => version.lifecycleStatus === "staged").map((version) => <option key={version.id} value={version.id}>{version.datasetTitle} · v{version.versionNo}</option>)}</select>
            </Field>
            <div className="grid gap-2 sm:grid-cols-2">
              <button className={buttonClass} type="button" onClick={() => void pickDatasetAsset()}>Choose file: {selectedFilePath.split(/[\\/]/).slice(-1)[0] || "none"}</button>
              <Field label="Media type"><input className={inputClass} name="mediaType" defaultValue="application/octet-stream" required /></Field>
              <Field label="Expected SHA-256 (optional)"><input className={inputClass} name="expectedSha256" /></Field>
            </div>
            <button className={buttonClass} type="submit" disabled={!selectedFilePath}>Copy, hash, and register asset</button>
          </form>
          <form onSubmit={(event) => void addSplitGroup(event)} className="space-y-2 border-t border-[#292c34] pt-3">
            <h5 className="text-xs text-gray-300">Assign explicit split roles</h5>
            <Field label="Staged dataset version"><select className={inputClass} name="datasetVersionId" required defaultValue=""><option value="">Select staged version</option>{allDatasetVersions.filter((version) => version.lifecycleStatus === "staged").map((version) => <option key={version.id} value={version.id}>{version.datasetTitle} · v{version.versionNo}</option>)}</select></Field>
            <Field label="Split group ID"><input className={inputClass} name="splitGroupKey" required placeholder="evaluation-2026" /></Field>
            <label className="flex items-center gap-2 text-[11px] text-gray-400"><input type="checkbox" name="requireDisjoint" defaultChecked /> Enforce disjoint samples in this group</label>
            <Field label="Assignments JSON (sampleId, role, splitKey)">
              <textarea className={`${inputClass} font-mono`} name="assignments" rows={4} placeholder={'[{"sampleId":"s1","role":"training","splitKey":"train"}]'} required />
            </Field>
            <button className={buttonClass} type="submit">Save split assignments</button>
          </form>
        </section>

        <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
          <h4 className="text-xs font-semibold text-gray-200">Dataset version inspection</h4>
          {allDatasetVersions.map((version) => (
            <article key={version.id} className="space-y-1 rounded border border-[#292c34] p-2 text-xs">
              <p className="font-medium text-gray-200">{version.datasetTitle} · v{version.versionNo} · {version.lifecycleStatus}</p>
              <p className={mutedClass}>{version.actualSampleCount}/{version.expectedSampleCount} samples · schema {version.schema.schemaId}@{version.schema.schemaVersion}</p>
              {version.contentSha256 && <p className="break-all font-mono text-[10px] text-gray-500">SHA-256 {version.contentSha256}</p>}
              <p className={mutedClass}>{version.assets.length} managed asset(s): {version.assets.map((asset) => `${asset.originalName} · ${asset.byteLength} bytes`).join(", ") || "none"}</p>
              <div className="flex gap-2">
                {version.lifecycleStatus === "staged" && <button className={buttonClass} type="button" onClick={() => void act(() => experimentService.sealDatasetVersion(version.id))}>Validate and seal</button>}
                {version.lifecycleStatus === "sealed" && <button className={buttonClass} type="button" onClick={() => void act(() => experimentService.verifyDatasetVersion(version.id))}>Verify checksum</button>}
              </div>
            </article>
          ))}
          <form onSubmit={(event) => void inspectSamples(event)} className="grid gap-2 sm:grid-cols-3">
            <Field label="Query samples from exact version"><select className={inputClass} name="datasetVersionId" required defaultValue=""><option value="">Select dataset version</option>{allDatasetVersions.map((version) => <option key={version.id} value={version.id}>{version.datasetTitle} · v{version.versionNo}</option>)}</select></Field>
            <Field label="Sample ID search"><input className={inputClass} name="query" /></Field>
            <button className={buttonClass} type="submit">Inspect samples</button>
          </form>
          {samplePage && <div className="space-y-1">
            <p className={mutedClass}>{samplePage.totalCount} sample(s) in version {samplePage.datasetVersionId}</p>
            {samplePage.samples.map((item, index) => (
              <pre key={`${item.sampleKind}-${item.sample.sampleId}-${index}`} className="overflow-auto rounded bg-[#0b0c0e] p-2 text-[10px] text-gray-300">{JSON.stringify(item, null, 2)}</pre>
            ))}
          </div>}
          {workspace?.splitGroups.map((group) => (
            <article key={group.id} className="rounded border border-[#292c34] p-2 text-xs">
              <p>{group.splitGroupKey} · {group.requireDisjoint ? "disjoint" : "overlap permitted"}</p>
              <p className={mutedClass}>{group.assignments.map((item) => `${item.role.toUpperCase()}: ${item.sampleId} (${item.splitKey})`).join(" · ")}</p>
            </article>
          ))}
          {workspace?.transformations.map((item) => (
            <article key={item.id} className="rounded border border-[#292c34] p-2 text-xs">
              <p>Transformation: {item.method} → {item.outputDatasetVersionId}</p>
              <p className={mutedClass}>Inputs: {item.inputs.map((input) => `${input.datasetVersionId} (${input.role})`).join(", ")}</p>
            </article>
          ))}
        </section>
      </div>
    </section>
  );
}

interface RunMonitorProps {
  run: ExperimentRunDto;
  datasets: (ExperimentDatasetVersionDto & { datasetTitle: string; projectId: string })[];
  onTransition: (
    run: ExperimentRunDto,
    status: ExperimentRunStatus,
    options: {
      details: string;
      completionSummary?: string | null;
      failureDetails?: string | null;
      outputDatasetPins?: ExperimentDesignSpecificationDto["datasets"];
    },
  ) => Promise<void>;
  onDeviation: (request: {
    runId: string;
    description: string;
    rationale: string;
    impactAssessment: string;
    recordedBy: string | null;
  }) => Promise<unknown>;
  onLog: (request: {
    runId: string;
    severity: "debug" | "info" | "warning" | "error";
    message: string;
    details: unknown;
  }) => Promise<unknown>;
}

function RunMonitor({ run, datasets, onTransition, onDeviation, onLog }: RunMonitorProps): ReactElement {
  const complete = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const outputDatasetVersionId = String(form.get("outputDatasetVersionId") ?? "");
    const outputRole = String(form.get("outputRole") ?? "prospective");
    const outputDatasetPins: ExperimentDesignSpecificationDto["datasets"] = [];
    if (outputDatasetVersionId && isDatasetRole(outputRole)) {
      outputDatasetPins.push({ datasetVersionId: outputDatasetVersionId, role: outputRole, rationale: "Explicit run output role." });
    }
    await onTransition(run, "completed", {
      details: "Execution finalized by operator.",
      completionSummary: String(form.get("completionSummary") ?? ""),
      outputDatasetPins,
    });
  };

  const fail = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    await onTransition(run, "failed", {
      details: String(form.get("failureDetails") ?? ""),
      failureDetails: String(form.get("failureDetails") ?? ""),
    });
  };

  const recordDeviation = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const result = await onDeviation({
      runId: run.id,
      description: String(form.get("description") ?? ""),
      rationale: String(form.get("rationale") ?? ""),
      impactAssessment: String(form.get("impactAssessment") ?? ""),
      recordedBy: null,
    });
    if (result) target.reset();
  };

  const recordLog = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const result = await onLog({
      runId: run.id,
      severity: "info",
      message: String(form.get("message") ?? ""),
      details: {},
    });
    if (result) target.reset();
  };

  return (
    <article className="space-y-2 rounded border border-[#292c34] p-2 text-xs">
      <p className="font-medium text-gray-200">Run {run.id} · {run.lifecycleStatus}</p>
      <p className={mutedClass}>Pins design {run.designVersionId} · {run.datasetPins.map((pin) => `${pin.usageKind} ${pin.role}: ${pin.datasetTitle} v${pin.versionNo} (${pin.datasetVersionId})`).join(" · ") || "no dataset input"}</p>
      <div className="flex flex-wrap gap-2">
        {run.lifecycleStatus === "created" && <button className={buttonClass} type="button" onClick={() => void onTransition(run, "ready", { details: "Inputs and environment are ready." })}>Mark ready</button>}
        {run.lifecycleStatus === "ready" && <button className={buttonClass} type="button" onClick={() => void onTransition(run, "running", { details: "Execution started; persistent state saved before work." })}>Start run</button>}
        {["created", "ready", "running"].includes(run.lifecycleStatus) && <button className={buttonClass} type="button" onClick={() => void onTransition(run, "cancelled", { details: "Cancelled by operator." })}>Cancel</button>}
        {run.lifecycleStatus === "running" && <button className={buttonClass} type="button" onClick={() => void onTransition(run, "inconclusive", { details: "Marked inconclusive by operator." })}>Mark inconclusive</button>}
      </div>
      {run.lifecycleStatus === "running" && (
        <div className="grid gap-2 md:grid-cols-3">
          <form onSubmit={(event) => void recordDeviation(event)} className="space-y-1">
            <Field label="Deviation description"><input className={inputClass} name="description" required /></Field>
            <Field label="Rationale"><input className={inputClass} name="rationale" required /></Field>
            <Field label="Impact assessment"><input className={inputClass} name="impactAssessment" /></Field>
            <button className={buttonClass} type="submit">Record deviation before interpretation</button>
          </form>
          <form onSubmit={(event) => void recordLog(event)} className="space-y-1">
            <Field label="Execution log entry"><input className={inputClass} name="message" required /></Field>
            <button className={buttonClass} type="submit">Append log</button>
          </form>
          <form onSubmit={(event) => void fail(event)} className="space-y-1">
            <Field label="Failure details"><input className={inputClass} name="failureDetails" required /></Field>
            <button className={buttonClass} type="submit">Finalize as failed</button>
          </form>
          <form onSubmit={(event) => void complete(event)} className="space-y-1 md:col-span-3">
            <Field label="Completion summary"><input className={inputClass} name="completionSummary" required /></Field>
            <div className="grid gap-2 sm:grid-cols-2">
              <Field label="Sealed output dataset version (optional)"><select className={inputClass} name="outputDatasetVersionId" defaultValue=""><option value="">No output dataset</option>{datasets.map((dataset) => <option key={dataset.id} value={dataset.id}>{dataset.datasetTitle} · v{dataset.versionNo}</option>)}</select></Field>
              <Field label="Output role"><select className={inputClass} name="outputRole" defaultValue="prospective">{roles.map((role) => <option key={role} value={role}>{role.toUpperCase()}</option>)}</select></Field>
            </div>
            <button className={buttonClass} type="submit">Finalize completed run atomically</button>
          </form>
        </div>
      )}
      {run.failureDetails && <p className="text-rose-300">Failure: {run.failureDetails}</p>}
      {run.completionSummary && <p className="text-emerald-300">Completion: {run.completionSummary}</p>}
      <details>
        <summary className="cursor-pointer text-gray-400">Run history ({run.events.length} events, {run.deviations.length} deviations, {run.logs.length} logs)</summary>
        <ul className="mt-1 space-y-1">
          {run.events.map((event) => <li key={event.id} className={mutedClass}>{event.fromStatus ?? "—"} → {event.toStatus ?? event.eventKind}: {event.details}</li>)}
          {run.deviations.map((item) => <li key={item.id} className="text-amber-300">Deviation: {item.description} — {item.rationale}; impact: {item.impactAssessment || "not stated"}</li>)}
          {run.logs.map((item) => <li key={item.id} className={mutedClass}>[{item.severity}] {item.message}</li>)}
        </ul>
      </details>
    </article>
  );
}
