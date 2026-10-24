// src/hooks/useAutoResizeTextarea.ts
import { useLayoutEffect, useRef } from "react";

export function useAutoResizeTextarea(value: string, fontSize?: number) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = "0px";
      el.style.height = `${el.scrollHeight}px`;
    }
  }, [value, fontSize]);

  return textareaRef;
}
