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

export interface VoraCard {
  id: string;
  title: string;
  content: string;
  boardId?: string;
  librarySourceCardId?: string;
  version?: string;
  tags?: string[];
  created_at?: number;
  updated_at?: number;
  deleted_at?: number | null;
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

export interface LibraryItemGroupSnapshot {
  nodes: LibraryItemGroupNode[];
  edges: LibraryItemGroupEdge[];
  cards: LibraryItemGroupCard[];
}

export interface LibraryItemGroup {
  id: string;
  board_id: string;
  name: string;
  version: string;
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