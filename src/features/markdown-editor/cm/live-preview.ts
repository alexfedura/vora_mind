/**
 * Live Preview extension for CodeMirror 6.
 *
 * Markdown source remains the source of truth. Decorations hide syntax and
 * render rich widgets only when the corresponding element is away from the
 * current selection.
 */
import {
  Decoration,
  EditorView,
  ViewPlugin,
  WidgetType,
  type DecorationSet,
  type ViewUpdate,
} from '@codemirror/view';
import {
  EditorState,
  RangeSetBuilder,
  StateEffect,
  type Extension,
} from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import type { SyntaxNode } from '@lezer/common';
import { CalloutWidget } from './callout-widget';
import { TaskCheckboxWidget } from './task-checkbox';
import { MathWidget } from './math-render';
import { MermaidWidget } from './mermaid-render';
import { HighlightWidget } from './highlight-mark';

const REBUILD_DEBOUNCE_MS = 50;
const NEARBY_LINE_TOLERANCE = 1;
const MAX_VISIBLE_BUFFER_LINES = 20;
const MAX_WIDGET_CACHE = 500;

const boldMark = Decoration.mark({ class: 'vm-md-bold' });
const italicMark = Decoration.mark({ class: 'vm-md-italic' });
const strikeMark = Decoration.mark({ class: 'vm-md-strike' });
const codeMark = Decoration.mark({ class: 'vm-md-code' });
const linkMark = Decoration.mark({ class: 'vm-md-link' });
const listLine = Decoration.line({
  attributes: { class: 'vm-md-list-line' },
});
const headingMarks = [
  Decoration.mark({ class: 'vm-md-h1' }),
  Decoration.mark({ class: 'vm-md-h2' }),
  Decoration.mark({ class: 'vm-md-h3' }),
  Decoration.mark({ class: 'vm-md-h4' }),
  Decoration.mark({ class: 'vm-md-h5' }),
  Decoration.mark({ class: 'vm-md-h6' }),
];

interface WidgetCacheEntry {
  widget: WidgetType;
  from: number;
  to: number;
}

const widgetCache = new Map<string, WidgetCacheEntry>();

function hashString(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = (hash + ((hash << 1) + (hash << 4) + (hash << 7) + (hash << 8) + (hash << 24))) >>> 0;
  }
  return hash.toString(36);
}

function getWidget(key: string, from: number, to: number, create: () => WidgetType): WidgetType {
  const cached = widgetCache.get(key);
  if (cached?.from === from && cached.to === to) return cached.widget;

  if (widgetCache.size >= MAX_WIDGET_CACHE) {
    const oldestKey = widgetCache.keys().next().value;
    if (oldestKey !== undefined) widgetCache.delete(oldestKey);
  }
  const widget = create();
  widgetCache.set(key, { widget, from, to });
  return widget;
}

function isNearSelection(state: EditorState, from: number, to: number): boolean {
  const fromLine = state.doc.lineAt(from).number;
  const toLine = state.doc.lineAt(to).number;
  const minLine = Math.max(1, fromLine - NEARBY_LINE_TOLERANCE);
  const maxLine = Math.min(state.doc.lines, toLine + NEARBY_LINE_TOLERANCE);

  return state.selection.ranges.some((range) => {
    if (range.from <= to && range.to >= from) return true;
    const cursorLine = state.doc.lineAt(range.from).number;
    return cursorLine >= minLine && cursorLine <= maxLine;
  });
}

interface PendingDecoration {
  from: number;
  to: number;
  decoration: Decoration;
}

interface BuildContext {
  view: EditorView;
  pending: PendingDecoration[];
  covered: Array<{ from: number; to: number }>;
}

function overlapsCovered(ctx: BuildContext, from: number, to: number): boolean {
  return ctx.covered.some((range) => from < range.to && to > range.from);
}

function addMark(ctx: BuildContext, from: number, to: number, decoration: Decoration): void {
  if (to <= from || overlapsCovered(ctx, from, to)) return;
  ctx.pending.push({ from, to, decoration });
  ctx.covered.push({ from, to });
}

function addReplace(
  ctx: BuildContext,
  from: number,
  to: number,
  widget: WidgetType,
  block = false,
): void {
  if (to <= from || overlapsCovered(ctx, from, to)) return;
  ctx.pending.push({ from, to, decoration: Decoration.replace({ widget, block }) });
  ctx.covered.push({ from, to });
}

