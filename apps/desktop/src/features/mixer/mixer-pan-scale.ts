export function scaledMixerPan(
  pan: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? pan / 2 : Math.min(1, Math.max(-1, pan * 2))
}

export function nextMixerPanScale(
  pan: number,
  factor: "half" | "double"
): number | null {
  const next = scaledMixerPan(pan, factor)
  return Math.abs(next - pan) < 0.001 ? null : next
}
