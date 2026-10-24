declare module 'markdown-it-mark' {
  import type MarkdownIt from 'markdown-it';

  const markPlugin: (md: MarkdownIt, options?: { html?: boolean }) => void;
  export default markPlugin;
}

declare module 'markdown-it-task-lists' {
  import type MarkdownIt from 'markdown-it';

  interface TaskListsOptions {
    enabled?: boolean;
    label?: boolean;
    labelAfter?: boolean;
  }

  const taskListsPlugin: (md: MarkdownIt, options?: TaskListsOptions) => void;
  export default taskListsPlugin;
}

declare module 'markdown-it-footnote' {
  import type MarkdownIt from 'markdown-it';

  const footnotePlugin: (md: MarkdownIt) => void;
  export default footnotePlugin;
}
