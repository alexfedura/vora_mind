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
  autoLineDirection?: boolean;
  ariaLabel: string;
  onBlur?: () => void;
}

export function MarkdownTextEditor({
  value,
  onChange,
  className = "",
  placeholder = "Start writing...",
  direction = "auto",
  autoLineDirection = false,
  ariaLabel,
  onBlur,
}: MarkdownTextEditorProps): ReactElement {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const onBlurRef = useRef(onBlur);
  onChangeRef.current = onChange;
  onBlurRef.current = onBlur;

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
          EditorView.domEventHandlers({
            blur: () => {
              onBlurRef.current?.();
              return false;
            },
          }),
        ],
      }),
      parent: host,
    });
    viewRef.current = view;

    const applyLineDirection = (): void => {
      for (const line of view.contentDOM.querySelectorAll<HTMLElement>(".cm-line")) {
        if (line.getAttribute("dir") !== "auto") line.setAttribute("dir", "auto");
      }
    };
    let lineObserver: MutationObserver | undefined;
    if (autoLineDirection) {
      applyLineDirection();
      lineObserver = new MutationObserver(applyLineDirection);
      lineObserver.observe(view.contentDOM, {
        childList: true,
        characterData: true,
        subtree: true,
      });
    }

    return () => {
      lineObserver?.disconnect();
      view.destroy();
      viewRef.current = null;
    };
  }, [ariaLabel, autoLineDirection, direction, placeholder]);

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
