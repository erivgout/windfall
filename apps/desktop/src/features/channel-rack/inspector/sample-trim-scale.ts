export function scaledTrimEnd(
  start: number,
  end: number,
  factor: "half" | "double"
): number {
  const span = end - start
  return factor === "half"
    ? start + span / 2
    : Math.min(1, start + span * 2)
}

export function nextSampleTrimScale(
  start: number,
  end: number,
  factor: "half" | "double"
): number | null {
  const next = scaledTrimEnd(start, end, factor)
  return Math.abs(next - end) < 0.001 ? null : next
}
