/**
 * Callout block widget rendered through the shared markdown-it pipeline.
 */
import { WidgetType } from '@codemirror/view';
import { renderMarkdown } from '../markdown/renderer';

export class CalloutWidget extends WidgetType {
  constructor(readonly raw: string) {
    super();
  }

  eq(other: CalloutWidget): boolean {
    return other.raw === this.raw;
  }

  toDOM(): HTMLElement {
    const container = document.createElement('div');
    container.className = 'vm-callout-block';
    try {
      container.innerHTML = renderMarkdown(this.raw);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const fallback = document.createElement('pre');
      fallback.className = 'vm-callout-error';
      fallback.textContent = `Callout render error: ${message}\n\n${this.raw}`;
      container.replaceChildren(fallback);
    }
    return container;
  }

  ignoreEvent(): boolean {
    return true;
  }
}
