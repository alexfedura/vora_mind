/**
 * CodeMirror theme for VoraMind.
 * Theme colors use --vm-* variables with fallbacks for the dark theme.
 */
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags as t } from '@lezer/highlight';

const FALLBACK = {
  bg: '#1e1e1e',
  bgPanel: '#252525',
  fg: '#dcddde',
  fgMuted: '#888888',
  fgFaint: '#666666',
  border: '#3d3d3d',
  accent: '#7c3aed',
  accentSoft: 'rgba(124, 58, 237, 0.12)',
  selection: 'rgba(124, 58, 237, 0.28)',
  selectionMatch: 'rgba(124, 58, 237, 0.18)',
  cursor: '#a78bfa',
  activeLine: 'rgba(255, 255, 255, 0.04)',
  link: '#818cf8',
  codeBg: 'rgba(255, 255, 255, 0.06)',
  hlKeyword: '#c792ea',
  hlString: '#c3e88d',
  hlNumber: '#f78c6c',
  hlComment: '#6a7a82',
  hlType: '#ffcb6b',
  hlFunction: '#82aaff',
  hlProperty: '#82aaff',
  hlTag: '#f07178',
  hlInvalid: '#ff5370',
} as const;

function v(name: keyof typeof FALLBACK): string {
  return `var(--vm-${name}, ${FALLBACK[name]})`;
}

export const vmEditorTheme = EditorView.theme(
  {
    '&': {
      color: v('fg'),
      backgroundColor: v('bg'),
      fontSize: '16px',
      fontFamily:
        'ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    },
    '&.cm-focused': {
      outline: 'none',
    },
    '.cm-scroller': {
      fontFamily: 'inherit',
      lineHeight: '1.65',
      overflow: 'auto',
    },
    '.cm-content': {
      caretColor: v('cursor'),
      padding: '24px 0',
      maxWidth: '720px',
      margin: '0 auto',
    },
    '.cm-line': {
      padding: '0 16px',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: v('cursor'),
      borderLeftWidth: '2px',
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: v('selection'),
    },
    '& .cm-selectionMatch': {
      backgroundColor: v('selectionMatch'),
    },
    '&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket': {
      backgroundColor: v('accentSoft'),
      outline: `1px solid ${v('accent')}`,
    },
    '.cm-activeLine': {
      backgroundColor: v('activeLine'),
    },
    '.cm-gutters': {
      backgroundColor: v('bgPanel'),
      color: v('fgFaint'),
      border: 'none',
      borderRight: `1px solid ${v('border')}`,
    },
    '.cm-activeLineGutter': {
      backgroundColor: v('activeLine'),
      color: v('fgMuted'),
    },
    '.cm-foldPlaceholder': {
      backgroundColor: 'transparent',
      border: 'none',
      color: v('fgMuted'),
    },
    '.cm-tooltip': {
      backgroundColor: v('bgPanel'),
      border: `1px solid ${v('border')}`,
      color: v('fg'),
      borderRadius: '6px',
    },
    '.cm-tooltip-autocomplete': {
      '& > ul > li[aria-selected]': {
        backgroundColor: v('accentSoft'),
        color: v('fg'),
      },
    },
    '.cm-panels': {
      backgroundColor: v('bgPanel'),
      color: v('fg'),
      borderTop: `1px solid ${v('border')}`,
      borderBottom: `1px solid ${v('border')}`,
    },
    '.cm-searchMatch': {
      backgroundColor: v('selection'),
      outline: `1px solid ${v('accent')}`,
    },
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: v('selectionMatch'),
    },
    '.vm-math-placeholder, .vm-mermaid-placeholder': {
      opacity: '0.6',
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
      fontSize: '0.9em',
    },
    '.vm-math-error, .vm-mermaid-error': {
      color: v('hlInvalid'),
      borderLeft: `3px solid ${v('hlInvalid')}`,
      paddingLeft: '8px',
    },
  },
  { dark: true },
);

export const vmHighlightStyle = HighlightStyle.define([
  { tag: t.heading1, fontSize: '1.8em', fontWeight: '700', lineHeight: '1.3' },
  { tag: t.heading2, fontSize: '1.5em', fontWeight: '700', lineHeight: '1.35' },
  { tag: t.heading3, fontSize: '1.25em', fontWeight: '600', lineHeight: '1.4' },
  { tag: t.heading4, fontSize: '1.1em', fontWeight: '600' },
  { tag: t.heading5, fontSize: '1.05em', fontWeight: '600' },
  { tag: t.heading6, fontSize: '1em', fontWeight: '600', color: v('fgMuted') },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strikethrough, textDecoration: 'line-through', color: v('fgMuted') },
  { tag: t.link, color: v('link'), textDecoration: 'underline' },
  { tag: t.url, color: v('link') },
  {
    tag: t.monospace,
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
    fontSize: '0.9em',
    backgroundColor: v('codeBg'),
    borderRadius: '3px',
    padding: '1px 4px',
  },
  { tag: t.quote, color: v('fgMuted'), fontStyle: 'italic' },
  { tag: t.list, color: v('fgMuted'), fontSize: '1.2em', fontWeight: '700' },
  { tag: t.contentSeparator, color: v('border') },
  { tag: t.processingInstruction, color: v('fgFaint') },
  { tag: t.meta, color: v('fgFaint') },
  { tag: t.keyword, color: v('hlKeyword') },
  { tag: t.string, color: v('hlString') },
  { tag: t.number, color: v('hlNumber') },
  { tag: t.comment, color: v('hlComment'), fontStyle: 'italic' },
  { tag: t.typeName, color: v('hlType') },
  { tag: [t.function(t.variableName), t.labelName], color: v('hlFunction') },
  { tag: t.propertyName, color: v('hlProperty') },
  { tag: t.tagName, color: v('hlTag') },
  { tag: t.invalid, color: v('hlInvalid') },
]);

export const vmSyntaxHighlighting = syntaxHighlighting(vmHighlightStyle);
