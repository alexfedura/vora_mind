import { contextBridge, ipcRenderer } from "electron";

const api = Object.freeze({
  getShellStatus: (): Promise<unknown> => ipcRenderer.invoke("vora:shell:status")
});

contextBridge.exposeInMainWorld("voraElectron", api);
