/**
 * markdown-it plugin for Obsidian-compatible blockquote callouts.
 * Syntax: > [!TYPE] Optional title, followed by one or more quoted body lines.
 */
import type MarkdownIt from 'markdown-it';
import type Token from 'markdown-it/lib/token.mjs';

export const CALLOUT_TYPES = [
  'note',
  'tip',
  'warning',
  'danger',
  'info',
  'success',
  'question',
  'quote',
  'example',
] as const;

export type CalloutType = (typeof CALLOUT_TYPES)[number];

const ICONS: Record<CalloutType, string> = {
  note: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="16" y2="17"/></svg>',
  tip: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18h6"/><path d="M10 22h4"/><path d="M12 2a7 7 0 0 0-4 12.7c.6.5 1 1.3 1 2.1V18h6v-1.2c0-.8.4-1.6 1-2.1A7 7 0 0 0 12 2z"/></svg>',
  warning: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
  danger: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>',
  info: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>',
  success: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>',
  question: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
  quote: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 21c3 0 7-1 7-8V5c0-1.25-.756-2.017-2-2H4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2 1 0 1 0 1 1v1c0 1-1 2-2 2s-1 .008-1 1.031V20c0 1 0 1 1 1z"/><path d="M15 21c3 0 7-1 7-8V5c0-1.25-.757-2.017-2-2h-4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2h.75c0 2.25.25 4-2.75 4v3c0 1 0 1 1 1z"/></svg>',
  example: '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>',
};

const HEADER_RE = /^\[!([A-Za-z]+)\]([+-])?\s*(.*)$/;

function isCalloutType(value: string): value is CalloutType {
  return (CALLOUT_TYPES as readonly string[]).includes(value);
}

function isCalloutMeta(value: unknown): value is { type: CalloutType; title: string } {
  if (typeof value !== 'object' || value === null) return false;
  if (!('type' in value) || !('title' in value)) return false;
  return typeof value.type === 'string' && isCalloutType(value.type) && typeof value.title === 'string';
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export default function calloutPlugin(md: MarkdownIt): void {
  md.core.ruler.push('vora_callout', (state) => {
    const { tokens } = state;
    for (let index = 0; index < tokens.length; index += 1) {
      const open = tokens[index];
      const paragraph = tokens[index + 1];
      const inline = tokens[index + 2];
      if (
        open?.type !== 'blockquote_open' ||
        paragraph?.type !== 'paragraph_open' ||
        inline?.type !== 'inline'
      ) continue;

      const header = HEADER_RE.exec(inline.content.split('\n')[0] ?? '');
      const typeName = header?.[1]?.toLowerCase();
      if (!header || !typeName || !isCalloutType(typeName)) continue;

      const calloutMeta = { type: typeName, title: header[3] ?? '' };
      open.meta = calloutMeta;

      let blockquoteDepth = 0;
      let close: Token | undefined;
      for (let tokenIndex = index; tokenIndex < tokens.length; tokenIndex += 1) {
        const token = tokens[tokenIndex];
        if (token?.type === 'blockquote_open') blockquoteDepth += 1;
        if (token?.type === 'blockquote_close') {
          blockquoteDepth -= 1;
          if (blockquoteDepth === 0) {
            close = token;
            break;
          }
        }
      }
      if (close) close.meta = calloutMeta;

      inline.content = inline.content.split('\n').slice(1).join('\n');
      if (inline.children) {
        const lineBreakIndex = inline.children.findIndex(
          (token) => token.type === 'softbreak' || token.type === 'hardbreak',
        );
        inline.children.splice(0, lineBreakIndex >= 0 ? lineBreakIndex + 1 : inline.children.length);
      }
    }
    return true;
  });

  md.renderer.rules.blockquote_open = (tokens, index, options, _env, self) => {
    const meta: unknown = tokens[index]?.meta;
    if (!isCalloutMeta(meta)) return self.renderToken(tokens, index, options);

    const displayTitle = meta.title || meta.type.charAt(0).toUpperCase() + meta.type.slice(1);
    return (
      `<div class="vm-callout vm-callout-${meta.type}">` +
      `<div class="vm-callout-title">` +
      `<span class="vm-callout-icon">${ICONS[meta.type]}</span>` +
      `<span class="vm-callout-title-text">${escapeHtml(displayTitle)}</span>` +
      `</div>` +
      `<div class="vm-callout-body">\n`
    );
  };

  md.renderer.rules.blockquote_close = (tokens, index, options, _env, self) => {
    const meta: unknown = tokens[index]?.meta;
    return isCalloutMeta(meta)
      ? '</div></div>\n'
      : self.renderToken(tokens, index, options);
  };
}
