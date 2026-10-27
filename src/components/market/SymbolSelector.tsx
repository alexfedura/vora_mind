import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { ChevronDown, Search } from "lucide-react";
import { LOCAL_MARKET_INSTRUMENTS } from "@/lib/market/marketApi";
import { useMarketSymbol } from "@/lib/market/useMarketSymbol";

export function SymbolSelector(): ReactElement {
  const { symbol, setSymbol } = useMarketSymbol();
  const items = LOCAL_MARKET_INSTRUMENTS;
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (!containerRef.current) return;
      if (!containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }

    window.addEventListener("mousedown", handlePointerDown);
    return () => window.removeEventListener("mousedown", handlePointerDown);
  }, []);

  const filteredItems = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return items;

    return items.filter((item) => {
      const haystack = `${item.symbol} ${item.kind}`.toLowerCase();
      return haystack.includes(needle);
    });
  }, [items, query]);

  const selectedLabel = symbol || "XAUUSD";

  return (
    <div ref={containerRef} className="relative shrink-0 z-30">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        className="flex items-center gap-2 rounded border border-[#2a2a2a] bg-[#1a1a1a] px-2.5 h-7 text-[12px] text-[#d1d4dc] hover:border-[#4a4a4a] hover:bg-[#242424] shrink-0"
        aria-label="Select market symbol"
        title="Select market symbol"
      >
        <span className="font-mono font-semibold text-[#ffb74d] tracking-tight">{selectedLabel}</span>
        <ChevronDown size={13} className={`text-[#787b86] transition-transform ${open ? "rotate-180" : ""}`} />
      </button>

      {open && (
        <div className="absolute left-0 top-[calc(100%+6px)] w-[280px] overflow-hidden rounded border border-[#383838] bg-[#1e1e1e] shadow-[0_8px_24px_rgba(0,0,0,0.45)] z-40">
          <div className="flex items-center gap-2 border-b border-[#383838] bg-[#1e1e1e] px-2 py-2">
            <Search size={12} className="text-[#787b86]" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search symbol"
              className="w-full bg-transparent text-[12px] text-[#d1d4dc] placeholder:text-[#787b86] outline-none"
              autoFocus
            />
          </div>

          <div className="max-h-[260px] overflow-y-auto">
            {filteredItems.length === 0 ? (
              <div className="px-3 py-2 text-[12px] text-[#787b86]">No symbols found</div>
            ) : (
              filteredItems.map((item) => {
                const active = item.symbol === symbol;
                return (
                  <button
                    key={item.id || item.symbol}
                    type="button"
                    onClick={() => {
                      setSymbol(item.symbol);
                      setOpen(false);
                      setQuery("");
                    }}
                    className={`flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-[12px] transition-colors ${
                      active ? "bg-[#2a2a2a] text-[#ff9800]" : "text-[#d6dbe2] hover:bg-[#2a2a2a]"
                    }`}
                  >
                    <span className="font-mono font-medium">{item.symbol}</span>
                    <span className="text-[10px] uppercase tracking-[0.08em] text-[#787b86]">{item.kind}</span>
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}
