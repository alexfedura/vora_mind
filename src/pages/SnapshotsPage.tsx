import React, { useCallback, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { ArrowLeft, Trash2, Camera, Play } from "lucide-react";
import { listSnapshots, deleteSnapshot } from "@/lib/snapshot/storage";
import type { VoraSnapshot } from "@/lib/snapshot/types";

export function relativeTime(ts: number): string {
  const diff = Date.now() - ts;
  const s = Math.floor(diff / 1000);
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} minute${m === 1 ? "" : "s"} ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} hour${h === 1 ? "" : "s"} ago`;
  const d = Math.floor(h / 24);
  if (d < 30) return `${d} day${d === 1 ? "" : "s"} ago`;
  const mo = Math.floor(d / 30);
  if (mo < 12) return `${mo} month${mo === 1 ? "" : "s"} ago`;
  const y = Math.floor(mo / 12);
  return `${y} year${y === 1 ? "" : "s"} ago`;
}

export const SnapshotsPage: React.FC = () => {
  const navigate = useNavigate();
  const [snaps, setSnaps] = useState<VoraSnapshot[]>([]);
  const [loading, setLoading] = useState(true);
  const [toast, setToast] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const all = await listSnapshots();
      setSnaps(all);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const flash = (m: string): void => {
    setToast(m);
    window.setTimeout(() => setToast(null), 2200);
  };

  const onOpen = (id: string, playNarrative = false): void => {
    const query = "snapshot=" + encodeURIComponent(id) + (playNarrative ? "&playNarrative=1" : "");
    navigate("/chart?" + query);
  };

  const onDelete = async (id: string): Promise<void> => {
    await deleteSnapshot(id);
    await refresh();
    flash("Snapshot deleted");
  };

  return (
    <div className="h-screen w-screen flex flex-col overflow-hidden bg-[#0f0f0f] text-[#d6dbe2]">
      <div className="h-11 shrink-0 flex items-center gap-3 px-3 bg-[#0f0f0f] border-b border-[#2a2a2a]">
        <Link
          to="/chart"
          className="flex items-center gap-1.5 px-2 py-1 rounded hover:bg-[#2a2a2a] text-xs text-[#d6dbe2] shrink-0"
        >
          <ArrowLeft size={14} />
          <span>Chart</span>
        </Link>
        <span className="text-sm font-semibold whitespace-nowrap flex items-center gap-2">
          <Camera size={15} className="text-[#e8ad55]" />
          Snapshots
        </span>
        <span className="px-2 py-0.5 rounded bg-[#181818] text-xs font-mono text-[#e8ad55]">
          {snaps.length}
        </span>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto p-4">
        {loading ? (
          <div className="text-xs text-[#7f8997]">Loading snapshots...</div>
        ) : snaps.length === 0 ? (
          <div className="text-xs text-[#7f8997]">
            No snapshots yet. Open Chart and press the camera button to take one.
          </div>
        ) : (
          <div className="flex flex-col gap-2 max-w-3xl">
            {snaps.map((s) => (
              <div
                key={s.id}
                role="button"
                tabIndex={0}
                onClick={() => onOpen(s.id, Boolean(s.narrativeMode && s.narrativeLog))}
                onKeyDown={(e) => {
                  if (e.key === "Enter") onOpen(s.id, Boolean(s.narrativeMode && s.narrativeLog));
                }}
                className="flex items-center gap-3 px-3 py-2.5 rounded bg-[#181818] border border-[#2a2a2a] hover:border-[#e8ad55] cursor-pointer transition"
              >
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-[#d6dbe2] truncate">{s.name}</div>
                  <div className="text-[11px] text-[#7f8997] font-mono truncate">
                    {s.chart.symbol} · {s.chart.interval} · {s.drawings.length} drawing
                    {s.drawings.length === 1 ? "" : "s"} · {relativeTime(s.createdAt)}
                  </div>
                  {s.narrativeMode && s.narrativeLog && (
                    <div className="mt-1 flex items-center gap-1 text-[11px] text-[#e8ad55]">
                      <Play size={11} /> {s.narrativeMode === "FULL_VIDEO" ? "Full narrative" : "Compact steps"}
                    </div>
                  )}
                  {s.tags.length > 0 && (
                    <div className="text-[11px] text-[#e8ad55]/80 truncate">{s.tags.join(", ")}</div>
                  )}
                </div>
                <button
                  type="button"
                  aria-label={`Delete ${s.name}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    void onDelete(s.id);
                  }}
                  className="w-7 h-7 shrink-0 flex items-center justify-center rounded hover:bg-[#2a2a2a] text-[#7f8997] hover:text-[#e36b74]"
                >
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {toast && (
        <div
          style={{
            position: "fixed",
            bottom: 24,
            left: "50%",
            transform: "translateX(-50%)",
            background: "#181818",
            color: "#d6dbe2",
            padding: "6px 12px",
            border: "1px solid #2a2a2a",
            borderRadius: 4,
            fontSize: 12,
            zIndex: 60,
          }}
        >
          {toast}
        </div>
      )}
    </div>
  );
};