class HiddenWidget extends WidgetType {
  toDOM(): HTMLElement {
    const span = document.createElement('span');
    span.className = 'vm-md-hidden';
    span.setAttribute('aria-hidden', 'true');
    return span;
  }

  eq(): boolean {
    return true;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

class ListMarkerWidget extends WidgetType {
  constructor(private readonly marker: string) {
    super();
  }

  toDOM(): HTMLElement {
    const element = document.createElement('span');
    element.className = /^\d+[.)]$/.test(this.marker)
      ? 'vm-md-list-number'
      : 'vm-md-list-bullet';
    element.textContent = this.marker === '-' || this.marker === '*' || this.marker === '+'
      ? '•'
      : this.marker;
    element.setAttribute('aria-hidden', 'true');
    return element;
  }

  eq(other: WidgetType): boolean {
    return other instanceof ListMarkerWidget && other.marker === this.marker;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

function handleHeading(ctx: BuildContext, node: SyntaxNode): void {
  const { state } = ctx.view;
  const line = state.doc.lineAt(node.from);
  const raw = isNearSelection(state, node.from, node.to);
  const level = Number(node.name.replace('ATXHeading', '')) || 1;
  const headingMark = headingMarks[Math.min(6, Math.max(1, level)) - 1];
  let markerFrom = line.from;
  let markerTo = line.from;
  let child = node.firstChild;
  while (child) {
    if (child.name === 'HeaderMark') {
      markerFrom = child.from;
      markerTo = child.to;
      break;
    }
    child = child.nextSibling;
  }
  if (markerTo < line.to && state.doc.sliceString(markerTo, markerTo + 1) === ' ') markerTo += 1;

  if (raw) {
    if (headingMark && line.from < line.to) addMark(ctx, line.from, line.to, headingMark);
    return;
  }
  if (markerTo > markerFrom) addReplace(ctx, markerFrom, markerTo, new HiddenWidget());
  if (headingMark && markerTo < line.to) addMark(ctx, markerTo, line.to, headingMark);
}

function handleDelimitedInline(
  ctx: BuildContext,
  node: SyntaxNode,
  delimiterName: string,
  mark: Decoration,
): void {
  if (isNearSelection(ctx.view.state, node.from, node.to)) return;
  const first = node.firstChild;
  const last = node.lastChild;
  if (first?.name === delimiterName) addReplace(ctx, first.from, first.to, new HiddenWidget());
  if (last?.name === delimiterName && last.from !== first?.from) {
    addReplace(ctx, last.from, last.to, new HiddenWidget());
  }
  const contentFrom = first?.name === delimiterName ? first.to : node.from;
  const contentTo = last?.name === delimiterName ? last.from : node.to;
  if (contentTo > contentFrom) addMark(ctx, contentFrom, contentTo, mark);
}

function handleLink(ctx: BuildContext, node: SyntaxNode): void {
  if (isNearSelection(ctx.view.state, node.from, node.to)) return;
  const marks: SyntaxNode[] = [];
  let child = node.firstChild;
  while (child) {
    if (child.name === 'LinkMark') marks.push(child);
    child = child.nextSibling;
  }
  if (marks.length < 4) return;

  const opening = marks[0];
  const closingLabel = marks[1];
  const openingDestination = marks[2];
  const closingDestination = marks[3];
  if (!opening || !closingLabel || !openingDestination || !closingDestination) return;
  addReplace(ctx, opening.from, opening.to, new HiddenWidget());
  addReplace(ctx, closingLabel.from, closingLabel.to, new HiddenWidget());
  addReplace(ctx, openingDestination.from, closingDestination.to, new HiddenWidget());
  if (closingLabel.from > opening.to) addMark(ctx, opening.to, closingLabel.from, linkMark);
}

function handleListItem(ctx: BuildContext, node: SyntaxNode): void {
  const { state } = ctx.view;
  const line = state.doc.lineAt(node.from);
  const text = state.doc.sliceString(line.from, line.to);
  const match = /^([ \t]*)([-*+]|[0-9]+[.)])(?=[ \t]+)/.exec(text);
  if (!match) return;

  ctx.pending.push({ from: line.from, to: line.from, decoration: listLine });
  const markerFrom = line.from + match[1].length;
  const markerTo = markerFrom + match[2].length;
  const selectionTouchesMarker = state.selection.ranges.some((range) =>
    range.empty
      ? range.from === markerFrom || range.from === markerTo
      : range.from < markerTo && range.to > markerFrom,
  );
  const taskMatch = /^([ \t]*[-*+][ \t]+)(\[[ xX]\])[ \t]+/.exec(text);
  if (!taskMatch && !selectionTouchesMarker) {
    addReplace(ctx, markerFrom, markerTo, new ListMarkerWidget(match[2]));
  }

  if (!taskMatch || isNearSelection(state, line.from, line.to)) return;
  const taskMarkerFrom = line.from + taskMatch[1].length;
  const taskMarkerTo = taskMarkerFrom + 3;
  const checked = taskMatch[2].toLowerCase() === '[x]';
  addReplace(
    ctx,
    taskMarkerFrom,
    Math.min(line.to, taskMarkerTo + 1),
    new TaskCheckboxWidget(checked, taskMarkerFrom, taskMarkerTo),
  );
}

function handleCallout(ctx: BuildContext, node: SyntaxNode): void {
  const { state } = ctx.view;
  const line = state.doc.lineAt(node.from);
  const lineText = state.doc.sliceString(line.from, line.to);
  if (!/^\s*>\s*\[![A-Za-z]+\]/.test(lineText)) return;
  if (isNearSelection(state, node.from, node.to)) return;

  const raw = state.doc.sliceString(node.from, node.to);
  const widget = getWidget(
    `callout:${hashString(raw)}`,
    node.from,
    node.to,
    () => new CalloutWidget(raw),
  );
  addReplace(ctx, node.from, node.to, widget, true);
}

const INLINE_MATH_RE = /(?<!\\)(?<!\$)\$(?!\$)([^$\n]+?)(?<!\\)\$(?!\$)/g;
const BLOCK_MATH_RE = /^[ \t]*\$\$[ \t]*\r?\n([\s\S]*?)\r?\n[ \t]*\$\$[ \t]*$/gm;
const HIGHLIGHT_RE = /(?<![=])==([^=\n]+?)==(?![=])/g;

function isWithinCode(state: EditorState, position: number): boolean {
  let node: SyntaxNode | null = syntaxTree(state).resolveInner(position, -1);
  while (node) {
    if (node.name === 'FencedCode' || node.name === 'InlineCode') return true;
    node = node.parent;
  }
  return false;
}

function scanRegexInRange(ctx: BuildContext, from: number, to: number): void {
  const { state } = ctx.view;
  const text = state.doc.sliceString(from, to);
  let match: RegExpExecArray | null;

  INLINE_MATH_RE.lastIndex = 0;
  while ((match = INLINE_MATH_RE.exec(text)) !== null) {
    const source = match[1];
    if (!source) continue;
    const start = from + match.index;
    const end = start + match[0].length;
    if (isWithinCode(state, start) || isNearSelection(state, start, end)) continue;
    const widget = getWidget(
      `math-inline:${hashString(source)}`,
      start,
      end,
      () => new MathWidget(source, false),
    );
    addReplace(ctx, start, end, widget);
  }

  BLOCK_MATH_RE.lastIndex = 0;
  while ((match = BLOCK_MATH_RE.exec(text)) !== null) {
    const source = match[1]?.trim();
    if (!source) continue;
    const start = from + match.index;
    const end = start + match[0].length;
    if (isWithinCode(state, start) || isNearSelection(state, start, end)) continue;
    const widget = getWidget(
      `math-block:${hashString(source)}`,
      start,
      end,
      () => new MathWidget(source, true),
    );
    addReplace(ctx, start, end, widget, true);
  }

  HIGHLIGHT_RE.lastIndex = 0;
  while ((match = HIGHLIGHT_RE.exec(text)) !== null) {
    const source = match[1];
    if (!source) continue;
    const start = from + match.index;
    const end = start + match[0].length;
    if (isWithinCode(state, start) || isNearSelection(state, start, end)) continue;
    const widget = getWidget(
      `highlight:${hashString(source)}`,
      start,
      end,
      () => new HighlightWidget(source),
    );
    addReplace(ctx, start, end, widget);
  }
}

function handleFencedCode(ctx: BuildContext, node: SyntaxNode): void {
  const { state } = ctx.view;
  if (isNearSelection(state, node.from, node.to)) return;

  let language = '';
  let codeFrom = -1;
  let codeTo = -1;
  let child = node.firstChild;
  while (child) {
    if (child.name === 'CodeMark' && codeFrom < 0) {
      const line = state.doc.lineAt(child.from);
      language = state.doc.sliceString(child.to, line.to).trim().split(/\s+/, 1)[0] ?? '';
    } else if (child.name === 'CodeText') {
      codeFrom = child.from;
      codeTo = child.to;
    }
    child = child.nextSibling;
  }
  if (language !== 'mermaid' || codeFrom < 0 || codeTo < 0) return;

  const source = state.doc.sliceString(codeFrom, codeTo);
  const widget = getWidget(
    `mermaid:${hashString(source)}`,
    node.from,
    node.to,
    () => new MermaidWidget(source),
  );
  addReplace(ctx, node.from, node.to, widget, true);
}

function visibleLineRange(
  state: EditorState,
  from: number,
  to: number,
): { from: number; to: number } {
  const firstLine = state.doc.lineAt(from);
  const lastLine = state.doc.lineAt(to);
  const fromLineNumber = Math.max(1, firstLine.number - MAX_VISIBLE_BUFFER_LINES);
  const toLineNumber = Math.min(state.doc.lines, lastLine.number + MAX_VISIBLE_BUFFER_LINES);
  return {
    from: state.doc.line(fromLineNumber).from,
    to: state.doc.line(toLineNumber).to,
  };
}

function containsMarkdownSyntax(state: EditorState, from: number, to: number): boolean {
  const text = state.doc.sliceString(from, to);
  return /[#*_~`[\]()>$=!+-]/.test(text) || /^\s*\d+[.)]\s/m.test(text);
}

function buildDecorations(view: EditorView): DecorationSet {
  const { state } = view;
  if (state.doc.length === 0 || view.visibleRanges.length === 0) return Decoration.none;

  const ranges = view.visibleRanges.map(({ from, to }) => visibleLineRange(state, from, to));
  if (!ranges.some((range) => containsMarkdownSyntax(state, range.from, range.to))) {
    return Decoration.none;
  }

  const ctx: BuildContext = { view, pending: [], covered: [] };
  for (const range of ranges) {
    syntaxTree(state).iterate({
      from: range.from,
      to: range.to,
      enter: (node) => {
        if (node.name.startsWith('ATXHeading')) handleHeading(ctx, node.node);
        else if (node.name === 'StrongEmphasis') {
          handleDelimitedInline(ctx, node.node, 'EmphasisMark', boldMark);
        } else if (node.name === 'Emphasis') {
          handleDelimitedInline(ctx, node.node, 'EmphasisMark', italicMark);
        } else if (node.name === 'Strikethrough') {
          handleDelimitedInline(ctx, node.node, 'StrikethroughMark', strikeMark);
        } else if (node.name === 'InlineCode') {
          addMark(ctx, node.from, node.to, codeMark);
        } else if (node.name === 'Link') handleLink(ctx, node.node);
        else if (node.name === 'ListItem') handleListItem(ctx, node.node);
        else if (node.name === 'Blockquote') handleCallout(ctx, node.node);
        else if (node.name === 'FencedCode') handleFencedCode(ctx, node.node);
      },
    });
    scanRegexInRange(ctx, range.from, range.to);
  }

  ctx.pending.sort((a, b) =>
    a.from - b.from ||
    a.decoration.startSide - b.decoration.startSide ||
    a.to - b.to,
  );
  const builder = new RangeSetBuilder<Decoration>();
  for (const range of ctx.pending) {
    builder.add(range.from, range.to, range.decoration);
  }
  return builder.finish();
}

export const livePreview = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    private timer: ReturnType<typeof setTimeout> | null = null;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate): void {
      const forceRebuild = update.transactions.some((transaction) =>
        transaction.effects.some((effect) => effect.is(forceLivePreviewRebuild)),
      );
      if (forceRebuild) {
        if (this.timer) clearTimeout(this.timer);
        this.timer = null;
        this.decorations = buildDecorations(update.view);
        return;
      }
      if (!update.docChanged && !update.selectionSet && !update.viewportChanged) return;
      if (this.timer) clearTimeout(this.timer);

      if (update.viewportChanged && !update.docChanged && !update.selectionSet) {
        this.decorations = buildDecorations(update.view);
        return;
      }

      const view = update.view;
      this.timer = setTimeout(() => {
        this.timer = null;
        this.decorations = buildDecorations(view);
        view.dispatch({});
      }, REBUILD_DEBOUNCE_MS);
    }

    destroy(): void {
      if (this.timer) clearTimeout(this.timer);
      this.timer = null;
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
    provide: (plugin) =>
      EditorView.atomicRanges.of((view) => view.plugin(plugin)?.decorations ?? Decoration.none),
  },
);

export const forceLivePreviewRebuild = StateEffect.define<void>();

export const livePreviewExtension: Extension = livePreview;
