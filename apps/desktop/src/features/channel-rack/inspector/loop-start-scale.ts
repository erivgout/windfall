export function scaledLoopStart(
  start: number,
  end: number,
  factor: "half" | "double"
): number {
  const span = end - start
  return factor === "half" ? start + span / 2 : Math.max(0, start - span)
}

export function nextLoopStartScale(
  start: number,
  end: number,
  factor: "half" | "double"
): number | null {
  const next = scaledLoopStart(start, end, factor)
  return Math.abs(next - start) < 0.001 ? null : next
}
