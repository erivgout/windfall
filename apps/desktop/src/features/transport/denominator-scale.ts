export function scaledDenominator(
  denominator: number,
  factor: "half" | "double"
): number | null {
  if (![2, 4, 8, 16].includes(denominator)) return null

  return factor === "half"
    ? Math.max(2, denominator / 2)
    : Math.min(16, denominator * 2)
}

export function nextDenominatorScale(
  denominator: number,
  factor: "half" | "double"
): number | null {
  const next = scaledDenominator(denominator, factor)
  return next === denominator ? null : next
}
