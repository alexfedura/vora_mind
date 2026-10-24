/**
 * Optional highlight widget for read-only inline rendering.
 * Live preview uses a mark decoration instead.
 */
import { WidgetType } from '@codemirror/view';

export const HIGHLIGHT_CLASS = 'vm-md-highlight';

export class HighlightWidget extends WidgetType {
  constructor(readonly text: string) {
    super();
  }

  eq(other: HighlightWidget): boolean {
    return other.text === this.text;
  }

  toDOM(): HTMLElement {
    const span = document.createElement('span');
    span.className = HIGHLIGHT_CLASS;
    span.textContent = this.text;
    return span;
  }

  ignoreEvent(): boolean {
    return false;
  }
}
