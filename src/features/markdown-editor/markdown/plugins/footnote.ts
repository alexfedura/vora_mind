/**
 * markdown-it plugin for footnotes with stable CSS class names.
 */
import type MarkdownIt from 'markdown-it';
import type Token from 'markdown-it/lib/token.mjs';
import footnotePluginImpl from 'markdown-it-footnote';

interface FootnoteMeta {
  id: number;
  subId: number;
}

function readFootnoteMeta(token: Token): FootnoteMeta {
  const meta: unknown = token.meta;
  if (
    typeof meta !== 'object' ||
    meta === null ||
    !('id' in meta) ||
    typeof meta.id !== 'number' ||
    !Number.isInteger(meta.id)
  ) {
    throw new Error(`Invalid markdown footnote metadata on ${token.type} token.`);
  }
  const subId = 'subId' in meta ? meta.subId : 0;
  if (typeof subId !== 'number' || !Number.isInteger(subId)) {
    throw new Error(`Invalid markdown footnote reference metadata on ${token.type} token.`);
  }
  return { id: meta.id + 1, subId };
}

function referenceSuffix(subId: number): string {
  return subId > 0 ? `:${subId}` : '';
}

export default function footnotePlugin(md: MarkdownIt): void {
  md.use(footnotePluginImpl);

  md.renderer.rules.footnote_ref = (tokens, index) => {
    const { id, subId } = readFootnoteMeta(tokens[index]);
    const refId = referenceSuffix(subId);
    return (
      `<sup class="vm-footnote-ref">` +
      `<a href="#fn${id}" id="fnref${id}${refId}" class="vm-footnote-link">[${id}]</a>` +
      `</sup>`
    );
  };

  md.renderer.rules.footnote_block_open = () =>
    `<hr class="vm-footnote-sep"/>\n` +
    `<section class="vm-footnotes">\n` +
    `<ol class="vm-footnotes-list">\n`;

  md.renderer.rules.footnote_block_close = () => '</ol>\n</section>\n';

  md.renderer.rules.footnote_open = (tokens, index) => {
    const { id } = readFootnoteMeta(tokens[index]);
    return `<li id="fn${id}" class="vm-footnote-item">`;
  };

  md.renderer.rules.footnote_close = () => '</li>\n';

  md.renderer.rules.footnote_anchor = (tokens, index) => {
    const { id, subId } = readFootnoteMeta(tokens[index]);
    const refId = referenceSuffix(subId);
    return ` <a href="#fnref${id}${refId}" class="vm-footnote-backref" aria-label="Back to reference">↩</a>`;
  };
}
