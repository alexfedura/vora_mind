export interface WorkerNodeRecord {
  worker_id: string;
  last_seen_epoch: number;
  current_task_id: string | null;
  status: string;
  version: string;
  port: number;
  is_alive: boolean;
}

export interface WorkerHealthResponse {
  status: string;
  version: string;
  current_node: string;
  database: string;
  auth_enabled: boolean;
  cluster: {
    active_nodes: number;
    total_nodes: number;
  };
  queue: {
    pending: number;
    completed: number;
    failed: number;
  };
  nodes: WorkerNodeRecord[];
}

export interface WorkerTaskSummary {
  task_id: string;
  status: string;
  intent: string;
  claimed_by?: string | null;
  error: string | null;
}

export interface WorkerTaskSubmission {
  intent: string;
  project_root?: string;
  target_file?: string;
  target_symbol?: string;
  patch_code?: string;
}

const DEFAULT_WORKER_URL = "http://127.0.0.1:4040";

export class VoraWorkerClient {
  private baseUrl: string;
  private authToken: string | null = null;

  constructor(baseUrl: string = DEFAULT_WORKER_URL) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
  }

  setBaseUrl(url: string) {
    this.baseUrl = (url.trim() || DEFAULT_WORKER_URL).replace(/\/+$/, "");
  }

  getBaseUrl(): string {
    return this.baseUrl;
  }

  setAuthToken(token: string | null) {
    this.authToken = token ? token.trim() : null;
  }

  getAuthToken(): string | null {
    return this.authToken;
  }

  private getHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.authToken) {
      headers["Authorization"] = `Bearer ${this.authToken}`;
    }
    return headers;
  }

  async getHealth(): Promise<WorkerHealthResponse> {
    const res = await fetch(`${this.baseUrl}/api/health`, {
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Worker health check failed: HTTP ${res.status}`);
    }
    return res.json();
  }

  async getNodes(): Promise<WorkerNodeRecord[]> {
    const res = await fetch(`${this.baseUrl}/api/nodes`, {
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Failed to fetch cluster nodes: HTTP ${res.status}`);
    }
    const data = await res.json();
    return data.nodes || [];
  }

  async getTasks(): Promise<WorkerTaskSummary[]> {
    const res = await fetch(`${this.baseUrl}/api/tasks`, {
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Failed to fetch tasks: HTTP ${res.status}`);
    }
    const data = await res.json();
    return data.tasks || [];
  }

  async submitTask(
    intentOrPayload: string | WorkerTaskSubmission,
    projectRoot: string = "."
  ): Promise<string> {
    const payload =
      typeof intentOrPayload === "string"
        ? { intent: intentOrPayload, project_root: projectRoot }
        : {
            intent: intentOrPayload.intent,
            project_root: intentOrPayload.project_root || projectRoot,
            target_file: intentOrPayload.target_file,
            target_symbol: intentOrPayload.target_symbol,
            patch_code: intentOrPayload.patch_code,
          };

    const res = await fetch(`${this.baseUrl}/api/tasks`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(payload),
    });

    if (!res.ok) {
      throw new Error(`Failed to submit task: HTTP ${res.status}`);
    }

    const data = await res.json();
    return data.task_id;
  }

  streamLogs(onMessage: (logLine: string) => void, taskId?: string): () => void {
    const streamUrl = taskId
      ? `${this.baseUrl}/api/tasks/${taskId}/stream`
      : `${this.baseUrl}/api/logs/stream`;

    const eventSource = new EventSource(streamUrl);
    eventSource.onmessage = (event) => {
      onMessage(event.data);
    };

    return () => {
      eventSource.close();
    };
  }
}

export const workerClient = new VoraWorkerClient();