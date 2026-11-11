// Typed wrappers around the 9 source IPC commands.
// The UI in SRC-5/6/7 will import from here.
// Mirrors the style of src/mind/services/dbService.ts.

import { invoke } from "@tauri-apps/api/core";
import type {
  AcquisitionProvenanceTargetKind,
  AddFrameworkItemReq,
  ClaimRelationDto,
  CorrectExtractionReq,
  CreateClaimRelationReq,
  CreateFrameworkReq,
  CreateFrameworkVersionReq,
  CreateKnowledgeExtractionReq,
  CreateKnowledgeItemReq,
  CreateSourceClaimReq,
  CreateSourceLocationReq,
  CreateSourceVersionReq,
  FrameworkDto,
  FrameworkReviewDto,
  FrameworkVersionDto,
  KnowledgeExtractionDto,
  KnowledgeItemDto,
  ProvenanceEventDto,
  ReorderFrameworkItemsReq,
  ReviewExtractionReq,
  ReviewFrameworkReq,
  SourceClaimDto,
  SourceLocationDto,
  SourceVersionDto,
  UpdateFrameworkVersionReq,
} from "@/types";

// ── Response types ────────────────────────────────────────────────

export interface SourceSummary {
  id: string;
  kind: string;
  title: string;
  author: string | null;
  duration_sec: number | null;
  status: string;
}

export interface SegmentNode {
  id: string;
  label: string;
  start_sec: number | null;
  end_sec: number | null;
  page_start: number | null;
  page_end: number | null;
  order_index: number;
  children: SegmentNode[];
}

export interface SourceBundle {
  source: SourceRecord;
  segment_tree: SegmentNode[];
  segment_count: number;
  extract_count: number;
}

export interface SourceRecord {
  id: string;
  kind: string;
  path: string | null;
  title: string;
  author: string | null;
  language: string | null;
  duration_sec: number | null;
  page_count: number | null;
  metadata_json: string;
  notes: string;
  status: string;
  created_at: number;
  updated_at: number;
}

export interface ExtractSummary {
  id: string;
  segment_id: string;
  transform_kind: string;
  content_preview: string;
  language: string | null;
  confidence: string;
  created_by: string;
  reviewed_by_user: boolean;
  superseded_by: string | null;
  created_at: number;
}

export interface ExtractRecord {
  id: string;
  segment_id: string;
  transform_kind: string;
  content: string;
  language: string | null;
  confidence: string;
  created_by: string;
  reviewed_by_user: boolean;
  parent_extract_id: string | null;
  superseded_by: string | null;
  notes: string;
  status: string;
  created_at: number;
  updated_at: number;
}

export interface SegmentRecord {
  id: string;
  source_id: string;
  label: string;
  start_sec: number | null;
  end_sec: number | null;
  page_start: number | null;
  page_end: number | null;
  line_start: number | null;
  line_end: number | null;
  notes: string;
  status: string;
  created_at: number;
  updated_at: number;
  parent_segment_id: string | null;
  order_index: number;
}

export interface LinkRecord {
  id: string;
  extract_id: string;
  target_kind: string;
  target_id: string;
  link_kind: string;
  note: string;
  status: string;
  created_at: number;
  updated_at: number;
}

export interface ExtractDetail {
  extract: ExtractRecord;
  segment: SegmentRecord;
  source: SourceRecord;
  links: LinkRecord[];
}

// ── Request types ─────────────────────────────────────────────────

export interface CreateSourceReq {
  kind: string;
  title: string;
  path?: string | null;
  author?: string | null;
  language?: string | null;
  duration_sec?: number | null;
  page_count?: number | null;
  notes?: string | null;
}

export interface CreateSegmentReq {
  source_id: string;
  label: string;
  start_sec?: number | null;
  end_sec?: number | null;
  page_start?: number | null;
  page_end?: number | null;
  line_start?: number | null;
  line_end?: number | null;
  parent_segment_id?: string | null;
  order_index?: number | null;
  notes?: string | null;
}

export interface CreateExtractReq {
  segment_id: string;
  transform_kind: string;
  content: string;
  language?: string | null;
  confidence?: string | null;
  parent_extract_id?: string | null;
  notes?: string | null;
}

export interface CreateLinkReq {
  extract_id: string;
  target_kind: string;
  target_id: string;
  link_kind?: string | null;
  note?: string | null;
}

// ── API functions ─────────────────────────────────────────────────

/** List all sources, optionally filtered by kind (video|audio|doc|url|book|image). */
export async function srcListSources(
  kind?: string | null,
): Promise<SourceSummary[]> {
  return invoke<SourceSummary[]>("src_list_sources", { kind: kind ?? null });
}

/** Get the full source with its nested segment tree and counts. */
export async function srcGetSourceBundle(id: string): Promise<SourceBundle> {
  return invoke<SourceBundle>("src_get_source_bundle", { id });
}

/** List all extracts for a segment (summary view). */
export async function srcListExtracts(
  segmentId: string,
): Promise<ExtractSummary[]> {
  return invoke<ExtractSummary[]>("src_list_extracts", {
    segment_id: segmentId,
  });
}

/** Get full extract detail: extract + its segment + its source + its links. */
export async function srcGetExtract(id: string): Promise<ExtractDetail> {
  return invoke<ExtractDetail>("src_get_extract", { id });
}

/** Create a new source. Returns the new source id (e.g. "SRC-0001"). */
export async function srcCreateSource(req: CreateSourceReq): Promise<string> {
  return invoke<string>("src_create_source", { req });
}

/** Create a new segment under a source (optionally under a parent segment). */
export async function srcCreateSegment(
  req: CreateSegmentReq,
): Promise<string> {
  return invoke<string>("src_create_segment", { req });
}

