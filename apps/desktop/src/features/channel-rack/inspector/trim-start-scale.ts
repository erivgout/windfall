export function scaledTrimStart(
  start: number,
  end: number,
  factor: "half" | "double"
): number {
  const span = end - start
  return factor === "half" ? start + span / 2 : Math.max(0, start - span)
}

export function nextTrimStartScale(
  start: number,
  end: number,
  factor: "half" | "double"
): number | null {
  const next = scaledTrimStart(start, end, factor)
  return Math.abs(next - start) < 0.001 ? null : next
}
