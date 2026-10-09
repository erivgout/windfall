export function scaledMix(mix: number, factor: "halve" | "double"): number {
  return factor === "halve" ? mix / 2 : Math.min(mix * 2, 1)
}

export function nextMixScale(
  mix: number,
  factor: "halve" | "double"
): number | null {
  const next = scaledMix(mix, factor)
  return Math.abs(next - mix) < 0.001 ? null : next
}
