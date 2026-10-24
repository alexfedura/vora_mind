export interface PitchforkAnchor {
  ts: number;
  price: number;
}

export function pitchforkMedianOrigin(
  first: PitchforkAnchor,
  second: PitchforkAnchor,
  variant: "standard" | "schiff" | "modified-schiff" = "standard",
): PitchforkAnchor {
  if (variant === "modified-schiff") {
    return { ts: first.ts + (second.ts - first.ts) / 2, price: first.price + (second.price - first.price) / 2 };
  }
  if (variant === "schiff") {
    return { ts: first.ts, price: first.price + (second.price - first.price) / 2 };
  }
  return first;
}