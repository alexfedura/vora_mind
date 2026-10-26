import type { GradedRectConfig } from "@/lib/market/gradedRectStore";

interface Props {
  config: GradedRectConfig;
  onChange: (config: GradedRectConfig) => void;
}

export function GradedRectangleControls({ config, onChange }: Props): React.ReactElement {
  const update = (key: "height" | "width", text: string): void => {
    const value = Number(text);
    if (!Number.isFinite(value) || value <= 0) return;
    onChange({
      ...config,
      [key]: key === "width" ? Math.max(1, value) : value,
    });
  };

  return (
    <section
      aria-label="Graded Rectangle settings"
      className="vora-floating-ui absolute left-3 top-3 z-20 w-56 rounded border border-[#63358d] bg-[#17131d]/95 p-3 text-xs text-[#e9d5ff] shadow-lg"
    >
      <h2 className="mb-2 font-semibold text-purple-200">Graded Rectangle</h2>
      <label className="mb-2 flex items-center justify-between gap-2">
        <span>Fixed height (price)</span>
        <input
          aria-label="Graded rectangle height"
          type="number"
          min="0.00000001"
          step="any"
          value={config.height}
          onChange={(event) => update("height", event.currentTarget.value)}
          className="w-24 rounded border border-[#4c3b5c] bg-[#0f0f0f] px-2 py-1 text-right font-mono"
        />
      </label>
      <label className="mb-2 flex items-center justify-between gap-2">
        <span>Default width (bars)</span>
        <input
          aria-label="Graded rectangle width in bars"
          type="number"
          min="1"
          step="1"
          value={config.width}
          onChange={(event) => update("width", event.currentTarget.value)}
          className="w-24 rounded border border-[#4c3b5c] bg-[#0f0f0f] px-2 py-1 text-right font-mono"
        />
      </label>
      <label className="flex items-center justify-between gap-2">
        <span>Default baseline</span>
        <select
          aria-label="Graded rectangle default baseline"
          value={config.defaultBaseline}
          onChange={(event) => {
            const baseline = event.currentTarget.value;
            if (baseline === "top" || baseline === "bottom") {
              onChange({ ...config, defaultBaseline: baseline });
            }
          }}
          className="rounded border border-[#4c3b5c] bg-[#0f0f0f] px-2 py-1"
        >
          <option value="top">Top</option>
          <option value="bottom">Bottom</option>
        </select>
      </label>
      <p className="mt-2 text-[10px] text-[#b5a7c4]">Pivot snap radius: 50 px. Box height stays fixed after placement.</p>
    </section>
  );
}
