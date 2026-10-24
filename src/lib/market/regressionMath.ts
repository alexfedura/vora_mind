export interface RegressionSample {
  ts: number;
  close: number;
}

export interface RegressionBand {
  startPrice: number;
  endPrice: number;
  deviation: number;
}

export function calculateRegressionBand(
  samples: readonly RegressionSample[],
  startTs: number,
  endTs: number,
): RegressionBand | null {
  const lower = Math.min(startTs, endTs);
  const upper = Math.max(startTs, endTs);
  const points = samples.filter((sample) => sample.ts >= lower && sample.ts <= upper);
  if (points.length < 2) return null;

  const meanX = points.reduce((sum, point) => sum + point.ts, 0) / points.length;
  const meanY = points.reduce((sum, point) => sum + point.close, 0) / points.length;
  let covariance = 0;
  let variance = 0;
  for (const point of points) {
    const deltaX = point.ts - meanX;
    covariance += deltaX * (point.close - meanY);
    variance += deltaX * deltaX;
  }
  if (variance === 0) return null;

  const slope = covariance / variance;
  const residualSquares = points.reduce((sum, point) => {
    const residual = point.close - (meanY + slope * (point.ts - meanX));
    return sum + residual * residual;
  }, 0);
  const valueAt = (ts: number): number => meanY + slope * (ts - meanX);
  return {
    startPrice: valueAt(startTs),
    endPrice: valueAt(endTs),
    deviation: Math.sqrt(residualSquares / points.length),
  };
}