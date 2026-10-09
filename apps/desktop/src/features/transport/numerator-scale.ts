export function scaledNumerator(
  numerator: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.max(1, Math.floor(numerator / 2))
    : Math.min(16, numerator * 2)
}

export function nextNumeratorScale(
  numerator: number,
  factor: "half" | "double"
): number | null {
  const next = scaledNumerator(numerator, factor)
  return next === numerator ? null : next
}
