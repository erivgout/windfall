export function scaledProjectSwing(
  swing: number,
  factor: "halve" | "double"
): number {
  return factor === "halve" ? swing / 2 : Math.min(swing * 2, 1)
}

export function nextProjectSwingScale(
  swing: number,
  factor: "halve" | "double"
): number | null {
  const next = scaledProjectSwing(swing, factor)
  return Math.abs(next - swing) < 0.001 ? null : next
}
