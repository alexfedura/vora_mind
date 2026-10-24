/**
 * Interactive task-list checkbox widget.
 * Toggling a checkbox updates the Markdown source through a normal editor
 * transaction, preserving undo, redo, and document change tracking.
 */
import { EditorView, WidgetType } from '@codemirror/view';
import type { EditorState } from '@codemirror/state';

export class TaskCheckboxWidget extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly markerFrom: number,
    readonly markerTo: number,
  ) {
    super();
  }

  eq(other: TaskCheckboxWidget): boolean {
    return (
      other.checked === this.checked &&
      other.markerFrom === this.markerFrom &&
      other.markerTo === this.markerTo
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const wrapper = document.createElement('span');
    wrapper.className = 'vm-task-checkbox-wrap';

    const input = document.createElement('input');
    input.type = 'checkbox';
    input.className = 'vm-task-checkbox';
    input.checked = this.checked;
    input.setAttribute('aria-label', this.checked ? 'Mark as not done' : 'Mark as done');
    input.addEventListener('mousedown', (event) => event.preventDefault());
    input.addEventListener('click', (event) => {
      event.preventDefault();
      toggleTaskMarker(view, this.markerFrom, this.markerTo, this.checked);
    });

    wrapper.appendChild(input);
    return wrapper;
  }

  ignoreEvent(event: Event): boolean {
    return event.type !== 'mousedown' && event.type !== 'click';
  }
}

function toggleTaskMarker(
  view: EditorView,
  from: number,
  to: number,
  wasChecked: boolean,
): void {
  const state: EditorState = view.state;
  if (from < 0 || to > state.doc.length || to - from !== 3) return;

  const current = state.doc.sliceString(from, to);
  const expected = wasChecked ? ['[x]', '[X]'] : ['[ ]'];
  if (!expected.includes(current)) return;

  view.dispatch({
    changes: { from, to, insert: wasChecked ? '[ ]' : '[x]' },
    scrollIntoView: false,
  });
}
