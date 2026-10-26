/**
 * CodeMirror 6 extension composition for the VoraMind markdown editor.
 * This is the single source of truth for the editor extension set.
 */
import {
  EditorSelection,
  EditorState,
  Prec,
  type Extension,
} from '@codemirror/state';
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  rectangularSelection,
} from '@codemirror/view';
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from '@codemirror/commands';
import {
  bracketMatching,
  foldGutter,
  foldKeymap,
  indentUnit,
  indentOnInput,
} from '@codemirror/language';
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from '@codemirror/autocomplete';
import {
  highlightSelectionMatches,
  searchKeymap,
} from '@codemirror/search';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { vmEditorTheme, vmSyntaxHighlighting } from './theme';

export interface CreateEditorExtensionsOptions {
  /** Live preview extension, provided by the caller to avoid circular imports. */
  livePreview?: Extension;
  /** Additional host extensions, such as vim mode or collaboration. */
  extra?: Extension[];
}

function visualLineBoundary(forward: boolean, extend: boolean) {
  return (view: EditorView): boolean => {
    const selection = view.state.selection;
    const ranges = selection.ranges.map((range) => {
      const boundary = view.moveToLineBoundary(range, forward, true);
      return extend
        ? EditorSelection.range(range.anchor, boundary.head)
        : EditorSelection.cursor(boundary.head, boundary.assoc);
    });
    view.dispatch({
      selection: EditorSelection.create(ranges, selection.mainIndex),
      scrollIntoView: true,
      userEvent: 'select',
    });
    return true;
  };
}

/** Create the base extensions, optionally adding live preview and host extensions. */
export function createEditorExtensions(
  options: CreateEditorExtensionsOptions = {},
): Extension[] {
  const extensions: Extension[] = [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    history(),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    autocompletion(),
    rectangularSelection(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    EditorView.lineWrapping,
    vmEditorTheme,
    vmSyntaxHighlighting,
    markdown({
      base: markdownLanguage,
      codeLanguages: languages,
      addKeymap: true,
    }),
    EditorState.tabSize.of(4),
    indentUnit.of('    '),
    EditorView.contentAttributes.of({
      autocapitalize: 'sentences',
      autocorrect: 'on',
      spellcheck: 'true',
      'data-vm-editor': 'true',
    }),
  ];

  if (options.livePreview) extensions.push(options.livePreview);
  if (options.extra?.length) extensions.push(...options.extra);

  extensions.push(
    Prec.high(
      keymap.of([
        {
          key: 'Home',
          run: visualLineBoundary(false, false),
          shift: visualLineBoundary(false, true),
        },
        {
          key: 'End',
          run: visualLineBoundary(true, false),
          shift: visualLineBoundary(true, true),
        },
      ]),
    ),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
      indentWithTab,
    ]),
  );

  return extensions;
}

/** Create a fresh EditorState when switching documents. */
export function createEditorState(
  doc: string,
  options: CreateEditorExtensionsOptions = {},
): EditorState {
  return EditorState.create({
    doc,
    extensions: createEditorExtensions(options),
  });
}
