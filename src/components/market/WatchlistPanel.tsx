import { useMemo, useState, type ReactElement } from "react";
import { Search } from "lucide-react";
import { LOCAL_MARKET_INSTRUMENTS } from "@/lib/market/marketApi";
import { useMarketSymbol } from "@/lib/market/useMarketSymbol";

export function WatchlistPanel(): ReactElement {
  const { symbol, setSymbol } = useMarketSymbol();
  const items = LOCAL_MARKET_INSTRUMENTS;
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter((item) => {
      const hay = `${item.symbol} ${item.kind ?? ""}`.toLowerCase();
      return hay.includes(q);
    });
  }, [items, query]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-1.5 border-b border-[#383838] px-2 py-1.5">
        <Search size={12} className="text-[#787b86]" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter"
          className="w-full bg-transparent text-[12px] text-[#d6dbe2] placeholder:text-[#787b86] outline-none"
        />
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {filtered.length === 0 && (
          <div className="px-3 py-2 text-[11px] text-[#787b86]">No symbols</div>
        )}
        {filtered.map((item) => {
          const active = item.symbol === symbol;
          return (
            <button
              key={item.id || item.symbol}
              type="button"
              onClick={() => {
                if (item.symbol !== symbol) setSymbol(item.symbol);
              }}
              className={`flex w-full items-center justify-between gap-2 px-2.5 py-1.5 text-left text-[12px] transition-colors ${
                active ? "bg-[#2a2a2a] text-[#ff9800]" : "text-[#d6dbe2] hover:bg-[#2a2a2a]"
              }`}
            >
              <span className="font-mono font-medium truncate">{item.symbol}</span>
              <span className="text-[10px] uppercase tracking-wider text-[#787b86] shrink-0">
                {item.kind}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
