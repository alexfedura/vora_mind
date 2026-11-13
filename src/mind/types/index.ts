export type ActiveSidebarTab =
  | "desk"
  | "projects"
  | "journal"
  | "library"
  | "tags"
  | "highlights"
  | "properties"
  | "tasks";

export type OpenPanel = "none" | "boards" | "sidebar" | "left" | "right" | "both";

export const LIBRARY_ITEM_GROUP_DRAG_TYPE = "application/x-vora-library-item-group";

export interface VoraCardData {
  label: string;
  title: string;
  content: string;
  cardId: string;
  version?: string;
}

export interface VoraMatrixData extends Record<string, unknown> {
  title: string;
  columns: { id: string; name: string; weight: number }[];
  rows: { id: string; name: string; scores: Record<string, number> }[];
}

export interface VoraTextData extends Record<string, unknown> {
  text?: string;
  fontSize?: number;
  isBold?: boolean;
  isItalic?: boolean;
  isUnderline?: boolean;
  alignment?: "left" | "center" | "right";
  color?: string;
  autoFocus?: boolean;
}

export interface VoraHypothesisData extends Record<string, unknown> {
  hypothesisId: string;
  statementH1: string;
  status: string;
  verdict: string | null;
  sourceCardId: string;
  protocolLockedAt: number | null;
}

export interface VoraMediaData extends Record<string, unknown> {
  type: "image" | "audio" | "video";
  fileId?: string;
  filePath: string;
  fileName: string;
  width?: number;
  height?: number;
}

export interface VoraCard {
  id: string;
  title: string;
  content: string;
  boardId?: string;
  librarySourceCardId?: string;
  sourceDocumentId?: string;
  sourceHighlightId?: string;
  mediaLineage?: MediaCardLineage;
  version?: string;
  tags?: string[];
  created_at?: number;
  updated_at?: number;
  deleted_at?: number | null;
}

export interface MediaCardLineage {
  source_file_id: string;
  start_time: number;
  end_time: number;
}

export interface CardMediaMetadata {
  source_file_id: string | null;
  start_time: number | null;
  end_time: number | null;
}

export interface ParsedCardVersion {
  major: number;
  minor: number;
  patch: number;
}

export interface CardVersion {
  id: string;
  version: string;
  title: string | null;
  reason: string | null;
  created_at: number;
}

export interface CardVersionContent {
  title: string;
  content: string;
  version: string;
}

export interface VoraBoardTransferNode {
  id: string;
  type: "voraCard" | "voraText" | "voraShape" | "voraMedia" | "forgeAst";
  position: VoraShapePoint;
  width?: number;
  height?: number;
  data: Record<string, unknown>;
  draggable?: boolean;
  style?: Record<string, string | number>;
}

export interface VoraBoardTransferEdge {
  id: string;
  source: string;
  target: string;
}

export interface VoraBoardTransfer {
  format: "vora-mind-board";
  version: 1;
  boardName: string;
  exportedAt: string;
  nodes: VoraBoardTransferNode[];
  edges: VoraBoardTransferEdge[];
  cards: Pick<VoraCard, "id" | "title" | "content" | "version">[];
}

export type TextLinkField = "title" | "content";

export interface TextLink {
  id: string;
  source_board_id: string;
  source_card_id: string;
  source_field: TextLinkField;
  source_start: number;
  source_end: number;
  target_board_id: string;
  target_card_id: string;
  target_field: TextLinkField;
  target_start: number;
  target_end: number;
  created_at: number;
  updated_at: number;
}

export interface TextLinkSelection {
  cardId: string;
  field: TextLinkField;
  start: number;
  end: number;
}

export interface CanvasGroupMembership {
  canvasGroupId: string;
}

export interface TextLinkInput {
  source_board_id: string;
  source_card_id: string;
  source_field: TextLinkField;
  source_start: number;
  source_end: number;
  target_board_id: string;
  target_card_id: string;
  target_field: TextLinkField;
  target_start: number;
  target_end: number;
}

export interface MediaSource {
  id: string;
  path: string;
  fingerprint_size: number;
  fingerprint_modified_ms: number;
  duration_seconds: number | null;
  last_position_seconds: number;
  created_at: number;
  last_used_at: number;
  is_available: boolean;
}

