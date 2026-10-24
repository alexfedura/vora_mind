/**
 * Central markdown-it instance for VoraMind.
 * Keep plugin order aligned with the live-preview parser.
 */
import MarkdownIt from 'markdown-it';
import calloutPlugin from './plugins/callout';
import footnotePlugin from './plugins/footnote';
import highlightPlugin from './plugins/highlight';
import taskListPlugin from './plugins/task-list';

let markdownInstance: MarkdownIt | null = null;

function buildMarkdownIt(): MarkdownIt {
  const md = new MarkdownIt({
    html: false,
    linkify: true,
    typographer: true,
    breaks: false,
    xhtmlOut: false,
  });

  md.use(footnotePlugin);
  md.use(taskListPlugin);
  md.use(highlightPlugin);
  md.use(calloutPlugin);

  const defaultLinkOpen =
    md.renderer.rules.link_open ??
    ((_tokens, index, options, _env, self) => self.renderToken(_tokens, index, options));
  md.renderer.rules.link_open = (tokens, index, options, env, self) => {
    const token = tokens[index];
    if (token && !token.attrGet('class')) token.attrSet('class', 'vm-link');
    token?.attrSet('rel', 'noopener noreferrer');
    return defaultLinkOpen(tokens, index, options, env, self);
  };

  return md;
}

export function getMarkdownIt(): MarkdownIt {
  markdownInstance ??= buildMarkdownIt();
  return markdownInstance;
}

/**
 * Render markdown to HTML. Raw HTML is disabled by the shared instance.
 */
export function renderMarkdown(source: string): string {
  return source ? getMarkdownIt().render(source) : '';
}

/** Render inline markdown without block wrappers. */
export function renderInline(source: string): string {
  return source ? getMarkdownIt().renderInline(source) : '';
}

const INLINE_MATH_RE = /(^|[^\\])\$(?!\$)([^$\n]+?)\$(?!\$)/;
const BLOCK_MATH_RE = /^\s*\$\$[\s\S]*?\$\$\s*$/m;
const MERMAID_FENCE_RE = /^```mermaid\b/m;

export function hasMath(source: string): boolean {
  return INLINE_MATH_RE.test(source) || BLOCK_MATH_RE.test(source);
}

export function hasMermaid(source: string): boolean {
  return MERMAID_FENCE_RE.test(source);
}
