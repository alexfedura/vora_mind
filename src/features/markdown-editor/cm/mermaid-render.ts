/**
 * Mermaid block widget backed by the lazy Mermaid loader.
 */
import { WidgetType } from '@codemirror/view';
import { renderMermaidAsync } from '../../../shared/lazy/mermaid-loader';

let idCounter = 0;

function nextId(): string {
  idCounter = (idCounter + 1) % 1_000_000;
  return `vm-mmd-${Date.now().toString(36)}-${idCounter.toString(36)}`;
}

export class MermaidWidget extends WidgetType {
  constructor(readonly code: string) {
    super();
  }

  eq(other: MermaidWidget): boolean {
    return other.code === this.code;
  }

  toDOM(): HTMLElement {
    const container = document.createElement('div');
    container.className = 'vm-mermaid-container';
    const pre = document.createElement('pre');
    pre.className = 'vm-mermaid-placeholder';
    pre.textContent = this.code;
    container.appendChild(pre);

    void renderMermaidAsync(nextId(), this.code).then((svg) => {
      if (container.isConnected) container.innerHTML = svg;
    });
    return container;
  }

  ignoreEvent(): boolean {
    return true;
  }
}
