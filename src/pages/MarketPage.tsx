import React, { useCallback, useEffect, useMemo, useState } from "react";
import { mktFetchHistoricalDuka } from "@/lib/market/marketApi";
import { ONLINE_MARKET_INSTRUMENTS, type CandleDto, type MarketInstrument } from "@/lib/market/marketApi";
import { MarketWorkspace } from "./MarketWorkspace";

export const MarketPage: React.FC = () => {
  const [symbol, setSymbol] = useState("XAUUSD");
  const [timeframe, setTimeframe] = useState("1h");
  const [candles, setCandles] = useState<CandleDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hoveredCandle, setHoveredCandle] = useState<CandleDto | null>(null);
  const activeSource = useMemo<MarketInstrument>(() => ({
    id: `dukascopy-${symbol}`,
    symbol,
    kind: "Dukascopy",
    row_count: candles.length,
  }), [candles, symbol]);
  const sources = ONLINE_MARKET_INSTRUMENTS;

  const loadMarketData = useCallback(async () => {
    if (!symbol) {
      setCandles([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const candleList = await mktFetchHistoricalDuka({
        symbol,
        timeframe,
        endTimestamp: Math.floor(Date.now() / 1000),
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
