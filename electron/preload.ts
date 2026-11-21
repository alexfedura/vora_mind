import { contextBridge, ipcRenderer } from "electron";

const api = Object.freeze({
  getShellStatus: (): Promise<unknown> => ipcRenderer.invoke("vora:shell:status"),
  getRuntimeInfo: (): Promise<unknown> => ipcRenderer.invoke("vora:shell:runtime-info"),
  getSidecarStatus: (): Promise<unknown> => ipcRenderer.invoke("vora:sidecar:status"),
  getSidecarEngineInfo: (): Promise<unknown> => ipcRenderer.invoke("vora:sidecar:engine-info"),
  runSidecarSmoke: (): Promise<unknown> => ipcRenderer.invoke("vora:sidecar:smoke")
});

contextBridge.exposeInMainWorld("voraElectron", api);