/** Create a new extract from the UI (always created_by=user, reviewed=true). */
export async function srcCreateExtract(
  req: CreateExtractReq,
): Promise<string> {
  return invoke<string>("src_create_extract", { req });
}

/** Link an extract to a mind node, chart annotation, another source, or external URL. */
export async function srcCreateLink(req: CreateLinkReq): Promise<string> {
  return invoke<string>("src_create_link", { req });
}

/** Mark a pending-review extract as reviewed by the user. */
export async function srcMarkExtractReviewed(id: string): Promise<void> {
  return invoke<void>("src_mark_extract_reviewed", { id });
}

export async function srcListSourceVersions(sourceId: string): Promise<SourceVersionDto[]> {
  return invoke<SourceVersionDto[]>("src_list_source_versions", { sourceId });
}

export async function srcCreateSourceVersion(
  req: CreateSourceVersionReq,
): Promise<SourceVersionDto> {
  return invoke<SourceVersionDto>("src_create_source_version", { req });
}

export async function srcListSourceLocations(
  sourceVersionId: string,
): Promise<SourceLocationDto[]> {
  return invoke<SourceLocationDto[]>("src_list_source_locations", { sourceVersionId });
}

export async function srcCreateSourceLocation(
  req: CreateSourceLocationReq,
): Promise<SourceLocationDto> {
  return invoke<SourceLocationDto>("src_create_source_location", { req });
}

export async function srcListKnowledgeExtractions(
  sourceVersionId: string,
): Promise<KnowledgeExtractionDto[]> {
  return invoke<KnowledgeExtractionDto[]>("src_list_knowledge_extractions", { sourceVersionId });
}

export async function srcCreateKnowledgeExtraction(
  req: CreateKnowledgeExtractionReq,
): Promise<KnowledgeExtractionDto> {
  return invoke<KnowledgeExtractionDto>("src_create_knowledge_extraction", { req });
}

export async function srcReviewKnowledgeExtraction(req: ReviewExtractionReq): Promise<void> {
  return invoke<void>("src_review_knowledge_extraction", { req });
}

export async function srcCorrectKnowledgeExtraction(
  req: CorrectExtractionReq,
): Promise<KnowledgeExtractionDto> {
  return invoke<KnowledgeExtractionDto>("src_correct_knowledge_extraction", { req });
}

export async function srcListSourceClaims(sourceId: string): Promise<SourceClaimDto[]> {
  return invoke<SourceClaimDto[]>("src_list_source_claims", { sourceId });
}

export async function srcCreateSourceClaim(req: CreateSourceClaimReq): Promise<SourceClaimDto> {
  return invoke<SourceClaimDto>("src_create_source_claim", { req });
}

export async function srcListKnowledgeItems(): Promise<KnowledgeItemDto[]> {
  return invoke<KnowledgeItemDto[]>("src_list_knowledge_items");
}

export async function srcCreateKnowledgeItem(req: CreateKnowledgeItemReq): Promise<KnowledgeItemDto> {
  return invoke<KnowledgeItemDto>("src_create_knowledge_item", { req });
}

export async function srcListClaimRelations(sourceClaimId: string): Promise<ClaimRelationDto[]> {
  return invoke<ClaimRelationDto[]>("src_list_claim_relations", { sourceClaimId });
}

export async function srcCreateClaimRelation(req: CreateClaimRelationReq): Promise<string> {
  return invoke<string>("src_create_claim_relation", { req });
}

export async function srcListFrameworks(): Promise<FrameworkDto[]> {
  return invoke<FrameworkDto[]>("src_list_frameworks");
}

export async function srcCreateFramework(req: CreateFrameworkReq): Promise<FrameworkDto> {
  return invoke<FrameworkDto>("src_create_framework", { req });
}

export async function srcListFrameworkVersions(frameworkId: string): Promise<FrameworkVersionDto[]> {
  return invoke<FrameworkVersionDto[]>("src_list_framework_versions", { frameworkId });
}

export async function srcListFrameworkReviews(
  frameworkVersionId: string,
): Promise<FrameworkReviewDto[]> {
  return invoke<FrameworkReviewDto[]>("src_list_framework_reviews", { frameworkVersionId });
}

export async function srcCreateFrameworkVersion(
  req: CreateFrameworkVersionReq,
): Promise<FrameworkVersionDto> {
  return invoke<FrameworkVersionDto>("src_create_framework_version", { req });
}

export async function srcAddFrameworkItem(req: AddFrameworkItemReq): Promise<string> {
  return invoke<string>("src_add_framework_item", { req });
}

export async function srcRemoveFrameworkItem(itemId: string): Promise<void> {
  return invoke<void>("src_remove_framework_item", { itemId });
}

export async function srcReorderFrameworkItems(req: ReorderFrameworkItemsReq): Promise<void> {
  return invoke<void>("src_reorder_framework_items", { req });
}

export async function srcUpdateFrameworkVersion(req: UpdateFrameworkVersionReq): Promise<void> {
  return invoke<void>("src_update_framework_version", { req });
}

export async function srcReviewFrameworkVersion(req: ReviewFrameworkReq): Promise<void> {
  return invoke<void>("src_review_framework_version", { req });
}

export async function srcPublishFrameworkVersion(frameworkVersionId: string): Promise<void> {
  return invoke<void>("src_publish_framework_version", {
    req: { frameworkVersionId },
  });
}

export async function srcListProvenance(
  targetKind: AcquisitionProvenanceTargetKind,
  targetId: string,
): Promise<ProvenanceEventDto[]> {
  return invoke<ProvenanceEventDto[]>("src_list_provenance", { targetKind, targetId });
}
