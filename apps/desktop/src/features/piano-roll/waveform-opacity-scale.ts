export function scaledWaveformOpacity(
  opacity: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.max(0.01, opacity / 2)
    : Math.min(0.6, opacity * 2)
}

export function nextWaveformOpacityScale(
  opacity: number,
  factor: "half" | "double"
): number | null {
  const next = scaledWaveformOpacity(opacity, factor)
  return Math.abs(next - opacity) < 0.001 ? null : next
}
