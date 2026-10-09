export function scaledRecordingOffset(
  ms: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? ms / 2 : Math.min(1000, Math.max(-1000, ms * 2))
}

export function nextRecordingOffsetScale(
  ms: number,
  factor: "half" | "double"
): number | null {
  const next = scaledRecordingOffset(ms, factor)
  return Math.abs(next - ms) < 0.001 ? null : next
}