export interface VoraFile {
  id: string;
  original_name: string;
  current_path: string;
  file_type: "image" | "audio" | "video" | "pdf" | "docx" | "md";
  file_hash: string;
  size_bytes: number;
  modified_time_ms: number;
  created_at: number;
  duration_seconds: number | null;
  last_position_seconds: number;
  last_used_at: number;
  is_available: boolean;
}

export interface MediaSlice {
  id: string;
  file_id: string;
  title: string;
  start_seconds: number;
  end_seconds: number;
  note: string;
  created_at: number;
  updated_at: number;
}

export interface MediaSliceInput {
  id?: string | null;
  file_id: string;
  title: string;
  start_seconds: number;
  end_seconds: number;
  note: string;
}

export interface MediaSliceWithSource {
  slice: MediaSlice;
  source: MediaSource;
}

export interface MediaFileLibraryEntry {
  file: VoraFile;
  slices: MediaSlice[];
}

export interface MediaLinkPayload {
  sourceId: string;
  sliceId: string;
  startSeconds: number;
  endSeconds: number;
  path: string;
  fingerprintSize: number;
  fingerprintModifiedMs: number;
}

export interface TextChangeRange {
  from: number;
  to: number;
  inserted_length: number;
  inserted_text: string;
}

export interface TextChangeSet {
  field: TextLinkField;
  changes: TextChangeRange[];
}

export interface BoardItem {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  active_version_id: string;
  active_version: string;
  parent_board_id: string | null;
}

export interface LibraryItemGroupNode {
  id: string;
  node_type: string;
  title: string;
  content: string;
  version: string;
  data: Record<string, unknown>;
  x: number;
  y: number;
  width?: number;
  height?: number;
  style?: string;
}

export interface LibraryItemGroupEdge {
  id: string;
  source: string;
  target: string;
  link_type?: string;
}

export interface LibraryItemGroupCard {
  id: string;
  title: string;
  content: string;
  version: string;
  deleted_at: number | null;
}

export type LibraryItemType = "item" | "group";

export interface LibraryItemGroupSnapshot {
  item_type: LibraryItemType;
  nodes: LibraryItemGroupNode[];
  edges: LibraryItemGroupEdge[];
  cards: LibraryItemGroupCard[];
}

export interface LibraryItemGroup {
  id: string;
  board_id: string;
  name: string;
  version: string;
  item_type: LibraryItemType;
  snapshot: LibraryItemGroupSnapshot;
  created_at: number;
  updated_at: number;
  deleted_at: number | null;
}

export interface LibraryItemGroupVersion {
  id: string;
  group_id: string;
  version: string;
  snapshot: LibraryItemGroupSnapshot;
  created_at: number;
}

export interface LibraryItemGroupUsage {
  board_id: string;
  board_title: string;
  node_id: string;
  segment_id: string;
  segment_name: string;
  start_x: number;
  start_y: number;
  end_x: number;
  end_y: number;
}

export interface VoraShapePoint {
  x: number;
  y: number;
}

export const VORA_DIAGRAM_SHAPE_LABELS = {
  rectangle: "Rectangle",
  path: "Path",
  freehand: "Freehand",
  process: "Process",
  decision: "Decision",
  terminator: "Start / End",
  inputOutput: "Input / Output",
  document: "Document",
  database: "Database",
} as const;

export type VoraDiagramShape = keyof typeof VORA_DIAGRAM_SHAPE_LABELS;

export interface VoraPathSegment {
  id: string;
  name: string;
  details: string;
  color: string;
  libraryItemGroupId?: string;
}

export interface VoraShapeData extends Record<string, unknown> {
  shape: VoraDiagramShape;
  label: string;
  color: string;
  lineOpacity: number;
  lineWidth: number;
  lineStyle: "solid" | "dashed" | "dotted";
  textColor: string;
  textOpacity: number;
  fillColor: string;
  fillOpacity: number;
  points: VoraShapePoint[];
  segments?: VoraPathSegment[];
  startArrow: boolean;
  endArrow: boolean;
}

export type VoraShapeStyleSettings = Pick<
  VoraShapeData,
  "color" | "lineOpacity" | "lineWidth" | "lineStyle" | "textColor" | "textOpacity" | "fillColor" | "fillOpacity" | "startArrow" | "endArrow"
>;

export type VoraShapeDefaults = Record<VoraShapeData["shape"], VoraShapeStyleSettings>;