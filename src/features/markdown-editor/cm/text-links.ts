import {
  StateEffect,
  StateField,
  type Extension,
} from "@codemirror/state";
import {
  Decoration,
  EditorView,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import type { TextLink } from "@/mind/types";

export const replaceTextLinks = StateEffect.define<TextLink[]>();

const textLinksField = StateField.define<TextLink[]>({
  create: () => [],
  update(links, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(replaceTextLinks)) return effect.value;
    }
    return links;
  },
});

const linkDecoration = Decoration.mark({ class: "vm-text-link-anchor" });

function buildLinkDecorations(view: EditorView): DecorationSet {
  const ranges = new Map<string, { from: number; to: number }>();
  for (const link of view.state.field(textLinksField)) {
    if (
      link.source_field !== "content" ||
      link.source_start < 0 ||
      link.source_end > view.state.doc.length ||
      link.source_end <= link.source_start
    ) continue;
    ranges.set(`${link.source_start}:${link.source_end}`, {
      from: link.source_start,
      to: link.source_end,
    });
  }
  const decorations = [...ranges.values()].sort((first, second) =>
    first.from - second.from || first.to - second.to,
  );
  return Decoration.set(decorations.map(({ from, to }) => linkDecoration.range(from, to)), true);
}

const textLinkPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildLinkDecorations(view);
    }

    update(update: ViewUpdate): void {
      if (
        update.docChanged ||
        update.transactions.some((transaction) =>
          transaction.effects.some((effect) => effect.is(replaceTextLinks)),
        )
      ) {
        this.decorations = buildLinkDecorations(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

export const textLinkExtension: Extension = [textLinksField, textLinkPlugin];

export function getEditorTextLinks(view: EditorView): TextLink[] {
  return view.state.field(textLinksField);
}
