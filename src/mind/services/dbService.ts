import { invoke } from "@tauri-apps/api/core";
import { Node, Edge } from "@xyflow/react";
import type {
  BoardItem,
  LibraryItemGroup,
  LibraryItemGroupSnapshot,
  LibraryItemGroupUsage,
  LibraryItemGroupVersion,
  TextChangeSet,
  TextLink,
  TextLinkInput,
  TextLinkField,
} from "@/mind/types";
import type { JournalEntry, TaskFlowItem } from "@/types";

export interface CardDto {
  id: string;
  title: string;
  content: string;
  version: string;
  deleted_at: number | null;
  library_source_card_id: string | null;
}

export type BoardDto = BoardItem;

export interface NodePayload {
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

export interface EdgePayload {
  id: string;
  source: string;
  target: string;
  link_type?: string;
}

export interface BoardVersionCard {
  id: string;
  title: string;
  content: string;
  version: string;
  deleted_at: number | null;
}

export interface BoardVersionSnapshot {
  nodes: NodePayload[];
  edges: EdgePayload[];
  cards: BoardVersionCard[];
}

export interface BoardVersionSummary {
  id: string;
  board_id: string;
  major: number;
  minor: number;
  patch: number;
  version: string;
  parent_version_id: string | null;
  forked_from_version_id: string | null;
  forked_from_version: string | null;
  created_at: number;
  is_active: boolean;
}

export type BoardVersionIncrement = "major" | "minor" | "patch";
export type LibraryItemGroupDto = LibraryItemGroup;

export interface SnapshotSummary {
  id: string;
  name: string;
  parent_snapshot_id: string | null;
  depth: number;
  is_checkpoint: number;
  created_at: number;
}

export interface ResolvedSnapshot {
  snapshot_id: string;
  objects: Record<string, string>;
  node_data: Record<string, {
    node_type: string;
    data: unknown;
    title: string | null;
    content: string | null;
    version: string | null;
  }>;
  placements: Record<
    string,
    {
      x: number;
      y: number;
      width: number;
      height: number;
      rotation: number;
      z_index: number;
      style?: string;
    }
  >;
  links: Record<
    string,
    {
      source_object_id: string;
      target_object_id: string;
      link_type: string;
    }
  >;
}

export function createLibraryItemGroupSnapshot(
  nodes: Node[],
  edges: Edge[],
  cards: Record<string, { id: string; title: string; content: string; version?: string; deleted_at?: number | null }>,
): LibraryItemGroupSnapshot {
  if (nodes.length === 0) {
    throw new Error("A Library schematic must contain at least one canvas item.");
  }
  const selectedNodeIds = new Set(nodes.map((node) => node.id));
  const payloadNodes = toNodePayloads(nodes, cards);
  const minX = Math.min(...payloadNodes.map((node) => node.x));
  const minY = Math.min(...payloadNodes.map((node) => node.y));
  const cardIds = new Set(payloadNodes
    .filter((node) => node.node_type === "voraCard")
    .map((node) => {
      const cardId = node.data.cardId;
      return typeof cardId === "string" ? cardId : node.id;
    }));
  return {
    nodes: payloadNodes.map((node) => ({ ...node, x: node.x - minX, y: node.y - minY })),
    edges: toEdgePayloads(edges.filter((edge) =>
      selectedNodeIds.has(edge.source) && selectedNodeIds.has(edge.target)
    )),
    cards: Object.values(cards)
      .filter((card) => cardIds.has(card.id))
      .map((card) => ({
        id: card.id,
        title: card.title,
        content: card.content,
        version: card.version ?? "v0.1.0",
        deleted_at: card.deleted_at ?? null,
      })),
  };
}

export const dbService = {
  async getJournalEntries(startDate: string, endDate: string): Promise<JournalEntry[]> {
    return await invoke<JournalEntry[]>("get_journal_entries", { startDate, endDate });
  },

  async saveJournalEntry(date: string, content: string): Promise<JournalEntry> {
    return await invoke<JournalEntry>("save_journal_entry", { date, content });
  },

  async getTaskFlowItems(): Promise<TaskFlowItem[]> {
    return await invoke<TaskFlowItem[]>("get_task_flow_items");
  },

  async createTaskFlowItem(title: string): Promise<TaskFlowItem> {
    return await invoke<TaskFlowItem>("create_task_flow_item", { title });
  },

  async setTaskFlowItemCompleted(id: string, completed: boolean): Promise<TaskFlowItem> {
    return await invoke<TaskFlowItem>("set_task_flow_item_completed", { id, completed });
  },

  async reorderTaskFlowItems(ids: string[]): Promise<TaskFlowItem[]> {
    return await invoke<TaskFlowItem[]>("reorder_task_flow_items", { ids });
  },

  async deleteTaskFlowItem(id: string): Promise<void> {
    await invoke("delete_task_flow_item", { id });
  },

  async getAllBoards(): Promise<BoardDto[]> {
    return await invoke<BoardDto[]>("get_all_boards");
  },

  async createBoard(title: string): Promise<BoardDto> {
    return await invoke<BoardDto>("create_board", { title });
  },

  async getBoardVersions(boardId: string): Promise<BoardVersionSummary[]> {
    return await invoke<BoardVersionSummary[]>("get_board_versions", { boardId });
  },

  async createBoardVersion(
    boardId: string,
    increment: BoardVersionIncrement,
    nodes: Node[],
    edges: Edge[],
    cards: Record<string, { id: string; title: string; content: string; version?: string; deleted_at?: number | null }>,
  ): Promise<BoardVersionSummary> {
    const snapshot: BoardVersionSnapshot = {
      nodes: toNodePayloads(nodes, cards),
      edges: toEdgePayloads(edges),
      cards: Object.values(cards).map((card) => ({
        id: card.id,
        title: card.title,
        content: card.content,
        version: card.version ?? "v0.1.0",
        deleted_at: card.deleted_at ?? null,
      })),
    };
    return await invoke<BoardVersionSummary>("create_board_version", {
      boardId,
      increment,
      snapshot,
    });
  },

  async checkoutBoardVersion(boardId: string, versionId: string): Promise<BoardVersionSnapshot> {
    return await invoke<BoardVersionSnapshot>("checkout_board_version", { boardId, versionId });
  },

  async getLibraryItemGroups(): Promise<LibraryItemGroupDto[]> {
    return await invoke<LibraryItemGroupDto[]>("get_library_item_groups");
  },

  async getLibraryItemGroupVersions(groupId: string): Promise<LibraryItemGroupVersion[]> {
    return await invoke<LibraryItemGroupVersion[]>("get_library_item_group_versions", { groupId });
  },

  async getLibraryItemGroupUsages(groupId: string): Promise<LibraryItemGroupUsage[]> {
    return await invoke<LibraryItemGroupUsage[]>("get_library_item_group_usages", { groupId });
  },

  async commitLibraryItemGroupVersion(
    groupId: string,
    version: string,
    snapshot: LibraryItemGroupSnapshot,
  ): Promise<LibraryItemGroupDto> {
    return await invoke<LibraryItemGroupDto>("commit_library_item_group_version", {
      groupId,
      version,
      snapshot,
    });
  },

  async createLibraryItemGroup(
    name: string,
    version: string,
    nodes: Node[],
    edges: Edge[],
    cards: Record<string, { id: string; title: string; content: string; version?: string; deleted_at?: number | null }>,
  ): Promise<LibraryItemGroupDto> {
    const snapshot = createLibraryItemGroupSnapshot(nodes, edges, cards);
    return await invoke<LibraryItemGroupDto>("create_library_item_group", {
      name,
      version,
      snapshot,
    });
  },

  async moveLibraryItemGroupToTrash(groupId: string): Promise<void> {
    await invoke("move_library_item_group_to_trash", { groupId });
  },

  async restoreLibraryItemGroup(groupId: string): Promise<void> {
    await invoke("restore_library_item_group", { groupId });
  },

  async renameBoard(boardId: string, title: string): Promise<void> {
    await invoke("rename_board", { boardId, title });
  },

  async deleteBoard(boardId: string): Promise<void> {
    await invoke("delete_board", { boardId });
  },

  async getAllCards(boardId: string): Promise<CardDto[]> {
    return await invoke<CardDto[]>("get_all_cards", { boardId });
  },

  async saveCardDirect(
    boardId: string,
    id: string,
    title: string,
    content: string,
    version: string,
    textChangeSets: TextChangeSet[] = [],
    librarySourceCardId?: string,
  ): Promise<void> {
    await invoke("save_card_direct", {
      boardId,
      id,
      title,
      content,
      version,
      textChangeSets,
      librarySourceCardId: librarySourceCardId ?? null,
    });
  },

  async getTextLinksForCard(boardId: string, cardId: string): Promise<TextLink[]> {
    return await invoke<TextLink[]>("get_text_links_for_card", { boardId, cardId });
  },

  async createTextLink(input: TextLinkInput): Promise<TextLink> {
    return await invoke<TextLink>("create_text_link", { input });
  },

  async deleteTextLink(linkId: string): Promise<void> {
    await invoke("delete_text_link", { linkId });
  },

  async moveToBin(boardId: string, id: string): Promise<void> {
    await invoke("move_card_to_bin", { boardId, id });
  },

  async restoreFromBin(boardId: string, id: string): Promise<void> {
    await invoke("restore_card_from_bin", { boardId, id });
  },

  async commitSnapshot(
    boardId: string,
    snapshotName: string,
    parentSnapshotId: string | null,
    reason: string,
    summary: string,
    nodes: Node[],
    edges: Edge[],
    cards: Record<string, { title: string; content: string; version?: string }>
  ): Promise<string> {
    const nodePayloads = toNodePayloads(nodes, cards);
    const edgePayloads = toEdgePayloads(edges);

    return await invoke<string>("commit_snapshot", {
      boardId,
      snapshotName,
      parentSnapshotId,
      reason,
      summary,
      nodes: nodePayloads,
      edges: edgePayloads,
    });
  },

  async resolveSnapshot(boardId: string, snapshotId: string): Promise<ResolvedSnapshot> {
    return await invoke<ResolvedSnapshot>("resolve_snapshot", {
      boardId,
      snapshotId,
    });
  },

  async getAllSnapshots(boardId: string): Promise<SnapshotSummary[]> {
    return await invoke<SnapshotSummary[]>("get_all_snapshots", { boardId });
  },
};

export function createTextChangeSet(
  field: TextLinkField,
  previous: string,
  next: string,
): TextChangeSet | null {
  if (previous === next) return null;

  let prefix = 0;
  const sharedLength = Math.min(previous.length, next.length);
  while (prefix < sharedLength && previous.charCodeAt(prefix) === next.charCodeAt(prefix)) {
    prefix += 1;
  }
  if (prefix > 0) {
    const previousCode = previous.charCodeAt(prefix - 1);
    const nextCode = next.charCodeAt(prefix - 1);
    if (isHighSurrogate(previousCode) || isHighSurrogate(nextCode)) prefix -= 1;
  }

  let suffix = 0;
  while (
    suffix < previous.length - prefix &&
    suffix < next.length - prefix &&
    previous.charCodeAt(previous.length - suffix - 1) === next.charCodeAt(next.length - suffix - 1)
  ) {
    suffix += 1;
  }
  while (suffix > 0) {
    const previousBoundary = previous.length - suffix;
    const nextBoundary = next.length - suffix;
    const previousSplitsSurrogate =
      previousBoundary > 0 &&
      previousBoundary < previous.length &&
      isHighSurrogate(previous.charCodeAt(previousBoundary - 1));
    const nextSplitsSurrogate =
      nextBoundary > 0 &&
      nextBoundary < next.length &&
      isHighSurrogate(next.charCodeAt(nextBoundary - 1));
    if (!previousSplitsSurrogate && !nextSplitsSurrogate) break;
    suffix -= 1;
  }

  return {
    field,
    changes: [{
      from: prefix,
      to: previous.length - suffix,
      inserted_length: next.length - prefix - suffix,
      inserted_text: next.slice(prefix, next.length - suffix),
    }],
  };
}

function isHighSurrogate(value: number): boolean {
  return value >= 0xd800 && value <= 0xdbff;
}

function toNodePayloads(
  nodes: Node[],
  cards: Record<string, { title: string; content: string; version?: string }>,
): NodePayload[] {
  return nodes.map((node) => {
    const cardId = typeof node.data?.cardId === "string" ? node.data.cardId : node.id;
    const card = node.type === "voraCard" ? cards[cardId] : undefined;
    const label = node.data?.label;
    const title = typeof node.data?.title === "string"
      ? node.data.title
      : typeof label === "string" ? label : "Untitled";
    const content = typeof node.data?.content === "string" ? node.data.content : "";
    const version = typeof node.data?.version === "string" ? node.data.version : "v0.1.0";
    return {
      id: node.id,
      node_type: node.type ?? "voraCard",
      title: card?.title ?? title,
      content: card?.content ?? content,
      version: card?.version ?? version,
      data: node.data ?? {},
      x: node.position.x,
      y: node.position.y,
      width: node.measured?.width || node.width || 320,
      height: node.measured?.height || node.height || 180,
      style: node.style ? JSON.stringify(node.style) : undefined,
    };
  });
}

function toEdgePayloads(edges: Edge[]): EdgePayload[] {
  return edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    link_type: "connects_to",
  }));
}
