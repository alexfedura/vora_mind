import { createContext, useContext } from "react";

export interface CanvasZoomScales {
  x: number;
  y: number;
}

const CanvasZoomContext = createContext<CanvasZoomScales>({ x: 1, y: 1 });

export function useCanvasZoomScales(): CanvasZoomScales {
  return useContext(CanvasZoomContext);
}

export { CanvasZoomContext };
