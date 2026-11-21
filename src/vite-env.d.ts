/// <reference types="vite/client" />
interface ElectronRuntimeResult {
  ok: true;
  result: {
    electron: string;
    node: string;
    platform: string;
    arch: string;
    packaged: boolean;
    appVersion: string;
  };
  error?: undefined;
}

interface ElectronRuntimeError {
  ok: false;
  result?: undefined;
  error: {
    code: string;
    message: string;
  };
}

declare global {
  interface Window {
    voraElectron?: {
      getRuntimeInfo(): Promise<ElectronRuntimeResult | ElectronRuntimeError>;
      getShellStatus(): Promise<unknown>;
    };
  }
}

export {};
