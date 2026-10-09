export function scaledDialogMonitor(gain: number, factor: "half" | "double"): number {
  return factor === "half" ? gain / 2 : Math.min(gain * 2, 1)
}

export function nextDialogMonitorScale(gain: number, factor: "half" | "double"): number | null {
  const next = scaledDialogMonitor(gain, factor)
  return Math.abs(next - gain) < 0.001 ? null : next
}
