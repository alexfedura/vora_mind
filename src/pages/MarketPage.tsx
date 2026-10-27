import React, { useCallback, useEffect, useMemo, useState } from "react";
import { mktFetchLocalCandles } from "@/lib/market/marketApi";
import { LOCAL_MARKET_INSTRUMENTS, type CandleDto, type MarketInstrument } from "@/lib/market/marketApi";
import { intervalWindowSeconds } from "@/lib/market/chartIntervals";
import { MarketWorkspace } from "./MarketWorkspace";

export const MarketPage: React.FC = () => {
  const [symbol, setSymbol] = useState("XAUUSD");
  const [timeframe, setTimeframe] = useState("1h");
  const [candles, setCandles] = useState<CandleDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hoveredCandle, setHoveredCandle] = useState<CandleDto | null>(null);
  const activeSource = useMemo<MarketInstrument>(() => ({
    id: `local-${symbol}`,
    symbol,
    kind: "Local Parquet",
    row_count: candles.length,
  }), [candles, symbol]);
  const sources = LOCAL_MARKET_INSTRUMENTS;

  const loadMarketData = useCallback(async () => {
    if (!symbol) {
      setCandles([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const endTs = Math.floor(Date.now() / 1000);
      const intervalSeconds = intervalWindowSeconds(timeframe);
      const candleList = await mktFetchLocalCandles({
        symbol,
        intervalMinutes: intervalSeconds / 60,
        startTs: endTs - 5000 * intervalSeconds,
        endTs,
        limit: 5000,
      });
      setCandles(candleList);
    } catch (requestError) {
      setError(String(requestError));
    } finally {
      setLoading(false);
    }
  }, [symbol, timeframe]);

  useEffect(() => {
    void loadMarketData();
  }, [loadMarketData]);

  return (
    <MarketWorkspace
      sources={sources}
      activeSource={activeSource}
      symbol={symbol}
      timeframe={timeframe}
      candles={candles}
      loading={loading}
      error={error}
      hoveredCandle={hoveredCandle}
      onSelectSource={(source) => setSymbol(source.symbol)}
      onTimeframeChange={setTimeframe}
      onRefresh={loadMarketData}
      onCrosshairMove={setHoveredCandle}
    />
  );
};
