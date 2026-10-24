import { useState } from "react";
import { Bell, X } from "lucide-react";
import { FIB_GANN_TOOL_LABELS, GEOMETRY_TOOL_LABELS, type ChartDrawingAlert } from "@/lib/market/drawingTypes";
import type { DrawingAny } from "@/lib/market/lineDrawings";

interface CreateDrawingAlertDialogProps {
  symbol: string;
  timeframe: number;
  drawing: DrawingAny;
  onClose: () => void;
  onCreate: (alert: ChartDrawingAlert) => Promise<void>;
}

function drawingTitle(drawing: DrawingAny): string {
  if (drawing.name) return drawing.name;
  if (drawing.type === "fib-gann") return FIB_GANN_TOOL_LABELS[drawing.tool];
  if (drawing.type === "icon") return drawing.symbol;
  if (drawing.type === "geometry") return GEOMETRY_TOOL_LABELS[drawing.tool];
  return ({ rectangle: "Rectangle", vline: "Vertical line", hline: "Horizontal line", hray: "Horizontal ray", trendline: "Trend line", ray: "Ray", "info-line": "Info line", "extended-line": "Extended line", "trend-angle": "Trend angle", crossline: "Crossline", "parallel-channel": "Parallel channel", pitchfork: "Pitchfork", "regression-trend": "Regression trend" }[drawing.type]);
}

export function CreateDrawingAlertDialog({ symbol, timeframe, drawing, onClose, onCreate }: CreateDrawingAlertDialogProps): React.ReactElement {
  const initialCondition = drawing.type === "rectangle" ? "entering_drawing" : "crossing";
  const [condition, setCondition] = useState<ChartDrawingAlert["condition"]>(initialCondition);
  const [triggerOnce, setTriggerOnce] = useState(true);
  const [expiration, setExpiration] = useState("never");
  const [message, setMessage] = useState(`${symbol}, M${timeframe} ${drawingTitle(drawing)}`);
  const [notifyApp, setNotifyApp] = useState(true);
  const [notifyToast, setNotifyToast] = useState(true);
  const [notifySound, setNotifySound] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!message.trim() || saving) return;
    setSaving(true);
    setError(null);
    try {
      let appNotificationEnabled = notifyApp && "Notification" in window;
      if (appNotificationEnabled && Notification.permission === "default") {
        appNotificationEnabled = await Notification.requestPermission() === "granted";
      }
      const now = Math.floor(Date.now() / 1000);
      const expirationSeconds: Record<string, number> = { hour: 3600, day: 86400, week: 604800 };
      const duration = expirationSeconds[expiration];
      const alert: ChartDrawingAlert = {
        id: crypto.randomUUID(),
        symbol,
        timeframe,
        drawingId: drawing.id,
        condition,
        message: message.trim(),
        triggerOnce,
        notifyApp: appNotificationEnabled,
        notifyToast,
        notifyEmail: false,
        notifySound,
        expirationAt: duration ? now + duration : null,
        enabled: true,
        createdAt: now,
        lastEvaluatedAt: null,
      };
      await onCreate(alert);
      onClose();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not create alert");
      setSaving(false);
    }
  };

  return (
    <div className="drawing-alert-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <form className="drawing-alert-dialog" role="dialog" aria-modal="true" aria-labelledby="drawing-alert-title" onKeyDown={(event) => { if (event.key === "Escape") { event.preventDefault(); onClose(); } }} onSubmit={(event) => void submit(event)}>
        <header className="drawing-alert-dialog__header"><div><Bell size={17} /><h2 id="drawing-alert-title">Create alert on</h2><strong>{symbol}, M{timeframe}</strong></div><button type="button" aria-label="Close alert dialog" onClick={onClose}><X size={18} /></button></header>
        <div className="drawing-alert-dialog__body">
          <label className="drawing-alert-dialog__field"><span>Condition</span><select value={condition} onChange={(event) => setCondition(event.currentTarget.value as ChartDrawingAlert["condition"])}><option value="entering_drawing">Entering {drawingTitle(drawing).toLowerCase()}</option><option value="crossing">Crossing {drawingTitle(drawing).toLowerCase()}</option></select></label>
          <div className="drawing-alert-dialog__drawing"><span className="drawing-alert-dialog__drawing-icon">↗</span><span>{drawingTitle(drawing)}</span></div>
          <label className="drawing-alert-dialog__field"><span>Message</span><input value={message} maxLength={200} onChange={(event) => setMessage(event.currentTarget.value)} /></label>
          <label className="drawing-alert-dialog__field"><span>Trigger</span><select value={triggerOnce ? "once" : "every"} onChange={(event) => setTriggerOnce(event.currentTarget.value === "once")}><option value="once">Once only</option><option value="every">Every time</option></select></label>
          <label className="drawing-alert-dialog__field"><span>Expiration</span><select value={expiration} onChange={(event) => setExpiration(event.currentTarget.value)}><option value="never">No expiration</option><option value="hour">In 1 hour</option><option value="day">In 1 day</option><option value="week">In 1 week</option></select></label>
          <div className="drawing-alert-dialog__notifications"><span>Notifications</span><div>
            <label><input type="checkbox" checked={notifyApp} onChange={(event) => setNotifyApp(event.currentTarget.checked)} />App</label>
            <label><input type="checkbox" checked={notifyToast} onChange={(event) => setNotifyToast(event.currentTarget.checked)} />Toasts</label>
            <label title="Email delivery is not configured"><input type="checkbox" disabled />Email</label>
            <label><input type="checkbox" checked={notifySound} onChange={(event) => setNotifySound(event.currentTarget.checked)} />Sound</label>
          </div></div>
          {error && <p className="drawing-alert-dialog__error" role="alert">{error}</p>}
        </div>
        <footer className="drawing-alert-dialog__footer"><button type="button" onClick={onClose} disabled={saving}>Cancel</button><button type="submit" className="is-primary" disabled={!message.trim() || saving}>{saving ? "Creating..." : "Create"}</button></footer>
      </form>
    </div>
  );
}