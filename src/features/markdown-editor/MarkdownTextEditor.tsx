import { useEffect, useRef } from "react";
import type { ReactElement } from "react";
import { EditorView, placeholder as editorPlaceholder } from "@codemirror/view";
import { createEditorState } from "./cm/setup";
import { livePreviewExtension } from "./cm/live-preview";

export interface MarkdownTextEditorProps {
  value: string;
  onChange: (value: string) => void;
  className?: string;
  placeholder?: string;
  direction?: "auto" | "ltr" | "rtl";
  ariaLabel: string;
}

export function MarkdownTextEditor({
  value,
  onChange,
  className = "",
  placeholder = "Start writing...",
  direction = "auto",
  ariaLabel,
}: MarkdownTextEditorProps): ReactElement {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const view = new EditorView({
      state: createEditorState(value, {
        livePreview: livePreviewExtension,
        extra: [
          editorPlaceholder(placeholder),
          EditorView.contentAttributes.of({
            dir: direction,
            "aria-label": ariaLabel,
          }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) onChangeRef.current(update.state.doc.toString());
          }),
        ],
      }),
      parent: host,
    });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [ariaLabel, direction, placeholder]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view || view.state.doc.toString() === value) return;
    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: value,
      },
    });
  }, [value]);

  return <div ref={hostRef} className={`vm-editor-wrapper ${className}`} />;
}
