/**
 * markdown-it plugin for ==highlight== syntax.
 * Renders as <mark class="vm-highlight">text</mark>.
 */
import mark from 'markdown-it-mark';
import type MarkdownIt from 'markdown-it';

export default function highlightPlugin(md: MarkdownIt): void {
  md.use(mark, { html: false });
  md.renderer.rules.mark_open = () => '<mark class="vm-highlight">';
  md.renderer.rules.mark_close = () => '</mark>';
}
