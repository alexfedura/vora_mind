import React, { useEffect, useState, useRef } from "react";
import {
  X,
  SlidersHorizontal,
  RefreshCw,
  AlertCircle,
  Cpu,
  Activity,
  Send,
  Key,
  Terminal,
  Radio,
  Network,
  Server,
  Globe,
} from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";
import { DataBackup } from "@/components/DataBackup";
import { DatabaseSettings } from "@/components/DatabaseSettings";
import { FontSettingsPanel } from "@/shared/components/FontSettingsPanel";
import {
  workerClient,
  WorkerHealthResponse,
  WorkerTaskSummary,
  WorkerNodeRecord,
} from "@/shared/services/workerClient";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose }) => {
  const { textDirection, toggleTextDirection } = useAppStore();

  // پیکربندی حالت ورکر (Local vs Remote)
  const [workerMode, setWorkerMode] = useState<"local" | "remote">(() => {
    return getSetting("vora_worker_mode") === "remote" ? "remote" : "local";
  });
  const [workerUrl, setWorkerUrl] = useState(() => {
    return getSetting("vora_worker_url") || "http://127.0.0.1:4040";
  });

  // وضعیت ورکر خودکار و کلاستر
  const [workerHealth, setWorkerHealth] = useState<WorkerHealthResponse | null>(null);
  const [workerTasks, setWorkerTasks] = useState<WorkerTaskSummary[]>([]);
  const [clusterNodes, setClusterNodes] = useState<WorkerNodeRecord[]>([]);
  const [workerLoading, setWorkerLoading] = useState(false);
  const [workerError, setWorkerError] = useState<string | null>(null);

  // فرم ثبت تسک مستقیم و تنظیمات توکن
  const [taskIntent, setTaskIntent] = useState("");
  const [submittingTask, setSubmittingTask] = useState(false);
  const [taskSubmitMsg, setTaskSubmitMsg] = useState<string | null>(null);
  const [authToken, setAuthToken] = useState(
    () => getSetting("vora_worker_token") || ""
  );

  // تلمتری زنده و استریم لاگ‌ها (SSE)
  const [isStreaming, setIsStreaming] = useState(false);
  const [liveLogs, setLiveLogs] = useState<string[]>([]);
  const logContainerRef = useRef<HTMLDivElement>(null);
  const streamCloserRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    if (isOpen) {
      window.addEventListener("keydown", handleKeyDown);
      fetchWorkerStatus();
    } else {
      stopLogStream();
    }
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      stopLogStream();
    };
  }, [isOpen, onClose]);

  useEffect(() => {
    if (logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [liveLogs]);

  const stopLogStream = () => {
    if (streamCloserRef.current) {
      streamCloserRef.current();
      streamCloserRef.current = null;
    }
    setIsStreaming(false);
  };

  const toggleLogStream = () => {
    if (isStreaming) {
      stopLogStream();
    } else {
      setIsStreaming(true);
      setLiveLogs((prev) => [...prev, "[TELEMETRY] Connected to SSE live stream..."]);
      const closer = workerClient.streamLogs((logLine) => {
        setLiveLogs((prev) => [...prev.slice(-100), logLine]);
      });
      streamCloserRef.current = closer;
    }
  };

  const handleModeChange = (mode: "local" | "remote") => {
    setWorkerMode(mode);
    setSetting("vora_worker_mode", mode);
    const newUrl = mode === "local" ? "http://127.0.0.1:4040" : workerUrl;
    if (mode === "local") {
      setWorkerUrl(newUrl);
      setSetting("vora_worker_url", newUrl);
    }
    workerClient.setBaseUrl(newUrl);
    fetchWorkerStatus();
  };

  const handleUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const url = e.target.value;
    setWorkerUrl(url);
    setSetting("vora_worker_url", url);
    workerClient.setBaseUrl(url);
  };

  const handleTokenChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const token = e.target.value;
    setAuthToken(token);
    setSetting("vora_worker_token", token);
    workerClient.setAuthToken(token.trim() || null);
  };

  const fetchWorkerStatus = async () => {
    try {
      setWorkerLoading(true);
      setWorkerError(null);
      workerClient.setBaseUrl(workerUrl);
      workerClient.setAuthToken(authToken.trim() || null);

      const health = await workerClient.getHealth();
      setWorkerHealth(health);
      setClusterNodes(health.nodes || []);
      const tasks = await workerClient.getTasks();
      setWorkerTasks(tasks);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setWorkerError(msg);
      setWorkerHealth(null);
      setClusterNodes([]);
    } finally {
      setWorkerLoading(false);
    }
  };

  const handleSubmitTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!taskIntent.trim()) return;

    try {
      setSubmittingTask(true);
      setTaskSubmitMsg(null);
      const taskId = await workerClient.submitTask(taskIntent.trim(), "src-tauri");
      setTaskSubmitMsg(`Enqueued: ${taskId}`);
      setTaskIntent("");
      if (!isStreaming) {
        toggleLogStream();
      }
      await fetchWorkerStatus();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setTaskSubmitMsg(`Failed: ${msg}`);
    } finally {
      setSubmittingTask(false);
    }
  };


  if (!isOpen) return null;

  const isRtl = textDirection === "rtl";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center select-none animate-in fade-in duration-150">
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />

      <div className="relative w-[520px] max-h-[85vh] flex flex-col bg-[#15171e] border border-[#262b37] rounded-2xl shadow-2xl overflow-hidden z-10">
        {/* هدر مدال */}
        <div className="h-13 px-5 py-3.5 flex items-center justify-between border-b border-[#212531] bg-[#111318]">
          <div className="flex items-center gap-2.5 text-gray-200">
            <SlidersHorizontal size={16} className="text-purple-400" />
            <span className="text-sm font-bold tracking-wide">Settings & Engine Status</span>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-gray-400 hover:text-white hover:bg-[#1f232f] transition"
          >
            <X size={16} />
          </button>
        </div>

        {/* محتوای تنظیمات */}
        <div className="p-5 flex flex-col gap-4 overflow-y-auto">
          <FontSettingsPanel />

          <section className="flex flex-col gap-3" aria-labelledby="data-portability-heading">
            <h3 id="data-portability-heading" className="text-[10px] font-bold uppercase tracking-wider text-gray-400">
              Data Portability & Backups
            </h3>
            <DatabaseSettings />
            <DataBackup />
          </section>

          {/* بخش وضعیت ورکر خودکار ۲۴/۷ و کلاستر */}
          <div className="flex flex-col gap-3 p-4 rounded-xl bg-[#1a1d25] border border-[#242835]">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Cpu size={15} className="text-cyan-400" />
                <span className="text-xs font-semibold text-gray-100">
                  Autonomous Worker (24/7 Daemon)
                </span>
              </div>
              <div className="flex items-center gap-2">
                {/* تاگل سوییچ بین حالت Local و Remote */}
                <div className="flex bg-[#12141a] p-0.5 rounded-lg border border-[#242835]">
                  <button
                    type="button"
                    onClick={() => handleModeChange("local")}
                    className={`px-2 py-0.5 text-[10px] font-semibold rounded transition ${
                      workerMode === "local"
                        ? "bg-cyan-600 text-white shadow"
                        : "text-gray-400 hover:text-gray-200"
                    }`}
                  >
                    Local
                  </button>
                  <button
                    type="button"
                    onClick={() => handleModeChange("remote")}
                    className={`px-2 py-0.5 text-[10px] font-semibold rounded transition ${
                      workerMode === "remote"
                        ? "bg-cyan-600 text-white shadow"
                        : "text-gray-400 hover:text-gray-200"
                    }`}
                  >
                    Remote
                  </button>
                </div>

                <button
                  onClick={fetchWorkerStatus}
                  disabled={workerLoading}
                  className="p-1 rounded-md text-gray-400 hover:text-white hover:bg-[#252a37] transition"
                >
                  <RefreshCw size={13} className={workerLoading ? "animate-spin" : ""} />
                </button>
              </div>
            </div>

            {/* آدرس ورکر در حالت Remote */}
            {workerMode === "remote" && (
              <div className="flex items-center gap-2 bg-[#111318] px-3 py-1.5 rounded-lg border border-[#212634] animate-in fade-in">
                <Globe size={13} className="text-gray-400 shrink-0" />
                <input
                  type="text"
                  placeholder="http://remote-server-ip:4040"
                  value={workerUrl}
                  onChange={handleUrlChange}
                  className="w-full bg-transparent text-xs text-gray-200 outline-none placeholder:text-gray-500"
                />
              </div>
            )}

            {workerHealth ? (
              <div className="flex flex-col gap-2.5 text-[11px] bg-[#111318] p-3 rounded-lg border border-[#212634]">
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Node ID / Status:</span>
                  <span className="flex items-center gap-1 text-green-400 font-mono">
                    <Activity size={12} /> {workerHealth.current_node} (ONLINE)
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Target Environment:</span>
                  <span className="text-gray-200 font-mono">
                    {workerMode === "remote" ? "Remote VPS/Cloud" : "Local Daemon"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Queue Metrics:</span>
                  <span className="text-gray-200 font-mono">
                    Completed: <span className="text-green-400">{workerHealth.queue.completed}</span> | Pending:{" "}
                    <span className="text-yellow-400">{workerHealth.queue.pending}</span> | Failed:{" "}
                    <span className="text-red-400">{workerHealth.queue.failed}</span>
                  </span>
                </div>
              </div>
            ) : workerError ? (
              <div className="flex items-center gap-2 text-[11px] text-amber-300/90 bg-[#1e1a14] p-2.5 rounded-lg border border-amber-500/20">
                <AlertCircle size={14} className="text-amber-400 shrink-0" />
                <span>
                  Worker Offline (
                  {workerMode === "local"
                    ? "Run `cargo run --bin vora_worker -- serve 4040`"
                    : `Cannot reach ${workerUrl}`}
                  )
                </span>
              </div>
            ) : null}

            {/* لیست نودهای کلاستر */}
            {clusterNodes.length > 0 && (
              <div className="flex flex-col gap-1.5">
                <div className="flex items-center gap-1.5 text-[10px] uppercase font-bold text-gray-400">
                  <Network size={12} className="text-cyan-400" />
                  <span>Cluster Nodes ({clusterNodes.filter((n) => n.is_alive).length} Active)</span>
                </div>
                <div className="max-h-28 overflow-y-auto flex flex-col gap-1">
                  {clusterNodes.map((node) => (
                    <div
                      key={node.worker_id}
                      className="flex items-center justify-between bg-[#111318] px-2.5 py-1.5 rounded border border-[#202430] text-[11px]"
                    >
                      <div className="flex items-center gap-2">
                        <Server
                          size={12}
                          className={node.is_alive ? "text-green-400" : "text-gray-600"}
                        />
                        <span className="font-mono text-gray-200 truncate max-w-[180px]">
                          {node.worker_id}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 font-mono text-[10px]">
                        <span className="text-gray-500">:{node.port}</span>
                        <span
                          className={`px-1.5 py-0.2 rounded ${
                            node.is_alive
                              ? "text-green-300 bg-green-950/40"
                              : "text-gray-500 bg-gray-900/40"
                          }`}
                        >
                          {node.is_alive ? node.status : "OFFLINE"}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* فیلد توکن احراز هویت ورکر */}
            <div className="flex items-center gap-2 bg-[#111318] px-3 py-1.5 rounded-lg border border-[#212634]">
              <Key size={13} className="text-gray-400 shrink-0" />
              <input
                type="password"
                placeholder={
                  workerMode === "remote"
                    ? "Worker Auth Token (Required for Remote)"
                    : "Worker Auth Token (Optional for Local)"
                }
                value={authToken}
                onChange={handleTokenChange}
                className="w-full bg-transparent text-xs text-gray-200 outline-none placeholder:text-gray-500"
              />
            </div>

            {/* فرم ارسال تسک مستقیم */}
            <form onSubmit={handleSubmitTask} className="flex gap-2 mt-1">
              <input
                type="text"
                placeholder="Submit autonomous task intent..."
                value={taskIntent}
                onChange={(e) => setTaskIntent(e.target.value)}
                disabled={submittingTask || !workerHealth}
                className="flex-1 bg-[#111318] px-3 py-1.5 rounded-lg border border-[#242938] text-xs text-gray-200 outline-none focus:border-cyan-500/50 disabled:opacity-50"
              />
              <button
                type="submit"
                disabled={submittingTask || !taskIntent.trim() || !workerHealth}
                className="flex items-center gap-1 px-3 py-1.5 bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 border border-cyan-500/30 rounded-lg text-xs font-medium transition disabled:opacity-50"
              >
                <Send size={12} className={submittingTask ? "animate-pulse" : ""} />
                Enqueue
              </button>
            </form>

            {taskSubmitMsg && (
              <span className="text-[11px] text-cyan-400 font-mono">{taskSubmitMsg}</span>
            )}

            {/* کنسول زنده استریم تلمتری (Live SSE Console) */}
            <div className="flex flex-col gap-1.5 mt-1">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 text-[10px] uppercase font-bold text-gray-400">
                  <Terminal size={12} className="text-cyan-400" />
                  <span>Live Telemetry Stream</span>
                </div>
                <div className="flex items-center gap-2">
                  {liveLogs.length > 0 && (
                    <button
                      onClick={() => setLiveLogs([])}
                      className="text-[10px] text-gray-500 hover:text-gray-300 transition"
                    >
                      Clear
                    </button>
                  )}
                  <button
                    onClick={toggleLogStream}
                    disabled={!workerHealth}
                    className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-mono border transition ${
                      isStreaming
                        ? "bg-green-950/50 text-green-300 border-green-500/40 animate-pulse"
                        : "bg-[#141720] text-gray-400 border-[#242a38] hover:text-gray-200"
                    }`}
                  >
                    <Radio
                      size={10}
                      className={isStreaming ? "text-green-400" : "text-gray-500"}
                    />
                    {isStreaming ? "LIVE" : "Connect"}
                  </button>
                </div>
              </div>

              {isStreaming || liveLogs.length > 0 ? (
                <div
                  ref={logContainerRef}
                  className="h-28 overflow-y-auto bg-[#0d0e12] p-2 rounded-lg border border-[#1e2330] font-mono text-[10px] text-gray-300 flex flex-col gap-0.5 select-text"
                >
                  {liveLogs.length === 0 ? (
                    <span className="text-gray-600 italic">Listening for daemon events...</span>
                  ) : (
                    liveLogs.map((log, idx) => (
                      <span
                        key={idx}
                        className={`leading-relaxed whitespace-pre-wrap ${
                          log.includes("✗") || log.includes("FAILED")
                            ? "text-red-400"
                            : log.includes("✓") || log.includes("COMPLETED")
                            ? "text-green-400"
                            : log.includes("[SYSTEM]") || log.includes("TELEMETRY")
                            ? "text-cyan-400/80"
                            : "text-gray-300"
                        }`}
                      >
                        {log}
                      </span>
                    ))
                  )}
                </div>
              ) : null}
            </div>

            {/* لیست آخرین تسک‌های پردازش‌شده */}
            {workerTasks.length > 0 && (
              <div className="flex flex-col gap-1.5 mt-1">
                <span className="text-[10px] uppercase font-bold text-gray-400">
                  Recent Completed Tasks:
                </span>
                <div className="max-h-24 overflow-y-auto flex flex-col gap-1">
                  {workerTasks.slice(0, 3).map((t) => (
                    <div
                      key={t.task_id}
                      className="flex items-center justify-between bg-[#13151b] px-2.5 py-1.5 rounded border border-[#202430] text-[11px]"
                    >
                      <span className="text-gray-300 truncate max-w-[280px]">{t.intent}</span>
                      <span
                        className={`text-[10px] font-mono px-1.5 py-0.5 rounded ${
                          t.status === "COMPLETED"
                            ? "text-green-300 bg-green-950/40"
                            : "text-red-300 bg-red-950/40"
                        }`}
                      >
                        {t.status}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* بخش جهت متن */}
          <div className="flex items-center justify-between p-4 rounded-xl bg-[#1a1d25] border border-[#242835]">
            <div className="flex flex-col gap-1 pr-4">
              <span className="text-xs font-semibold text-gray-100">
                Default RTL Layout (زبان غالب راست‌به‌چپ)
              </span>
              <span className="text-[11px] text-gray-400 leading-relaxed">
                متن‌ها به صورت خودکار جهت‌یابی می‌شوند. با فعال بودن این گزینه، جهت غالب متن‌های دوزبانه راست‌به‌چپ خواهد بود.
              </span>
            </div>

            <button
              onClick={toggleTextDirection}
              type="button"
              className={`relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                isRtl ? "bg-purple-600 shadow-sm shadow-purple-600/40" : "bg-[#2b303d]"
              }`}
            >
              <span
                className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-md transition duration-200 ease-in-out ${
                  isRtl ? "translate-x-4" : "translate-x-0"
                }`}
              />
            </button>
          </div>

        </div>
      </div>
    </div>
  );
};