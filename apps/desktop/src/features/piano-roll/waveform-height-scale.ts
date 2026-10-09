export function scaledWaveformHeight(
  height: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.max(2, Math.floor(height / 2))
    : Math.min(128, height * 2)
}

export function nextWaveformHeightScale(
  height: number,
  factor: "half" | "double"
): number | null {
  const next = scaledWaveformHeight(height, factor)
  return next === height ? null : next
}
