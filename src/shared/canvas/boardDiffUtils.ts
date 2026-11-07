import type {
  BoardVersionCard,
  BoardVersionSnapshot,
  EdgePayload,
  NodePayload,
} from "@/mind/services/dbService";

export interface BoardCardState extends BoardVersionCard {
  x: number | null;
  y: number | null;
}

export interface ModifiedBoardCard {
  previous: BoardCardState;
  current: BoardCardState;
  titleChanged: boolean;
  contentChanged: boolean;
  positionChanged: boolean;
}

export interface ModifiedBoardObject {
  previous: NodePayload;
  current: NodePayload;
  titleChanged: boolean;
  contentChanged: boolean;
  positionChanged: boolean;
}

export interface BoardDiffResult {
  addedCards: BoardCardState[];
  removedCards: BoardCardState[];
  modifiedCards: ModifiedBoardCard[];
  addedObjects: NodePayload[];
  removedObjects: NodePayload[];
  modifiedObjects: ModifiedBoardObject[];
  addedEdges: EdgePayload[];
  removedEdges: EdgePayload[];
}

function cardPositions(snapshot: BoardVersionSnapshot): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  for (const node of snapshot.nodes) {
    if (node.node_type !== "voraCard") continue;
    const cardId = typeof node.data.cardId === "string" ? node.data.cardId : node.id;
    positions.set(cardId, { x: node.x, y: node.y });
  }
  return positions;
}

function cardsWithPositions(snapshot: BoardVersionSnapshot): Map<string, BoardCardState> {
  const positions = cardPositions(snapshot);
  return new Map(snapshot.cards.map((card) => {
    const position = positions.get(card.id);
    return [card.id, { ...card, x: position?.x ?? null, y: position?.y ?? null }];
  }));
}

function compareEdges(
  oldEdges: EdgePayload[],
  newEdges: EdgePayload[],
): { addedEdges: EdgePayload[]; removedEdges: EdgePayload[] } {
  const edgeKey = (edge: EdgePayload): string =>
    JSON.stringify([edge.id, edge.source, edge.target, edge.link_type ?? "connects_to"]);
  const oldKeys = new Set(oldEdges.map(edgeKey));
  const newKeys = new Set(newEdges.map(edgeKey));
  return {
    addedEdges: newEdges.filter((edge) => !oldKeys.has(edgeKey(edge))),
    removedEdges: oldEdges.filter((edge) => !newKeys.has(edgeKey(edge))),
  };
}

function compareObjects(
  oldNodes: NodePayload[],
  newNodes: NodePayload[],
): Pick<BoardDiffResult, "addedObjects" | "removedObjects" | "modifiedObjects"> {
  const oldObjects = new Map(oldNodes.filter((node) => node.node_type !== "voraCard").map((node) => [node.id, node]));
  const newObjects = new Map(newNodes.filter((node) => node.node_type !== "voraCard").map((node) => [node.id, node]));
  const addedObjects = [...newObjects.values()].filter((node) => !oldObjects.has(node.id));
  const removedObjects = [...oldObjects.values()].filter((node) => !newObjects.has(node.id));
  const modifiedObjects: ModifiedBoardObject[] = [];

  for (const [id, current] of newObjects) {
    const previous = oldObjects.get(id);
    if (!previous) continue;
    const titleChanged = previous.title !== current.title;
    const contentChanged = previous.content !== current.content;
    const positionChanged = previous.x !== current.x || previous.y !== current.y;
    if (titleChanged || contentChanged || positionChanged) {
      modifiedObjects.push({ previous, current, titleChanged, contentChanged, positionChanged });
    }
  }
  return { addedObjects, removedObjects, modifiedObjects };
}

export function diffBoardSnapshots(
  oldSnapshot: BoardVersionSnapshot,
  newSnapshot: BoardVersionSnapshot,
): BoardDiffResult {
  const oldCards = cardsWithPositions(oldSnapshot);
  const newCards = cardsWithPositions(newSnapshot);
  const addedCards = [...newCards.values()].filter((card) => !oldCards.has(card.id));
  const removedCards = [...oldCards.values()].filter((card) => !newCards.has(card.id));
  const modifiedCards: ModifiedBoardCard[] = [];

  for (const [id, current] of newCards) {
    const previous = oldCards.get(id);
    if (!previous) continue;
    const titleChanged = previous.title !== current.title;
    const contentChanged = previous.content !== current.content;
    const positionChanged = previous.x !== current.x || previous.y !== current.y;
    if (titleChanged || contentChanged || positionChanged) {
      modifiedCards.push({ previous, current, titleChanged, contentChanged, positionChanged });
    }
  }

  const objectChanges = compareObjects(oldSnapshot.nodes, newSnapshot.nodes);
  const edgeChanges = compareEdges(oldSnapshot.edges, newSnapshot.edges);
  return {
    addedCards,
    removedCards,
    modifiedCards,
    ...objectChanges,
    ...edgeChanges,
  };
}
