export function scaledLoopCrossfade(
  crossfade: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? crossfade / 2 : Math.min(crossfade * 2, 1)
}

export function nextLoopCrossfadeScale(
  crossfade: number,
  factor: "half" | "double"
): number | null {
  const next = scaledLoopCrossfade(crossfade, factor)
  return Math.abs(next - crossfade) < 0.001 ? null : next
}
