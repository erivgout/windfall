import { MAX_GAIN } from "@/lib/units"

export function scaledSendGain(
  gain: number,
  factor: "half" | "double"
): number {
  return factor === "half" ? gain / 2 : Math.min(gain * 2, MAX_GAIN)
}

export function nextSendGainScale(
  gain: number,
  factor: "half" | "double"
): number | null {
  const next = scaledSendGain(gain, factor)
  return Math.abs(next - gain) < 0.001 ? null : next
}
