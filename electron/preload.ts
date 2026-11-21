import { contextBridge, ipcRenderer } from "electron";

const api = Object.freeze({
  getShellStatus: (): Promise<unknown> => ipcRenderer.invoke("vora:shell:status"),
  getRuntimeInfo: (): Promise<unknown> => ipcRenderer.invoke("vora:shell:runtime-info")
});

contextBridge.exposeInMainWorld("voraElectron", api);
