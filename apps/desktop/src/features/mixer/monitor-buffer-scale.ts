export function scaledMonitorBuffer(
  ms: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? Math.max(5, Math.floor(ms / 2)) : Math.min(100, ms * 2)
}

export function nextMonitorBufferScale(
  ms: number,
  factor: "half" | "double"
): number | null {
  const next = scaledMonitorBuffer(ms, factor)
  return next === ms ? null : next
}
