export interface TextOffsets {
  startOffset: number;
  endOffset: number;
}

export interface TextRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export function getTextOffsets(root: HTMLElement, range: Range): TextOffsets | null {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node = walker.nextNode();
  let offset = 0;
  let startOffset: number | null = null;
  let endOffset: number | null = null;

  while (node) {
    const textLength = node.textContent?.length ?? 0;
    if (node === range.startContainer) startOffset = offset + range.startOffset;
    if (node === range.endContainer) endOffset = offset + range.endOffset;
    offset += textLength;
    node = walker.nextNode();
  }

  if (startOffset === null || endOffset === null || endOffset <= startOffset) return null;
  return { startOffset, endOffset };
}

export function getTextRects(
  root: HTMLElement,
  startOffset: number,
  endOffset: number,
): TextRect[] {
  if (startOffset < 0 || endOffset <= startOffset) return [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node = walker.nextNode();
  let textOffset = 0;
  const rects: TextRect[] = [];
  const rootRect = root.getBoundingClientRect();

  while (node) {
    const textLength = node.textContent?.length ?? 0;
    const nodeStart = textOffset;
    const nodeEnd = textOffset + textLength;
    const rangeStart = Math.max(startOffset, nodeStart);
    const rangeEnd = Math.min(endOffset, nodeEnd);
    if (rangeStart < rangeEnd) {
      const range = document.createRange();
      range.setStart(node, rangeStart - nodeStart);
      range.setEnd(node, rangeEnd - nodeStart);
      for (const rect of Array.from(range.getClientRects())) {
        rects.push({
          left: rect.left - rootRect.left + root.scrollLeft,
          top: rect.top - rootRect.top + root.scrollTop,
          width: rect.width,
          height: rect.height,
        });
      }
    }
    textOffset = nodeEnd;
    node = walker.nextNode();
  }
  return rects;
}
