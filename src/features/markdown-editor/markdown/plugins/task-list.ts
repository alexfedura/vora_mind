/**
 * markdown-it plugin for read-only GitHub-style task lists.
 * Live-preview interactivity is provided by the CodeMirror checkbox widget.
 */
import type MarkdownIt from 'markdown-it';
import taskLists from 'markdown-it-task-lists';

export default function taskListPlugin(md: MarkdownIt): void {
  md.use(taskLists, { enabled: false, label: true, labelAfter: true });

  md.renderer.rules.task_list_item = (tokens, index) => {
    const token = tokens[index];
    const checked = token?.attrGet('checked') === 'checked' ? 'checked' : '';
    return (
      `<li class="vm-task-item">` +
      `<input type="checkbox" class="vm-task-checkbox" ${checked} disabled/>` +
      `<span class="vm-task-text">`
    );
  };
}
