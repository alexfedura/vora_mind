/**
 * Inline or block math widget backed by the lazy KaTeX loader.
 */
import { WidgetType } from '@codemirror/view';
import {
  isKatexReady,
  onKatexReady,
  renderMath,
} from '../../../shared/lazy/katex-loader';

export class MathWidget extends WidgetType {
  private unsubscribe: (() => void) | null = null;

  constructor(
    readonly src: string,
    readonly displayMode: boolean,
  ) {
    super();
  }

  eq(other: MathWidget): boolean {
    return other.src === this.src && other.displayMode === this.displayMode;
  }

  toDOM(): HTMLElement {
    const container = document.createElement(this.displayMode ? 'div' : 'span');
    container.className = this.displayMode
      ? 'vm-math-inline vm-math-display'
      : 'vm-math-inline';

    const rendered = renderMath(this.src, this.displayMode);
    container.innerHTML = rendered.html;

    if (rendered.isPlaceholder && !isKatexReady()) {
      this.unsubscribe = onKatexReady(() => {
        container.innerHTML = renderMath(this.src, this.displayMode).html;
      });
    }

    return container;
  }

  destroy(): void {
    this.unsubscribe?.();
    this.unsubscribe = null;
  }

  ignoreEvent(): boolean {
    return true;
  }
}
